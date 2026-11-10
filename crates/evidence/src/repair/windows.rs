use std::{
    path::{Component, Prefix},
    process::Command,
    sync::atomic::AtomicBool,
};

use archrescue_backup::{sha256, transaction};
use archrescue_storage::{
    identity, inventory, require_separate, safe_existing, space, windows::mounted_partition,
};
use windows_sys::Win32::System::SystemInformation::{
    FirmwareTypeUefi, GetFirmwareType, GetSystemWindowsDirectoryW,
};

use super::*;

fn blocked(reason: &str) -> Error {
    Error::new(ErrorKind::Unsupported, reason)
}

fn drive(path: &Path) -> Result<String> {
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return Err(blocked(
            "A ESP exige uma letra de unidade atribuída pelo laboratório",
        ));
    };
    match prefix.kind() {
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => Ok(format!("{}:", letter as char)),
        _ => Err(blocked(
            "A ESP exige uma letra de unidade atribuída pelo laboratório",
        )),
    }
}

fn system_root() -> Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;

    let mut buffer = [0u16; 1024];
    // SAFETY: the system API can write up to the supplied number of UTF-16 units.
    let length = unsafe { GetSystemWindowsDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 || length as usize >= buffer.len() {
        return Err(blocked("Diretório do Windows em execução indisponível"));
    }

    Ok(std::ffi::OsString::from_wide(&buffer[..length as usize]).into())
}

pub(super) fn identify(plan: &mut RepairPlan) -> Result<()> {
    let source_root = safe_existing(&plan.windows_root)?;
    let efi_root = safe_existing(&plan.efi_root)?;
    let destination = safe_existing(&plan.backup_destination)?;
    let source = mounted_partition(&source_root)?;
    let efi = mounted_partition(&efi_root)?;

    if source.mount_path != source_root
        || efi.mount_path != efi_root
        || source.disk_number != efi.disk_number
        || source.disk_id != efi.disk_id
        || source.style != "GPT"
        || source.partition.efi
        || !efi.partition.efi
        || source.filesystem != "NTFS"
        || efi.filesystem != "FAT32"
    {
        return Err(blocked(
            "Exige Windows NTFS e ESP FAT32 distintos no mesmo disco GPT",
        ));
    }

    let inventory = inventory();
    let mut same_id = inventory
        .disks
        .iter()
        .filter(|disk| disk.identity == source.disk_id);
    let disk = same_id
        .next()
        .ok_or_else(|| blocked("Disco alvo não encontrado no inventário"))?;
    if same_id.next().is_some()
        || disk.number != Some(source.disk_number)
        || disk
            .partitions
            .iter()
            .filter(|partition| partition.efi)
            .count()
            != 1
    {
        return Err(blocked("Identidade de disco ou ESP ambígua"));
    }

    require_separate(&identity(&source_root)?, &identity(&destination)?)?;
    if space(&destination)?.available < plan.scratch_estimate_bytes {
        plan.blockers
            .push("Espaço insuficiente para cópia temporária do BCD".into());
    }

    let discovery = archrescue_windows_detection::discover(std::slice::from_ref(&source_root));
    let [installation] = discovery.installations.as_slice() else {
        return Err(blocked(
            "Não há uma instalação Windows offline corroborada na origem",
        ));
    };
    if installation.architecture.as_deref() != Some("x64")
        || installation.loader_architecture.as_deref() != Some("x64")
        || !safe_existing(&installation.path.join("System32/config/BCD-Template"))
            .is_ok_and(|path| path.is_file())
    {
        return Err(blocked(
            "Kernel, carregador x64 ou BCD-Template indisponíveis",
        ));
    }

    let windows_partition_id = source
        .partition
        .id
        .filter(|id| !nil_guid(id))
        .ok_or_else(|| blocked("GUID da partição Windows indisponível"))?;
    let efi_partition_id = efi
        .partition
        .id
        .filter(|id| !nil_guid(id))
        .ok_or_else(|| blocked("GUID da ESP indisponível"))?;
    let target = RepairTarget {
        disk_id: source.disk_id,
        windows_partition_id,
        efi_partition_id,
        windows_volume_guid: source.volume_guid,
        efi_volume_guid: efi.volume_guid,
    };

    plan.command = vec![
        system_root()?
            .join("System32/bcdboot.exe")
            .display()
            .to_string(),
        format!(r"{}\Windows", drive(&source_root)?),
        "/s".into(),
        drive(&efi_root)?,
        "/f".into(),
        "UEFI".into(),
        "/v".into(),
    ];
    let registration = format!(
        "{};{};{}",
        target.disk_id, target.windows_partition_id, target.efi_partition_id
    );
    if option_env!("ARCHRESCUE_LAB_FIXTURE") != Some(registration.as_str()) {
        plan.blockers.push(
            "Identificadores do clone descartável ausentes no registro desta compilação".into(),
        );
    }

    if !is_winpe_uefi()? {
        plan.blockers
            .push("Execução disponível somente no WinPE x64 iniciado via UEFI".into());
    }

    let boot = archrescue_diagnostics::inspect_with_scratch(
        std::slice::from_ref(&efi_root),
        Some(&destination),
    );
    let observed_fault = boot.observations.iter().any(|item| {
        (item.path.ends_with("BCD") && item.path.starts_with(efi_root.join("EFI"))
            || item.path.ends_with("bootmgfw.efi"))
            && (item.status == "missing"
                || item
                    .error
                    .as_ref()
                    .is_some_and(|error| error.kind == ErrorKind::Validation))
    });
    if !observed_fault {
        plan.blockers
            .push("Cenário exige BCD ou gerenciador de boot ausente ou inválido na ESP".into());
    }

    plan.target = Some(target);
    plan.windows_root = source_root;
    plan.efi_root = efi_root;
    plan.backup_destination = destination;

    Ok(())
}

fn nil_guid(id: &str) -> bool {
    id == "00000000-0000-0000-0000-000000000000"
}

fn is_winpe_uefi() -> Result<bool> {
    let root = system_root()?;
    let mut firmware = 0;
    // SAFETY: firmware is writable for the duration of the call.
    let uefi = unsafe { GetFirmwareType(&mut firmware) } != 0 && firmware == FirmwareTypeUefi;

    Ok(uefi && drive(&root)? == "X:" && root.join("System32/wpeinit.exe").is_file())
}

fn verified_checkpoint(
    root: &Path,
    path: &Path,
    destination: &Path,
) -> Result<transaction::TransactionReport> {
    let path = safe_existing(path)?;
    if path.parent() != Some(destination) {
        return Err(blocked(
            "Checkpoint não pertence ao destino independente do plano",
        ));
    }

    transaction::verify(root, &path, &AtomicBool::new(false))
}

pub(super) fn execute(requested: &RepairPlan, authorization: &str) -> Result<RepairAttempt> {
    let fresh = super::plan(
        &requested.windows_root,
        &requested.efi_root,
        &requested.backup_destination,
        requested.data_checkpoint.as_deref(),
        requested.efi_checkpoint.as_deref(),
    )?;
    if !fresh.blockers.is_empty()
        || fresh.id != requested.id
        || fresh.target != requested.target
        || fresh.command != requested.command
    {
        return Err(blocked(
            "Identidade, comando ou pré-condições mudaram desde o plano",
        ));
    }

    let data = fresh
        .data_checkpoint
        .as_deref()
        .ok_or_else(|| blocked("Backup dos dados ausente"))?;
    let efi = fresh
        .efi_checkpoint
        .as_deref()
        .ok_or_else(|| blocked("Backup EFI ausente"))?;
    let data_report = verified_checkpoint(&fresh.windows_root, data, &fresh.backup_destination)?;
    let efi_report = verified_checkpoint(&fresh.efi_root, efi, &fresh.backup_destination)?;
    if data_report.manifest.files.is_empty()
        || efi_report.manifest.selections.as_slice() != [PathBuf::from("EFI")]
        || efi_report.manifest.files.is_empty()
    {
        return Err(blocked(
            "Preservação exige dados selecionados e toda a árvore EFI",
        ));
    }

    for root in [&fresh.windows_root, &fresh.efi_root] {
        archrescue_storage::windows::require_clean_volume(root)?;
    }
    let rechecked = super::plan(
        &fresh.windows_root,
        &fresh.efi_root,
        &fresh.backup_destination,
        Some(data),
        Some(efi),
    )?;
    if rechecked.id != fresh.id || !rechecked.blockers.is_empty() {
        return Err(blocked("Validação final do alvo divergiu do plano"));
    }

    let output = Command::new(&fresh.command[0])
        .args(&fresh.command[1..])
        .output();
    let mut attempt = RepairAttempt {
        plan: fresh.clone(),
        authorization: authorization.into(),
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        post_repair: Default::default(),
        backup_rechecked: false,
        postconditions_met: false,
        boot_observed: false,
        errors: vec![],
    };

    match output {
        Ok(output) => {
            attempt.exit_code = output.status.code();
            attempt.stdout = String::from_utf8_lossy(&output.stdout).into();
            attempt.stderr = String::from_utf8_lossy(&output.stderr).into();
            if !output.status.success() {
                attempt.errors.push(Error::new(
                    ErrorKind::Incomplete,
                    format!("BCDBoot: {}", output.status),
                ));
            }
        }
        Err(error) => attempt.errors.push(Error::io("Executar BCDBoot", error)),
    }

    attempt.post_repair = archrescue_diagnostics::inspect_with_scratch(
        std::slice::from_ref(&fresh.efi_root),
        Some(&fresh.backup_destination),
    );
    let verified = verify_post_repair(&attempt, &data_report, &efi_report);
    match verified {
        Ok(()) => {
            attempt.backup_rechecked = true;
            attempt.postconditions_met = attempt.errors.is_empty();
        }
        Err(error) => attempt.errors.push(error),
    }

    Ok(attempt)
}

fn verify_post_repair(
    attempt: &RepairAttempt,
    data: &transaction::TransactionReport,
    efi: &transaction::TransactionReport,
) -> Result<()> {
    let target = attempt.plan.target.as_ref().expect("validated target");
    let windows = mounted_partition(&attempt.plan.windows_root)?;
    let esp = mounted_partition(&attempt.plan.efi_root)?;
    if target.windows_volume_guid != windows.volume_guid
        || target.efi_volume_guid != esp.volume_guid
        || target.disk_id != windows.disk_id
        || target.disk_id != esp.disk_id
        || windows.partition.id.as_deref() != Some(&target.windows_partition_id)
        || esp.partition.id.as_deref() != Some(&target.efi_partition_id)
    {
        return Err(blocked("Identidade do alvo mudou após BCDBoot"));
    }

    let cancel = AtomicBool::new(false);
    transaction::verify(&attempt.plan.windows_root, &data.directory, &cancel)?;
    for file in &efi.manifest.files {
        if sha256(&efi.directory.join("payload").join(&file.relative), &cancel)? != file.sha256 {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Backup independente da ESP mudou",
            ));
        }
    }

    let windows_drive = drive(&attempt.plan.windows_root)?.to_ascii_lowercase();
    let windows_reference = format!("partition={windows_drive}");
    let boot = &attempt.post_repair.observations;
    let manager = boot
        .iter()
        .any(|item| item.path.ends_with("bootmgfw.efi") && item.status == "readable");
    let fallback = boot
        .iter()
        .any(|item| item.path.ends_with("bootx64.efi") && item.status == "readable");
    let store = boot
        .iter()
        .find(|item| item.path == attempt.plan.efi_root.join("EFI/Microsoft/Boot/BCD"));
    let referenced = store.is_some_and(|item| {
        item.status == "readable"
            && item.detail.contains("from an isolated copy")
            && loader_references(&item.detail, &windows_reference)
    });

    if !manager || !fallback || !referenced {
        return Err(Error::new(
            ErrorKind::Incomplete,
            "Arquivos UEFI ou referência do carregador BCD ainda indisponíveis",
        ));
    }

    Ok(())
}

fn loader_references(enumeration: &str, partition: &str) -> bool {
    let text = enumeration.to_ascii_lowercase().replace("\r\n", "\n");
    text.split("\n\n").any(|section| {
        let fields: Vec<_> = section
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                Some((parts.next()?, parts.next()?))
            })
            .collect();
        fields.contains(&("path", r"\windows\system32\winload.efi"))
            && fields.contains(&("device", partition))
            && fields.contains(&("osdevice", partition))
    })
}

#[cfg(test)]
mod tests {
    use super::loader_references;

    #[test]
    fn loader_and_both_devices_must_be_in_one_bcd_entry() {
        let good =
            "device partition=C:\r\npath \\Windows\\system32\\winload.efi\r\nosdevice partition=C:";
        assert!(loader_references(good, "partition=c:"));
        assert!(!loader_references(good, "partition=d:"));
        assert!(!loader_references(
            &good.replace("\r\nosdevice", "\r\n\r\nosdevice"),
            "partition=c:"
        ));
    }
}
