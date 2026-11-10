//! Checksummed snapshots and filesystem publication primitives.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use archrescue_core::{Error, ErrorKind, Operation, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_RECORD_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Record {
    sha256: String,
    data: serde_json::Value,
}

fn integrity(error: impl std::fmt::Display) -> Error {
    Error::new(
        ErrorKind::Integrity,
        format!("Checkpoint inválido: {error}"),
    )
}

pub fn fingerprint(value: &impl Serialize) -> Result<String> {
    let bytes = serde_json::to_vec(value).map_err(integrity)?;

    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let path = archrescue_storage::safe_existing(path)?;
    let file = File::open(path).map_err(|error| Error::io("Open checkpoint", error))?;
    let mut bytes = Vec::new();
    file.take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::io("Read checkpoint", error))?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(integrity("record exceeds size limit"));
    }

    let record: Record = serde_json::from_slice(&bytes).map_err(integrity)?;
    if fingerprint(&record.data)? != record.sha256 {
        return Err(integrity("SHA-256 mismatch"));
    }

    serde_json::from_value(record.data).map_err(integrity)
}

pub fn save(path: &Path, data: &impl Serialize, replace: bool) -> Result<()> {
    let data = serde_json::to_value(data).map_err(integrity)?;
    let record = Record {
        sha256: fingerprint(&data)?,
        data,
    };
    let bytes = serde_json::to_vec(&record).map_err(integrity)?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(integrity("record exceeds size limit"));
    }

    let temporary = path.with_extension(format!("{}.next", Operation::new().id));
    let mut file = File::create_new(&temporary)
        .map_err(|error| Error::io("Create checkpoint snapshot", error))?;
    let result = (|| {
        file.write_all(&bytes)
            .map_err(|error| Error::io("Write checkpoint snapshot", error))?;
        file.sync_all()
            .map_err(|error| Error::io("Flush checkpoint snapshot", error))?;
        drop(file);

        publish(&temporary, path, replace)
    })();

    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }

    result
}

pub fn lock(path: &Path) -> Result<File> {
    if exists(path)? {
        archrescue_storage::safe_existing(path)?;
    }

    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }

    let file = options
        .open(path)
        .map_err(|error| Error::io("Open exclusive operation lock", error))?;

    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        // SAFETY: the descriptor is valid for the lifetime of the returned File.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(Error::io(
                "Acquire exclusive operation lock",
                std::io::Error::last_os_error(),
            ));
        }
    }

    Ok(file)
}

pub fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error::io("Inspect transaction path", error)),
    }
}

pub fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| Error::io("Flush transaction directory", error))?;

    #[cfg(not(unix))]
    let _ = path;

    Ok(())
}

pub fn publish(source: &Path, target: &Path, replace: bool) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };

        let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let target: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut flags = MOVEFILE_WRITE_THROUGH;
        if replace {
            flags |= MOVEFILE_REPLACE_EXISTING;
        }

        // SAFETY: both input paths are terminated UTF-16 strings valid through the call.
        if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), flags) } == 0 {
            return Err(Error::io(
                "Publish transaction file",
                std::io::Error::last_os_error(),
            ));
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};

        let source = CString::new(source.as_os_str().as_bytes())
            .map_err(|_| Error::new(ErrorKind::Validation, "NUL in transaction path"))?;
        let target = CString::new(target.as_os_str().as_bytes())
            .map_err(|_| Error::new(ErrorKind::Validation, "NUL in transaction path"))?;
        let flags = if replace { 0 } else { libc::RENAME_NOREPLACE };

        // SAFETY: both C strings remain valid through this same-filesystem rename.
        if unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                target.as_ptr(),
                flags,
            )
        } != 0
        {
            return Err(Error::io(
                "Publish transaction file",
                std::io::Error::last_os_error(),
            ));
        }
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    return Err(Error::new(
        ErrorKind::Unsupported,
        "Safe publication unavailable",
    ));

    #[cfg(any(windows, target_os = "linux"))]
    {
        sync_directory(target.parent().expect("transaction target has a parent"))?;
        if source.parent() != target.parent() {
            sync_directory(source.parent().expect("transaction source has a parent"))?;
        }

        Ok(())
    }
}
