//! Read-only boot evidence. No repair or firmware mutation APIs.

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use archrescue_core::{Error, ErrorKind};
use archrescue_storage::safe_existing;

pub mod recovery;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub path: PathBuf,
    pub status: String,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootReport {
    pub observations: Vec<Observation>,
    pub errors: Vec<Error>,
    pub limitations: Vec<String>,
}

pub fn inspect(roots: &[PathBuf]) -> BootReport {
    inspect_with_scratch(roots, None)
}

pub fn inspect_with_scratch(roots: &[PathBuf], scratch: Option<&Path>) -> BootReport {
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
                    error: None,
                }),
                Err(error) => report.errors.push(Error::io("Probe boot path", error)),
                Ok(true) => match inspect_boot_file(&path, relative.ends_with("BCD"), scratch) {
                    Ok(detail) => report.observations.push(Observation {
                        path,
                        status: "readable".into(),
                        detail,
                        error: None,
                    }),
                    Err(error) => {
                        report.observations.push(Observation {
                            path,
                            status: "unavailable".into(),
                            detail: error.to_string(),
                            error: Some(error.clone()),
                        });
                        report.errors.push(error);
                    }
                },
            }
        }
    }

    report
}

fn inspect_boot_file(
    path: &Path,
    is_bcd: bool,
    scratch: Option<&Path>,
) -> std::result::Result<String, Error> {
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

        #[cfg(windows)]
        {
            enumerate_bcd_snapshot(&path, file, scratch)
        }

        #[cfg(not(windows))]
        {
            let _ = scratch;
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

#[cfg(windows)]
fn enumerate_bcd_snapshot(
    path: &Path,
    mut source_file: File,
    scratch: Option<&Path>,
) -> std::result::Result<String, Error> {
    use std::{
        fs,
        io::{self, Seek},
        process::Command,
    };

    use archrescue_core::Operation;
    use archrescue_storage::{identity, require_separate};

    const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;

    let source_size = source_file
        .metadata()
        .map_err(|error| Error::io("Read source BCD size", error))?
        .len();
    if source_size > MAX_SNAPSHOT_BYTES {
        return Ok("Registry signature present; BCD exceeds the 64 MiB snapshot limit".into());
    }

    let default_scratch = std::env::temp_dir();
    let temporary_root = match safe_existing(scratch.unwrap_or(&default_scratch)).and_then(|root| {
        require_separate(&identity(path)?, &identity(&root)?)?;
        Ok(root)
    }) {
        Ok(root) => root,
        Err(error) => {
            return Ok(format!(
                "Registry signature present; BCD enumeration requires temporary storage on a \
                 verified separate disk: {error}"
            ));
        }
    };

    let executable = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .ok_or_else(|| Error::new(ErrorKind::Environment, "SystemRoot unavailable"))?
        .join("System32/bcdedit.exe");

    let working_directory = temporary_root.join(format!("archrescue-bcd-{}", Operation::new().id));
    fs::create_dir(&working_directory)
        .map_err(|error| Error::io("Create isolated BCD workspace", error))?;

    // Even /enum can update the store through registry recovery. Give bcdedit only the copy.
    let enumeration = (|| {
        let snapshot_path = working_directory.join("BCD");
        let mut snapshot_file = File::create_new(&snapshot_path)
            .map_err(|error| Error::io("Create temporary BCD copy", error))?;
        source_file
            .rewind()
            .map_err(|error| Error::io("Rewind source BCD", error))?;
        let copied_bytes = io::copy(&mut source_file.take(source_size + 1), &mut snapshot_file)
            .map_err(|error| Error::io("Copy source BCD for enumeration", error))?;
        if copied_bytes != source_size {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Source BCD size changed while copying",
            ));
        }

        drop(snapshot_file);

        Command::new(executable)
            .arg("/store")
            .arg(snapshot_path)
            .args(["/enum", "all"])
            .output()
            .map_err(|error| Error::io("Enumerate temporary BCD copy", error))
    })();

    let cleanup = fs::remove_dir_all(&working_directory)
        .map_err(|error| Error::io("Remove isolated BCD workspace", error));

    let output = enumeration?;
    cleanup?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(Error::new(
            ErrorKind::Incomplete,
            format!(
                "BCD enumeration failed: {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    // Preserve supporting evidence without interpreting localized strings as boot health.
    Ok(format!(
        "BCD enumeration output from an isolated copy (lossy decoding may affect the OEM code \
         page):\n{}\nReference consistency remains unverified.",
        String::from_utf8_lossy(&output.stdout)
    ))
}
