//! Structured offline diagnosis. Same-disk topology does not establish BCD references.

use std::path::{Path, PathBuf};

use archrescue_core::ErrorKind;
use archrescue_storage::{Inventory, Partition};
use archrescue_windows_detection::{Discovery, ProbeState};
use serde::{Deserialize, Serialize};

use crate::BootReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Observed,
    ConfirmedFailure,
    SuspectedFailure,
    MissingEvidence,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: String,
    pub confidence: Confidence,
    pub affected: PathBuf,
    pub evidence: String,
    pub next_action: String,
    pub blocks_repair: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Association {
    pub installation: PathBuf,
    pub disk_id: String,
    pub disk_number: u32,
    pub windows_partition: Partition,
    pub efi_partitions: Vec<Partition>,
    pub recovery_partitions: Vec<Partition>,
    pub mounted_efi: Vec<PathBuf>,
    pub bcd_reference_verified: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Diagnosis {
    pub findings: Vec<Finding>,
    pub associations: Vec<Association>,
}

impl Diagnosis {
    fn record(
        &mut self,
        id: &str,
        confidence: Confidence,
        affected: &Path,
        evidence: impl Into<String>,
        next_action: &str,
        blocks_repair: bool,
    ) {
        let severity = match confidence {
            Confidence::ConfirmedFailure => "error",
            Confidence::Observed => "info",
            _ => "warning",
        };
        self.findings.push(Finding {
            id: id.into(),
            severity: severity.into(),
            confidence,
            affected: affected.into(),
            evidence: evidence.into(),
            next_action: next_action.into(),
            blocks_repair,
        });
    }
}

pub fn diagnose(discovery: &Discovery, boot: &BootReport, inventory: &Inventory) -> Diagnosis {
    use Confidence::*;

    let mut result = Diagnosis::default();
    for probe in &discovery.probes {
        let (id, confidence, evidence) = match probe.state {
            ProbeState::Inaccessible => (
                "WINDOWS_VOLUME_UNAVAILABLE",
                MissingEvidence,
                "Volume inacessível. Verifique conexão, permissões e bloqueio BitLocker.",
            ),
            ProbeState::Absent => (
                "WINDOWS_ABSENT",
                Observed,
                "Nenhum diretório Windows na raiz consultada.",
            ),
            ProbeState::Incomplete => (
                "WINDOWS_MARKERS_INCOMPLETE",
                ConfirmedFailure,
                "Diretório Windows presente com marcadores inválidos ou ilegíveis.",
            ),
            ProbeState::RunningEnvironment => (
                "RUNNING_ENVIRONMENT_EXCLUDED",
                Unsupported,
                "Ambiente Windows em execução excluído da busca offline.",
            ),
            ProbeState::Candidate => (
                "WINDOWS_CANDIDATE",
                Observed,
                "Assinaturas SYSTEM e SOFTWARE e cabeçalho do kernel observados.",
            ),
        };

        result.record(
            id,
            confidence,
            &probe.root,
            evidence,
            "Confira a instalação e o disco antes de escolher um alvo.",
            confidence != Observed,
        );
        if probe.windows_old {
            result.record(
                "WINDOWS_OLD_PRESENT",
                Observed,
                &probe.root,
                "Diretório Windows.old encontrado.",
                "Verifique se contém dados a preservar.",
                false,
            );
        }
    }

    if discovery.installations.len() > 1 {
        for installation in &discovery.installations {
            result.record(
                "MULTIPLE_WINDOWS",
                Unsupported,
                &installation.path,
                "Mais de um candidato Windows foi observado.",
                "Identifique a instalação pelo disco e pela partição.",
                true,
            );
        }
    }

    for installation in &discovery.installations {
        let (id, confidence, evidence) = match (
            &installation.architecture,
            &installation.loader_architecture,
        ) {
            (Some(kernel), Some(loader)) if kernel == loader => (
                "WINDOWS_LOADER_ARCH_MATCH",
                Observed,
                format!("Kernel e winload.efi apresentam arquitetura {loader}."),
            ),
            (Some(kernel), Some(loader)) => (
                "WINDOWS_LOADER_ARCH_MISMATCH",
                ConfirmedFailure,
                format!("Kernel {kernel} e winload.efi {loader} divergem."),
            ),
            _ => (
                "WINDOWS_LOADER_UNAVAILABLE",
                MissingEvidence,
                "Cabeçalho PE de winload.efi indisponível.".into(),
            ),
        };
        result.record(
            id,
            confidence,
            &installation.path,
            evidence,
            "Verifique os arquivos de inicialização da instalação.",
            confidence != Observed,
        );

        if !installation.recovery_files.is_empty() {
            result.record(
                "WINDOWS_RE_FILES_PRESENT",
                Observed,
                &installation.path,
                format!("{:?}", installation.recovery_files),
                "Confirme a configuração de recuperação antes de depender do WinRE.",
                false,
            );
        }

        associate(
            &mut result,
            &installation.volume,
            &installation.path,
            inventory,
        );
    }

    for observation in &boot.observations {
        let bcd = observation.path.ends_with("BCD");
        let (id, confidence) = match observation.status.as_str() {
            "missing" if bcd => ("BCD_PATH_MISSING", MissingEvidence),
            "missing" => ("EFI_IMAGE_PATH_MISSING", MissingEvidence),
            "unavailable"
                if bcd
                    && observation
                        .error
                        .as_ref()
                        .is_some_and(|error| error.kind == ErrorKind::Validation) =>
            {
                ("BCD_INVALID_SIGNATURE", ConfirmedFailure)
            }
            "unavailable" if bcd => ("BCD_UNREADABLE_OR_INVALID", SuspectedFailure),
            "unavailable" => ("EFI_IMAGE_UNAVAILABLE", MissingEvidence),
            "readable" if bcd => ("BCD_REFERENCES_UNRESOLVED", MissingEvidence),
            _ => ("EFI_IMAGE_SIGNATURE_OBSERVED", Observed),
        };

        result.record(
            id,
            confidence,
            &observation.path,
            &observation.detail,
            "Compare com a ESP correta e confira os caminhos esperados.",
            confidence == ConfirmedFailure,
        );
    }

    for volume in &inventory.volumes {
        if !volume.errors.is_empty() && volume.filesystem.is_none() {
            let path = volume
                .paths
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from(&volume.identity));
            result.record(
                "VOLUME_FILESYSTEM_UNAVAILABLE",
                MissingEvidence,
                &path,
                format!("{:?}", volume.errors),
                "Confirme conexão, suporte do sistema de arquivos e estado do BitLocker.",
                true,
            );
        }
    }

    result
}

#[cfg(windows)]
fn associate(result: &mut Diagnosis, root: &Path, installation: &Path, inventory: &Inventory) {
    use archrescue_storage::windows::mounted_partition;

    let source = match mounted_partition(root) {
        Ok(source) => source,
        Err(error) => {
            result.record(
                "WINDOWS_PARTITION_UNRESOLVED",
                Confidence::MissingEvidence,
                installation,
                error.to_string(),
                "Confirme o volume e a partição do Windows.",
                true,
            );
            return;
        }
    };
    let Some(disk) = inventory
        .disks
        .iter()
        .find(|disk| disk.number == Some(source.disk_number))
    else {
        result.record(
            "WINDOWS_DISK_UNRESOLVED",
            Confidence::MissingEvidence,
            installation,
            "O disco físico não foi correlacionado ao inventário.",
            "Atualize o inventário.",
            true,
        );
        return;
    };

    let efi_partitions: Vec<_> = disk
        .partitions
        .iter()
        .filter(|partition| partition.efi)
        .cloned()
        .collect();
    let recovery_partitions = disk
        .partitions
        .iter()
        .filter(|partition| {
            partition.kind == "de94bba4-06d1-4d40-a16a-bfd50179d6ac" || partition.kind == "mbr:27"
        })
        .cloned()
        .collect();
    let mut mounted_efi = Vec::new();
    for volume in &inventory.volumes {
        if volume.physical_disks != [source.disk_number] {
            continue;
        }
        let Some(path) = volume.paths.first() else {
            continue;
        };
        if let Ok(target) = mounted_partition(path) {
            if target.partition.efi && target.disk_id == source.disk_id {
                mounted_efi.push(path.clone());
            }
        }
    }

    let (id, confidence, evidence) = match efi_partitions.len() {
        0 => (
            "EFI_ASSOCIATION_UNRESOLVED",
            Confidence::MissingEvidence,
            "Nenhuma ESP encontrada no mesmo disco.",
        ),
        1 if mounted_efi.len() == 1 => (
            "EFI_SAME_DISK_CANDIDATE",
            Confidence::Observed,
            "Uma ESP acessível no mesmo disco. Referências do BCD continuam sem comprovação.",
        ),
        1 => (
            "EFI_NOT_ACCESSIBLE",
            Confidence::MissingEvidence,
            "Uma ESP no mesmo disco sem caminho acessível no inventário.",
        ),
        _ => (
            "EFI_AMBIGUOUS",
            Confidence::Unsupported,
            "Múltiplas ESPs no disco da instalação.",
        ),
    };
    result.record(
        id,
        confidence,
        installation,
        evidence,
        "Confirme o identificador da ESP e a referência do BCD.",
        confidence != Confidence::Observed,
    );

    result.associations.push(Association {
        installation: installation.into(),
        disk_id: source.disk_id,
        disk_number: source.disk_number,
        windows_partition: source.partition,
        efi_partitions,
        recovery_partitions,
        mounted_efi,
        bcd_reference_verified: false,
    });
}

#[cfg(not(windows))]
fn associate(result: &mut Diagnosis, _: &Path, installation: &Path, _: &Inventory) {
    result.record(
        "EFI_TOPOLOGY_UNAVAILABLE",
        Confidence::Unsupported,
        installation,
        "Mapeamento de partições EFI requer Windows.",
        "Execute o diagnóstico em WinPE.",
        true,
    );
}
