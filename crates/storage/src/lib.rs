//! Read-only storage inventory and destination validation primitives.
use archrescue_core::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
#[cfg(windows)]
pub mod windows;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub total: u64,
    pub available: u64,
    pub free: u64,
    pub used: u64,
}
impl Space {
    pub fn checked(total: u64, free: u64, available: u64) -> Result<Self> {
        if free > total || available > free {
            return Err(Error::new(
                ErrorKind::Validation,
                "Inconsistent filesystem space metadata",
            ));
        }
        Ok(Self {
            total,
            free,
            available,
            used: total - free,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeIdentity {
    pub volume: String,
    pub physical_disks: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub identity: String,
    pub paths: Vec<PathBuf>,
    pub filesystem: Option<String>,
    pub space: Option<Space>,
    pub physical_disks: Vec<u32>,
    pub errors: Vec<Error>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub number: u32,
    pub offset: u64,
    pub length: u64,
    pub kind: String,
    pub efi: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub identity: String,
    pub device_path: String,
    pub capacity: Option<u64>,
    pub bus: Option<u32>,
    pub removable: Option<bool>,
    pub style: Option<String>,
    pub partitions: Vec<Partition>,
    pub errors: Vec<Error>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub disks: Vec<Disk>,
    pub volumes: Vec<Volume>,
    pub errors: Vec<Error>,
}

pub fn inventory() -> Inventory {
    #[cfg(windows)]
    {
        windows::inventory()
    }
    #[cfg(not(windows))]
    {
        Inventory {
            errors: vec![Error::new(
                ErrorKind::Unsupported,
                "Windows disk inventory requires Windows; no simulated disks are returned",
            )],
            ..Default::default()
        }
    }
}

/// Reject traversal and every existing symlink/reparse component before canonicalization.
pub fn safe_existing(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(Error::new(
            ErrorKind::Validation,
            "An absolute path without traversal is required",
        ));
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        let md =
            fs::symlink_metadata(&current).map_err(|e| Error::io("Inspect path component", e))?;
        if md.file_type().is_symlink() {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "Symbolic links are not supported",
            ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if md.file_attributes() & (0x400 | 0x4000) != 0 {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "Reparse points and encrypted files are not supported",
                ));
            }
        }
    }
    fs::canonicalize(path).map_err(|e| Error::io("Canonicalize path", e))
}

pub fn identity(path: &Path) -> Result<VolumeIdentity> {
    #[cfg(windows)]
    {
        windows::identity(path)
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let md = fs::metadata(path).map_err(|e| Error::io("Read filesystem identity", e))?;
        Ok(VolumeIdentity {
            volume: format!("unix-dev:{}", md.dev()),
            physical_disks: vec![],
        })
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = path;
        Err(Error::new(ErrorKind::Unsupported, "Unknown platform"))
    }
}
pub fn space(path: &Path) -> Result<Space> {
    #[cfg(windows)]
    {
        windows::space(path)
    }
    #[cfg(unix)]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let p = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| Error::new(ErrorKind::Validation, "NUL in path"))?;
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: CString is terminated and stat points to writable storage.
        if unsafe { libc::statvfs(p.as_ptr(), stat.as_mut_ptr()) } != 0 {
            return Err(Error::io("statvfs", std::io::Error::last_os_error()));
        }
        // SAFETY: statvfs succeeded and initialized the structure.
        let stat = unsafe { stat.assume_init() };
        let bytes = |n: u64| {
            n.checked_mul(stat.f_frsize)
                .ok_or_else(|| Error::new(ErrorKind::Validation, "Filesystem size overflow"))
        };
        Space::checked(
            bytes(stat.f_blocks)?,
            bytes(stat.f_bfree)?,
            bytes(stat.f_bavail)?,
        )
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = path;
        Err(Error::new(ErrorKind::Unsupported, "Unknown platform"))
    }
}
pub fn require_separate(source: &VolumeIdentity, destination: &VolumeIdentity) -> Result<()> {
    if source.volume == destination.volume
        || source
            .physical_disks
            .iter()
            .any(|d| destination.physical_disks.contains(d))
    {
        return Err(Error::new(
            ErrorKind::Validation,
            "Source and destination share a volume or physical disk",
        ));
    }
    #[cfg(windows)]
    if source.physical_disks.is_empty() || destination.physical_disks.is_empty() {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Cannot prove separate physical disks",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_space_rejected() {
        assert!(Space::checked(10, 11, 1).is_err());
        assert!(Space::checked(10, 5, 6).is_err());
    }
    #[test]
    fn shared_extent_rejected() {
        let a = VolumeIdentity {
            volume: "a".into(),
            physical_disks: vec![2],
        };
        let b = VolumeIdentity {
            volume: "b".into(),
            physical_disks: vec![1, 2],
        };
        assert!(require_separate(&a, &b).is_err());
    }
}
