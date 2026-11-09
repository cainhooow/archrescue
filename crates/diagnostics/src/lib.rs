//! Read-only boot evidence. No repair or firmware mutation APIs.

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use archrescue_core::{Error, ErrorKind};
use archrescue_storage::safe_existing;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub path: PathBuf,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootReport {
    pub observations: Vec<Observation>,
    pub errors: Vec<Error>,
    pub limitations: Vec<String>,
}

pub fn inspect(roots: &[PathBuf]) -> BootReport {
    let mut report = BootReport {
        limitations: vec![
            "Only supplied accessible roots are examined. \
             Unmounted EFI partitions are not mounted or modified."
                .into(),
            "File presence does not establish correct firmware boot order or valid BCD device \
             references."
                .into(),
        ],
        ..Default::default()
    };

    for root in roots {
        let root = match safe_existing(root) {
            Ok(path) => path,
            Err(error) => {
                report.errors.push(error);
                continue;
            }
        };

        for relative in [
            "EFI/Microsoft/Boot/bootmgfw.efi",
            "EFI/Boot/bootx64.efi",
            "EFI/Microsoft/Boot/BCD",
            "Boot/BCD",
        ] {
            let path = root.join(relative);
            match path.try_exists() {
                Ok(false) => report.observations.push(Observation {
                    path,
                    status: "missing".into(),
                    detail: "Expected path is absent from this root".into(),
                }),
                Err(error) => report.errors.push(Error::io("Probe boot path", error)),
                Ok(true) => match inspect_boot_file(&path, relative.ends_with("BCD")) {
                    Ok(detail) => report.observations.push(Observation {
                        path,
                        status: "readable".into(),
                        detail,
                    }),
                    Err(error) => {
                        report.observations.push(Observation {
                            path,
                            status: "unavailable".into(),
                            detail: error.to_string(),
                        });
                        report.errors.push(error);
                    }
                },
            }
        }
    }

    report
}

fn inspect_boot_file(path: &Path, is_bcd: bool) -> std::result::Result<String, Error> {
    let path = safe_existing(path)?;
    let mut file = File::open(&path).map_err(|error| Error::io("Open boot file", error))?;

    if is_bcd {
        let mut header = [0; 4];
        file.read_exact(&mut header)
            .map_err(|error| Error::io("Read BCD hive signature", error))?;

        if &header != b"regf" {
            return Err(Error::new(
                ErrorKind::Validation,
                "BCD lacks registry hive signature",
            ));
        }

        // P0 deliberately does not mount offline registry hives. bcdedit /enum is read-only.
        #[cfg(windows)]
        {
            let executable = std::env::var_os("SystemRoot")
                .map(PathBuf::from)
                .ok_or_else(|| Error::new(ErrorKind::Environment, "SystemRoot unavailable"))?
                .join("System32/bcdedit.exe");

            let output = std::process::Command::new(executable)
                .arg("/store")
                .arg(&path)
                .args(["/enum", "all"])
                .output()
                .map_err(|error| Error::io("Run read-only BCD inspection", error))?;

            if !output.status.success() || output.stdout.is_empty() {
                return Err(Error::new(
                    ErrorKind::Incomplete,
                    format!("BCD enumeration failed: {}", output.status),
                ));
            }

            // Preserve supporting evidence, without interpreting localized strings as health.
            Ok(format!(
                "BCD enumeration output (lossy decoding may affect the OEM code page):\n{}\n\
                 Reference consistency remains unverified.",
                String::from_utf8_lossy(&output.stdout)
            ))
        }

        #[cfg(not(windows))]
        {
            Ok(
                "Registry signature present; BCD semantic enumeration unavailable outside Windows"
                    .into(),
            )
        }
    } else {
        let mut magic = [0; 2];
        file.read_exact(&mut magic)
            .map_err(|error| Error::io("Read EFI image", error))?;

        if &magic != b"MZ" {
            return Err(Error::new(
                ErrorKind::Validation,
                "EFI image lacks DOS signature",
            ));
        }

        Ok(
            "DOS image signature present; signature trust and firmware reference not verified"
                .into(),
        )
    }
}
