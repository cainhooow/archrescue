//! Windows PE executor for host-attested, empty VirtualBox GPT disks.

use std::{
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    ptr::null_mut,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

use archrescue_backup::transaction::verify_payload;
use archrescue_core::{Error, ErrorKind, Result};
use archrescue_storage::{inventory, safe_existing, windows::mounted_partition, Disk, Partition};
use serde::Serialize;
use windows_sys::Win32::{
    Foundation::ERROR_SUCCESS,
    Storage::FileSystem::GetLogicalDrives,
    System::{
        Registry::{RegCloseKey, RegOpenKeyExW, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE},
        SystemInformation::GetSystemFirmwareTable,
    },
};

use super::{authenticate, permission, Attachment, Layout, Registration, RestoreBinding, MIB};
use crate::{
    media::{check_cancel, inspect_open, system_directory},
    plan::{build, observe_target, optical_media, select_disk},
    Media, Plan,
};

const TRUST_KEY: Option<&str> = option_env!("ARCHRESCUE_LAB_PUBLIC_KEY");
const CREATE_NO_WINDOW: u32 = 0x08000000;
const ESP: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
const MSR: &str = "e3c9e316-0b5c-4db8-817d-f92df00215ae";
const BASIC_DATA: &str = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7";
const ESP_ROOT: &str = "S:\\";
const WINDOWS_ROOT: &str = "W:\\";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Preflight,
    Partition,
    ApplyImage,
    ConfigureBoot,
    Verify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Running,
    Blocked,
    Failed,
    Cancelled,
    AwaitingFirstBoot,
}

#[derive(Debug, Serialize)]
pub struct CommandEvidence {
    pub executable: PathBuf,
    pub version: String,
    pub arguments: Vec<String>,
    pub started_unix: u64,
    pub finished_unix: Option<u64>,
    pub exit_code: Option<i32>,
    pub stdout_file: String,
    pub stderr_file: String,
}

#[derive(Debug, Serialize)]
pub struct DeploymentExecution {
    pub schema_version: u32,
    pub operation_id: String,
    pub plan_fingerprint: String,
    pub media_identity: Media,
    pub selected_image: crate::Image,
    pub target_disk_identity: Disk,
    pub disposable_lab_registration: Registration,
    pub preflight_result: Option<GuestObservation>,
    pub execution_state: ExecutionState,
    pub started_step: Step,
    pub completed_steps: Vec<Step>,
    pub failed_step: Option<Step>,
    pub observed_target_state: Option<Disk>,
    pub output_evidence: Vec<CommandEvidence>,
    pub verified_paths: Vec<String>,
    pub restore: RestoreBinding,
    pub errors: Vec<Error>,
    pub manual_intervention_required: bool,
}

#[derive(Debug, Serialize)]
pub struct GuestObservation {
    pub vm_uuid: String,
    pub target: Disk,
    pub attachment: Attachment,
}

pub struct Evidence {
    pub path: PathBuf,
    pub execution: DeploymentExecution,
}

pub fn capability_available() -> bool {
    TRUST_KEY.is_some_and(|key| super::decode_hex::<32>(key).is_ok())
}

pub fn read_registration(registration: &Path, signature: &Path) -> Result<Registration> {
    let key = TRUST_KEY.ok_or_else(|| permission("This QA executable has no host public key"))?;
    let payload = read_bounded(registration, 16 * 1024)?;
    let signature = read_bounded(signature, 256)?;
    let signature = std::str::from_utf8(&signature)
        .map_err(|_| permission("Invalid laboratory signature text"))?;

    authenticate(&payload, signature, key)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let path = safe_existing(path)?;
    let file = File::open(path).map_err(|error| Error::io("Open laboratory evidence", error))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::io("Read laboratory evidence", error))?;
    if bytes.len() as u64 > limit {
        return Err(permission("Laboratory evidence exceeds its size limit"));
    }

    Ok(bytes)
}

fn unix_time() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| {
            Error::new(
                ErrorKind::Environment,
                "Guest clock is before the Unix epoch",
            )
        })
}

fn require_winpe() -> Result<()> {
    let key: Vec<_> = OsStr::new(r"SYSTEM\CurrentControlSet\Control\MiniNT")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut handle = null_mut();
    // SAFETY: the registry path is NUL terminated and handle points to writable storage.
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut handle,
        )
    };
    if result != ERROR_SUCCESS {
        return Err(permission("Deployment requires the Windows PE runtime"));
    }

    // SAFETY: RegOpenKeyExW returned a valid owned registry handle.
    unsafe { RegCloseKey(handle) };

    Ok(())
}

fn smbios_uuid() -> Result<String> {
    let signature = u32::from_be_bytes(*b"RSMB");
    // SAFETY: a null output buffer with size zero queries the required byte count.
    let size = unsafe { GetSystemFirmwareTable(signature, 0, null_mut(), 0) };
    if !(8..=1024 * 1024).contains(&size) {
        return Err(permission("Guest SMBIOS size is missing or invalid"));
    }

    let mut data = vec![0; size as usize];
    // SAFETY: data is writable for the supplied byte count.
    if unsafe { GetSystemFirmwareTable(signature, 0, data.as_mut_ptr(), size) } != size {
        return Err(permission("Could not read the guest SMBIOS"));
    }
    if data[1] < 2 {
        return Err(permission("Unsupported SMBIOS version"));
    }

    let length = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let mut records = data
        .get(8..)
        .and_then(|bytes| bytes.get(..length))
        .ok_or_else(|| permission("Truncated guest SMBIOS"))?;
    while records.len() >= 4 {
        let end = usize::from(records[1]);
        let Some(record) = records.get(..end).filter(|_| end >= 4) else {
            break;
        };
        if record[0] == 1 && record.len() >= 24 {
            let uuid = &record[8..24];
            if uuid == [0; 16] || uuid == [255; 16] {
                break;
            }

            // VirtualBox publishes its hardware UUID using the Windows GUID byte order.
            let first = u32::from_le_bytes(uuid[0..4].try_into().unwrap());
            let second = u16::from_le_bytes(uuid[4..6].try_into().unwrap());
            let third = u16::from_le_bytes(uuid[6..8].try_into().unwrap());
            return Ok(format!(
                "{first:08x}-{second:04x}-{third:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                uuid[8], uuid[9], uuid[10], uuid[11], uuid[12], uuid[13], uuid[14], uuid[15],
            ));
        }
        let Some(terminator) = records[end..].windows(2).position(|pair| pair == [0, 0]) else {
            break;
        };
        records = &records[end + terminator + 2..];
    }

    Err(permission("Guest SMBIOS UUID unavailable"))
}

pub fn observe_target_identity(stable_id: &str) -> Result<GuestObservation> {
    require_winpe()?;
    let vm_uuid = smbios_uuid()?;
    let observed = inventory();
    if !observed.errors.is_empty() {
        return Err(permission("Incomplete guest storage inventory"));
    }
    let target = select_disk(&observed, stable_id)?;
    let attachment = archrescue_storage::windows::disk_attachment(&target.device_path)?;

    Ok(GuestObservation {
        vm_uuid,
        target,
        attachment: Attachment {
            serial: attachment.serial,
            product: attachment.product,
            bus: attachment.bus,
            logical_sector: attachment.logical_sector,
            physical_sector: attachment.physical_sector,
        },
    })
}

pub struct Request<'a> {
    pub plan: &'a Plan,
    pub registration: &'a Path,
    pub signature: &'a Path,
    pub confirmation: &'a str,
    pub backup: &'a Path,
    pub evidence: &'a Path,
}

pub fn execute(
    request: &Request<'_>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(Step),
) -> Result<Evidence> {
    let registration = read_registration(request.registration, request.signature)?;
    let layout = registration.validate(request.plan, request.confirmation, unix_time()?)?;

    require_winpe()?;
    let evidence = safe_existing(request.evidence)?;
    let evidence_mount = mounted_partition(&evidence)?;
    if !evidence.is_dir()
        || !evidence_mount
            .disk_id
            .eq_ignore_ascii_case(&registration.evidence_gpt_id)
    {
        return Err(permission(
            "Evidence must be written to the separately registered GPT disk",
        ));
    }

    let root = evidence_mount.mount_path.join(format!(
        "archrescue-deployment-{}",
        registration.operation_id
    ));
    fs::create_dir(&root)
        .map_err(|error| Error::io("Reserve single-use deployment journal", error))?;
    let mut journal = Journal::new(root, request.plan, registration);
    journal.checkpoint()?;

    progress(Step::Preflight);
    let result = perform(request, layout, cancel, &mut journal, &mut progress);
    if let Err(error) = result {
        let failed_step = journal.execution.started_step;
        journal.execution.failed_step = Some(failed_step);
        journal.execution.execution_state = if error.kind == ErrorKind::Cancelled {
            ExecutionState::Cancelled
        } else if failed_step == Step::Preflight {
            ExecutionState::Blocked
        } else {
            ExecutionState::Failed
        };
        journal.execution.errors.push(error);
        journal.execution.observed_target_state = inventory().disks.into_iter().find(|disk| {
            disk.identity
                .eq_ignore_ascii_case(&journal.execution.target_disk_identity.identity)
        });
    } else {
        journal.execution.execution_state = ExecutionState::AwaitingFirstBoot;
        journal.execution.manual_intervention_required = false;
    }
    let path = journal.checkpoint()?;

    Ok(Evidence {
        path,
        execution: journal.execution,
    })
}

fn require_empty_target(registration: &Registration, plan: &Plan) -> Result<GuestObservation> {
    let observed = observe_target_identity(&registration.target_gpt_id)?;
    registration.verify_attachment(&observed.vm_uuid, &observed.attachment)?;
    registration.verify_empty_disk(&observed.target)?;
    if observed.target.number != plan.target.disk.number
        || observed.target.device_path != plan.target.disk.device_path
        || observed.target.bus != plan.target.disk.bus
    {
        return Err(permission(
            "Guest disk attachment changed after plan review",
        ));
    }

    let current = observe_target(
        &registration.target_gpt_id,
        &plan.media,
        None,
        &AtomicBool::new(false),
    )?;
    if !current.volumes.is_empty() || current.firmware != "UEFI" || current.writable != Some(true) {
        return Err(permission(
            "Target no longer qualifies as empty writable UEFI/GPT storage",
        ));
    }
    registration.verify_empty_disk(&current.disk)?;
    if current.disk.number != observed.target.number
        || current.disk.device_path != observed.target.device_path
    {
        return Err(permission(
            "Target mapping changed between read-only observations",
        ));
    }

    Ok(observed)
}

fn verify_protected_disks(
    registration: &Registration,
    backup: &Path,
    evidence: &Path,
) -> Result<()> {
    let observed = inventory();
    if !observed.errors.is_empty() {
        return Err(permission(
            "Incomplete storage inventory for protected disks",
        ));
    }

    let source = select_disk(&observed, &registration.source_gpt_id)?;
    let backup_disk = select_disk(&observed, &registration.backup_gpt_id)?;
    let evidence_disk = select_disk(&observed, &registration.evidence_gpt_id)?;
    if source.read_only != Some(true)
        || backup_disk.read_only != Some(true)
        || !source.errors.is_empty()
        || !backup_disk.errors.is_empty()
        || !evidence_disk.errors.is_empty()
        || !mounted_partition(backup)?
            .disk_id
            .eq_ignore_ascii_case(&registration.backup_gpt_id)
        || !mounted_partition(evidence)?
            .disk_id
            .eq_ignore_ascii_case(&registration.evidence_gpt_id)
    {
        return Err(permission(
            "Source and backup must be online and write-protected on their registered disks",
        ));
    }

    Ok(())
}

fn require_free_letters() -> Result<()> {
    // SAFETY: GetLogicalDrives has no pointer arguments and returns a drive-letter bitmap.
    let drives = unsafe { GetLogicalDrives() };
    let reserved = (1 << (b'S' - b'A')) | (1 << (b'W' - b'A'));
    if drives == 0 || drives & reserved != 0 {
        return Err(permission(
            "The dedicated WinPE S: and W: letters must be free",
        ));
    }

    Ok(())
}

struct Tools {
    diskpart: PathBuf,
    dism: PathBuf,
    bcdboot: PathBuf,
    bcdedit: PathBuf,
}

impl Tools {
    fn load() -> Result<Self> {
        let directory = system_directory()?;
        let tools = Self {
            diskpart: directory.join("diskpart.exe"),
            dism: directory.join("dism.exe"),
            bcdboot: directory.join("bcdboot.exe"),
            bcdedit: directory.join("bcdedit.exe"),
        };
        for path in [&tools.diskpart, &tools.dism, &tools.bcdboot, &tools.bcdedit] {
            if archrescue_windows_detection::pe_file_version(path).is_none() {
                return Err(Error::new(
                    ErrorKind::Environment,
                    format!("Servicing tool unavailable: {}", path.display()),
                ));
            }
        }

        Ok(tools)
    }
}

fn partition_script(disk_number: u32, layout: &Layout) -> String {
    format!(
        "select disk {disk_number}\r\ncreate partition efi size={} offset=1024\r\nformat quick fs=fat32 label=SYSTEM\r\nassign letter=S\r\ncreate partition msr size={} offset={}\r\ncreate partition primary offset={}\r\nformat quick fs=ntfs label=Windows\r\nassign letter=W\r\nexit\r\n",
        layout.efi_mib,
        layout.msr_mib,
        (1 + layout.efi_mib) * 1024,
        layout.windows_offset_mib * 1024,
    )
}

fn perform(
    request: &Request<'_>,
    layout: &Layout,
    cancel: &AtomicBool,
    journal: &mut Journal,
    progress: &mut impl FnMut(Step),
) -> Result<()> {
    let registration = journal.execution.disposable_lab_registration.clone();
    let tools = Tools::load()?;
    let backup = safe_existing(request.backup)?;
    verify_protected_disks(&registration, &backup, &journal.root)?;
    let manifest = verify_payload(&backup, cancel)?;
    let source_prefix = format!("{};", registration.source_gpt_id.to_ascii_lowercase());
    if !manifest
        .source_identity
        .filesystem_id
        .to_ascii_lowercase()
        .starts_with(&source_prefix)
        || manifest.operation_id != registration.backup_operation
        || manifest.plan_fingerprint != registration.backup_manifest_sha256
    {
        return Err(permission(
            "Verified backup differs from the host-bound source or manifest",
        ));
    }

    let (media, media_lock) = inspect_open(
        &request.plan.media.path,
        Some(&request.plan.media.sha256),
        cancel,
        |_, _| {},
    )?;
    if !optical_media(&media.path) || media != request.plan.media {
        return Err(permission(
            "The WIM must match the reviewed image on mounted optical media",
        ));
    }

    let current_target = observe_target(&registration.target_gpt_id, &media, None, cancel)?;
    let current_plan = build(
        media,
        current_target,
        request.plan.selected_image.index,
        request.plan.intent,
    )?;
    if current_plan.fingerprint != request.plan.fingerprint {
        return Err(permission(
            "The reviewed plan changed during independent preflight",
        ));
    }
    let initial_target = require_empty_target(&registration, request.plan)?;
    let number = initial_target
        .target
        .number
        .ok_or_else(|| permission("Target disk number is missing"))?;
    journal.execution.preflight_result = Some(initial_target);
    journal.write_bytes("partition.txt", partition_script(number, layout).as_bytes())?;
    let script = journal.root.join("partition.txt");
    let _script_lock = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&script)
        .map_err(|error| Error::io("Pin generated partition script", error))?;

    let evidence_root = journal.root.clone();
    journal.complete(Step::Preflight)?;
    journal.start(Step::Partition)?;
    progress(Step::Partition);
    journal.command(
        &tools.diskpart,
        &["/s".into(), script.into_os_string()],
        || {
            check_cancel(cancel)?;
            registration.validate(request.plan, request.confirmation, unix_time()?)?;
            verify_protected_disks(&registration, &backup, &evidence_root)?;
            require_free_letters()?;
            require_empty_target(&registration, request.plan)?;

            Ok(())
        },
    )?;

    let prepared = verify_layout(&registration, layout)?;
    journal.execution.observed_target_state = Some(prepared.clone());
    journal.complete(Step::Partition)?;

    journal.start(Step::ApplyImage)?;
    progress(Step::ApplyImage);
    let mut image_argument = OsString::from("/ImageFile:");
    image_argument.push(crate::plan::dos_path(&request.plan.media.path));
    journal.command(
        &tools.dism,
        &[
            "/English".into(),
            "/Apply-Image".into(),
            image_argument,
            format!("/Index:{}", request.plan.selected_image.index).into(),
            "/ApplyDir:W:\\".into(),
            "/Verify".into(),
        ],
        || {
            check_cancel(cancel)?;
            verify_unchanged_layout(&registration, layout, &prepared)
        },
    )?;
    drop(media_lock);
    verify_unchanged_layout(&registration, layout, &prepared)?;
    verify_windows(journal)?;
    journal.complete(Step::ApplyImage)?;

    journal.start(Step::ConfigureBoot)?;
    progress(Step::ConfigureBoot);
    journal.command(
        &tools.bcdboot,
        &[
            "W:\\Windows".into(),
            "/s".into(),
            "S:".into(),
            "/f".into(),
            "UEFI".into(),
        ],
        || {
            check_cancel(cancel)?;
            verify_unchanged_layout(&registration, layout, &prepared)
        },
    )?;
    verify_unchanged_layout(&registration, layout, &prepared)?;
    verify_boot(journal)?;
    journal.complete(Step::ConfigureBoot)?;

    journal.start(Step::Verify)?;
    progress(Step::Verify);
    journal.command(
        &tools.bcdedit,
        &[
            "/store".into(),
            "S:\\EFI\\Microsoft\\Boot\\BCD".into(),
            "/enum".into(),
            "all".into(),
        ],
        || verify_unchanged_layout(&registration, layout, &prepared),
    )?;
    verify_protected_disks(&registration, &backup, &journal.root)?;
    let manifest = verify_payload(&backup, cancel)?;
    if manifest.plan_fingerprint != registration.backup_manifest_sha256 {
        return Err(permission("Backup manifest changed during deployment"));
    }
    journal.execution.observed_target_state = Some(verify_layout(&registration, layout)?);
    verify_windows(journal)?;
    verify_boot(journal)?;
    journal.complete(Step::Verify)
}

fn verify_layout(registration: &Registration, layout: &Layout) -> Result<Disk> {
    let observed = observe_target_identity(&registration.target_gpt_id)?;
    registration.verify_attachment(&observed.vm_uuid, &observed.attachment)?;
    let mut disk = observed.target;
    disk.partitions.sort_by_key(|partition| partition.offset);
    let [efi, msr, windows] = disk.partitions.as_slice() else {
        return Err(permission(
            "Expected exactly three partitions on the registered GPT disk",
        ));
    };

    let mut ids = [efi.id.as_deref(), msr.id.as_deref(), windows.id.as_deref()];
    ids.sort_unstable();
    let windows_end = windows.offset.checked_add(windows.length);
    if disk.capacity != Some(registration.target_capacity)
        || disk.style.as_deref() != Some("GPT")
        || disk.read_only != Some(false)
        || !disk.errors.is_empty()
        || !ids.iter().all(|id| id.is_some_and(super::is_uuid))
        || ids.windows(2).any(|pair| pair[0] == pair[1])
        || !matches_partition(efi, ESP, MIB, layout.efi_mib * MIB)
        || !matches_partition(msr, MSR, (1 + layout.efi_mib) * MIB, layout.msr_mib * MIB)
        || windows.kind != BASIC_DATA
        || windows.offset != layout.windows_offset_mib * MIB
        || windows.length < layout.minimum_windows_bytes
        || !windows_end.is_some_and(|end| {
            registration
                .target_capacity
                .checked_sub(end)
                .is_some_and(|tail| (33 * 512..=2 * MIB).contains(&tail))
        })
    {
        return Err(permission(
            "Observed GPT layout differs from the reviewed layout",
        ));
    }

    for (path, partition, filesystem) in [(ESP_ROOT, efi, "FAT32"), (WINDOWS_ROOT, windows, "NTFS")]
    {
        let mounted = mounted_partition(Path::new(path))?;
        if !mounted
            .disk_id
            .eq_ignore_ascii_case(&registration.target_gpt_id)
            || Some(mounted.disk_number) != disk.number
            || mounted.partition.id != partition.id
            || mounted.partition.offset != partition.offset
            || !mounted.filesystem.eq_ignore_ascii_case(filesystem)
        {
            return Err(permission(
                "Drive letter is not mapped to the expected GPT partition and filesystem",
            ));
        }
    }

    Ok(disk)
}

fn matches_partition(partition: &Partition, kind: &str, offset: u64, length: u64) -> bool {
    partition.kind == kind && partition.offset == offset && partition.length == length
}

fn verify_unchanged_layout(
    registration: &Registration,
    layout: &Layout,
    prepared: &Disk,
) -> Result<()> {
    let current = verify_layout(registration, layout)?;
    if current.number != prepared.number
        || current.device_path != prepared.device_path
        || !current
            .partitions
            .iter()
            .zip(&prepared.partitions)
            .all(|(left, right)| {
                left.id == right.id
                    && matches_partition(left, &right.kind, right.offset, right.length)
            })
    {
        return Err(permission("Prepared partition identities changed"));
    }

    Ok(())
}

fn verify_windows(journal: &mut Journal) -> Result<()> {
    let root = Path::new(WINDOWS_ROOT);
    let system = root.join("Windows/System32");
    for (relative, magic) in [
        ("config/SYSTEM", b"regf".as_slice()),
        ("config/SOFTWARE", b"regf".as_slice()),
        ("winload.efi", b"MZ".as_slice()),
        ("ntoskrnl.exe", b"MZ".as_slice()),
    ] {
        verify_file(journal, &system.join(relative), magic)?;
    }

    Ok(())
}

fn verify_boot(journal: &mut Journal) -> Result<()> {
    let root = Path::new(ESP_ROOT);
    for (relative, magic) in [
        ("EFI/Microsoft/Boot/bootmgfw.efi", b"MZ".as_slice()),
        ("EFI/Microsoft/Boot/BCD", b"regf".as_slice()),
        ("EFI/Boot/bootx64.efi", b"MZ".as_slice()),
    ] {
        // Some WinPE builds cannot canonicalize nested ESP paths as DOS names.
        // The caller has already corroborated this fresh FAT32 mount with GPT extents.
        let path = root.join(relative);
        verify_file(journal, &path, magic)?;
    }

    Ok(())
}

fn verify_file(journal: &mut Journal, path: &Path, magic: &[u8]) -> Result<()> {
    let mut file =
        File::open(path).map_err(|error| Error::io(path.display().to_string(), error))?;
    let mut probe = [0; 4];
    file.read_exact(&mut probe[..magic.len()])
        .map_err(|error| Error::io(path.display().to_string(), error))?;
    if &probe[..magic.len()] != magic {
        return Err(Error::new(
            ErrorKind::Integrity,
            format!("Unexpected file signature at {}", path.display()),
        ));
    }

    let observed = path.display().to_string();
    if !journal.execution.verified_paths.contains(&observed) {
        journal.execution.verified_paths.push(observed);
    }

    Ok(())
}

struct Journal {
    root: PathBuf,
    sequence: u32,
    execution: DeploymentExecution,
}

impl Journal {
    fn new(root: PathBuf, plan: &Plan, registration: Registration) -> Self {
        let execution = DeploymentExecution {
            schema_version: 1,
            operation_id: registration.operation_id.clone(),
            plan_fingerprint: plan.fingerprint.clone(),
            media_identity: plan.media.clone(),
            selected_image: plan.selected_image.clone(),
            target_disk_identity: plan.target.disk.clone(),
            restore: registration.restore_binding(),
            disposable_lab_registration: registration,
            preflight_result: None,
            execution_state: ExecutionState::Running,
            started_step: Step::Preflight,
            completed_steps: vec![],
            failed_step: None,
            observed_target_state: Some(plan.target.disk.clone()),
            output_evidence: vec![],
            verified_paths: vec![],
            errors: vec![],
            manual_intervention_required: false,
        };

        Self {
            root,
            sequence: 0,
            execution,
        }
    }

    fn start(&mut self, step: Step) -> Result<()> {
        self.execution.started_step = step;
        self.checkpoint()?;

        Ok(())
    }

    fn complete(&mut self, step: Step) -> Result<()> {
        self.execution.completed_steps.push(step);
        self.checkpoint()?;

        Ok(())
    }

    fn checkpoint(&mut self) -> Result<PathBuf> {
        let bytes = serde_json::to_vec_pretty(&self.execution)
            .map_err(|error| Error::new(ErrorKind::Io, error.to_string()))?;
        let name = format!("execution-{:03}.json", self.sequence);
        self.sequence += 1;
        self.write_bytes(&name, &bytes)?;

        Ok(self.root.join(name))
    }

    fn write_bytes(&self, name: &str, bytes: &[u8]) -> Result<()> {
        let path = self.root.join(name);
        let mut file = self.create(name)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| Error::io("Persist deployment evidence", error))?;
        drop(file);
        if fs::read(path).map_err(|error| Error::io("Reread deployment evidence", error))? != bytes
        {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Deployment evidence did not read back",
            ));
        }

        Ok(())
    }

    fn create(&self, name: &str) -> Result<File> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join(name))
            .map_err(|error| Error::io("Create exclusive deployment evidence", error))
    }

    fn command(
        &mut self,
        executable: &Path,
        arguments: &[OsString],
        final_check: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let index = self.execution.output_evidence.len();
        let stdout_file = format!("tool-{index:02}.stdout");
        let stderr_file = format!("tool-{index:02}.stderr");
        let stdout = self.create(&stdout_file)?;
        let stderr = self.create(&stderr_file)?;
        let mut command = Command::new(executable);
        command
            .args(arguments)
            .current_dir(system_directory()?)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(
                stdout
                    .try_clone()
                    .map_err(|error| Error::io("Open tool stdout", error))?,
            )
            .stderr(
                stderr
                    .try_clone()
                    .map_err(|error| Error::io("Open tool stderr", error))?,
            );

        self.execution.output_evidence.push(CommandEvidence {
            executable: executable.into(),
            version: archrescue_windows_detection::pe_file_version(executable)
                .ok_or_else(|| permission("Servicing tool version is unavailable"))?,
            arguments: arguments
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            started_unix: unix_time()?,
            finished_unix: None,
            exit_code: None,
            stdout_file,
            stderr_file,
        });
        self.checkpoint()?;

        final_check()?;
        self.execution.manual_intervention_required = true;
        let status = command
            .status()
            .map_err(|error| Error::io("Run servicing tool", error))?;
        let evidence = &mut self.execution.output_evidence[index];
        evidence.finished_unix = Some(unix_time()?);
        evidence.exit_code = status.code();
        stdout
            .sync_all()
            .and_then(|_| stderr.sync_all())
            .map_err(|error| Error::io("Persist tool output", error))?;
        self.checkpoint()?;
        if !status.success() {
            return Err(Error::new(
                ErrorKind::Incomplete,
                format!("{} returned {status}", executable.display()),
            ));
        }

        Ok(())
    }
}
