//! Offline candidates require multiple independent filesystem markers, beyond the folder name.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use archrescue_core::{Error, ErrorKind};
use archrescue_storage::safe_existing;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub path: PathBuf,
    pub volume: PathBuf,
    pub architecture: Option<String>,
    pub version: Option<String>,
    #[serde(default)]
    pub loader_architecture: Option<String>,
    #[serde(default)]
    pub recovery_files: Vec<PathBuf>,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Discovery {
    pub installations: Vec<Installation>,
    #[serde(default)]
    pub probes: Vec<RootProbe>,
    pub errors: Vec<Error>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeState {
    Inaccessible,
    Absent,
    Incomplete,
    RunningEnvironment,
    Candidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootProbe {
    pub root: PathBuf,
    pub state: ProbeState,
    pub windows_old: bool,
}

fn probe(result: &mut Discovery, root: &Path, state: ProbeState) {
    result.probes.push(RootProbe {
        root: root.to_owned(),
        state,
        windows_old: safe_existing(&root.join("Windows.old")).is_ok_and(|path| path.is_dir()),
    });
}

#[cfg(windows)]
fn kernel_version(path: &Path) -> Option<String> {
    use std::{mem::size_of, os::windows::ffi::OsStrExt, ptr};

    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
    };

    let path = safe_existing(path).ok()?;
    let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut handle = 0;
    // SAFETY: the filename is NUL-terminated and the handle output is writable.
    let length = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), &mut handle) };
    if !(1..=1024 * 1024).contains(&length) {
        return None;
    }

    let mut data = vec![0u8; length as usize];
    let mut info = ptr::null_mut();
    let mut info_length = 0;
    let root = [b'\\' as u16, 0];
    // SAFETY: the buffer covers the queried resource and both query outputs are writable.
    let version = unsafe {
        if GetFileVersionInfoW(path.as_ptr(), 0, length, data.as_mut_ptr().cast()) == 0
            || VerQueryValueW(
                data.as_ptr().cast(),
                root.as_ptr(),
                &mut info,
                &mut info_length,
            ) == 0
            || info.is_null()
            || info_length < size_of::<VS_FIXEDFILEINFO>() as u32
        {
            return None;
        }

        let offset = (info as usize).checked_sub(data.as_ptr() as usize)?;
        if offset.checked_add(size_of::<VS_FIXEDFILEINFO>())? > data.len() {
            return None;
        }

        ptr::read_unaligned(info.cast::<VS_FIXEDFILEINFO>())
    };
    if version.dwSignature != 0xfeef04bd {
        return None;
    }

    Some(format!(
        "{}.{}.{}.{}",
        version.dwFileVersionMS >> 16,
        version.dwFileVersionMS & 0xffff,
        version.dwFileVersionLS >> 16,
        version.dwFileVersionLS & 0xffff
    ))
}

#[cfg(not(windows))]
fn kernel_version(_: &Path) -> Option<String> {
    None
}

fn marker(path: &Path, signature: &[u8]) -> std::result::Result<bool, Error> {
    let path = safe_existing(path)?;
    let mut file = File::open(&path).map_err(|error| Error::io("Open offline marker", error))?;
    let mut bytes = vec![0; signature.len()];

    match file.read_exact(&mut bytes) {
        Ok(()) => Ok(bytes == signature),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(Error::io("Read offline marker", error)),
    }
}

pub fn pe_architecture(path: &Path) -> std::result::Result<String, Error> {
    let path = safe_existing(path)?;
    let mut file = File::open(path).map_err(|error| Error::io("Open PE header", error))?;
    let mut dos = [0u8; 64];
    file.read_exact(&mut dos)
        .map_err(|error| Error::io("Read DOS header", error))?;

    if &dos[..2] != b"MZ" {
        return Err(Error::new(ErrorKind::Validation, "Not a PE image"));
    }

    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    if !(64..=16 * 1024 * 1024).contains(&offset) {
        return Err(Error::new(ErrorKind::Validation, "Invalid PE offset"));
    }

    file.seek(SeekFrom::Start(offset))
        .map_err(|error| Error::io("Seek PE header", error))?;
    let mut pe = [0u8; 6];
    file.read_exact(&mut pe)
        .map_err(|error| Error::io("Read PE signature", error))?;

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

    let mut seen = std::collections::HashSet::new();
    for root in roots {
        let root = match safe_existing(root) {
            Ok(path) => path,
            Err(error) => {
                result.errors.push(error);
                probe(&mut result, root, ProbeState::Inaccessible);
                continue;
            }
        };
        if !seen.insert(root.clone()) {
            continue;
        }

        let windows = root.join("Windows");
        match windows.try_exists() {
            Ok(false) => {
                probe(&mut result, &root, ProbeState::Absent);
                continue;
            }
            Err(error) => {
                result
                    .errors
                    .push(Error::io("Probe Windows directory", error));
                probe(&mut result, &root, ProbeState::Inaccessible);
                continue;
            }
            _ => {}
        }

        #[cfg(windows)]
        if std::env::var_os("SystemRoot")
            .and_then(|path| std::fs::canonicalize(PathBuf::from(path)).ok())
            == std::fs::canonicalize(&windows).ok()
        {
            result.errors.push(Error::new(
                ErrorKind::Unsupported,
                "Running Windows/WinPE environment excluded from offline discovery",
            ));
            probe(&mut result, &root, ProbeState::RunningEnvironment);
            continue;
        }

        let system = windows.join("System32");
        let markers = [system.join("config/SYSTEM"), system.join("config/SOFTWARE")];
        let mut valid = true;
        for path in markers {
            match marker(&path, b"regf") {
                Ok(true) => {}
                Ok(false) => {
                    result.errors.push(Error::new(
                        ErrorKind::Validation,
                        "Invalid offline registry signature",
                    ));
                    valid = false;
                }
                Err(error) => {
                    result.errors.push(error);
                    valid = false;
                }
            }
        }

        let architecture = match pe_architecture(&system.join("ntoskrnl.exe")) {
            Ok(architecture) => Some(architecture),
            Err(error) => {
                result.errors.push(error);
                valid = false;
                None
            }
        };

        if valid {
            probe(&mut result, &root, ProbeState::Candidate);
            let version = kernel_version(&system.join("ntoskrnl.exe"));
            let loader_architecture = pe_architecture(&system.join("winload.efi")).ok();
            let recovery_files = [
                system.join("Recovery/ReAgent.xml"),
                system.join("Recovery/Winre.wim"),
                root.join("Recovery/WindowsRE/Winre.wim"),
            ]
            .into_iter()
            .filter(|path| safe_existing(path).is_ok_and(|path| path.is_file()))
            .collect();

            result.installations.push(Installation {
                path: windows,
                volume: root,
                architecture,
                version,
                loader_architecture,
                recovery_files,
                confidence: "candidate: corroborated filesystem markers".into(),
                evidence: vec![
                    "SYSTEM and SOFTWARE have regf signatures".into(),
                    "ntoskrnl.exe has a valid PE signature and supported machine type".into(),
                ],
                limitations: vec![
                    "Registry hive contents and kernel signatures are not authenticated; \
                     this is not proof of a healthy or bootable installation"
                        .into(),
                    "Version, when present, is the offline kernel file version; registry edition, activation and servicing state are unresolved".into(),
                ],
            });
        } else {
            probe(&mut result, &root, ProbeState::Incomplete);
        }
    }

    result
}
