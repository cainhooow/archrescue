//! Read-only storage inventory and destination validation primitives.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use archrescue_core::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};

pub mod layout;
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
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
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

        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| Error::io("Inspect path component", error))?;
        if metadata.file_type().is_symlink() {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "Symbolic links are not supported",
            ));
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;

            if metadata.file_attributes() & (0x400 | 0x4000) != 0 {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "Reparse points and encrypted files are not supported",
                ));
            }
        }
    }

    fs::canonicalize(path).map_err(|error| Error::io("Canonicalize path", error))
}

pub fn identity(path: &Path) -> Result<VolumeIdentity> {
    #[cfg(windows)]
    {
        windows::identity(path)
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let metadata =
            fs::metadata(path).map_err(|error| Error::io("Read filesystem identity", error))?;

        Ok(VolumeIdentity {
            volume: format!("unix-dev:{}", metadata.dev()),
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

        let encoded_path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| Error::new(ErrorKind::Validation, "NUL in path"))?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();

        // SAFETY: encoded_path is NUL-terminated and stats points to writable storage.
        if unsafe { libc::statvfs(encoded_path.as_ptr(), stats.as_mut_ptr()) } != 0 {
            return Err(Error::io("statvfs", std::io::Error::last_os_error()));
        }

        // SAFETY: statvfs succeeded and initialized the structure.
        let stats = unsafe { stats.assume_init() };
        let block_bytes = |blocks: u64| {
            blocks
                .checked_mul(stats.f_frsize)
                .ok_or_else(|| Error::new(ErrorKind::Validation, "Filesystem size overflow"))
        };

        Space::checked(
            block_bytes(stats.f_blocks)?,
            block_bytes(stats.f_bfree)?,
            block_bytes(stats.f_bavail)?,
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
            .any(|disk| destination.physical_disks.contains(disk))
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
        let source = VolumeIdentity {
            volume: "a".into(),
            physical_disks: vec![2],
        };
        let destination = VolumeIdentity {
            volume: "b".into(),
            physical_disks: vec![1, 2],
        };

        assert!(require_separate(&source, &destination).is_err());
    }
}
