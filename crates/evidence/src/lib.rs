//! Explicit, local report export. No telemetry or automatic persistent writes.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use archrescue_backup::BackupResult;
use archrescue_core::{Error, ErrorKind, Operation, Result, BUILD_ID, VERSION};
use archrescue_diagnostics::BootReport;
use archrescue_storage::{safe_existing, Inventory};
use archrescue_windows_detection::Discovery;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub version: String,
    pub build_id: String,
    pub runtime_os: String,
    pub architecture: String,
    pub generated_unix: u64,
    pub inventory: Inventory,
    pub installations: Discovery,
    pub boot: BootReport,
    pub operations: Vec<BackupResult>,
    pub limitations: Vec<String>,
}

impl Report {
    pub fn collect(roots: &[PathBuf]) -> Self {
        Self {
            schema_version: 1,
            version: VERSION.into(),
            build_id: BUILD_ID.into(),
            runtime_os: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            generated_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            inventory: archrescue_storage::inventory(),
            installations: archrescue_windows_detection::discover(roots),
            boot: archrescue_diagnostics::inspect(roots),
            operations: vec![],
            limitations: vec![
                "No hardware health or bootability verdict is inferred from enumeration".into(),
                "WinPE RAM-disk exports are volatile; choose a persistent authorized destination"
                    .into(),
                "Selected relative paths and volume identities are local recovery metadata; \
                 review before sharing"
                    .into(),
            ],
        }
    }

    pub fn readable(&self) -> String {
        let mut text = format!(
            "ArchRescue {} — Technical report\nBuild: {}\nRuntime: {} / {}\nSchema: {}\n\
             Disks: {} | Volumes: {} | Windows candidates: {}\n\n",
            self.version,
            self.build_id,
            self.runtime_os,
            self.architecture,
            self.schema_version,
            self.inventory.disks.len(),
            self.inventory.volumes.len(),
            self.installations.installations.len(),
        );

        for backup in &self.operations {
            let verified_files = backup.files.iter().filter(|file| file.verified).count();

            text.push_str(&format!(
                "Operation {}: {:?}\nCopied {} / {} bytes; verified {} / {} files\n",
                backup.operation.id,
                backup.operation.state,
                backup.copied_bytes,
                backup.planned_bytes,
                verified_files,
                backup.files.len()
            ));
        }

        text.push_str("\nStructured evidence (including errors and limitations):\n");
        match serde_json::to_string_pretty(self) {
            Ok(json) => text.push_str(&json),
            Err(error) => text.push_str(&format!("Serialization failed: {error}")),
        }

        text
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| Error::io("Create report file", error))?;

    file.write_all(bytes)
        .map_err(|error| Error::io("Write report", error))?;
    file.sync_all()
        .map_err(|error| Error::io("Flush report", error))?;

    let readback = fs::read(path).map_err(|error| Error::io("Verify report", error))?;
    if readback != bytes {
        return Err(Error::new(ErrorKind::Integrity, "Report readback mismatch"));
    }

    Ok(())
}

pub fn export(report: &Report, destination: &Path) -> Result<PathBuf> {
    let destination = safe_existing(destination)?;

    if !destination.is_dir() {
        return Err(Error::new(
            ErrorKind::Validation,
            "Report destination must be an existing directory",
        ));
    }

    let folder = destination.join(format!("archrescue-report-{}", Operation::new().id));
    fs::create_dir(&folder).map_err(|error| Error::io("Create report directory", error))?;

    let json = serde_json::to_vec_pretty(report)
        .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))?;
    write_new(&folder.join("report.json"), &json)?;
    write_new(&folder.join("report.txt"), report.readable().as_bytes())?;

    Ok(folder)
}
