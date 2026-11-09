//! Offline candidates require multiple independent filesystem markers, not a folder name.
use archrescue_core::{Error, ErrorKind};
use archrescue_storage::safe_existing;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub path: PathBuf,
    pub volume: PathBuf,
    pub architecture: Option<String>,
    pub version: Option<String>,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Discovery {
    pub installations: Vec<Installation>,
    pub errors: Vec<Error>,
}
fn marker(path: &Path, signature: &[u8]) -> std::result::Result<bool, Error> {
    let path = safe_existing(path)?;
    let mut file = File::open(&path).map_err(|e| Error::io("Open offline marker", e))?;
    let mut bytes = vec![0; signature.len()];
    match file.read_exact(&mut bytes) {
        Ok(()) => Ok(bytes == signature),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(Error::io("Read offline marker", e)),
    }
}
pub fn pe_architecture(path: &Path) -> std::result::Result<String, Error> {
    let path = safe_existing(path)?;
    let mut f = File::open(path).map_err(|e| Error::io("Open PE header", e))?;
    let mut dos = [0u8; 64];
    f.read_exact(&mut dos)
        .map_err(|e| Error::io("Read DOS header", e))?;
    if &dos[..2] != b"MZ" {
        return Err(Error::new(ErrorKind::Validation, "Not a PE image"));
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    if !(64..=16 * 1024 * 1024).contains(&offset) {
        return Err(Error::new(ErrorKind::Validation, "Invalid PE offset"));
    }
    f.seek(SeekFrom::Start(offset))
        .map_err(|e| Error::io("Seek PE header", e))?;
    let mut pe = [0u8; 6];
    f.read_exact(&mut pe)
        .map_err(|e| Error::io("Read PE signature", e))?;
    if &pe[..4] != b"PE\0\0" {
        return Err(Error::new(ErrorKind::Validation, "Invalid PE signature"));
    }
    match u16::from_le_bytes([pe[4], pe[5]]) {
        0x8664 => Ok("x64".into()),
        0x14c => Ok("x86".into()),
        0xaa64 => Ok("ARM64".into()),
        _ => Err(Error::new(ErrorKind::Unsupported, "Unknown PE machine")),
    }
}
pub fn discover(roots: &[PathBuf]) -> Discovery {
    let mut result = Discovery::default();
    for root in roots {
        let root = match safe_existing(root) {
            Ok(p) => p,
            Err(e) => {
                result.errors.push(e);
                continue;
            }
        };
        let windows = root.join("Windows");
        match windows.try_exists() {
            Ok(false) => continue,
            Err(e) => {
                result.errors.push(Error::io("Probe Windows directory", e));
                continue;
            }
            _ => {}
        }
        let system = windows.join("System32");
        let markers = [system.join("config/SYSTEM"), system.join("config/SOFTWARE")];
        let mut valid = true;
        for p in markers {
            match marker(&p, b"regf") {
                Ok(true) => {}
                Ok(false) => {
                    result.errors.push(Error::new(
                        ErrorKind::Validation,
                        "Invalid offline registry signature",
                    ));
                    valid = false;
                }
                Err(e) => {
                    result.errors.push(e);
                    valid = false;
                }
            }
        }
        let architecture = match pe_architecture(&system.join("ntoskrnl.exe")) {
            Ok(a) => Some(a),
            Err(e) => {
                result.errors.push(e);
                valid = false;
                None
            }
        };
        if valid {
            result.installations.push(Installation {path:windows,volume:root,architecture,version:None,confidence:"candidate: corroborated filesystem markers".into(),evidence:vec!["SYSTEM and SOFTWARE have regf signatures".into(),"ntoskrnl.exe has a valid PE signature and supported machine type".into()],limitations:vec!["Registry hive contents and kernel signatures are not authenticated; this is not proof of a healthy or bootable installation".into(),"Version/build is unavailable in P0; no running-OS version is substituted".into()]});
        }
    }
    result
}
