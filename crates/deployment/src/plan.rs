//! Evidence and authorization for a read-only deployment preview.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use archrescue_core::{Error, ErrorKind, Result};
use archrescue_storage::{safe_existing, Disk, Partition, Volume};
use archrescue_windows_detection::Discovery;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{inspect, Image, Media};

const SYSTEM_RESERVE: u64 = 8 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    Inspection,
    Repair,
    KeepData,
    Clean,
}

impl Intent {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "inspecao" => Ok(Self::Inspection),
            "reparo" => Ok(Self::Repair),
            "preservar" => Ok(Self::KeepData),
            "limpa" => Ok(Self::Clean),
            _ => Err(Error::new(
                ErrorKind::Validation,
                "Modo de implantação inválido",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Qualification {
    Supported,
    RequiresIntervention,
    Unsupported,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub result: Qualification,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preservation {
    pub directory: PathBuf,
    pub backup_operation: String,
    pub manifest_sha256: String,
    pub verified_files: usize,
    pub covers_entire_disk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub disk: Disk,
    pub volumes: Vec<Volume>,
    pub discovery: Discovery,
    pub firmware: String,
    pub storage_health: String,
    pub writable: Option<bool>,
    pub encryption: String,
    pub preservation: Option<Preservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub schema_version: u32,
    pub media: Media,
    pub selected_image: Image,
    pub target: Target,
    pub intent: Intent,
    pub result: Qualification,
    pub findings: Vec<Finding>,
    pub next_action: String,
    pub minimum_bytes: Option<u64>,
    pub intended_partitions: Vec<String>,
    pub observed_efi_partitions: Vec<Partition>,
    pub existing_data_at_risk: bool,
    pub planned_modifications: Vec<String>,
    pub expected_final_state: String,
    pub authorization_required: String,
    pub fingerprint: String,
}

fn select_disk(inventory: &archrescue_storage::Inventory, stable_id: &str) -> Result<Disk> {
    let matches: Vec<_> = inventory
        .disks
        .iter()
        .filter(|disk| disk.identity.eq_ignore_ascii_case(stable_id))
        .collect();

    let [disk] = matches.as_slice() else {
        return Err(Error::new(
            ErrorKind::Validation,
            "Identidade do disco ausente, alterada ou ambígua",
        ));
    };

    if disk.number.is_none()
        || !disk.identity.starts_with("gpt:")
        || disk.identity == "gpt:00000000-0000-0000-0000-000000000000"
        || inventory
            .disks
            .iter()
            .filter(|other| other.number == disk.number)
            .count()
            != 1
    {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Selecione um disco com identidade GPT única",
        ));
    }

    Ok((*disk).clone())
}

#[cfg(windows)]
fn observe_target(
    stable_id: &str,
    media: &Media,
    backup: Option<&Path>,
    cancel: &AtomicBool,
) -> Result<Target> {
    use archrescue_storage::{identity, inventory, windows};
    use windows_sys::Win32::System::SystemInformation::{
        FirmwareTypeBios, FirmwareTypeUefi, GetFirmwareType,
    };

    let inventory = inventory();
    if !inventory.errors.is_empty() {
        return Err(Error::new(
            ErrorKind::Validation,
            "Inventário de armazenamento incompleto",
        ));
    }
    let disk = select_disk(&inventory, stable_id)?;
    let number = disk.number.unwrap();
    let mut volumes: Vec<_> = inventory
        .volumes
        .into_iter()
        .filter(|volume| volume.physical_disks.contains(&number))
        .collect();
    volumes.sort_by(|left, right| left.identity.cmp(&right.identity));

    let separate_media = match identity(&media.path) {
        Ok(source) if !source.physical_disks.is_empty() => !source.physical_disks.contains(&number),
        _ => optical_media(&media.path),
    };
    if !separate_media {
        return Err(Error::new(
            ErrorKind::Validation,
            "A mídia de instalação deve estar em armazenamento independente do disco alvo",
        ));
    }

    let roots: Vec<_> = volumes
        .iter()
        .flat_map(|volume| volume.paths.iter().cloned())
        .collect();
    let discovery = archrescue_windows_detection::discover(&roots);
    let storage_health = match windows::disk_health(&disk.device_path) {
        windows::DiskHealth::FailurePredicted => "Falha prevista",
        windows::DiskHealth::NoFailurePredicted => "Sem falha prevista pelo dispositivo",
        windows::DiskHealth::Unknown => "Desconhecida",
    };
    let writable = match windows::require_writable_disk(&disk.device_path) {
        Ok(()) => Some(true),
        Err(error) if error.os_code == Some(19) => Some(false), // ERROR_WRITE_PROTECT
        Err(_) => None,
    };
    let mut firmware_type = 0;
    // SAFETY: firmware_type points to writable storage for the firmware enumeration.
    let firmware = match unsafe { GetFirmwareType(&mut firmware_type) } {
        0 => "Desconhecido",
        _ if firmware_type == FirmwareTypeUefi => "UEFI",
        _ if firmware_type == FirmwareTypeBios => "BIOS",
        _ => "Desconhecido",
    };

    let preservation = backup
        .map(|directory| -> Result<Preservation> {
            let directory = safe_existing(directory)?;
            let volume = identity(&directory)?;
            if volume.physical_disks.is_empty() || volume.physical_disks.contains(&number) {
                return Err(Error::new(
                    ErrorKind::Validation,
                    "Backup em disco não comprovadamente independente",
                ));
            }
            let manifest = archrescue_backup::transaction::verify_payload(&directory, cancel)?;
            if !manifest
                .source_identity
                .filesystem_id
                .starts_with(&format!("{};", disk.identity))
            {
                return Err(Error::new(
                    ErrorKind::Validation,
                    "O manifesto não identifica o disco selecionado como origem",
                ));
            }

            Ok(Preservation {
                directory,
                backup_operation: manifest.operation_id,
                manifest_sha256: manifest.plan_fingerprint,
                verified_files: manifest.files.len(),
                covers_entire_disk: false,
            })
        })
        .transpose()?;

    let encryption = if disk.partitions.is_empty() && volumes.is_empty() {
        "Sem partições observadas"
    } else if volumes.iter().any(|volume| volume.filesystem.is_none()) {
        "Não resolvida. Há volume sem sistema de arquivos acessível"
    } else {
        "Não resolvida. Acesso ao volume não comprova estado BitLocker"
    };

    Ok(Target {
        disk,
        volumes,
        discovery,
        firmware: firmware.into(),
        storage_health: storage_health.into(),
        writable,
        encryption: encryption.into(),
        preservation,
    })
}

#[cfg(windows)]
fn optical_media(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;

    let root = path.ancestors().last().unwrap_or(path);
    let wide: Vec<_> = root.as_os_str().encode_wide().chain(Some(0)).collect();

    // SAFETY: root is a terminated UTF-16 string. DRIVE_CDROM is 5.
    unsafe { GetDriveTypeW(wide.as_ptr()) == 5 }
}

#[cfg(not(windows))]
fn observe_target(stable_id: &str, _: &Media, _: Option<&Path>, _: &AtomicBool) -> Result<Target> {
    select_disk(&archrescue_storage::inventory(), stable_id)?;
    Err(Error::new(
        ErrorKind::Unsupported,
        "Inventário de destino requer Windows ou WinPE",
    ))
}

pub fn create_plan(
    media_path: &Path,
    media_hash: &str,
    image_index: u32,
    stable_disk_id: &str,
    intent: Intent,
    backup: Option<&Path>,
    cancel: &AtomicBool,
) -> Result<Plan> {
    let media = inspect(media_path, Some(media_hash), cancel, |_, _| {})?;
    let target = observe_target(stable_disk_id, &media, backup, cancel)?;

    build(media, target, image_index, intent)
}

fn build(media: Media, target: Target, image_index: u32, intent: Intent) -> Result<Plan> {
    let selected_image = media
        .images
        .iter()
        .find(|image| image.index == image_index)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Validation,
                "Índice ausente na imagem selecionada",
            )
        })?;
    let minimum_bytes = selected_image
        .expanded_bytes
        .filter(|size| *size > 0)
        .and_then(|size| size.checked_add(SYSTEM_RESERVE))
        .map(|bytes| bytes.max(64 * 1024 * 1024 * 1024));
    let mut findings = Vec::new();
    let mut add = |result, reason: &str| {
        findings.push(Finding {
            result,
            reason: reason.into(),
        })
    };
    let populated = !target.disk.partitions.is_empty() || !target.volumes.is_empty();

    if selected_image.architecture.as_deref() != Some("x64") {
        add(
            Qualification::Unsupported,
            "A implantação planejada exige imagem Windows x64",
        );
    }
    if selected_image.version.is_none() || minimum_bytes.is_none() {
        add(
            Qualification::Blocked,
            "Versão ou tamanho expandido indisponível",
        );
    }
    if !matches!(
        selected_image.installation.as_deref(),
        Some("Client" | "Server" | "Server Core")
    ) {
        add(
            Qualification::Unsupported,
            "A imagem não identifica uma instalação Windows suportada",
        );
    }
    if target.firmware != "UEFI" {
        add(
            Qualification::Unsupported,
            "O perfil atual de implantação exige firmware UEFI",
        );
    }
    if target.disk.style.as_deref() != Some("GPT") || target.disk.removable != Some(false) {
        add(
            Qualification::Unsupported,
            "O perfil atual exige disco fixo GPT",
        );
    }
    if !target.disk.errors.is_empty() || target.writable != Some(true) {
        add(
            Qualification::Blocked,
            "Consultas do disco incompletas ou disco protegido contra gravação",
        );
    }
    if !minimum_bytes
        .zip(target.disk.capacity)
        .is_some_and(|(minimum, capacity)| capacity >= minimum)
    {
        add(
            Qualification::Blocked,
            "Capacidade insuficiente ou desconhecida",
        );
    }
    if target.storage_health == "Falha prevista" {
        add(
            Qualification::Blocked,
            "O dispositivo prevê falha de armazenamento",
        );
    }
    if target.storage_health == "Desconhecida" {
        add(
            Qualification::Supported,
            "Estado de saúde indisponível no dispositivo",
        );
    }
    if matches!(intent, Intent::Repair | Intent::KeepData) {
        add(
            Qualification::Unsupported,
            "O modo selecionado exige um fluxo de implantação ainda indisponível",
        );
    }
    if populated && intent != Intent::Inspection {
        add(Qualification::RequiresIntervention, "Resolva criptografia e cobertura de preservação de todas as partições. Backup seletivo não cobre o disco inteiro");
    }
    if target
        .volumes
        .iter()
        .any(|volume| !volume.errors.is_empty() || volume.physical_disks.len() != 1)
    {
        add(
            Qualification::Blocked,
            "Há volumes inacessíveis ou com múltiplos discos",
        );
    }

    let result = [
        Qualification::Blocked,
        Qualification::Unsupported,
        Qualification::RequiresIntervention,
    ]
    .into_iter()
    .find(|result| findings.iter().any(|finding| finding.result == *result))
    .unwrap_or(Qualification::Supported);
    let clean = intent == Intent::Clean;
    let mut plan = Plan {
        schema_version: 1,
        media,
        selected_image,
        observed_efi_partitions: target.disk.partitions.iter().filter(|partition| partition.efi).cloned().collect(),
        target,
        intent,
        result,
        findings,
        next_action: match result {
            Qualification::Supported => "Revise e exporte o plano para análise técnica",
            _ => "Revise os bloqueios e intervenções antes de selecionar outro destino",
        }.into(),
        minimum_bytes,
        intended_partitions: if clean { vec!["EFI (FAT32)".into(), "MSR".into(), "Windows (NTFS)".into(), "Recuperação Windows (NTFS)".into()] } else { vec![] },
        existing_data_at_risk: clean && populated,
        planned_modifications: if clean { vec!["Recriar as partições planejadas e aplicar o índice selecionado".into()] } else { vec![] },
        expected_final_state: if clean { "Windows instalado em GPT e inicializável em UEFI" } else { "Nenhuma alteração no disco nesta revisão" }.into(),
        authorization_required: "Exige plano revalidado, confirmação vinculada ao SHA-256 e execução futura restrita a laboratório descartável".into(),
        fingerprint: String::new(),
    };

    plan.fingerprint = fingerprint(&plan)?;

    Ok(plan)
}

fn fingerprint(plan: &Plan) -> Result<String> {
    let mut content = plan.clone();
    content.fingerprint.clear();
    let bytes = serde_json::to_vec(&content)
        .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))?;

    Ok(format!("{:x}", Sha256::digest(bytes)))
}

impl Plan {
    pub fn validate_fingerprint(&self) -> Result<()> {
        if self.schema_version != 1 || self.fingerprint != fingerprint(self)? {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Versão ou SHA-256 do plano inválido",
            ));
        }

        Ok(())
    }

    /// Validate review authorization against freshly observed evidence. There is no executor.
    pub fn validate_authorization(&self, confirmation: &str, cancel: &AtomicBool) -> Result<()> {
        self.validate_fingerprint()?;
        if self.intent != Intent::Clean
            || self.result != Qualification::Supported
            || confirmation != format!("AUTORIZAR-IMPLANTACAO:{}", self.fingerprint)
        {
            return Err(Error::new(
                ErrorKind::Permission,
                "Plano ou autorização incompatível com a implantação",
            ));
        }

        let current = create_plan(
            &self.media.path,
            &self.media.sha256,
            self.selected_image.index,
            &self.target.disk.identity,
            self.intent,
            self.target
                .preservation
                .as_ref()
                .map(|backup| backup.directory.as_path()),
            cancel,
        )?;
        if current.fingerprint != self.fingerprint {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Evidências alteradas. Gere um novo plano",
            ));
        }

        Ok(())
    }
}

pub fn export(plan: &Plan, destination: &Path) -> Result<PathBuf> {
    plan.validate_fingerprint()?;
    let destination = safe_existing(destination)?;
    let path = destination.join(format!("archrescue-deployment-{}.json", plan.fingerprint));
    let bytes = serde_json::to_vec_pretty(plan)
        .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| Error::io("Criar plano de implantação", error))?;

    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| Error::io("Gravar plano de implantação", error))?;

    drop(file);
    if fs::read(&path).map_err(|error| Error::io("Reler plano exportado", error))? != bytes {
        return Err(Error::new(
            ErrorKind::Integrity,
            "Plano exportado diverge da prévia",
        ));
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ImageType;

    fn fixture() -> (Media, Target) {
        let media = Media {
            path: "install.wim".into(),
            image_type: ImageType::Wim,
            container_bytes: 10,
            sha256: "f".repeat(64),
            images: vec![Image {
                index: 1,
                name: Some("Windows".into()),
                edition: None,
                architecture: Some("x64".into()),
                version: Some("10.0.19041".into()),
                expanded_bytes: Some(10),
                installation: Some("Client".into()),
            }],
        };
        let target = Target {
            disk: Disk {
                number: Some(3),
                identity: "gpt:10987d18-1c59-4484-937c-381655d9ff2c".into(),
                device_path: "disk-fixture".into(),
                capacity: Some(80 * 1024 * 1024 * 1024),
                bus: Some(1),
                removable: Some(false),
                read_only: None,
                style: Some("GPT".into()),
                partitions: vec![],
                errors: vec![],
            },
            volumes: vec![],
            discovery: Discovery::default(),
            firmware: "UEFI".into(),
            storage_health: "Desconhecida".into(),
            writable: Some(true),
            encryption: "Sem partições observadas".into(),
            preservation: None,
        };

        (media, target)
    }

    #[test]
    fn immutable_preview_is_deterministic_and_detects_alterations() {
        let (media, target) = fixture();
        let first = build(media.clone(), target.clone(), 1, Intent::Clean).unwrap();
        let mut second = build(media, target, 1, Intent::Clean).unwrap();

        assert_eq!(first.result, Qualification::Supported);
        assert_eq!(first.fingerprint, second.fingerprint);
        assert!(first
            .validate_authorization("token errado", &AtomicBool::new(false))
            .is_err());
        second.target.disk.number = Some(4);
        assert_eq!(
            second.validate_fingerprint().unwrap_err().kind,
            ErrorKind::Integrity
        );
    }

    #[test]
    fn insufficient_capacity_and_unsupported_image_are_explicit() {
        let (mut media, mut target) = fixture();
        target.disk.capacity = Some(42);
        media.images[0].architecture = Some("arm64".into());

        let plan = build(media.clone(), target.clone(), 1, Intent::Clean).unwrap();
        assert_eq!(plan.result, Qualification::Blocked);
        assert!(plan
            .findings
            .iter()
            .any(|finding| finding.result == Qualification::Unsupported));
        assert!(build(media, target, 2, Intent::Clean).is_err());
    }

    #[test]
    fn changed_or_ambiguous_target_is_rejected() {
        let (_, target) = fixture();
        let id = &target.disk.identity;
        let mut inventory = archrescue_storage::Inventory::default();

        assert!(select_disk(&inventory, id).is_err());
        inventory.disks.push(target.disk.clone());
        assert!(select_disk(&inventory, id).is_ok());
        inventory.disks.push(target.disk.clone());
        assert!(select_disk(&inventory, id).is_err());
    }

    #[test]
    fn existing_data_requires_preservation() {
        let (media, mut target) = fixture();
        target.disk.partitions.push(Partition {
            number: 1,
            id: None,
            offset: 1024,
            length: 4096,
            kind: "data".into(),
            efi: false,
        });

        let plan = build(media, target, 1, Intent::Clean).unwrap();

        assert!(plan.existing_data_at_risk);
        assert_eq!(plan.result, Qualification::RequiresIntervention);
        assert!(plan
            .validate_authorization(
                &format!("AUTORIZAR-IMPLANTACAO:{}", plan.fingerprint),
                &AtomicBool::new(false)
            )
            .is_err());
    }
}
