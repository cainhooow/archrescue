//! Read-only boot evidence. No repair or firmware mutation APIs.
use archrescue_core::{Error, ErrorKind};
use archrescue_storage::safe_existing;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
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
    let mut report=BootReport { limitations:vec!["Only supplied accessible roots are examined. Unmounted EFI partitions are not mounted or modified.".into(),"File presence does not establish correct firmware boot order or valid BCD device references.".into()],..Default::default()};
    for root in roots {
        let root = match safe_existing(root) {
            Ok(p) => p,
            Err(e) => {
                report.errors.push(e);
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
                Err(e) => report.errors.push(Error::io("Probe boot path", e)),
                Ok(true) => match check(&path, relative.ends_with("BCD")) {
                    Ok(detail) => report.observations.push(Observation {
                        path,
                        status: "readable".into(),
                        detail,
                    }),
                    Err(e) => {
                        report.observations.push(Observation {
                            path,
                            status: "unavailable".into(),
                            detail: e.to_string(),
                        });
                        report.errors.push(e);
                    }
                },
            }
        }
    }
    report
}
fn check(path: &Path, bcd: bool) -> std::result::Result<String, Error> {
    let path = safe_existing(path)?;
    let mut f = File::open(&path).map_err(|e| Error::io("Open boot file", e))?;
    if bcd {
        let mut header = [0; 4];
        f.read_exact(&mut header)
            .map_err(|e| Error::io("Read BCD hive signature", e))?;
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
                .map_err(|e| Error::io("Run read-only BCD inspection", e))?;
            if !output.status.success() || output.stdout.is_empty() {
                return Err(Error::new(
                    ErrorKind::Incomplete,
                    format!("BCD enumeration failed: {}", output.status),
                ));
            }
            // Do not infer health from localized command output. Raw output is available only interactively.
            Ok(format!("Registry signature present; bcdedit /store /enum all returned {} bytes. Reference consistency remains unverified.",output.stdout.len()))
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
        f.read_exact(&mut magic)
            .map_err(|e| Error::io("Read EFI image", e))?;
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
