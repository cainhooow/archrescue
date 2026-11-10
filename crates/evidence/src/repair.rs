//! Explicit-target UEFI repair planning and a build-registered disposable fixture gate.

use std::path::{Path, PathBuf};

use archrescue_core::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};

#[cfg(windows)]
mod windows;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairTarget {
    pub disk_id: String,
    pub windows_partition_id: String,
    pub efi_partition_id: String,
    pub windows_volume_guid: String,
    pub efi_volume_guid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPlan {
    pub schema_version: u32,
    pub id: String,
    pub windows_root: PathBuf,
    pub efi_root: PathBuf,
    pub target: Option<RepairTarget>,
    pub command: Vec<String>,
    pub intended_changes: Vec<String>,
    pub backup_destination: PathBuf,
    pub data_checkpoint: Option<PathBuf>,
    pub efi_checkpoint: Option<PathBuf>,
    pub scratch_estimate_bytes: u64,
    pub rollback: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairAttempt {
    pub plan: RepairPlan,
    pub authorization: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub post_repair: archrescue_diagnostics::BootReport,
    pub backup_rechecked: bool,
    pub postconditions_met: bool,
    pub boot_observed: bool,
    pub errors: Vec<Error>,
}

impl RepairPlan {
    fn new(
        windows_root: &Path,
        efi_root: &Path,
        destination: &Path,
        data_checkpoint: Option<&Path>,
        efi_checkpoint: Option<&Path>,
    ) -> Self {
        Self {
            schema_version: 1,
            id: String::new(),
            windows_root: windows_root.into(),
            efi_root: efi_root.into(),
            target: None,
            command: vec![],
            intended_changes: vec![
                "Copiar arquivos de boot para EFI/Microsoft/Boot na ESP indicada".into(),
                "Recriar o armazenamento EFI/Microsoft/Boot/BCD e os arquivos UEFI de fallback"
                    .into(),
            ],
            backup_destination: destination.into(),
            data_checkpoint: data_checkpoint.map(PathBuf::from),
            efi_checkpoint: efi_checkpoint.map(PathBuf::from),
            scratch_estimate_bytes: 64 * 1024 * 1024,
            rollback: "Restaurar o snapshot externo do clone descartável se qualquer etapa falhar"
                .into(),
            blockers: vec![],
        }
    }

    pub fn authorization_text(&self) -> String {
        format!("AUTORIZO-{}", self.id)
    }
}

pub fn plan(
    windows_root: &Path,
    efi_root: &Path,
    destination: &Path,
    data_checkpoint: Option<&Path>,
    efi_checkpoint: Option<&Path>,
) -> Result<RepairPlan> {
    use sha2::{Digest, Sha256};

    let mut plan = RepairPlan::new(
        windows_root,
        efi_root,
        destination,
        data_checkpoint,
        efi_checkpoint,
    );

    #[cfg(windows)]
    if let Err(error) = windows::identify(&mut plan) {
        plan.blockers.push(error.to_string());
    }

    #[cfg(not(windows))]
    plan.blockers.push("Reparo UEFI requer WinPE x64".into());

    if data_checkpoint.is_none() || efi_checkpoint.is_none() {
        plan.blockers
            .push("Exige backups independentes dos dados e da árvore EFI antes da execução".into());
    }

    let bytes = serde_json::to_vec(&plan)
        .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))?;
    plan.id = format!("{:x}", Sha256::digest(bytes));

    Ok(plan)
}

pub fn execute(plan: &RepairPlan, authorization: &str) -> Result<RepairAttempt> {
    if authorization != plan.authorization_text() || plan.id.is_empty() {
        return Err(Error::new(
            ErrorKind::Permission,
            "Confirmação específica do plano ausente ou incorreta",
        ));
    }

    if !plan.blockers.is_empty() {
        return Err(Error::new(
            ErrorKind::Unsupported,
            format!("Reparo bloqueado: {}", plan.blockers.join(" | ")),
        ));
    }

    #[cfg(windows)]
    return windows::execute(plan, authorization);

    #[cfg(not(windows))]
    Err(Error::new(
        ErrorKind::Unsupported,
        "Reparo UEFI requer WinPE",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_blocks_missing_backup_and_rejects_mismatched_authorization() {
        let root = Path::new("missing");
        let plan = plan(root, root, root, None, None).unwrap();

        assert!(plan.command.is_empty());
        assert!(plan
            .blockers
            .iter()
            .any(|reason| reason.contains("backups")));
        assert_eq!(
            execute(&plan, "AUTORIZO").unwrap_err().kind,
            ErrorKind::Permission
        );
        assert_eq!(
            execute(&plan, &plan.authorization_text()).unwrap_err().kind,
            ErrorKind::Unsupported
        );
    }
}
