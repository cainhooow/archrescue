//! Selective file copy with strict preflight and content verification.
//! No source write, deletion, overwrite, privilege escalation, or link traversal.

use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};

use archrescue_core::{Error, ErrorKind, Operation, Result, State};
use archrescue_storage::{identity, require_separate, safe_existing, space, VolumeIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod transaction;

const RESERVED_DEVICE_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: PathBuf,
    pub directory: bool,
    pub bytes: u64,
    pub modified_unix: Option<u64>,
    pub error: Option<Error>,
}

pub fn browse(directory: &Path) -> Result<Vec<Entry>> {
    let directory = safe_existing(directory)?;
    let mut entries = Vec::new();

    for child in fs::read_dir(directory).map_err(|error| Error::io("List directory", error))? {
        let child = child.map_err(|error| Error::io("Read directory entry", error))?;
        let path = child.path();
        let metadata = safe_existing(&path).and_then(|path| {
            fs::metadata(path).map_err(|error| Error::io("Read file metadata", error))
        });

        match metadata {
            Ok(metadata) => entries.push(Entry {
                name: child.file_name().into(),
                directory: metadata.is_dir(),
                bytes: metadata.len(),
                modified_unix: metadata
                    .modified()
                    .ok()
                    .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs()),
                error: None,
            }),
            Err(error) => entries.push(Entry {
                name: child.file_name().into(),
                directory: false,
                bytes: 0,
                modified_unix: None,
                error: Some(error),
            }),
        }
    }

    entries.sort_by(|a, b| b.directory.cmp(&a.directory).then(a.name.cmp(&b.name)));

    Ok(entries)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    pub relative: PathBuf,
    pub bytes: u64,
    pub sha256: Option<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub operation: Operation,
    pub planned_bytes: u64,
    pub copied_bytes: u64,
    pub files: Vec<FileResult>,
    pub directories: Vec<PathBuf>,
    pub limitations: Vec<String>,
}

impl Default for BackupResult {
    fn default() -> Self {
        Self {
            operation: Operation::new(),
            planned_bytes: 0,
            copied_bytes: 0,
            files: vec![],
            directories: vec![],
            limitations: vec![
                "Content-only copy: ACLs, timestamps, alternate streams and sparse allocation \
                 are not preserved"
                    .into(),
                "Partial destination files are retained on failure and must not be treated as \
                 verified"
                    .into(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
struct PlannedFile {
    relative: PathBuf,
    length: u64,
}

#[derive(Debug)]
pub struct Plan {
    root: PathBuf,
    destination: PathBuf,
    source_id: VolumeIdentity,
    destination_id: VolumeIdentity,
    files: Vec<PlannedFile>,
    directories: Vec<PathBuf>,
    pub bytes: u64,
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::new(ErrorKind::Cancelled, "Cancelled by user"))
    } else {
        Ok(())
    }
}

pub fn portable_relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::new(
            ErrorKind::Validation,
            "Selection must be a relative path without traversal",
        ));
    }

    for component in path.components() {
        let name = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| Error::new(ErrorKind::Unsupported, "Non-Unicode filename"))?;
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();

        if name.ends_with([' ', '.'])
            || name
                .chars()
                .any(|character| character < ' ' || "<>:\"/\\|?*".contains(character))
            || RESERVED_DEVICE_NAMES.contains(&stem.as_str())
        {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "Filename is ambiguous on Windows",
            ));
        }
    }

    Ok(())
}

/// Preflight is read-only. Selection paths are relative to root; destination must exist.
pub fn plan(
    root: &Path,
    selections: &[PathBuf],
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<Plan> {
    let backup_plan = scan(root, selections, destination, cancel, true)?;

    if backup_plan.bytes > space(&backup_plan.destination)?.available {
        return Err(Error::new(
            ErrorKind::Validation,
            "Insufficient destination free space",
        ));
    }

    Ok(backup_plan)
}

fn scan(
    root: &Path,
    selections: &[PathBuf],
    destination: &Path,
    cancel: &AtomicBool,
    check_conflicts: bool,
) -> Result<Plan> {
    check_cancel(cancel)?;

    let root = safe_existing(root)?;
    let destination = safe_existing(destination)?;

    if !root.is_dir() || !destination.is_dir() || selections.is_empty() {
        return Err(Error::new(
            ErrorKind::Validation,
            "Source/destination directories and a nonempty selection are required",
        ));
    }

    if destination.starts_with(&root) || root.starts_with(&destination) {
        return Err(Error::new(
            ErrorKind::Validation,
            "Source and destination trees overlap",
        ));
    }

    let source_id = identity(&root)?;
    let destination_id = identity(&destination)?;
    require_separate(&source_id, &destination_id)?;

    let mut backup_plan = Plan {
        root,
        destination,
        source_id,
        destination_id,
        files: vec![],
        directories: vec![],
        bytes: 0,
    };
    let mut seen = HashSet::new();

    for selection in selections {
        portable_relative(selection)?;
        for other in selections {
            if selection == other {
                continue;
            }

            if selection.starts_with(other) || other.starts_with(selection) {
                return Err(Error::new(ErrorKind::Validation, "Overlapping selections"));
            }
        }

        collect(
            &mut backup_plan,
            selection,
            &mut seen,
            cancel,
            0,
            check_conflicts,
        )?;
    }

    Ok(backup_plan)
}

fn collect(
    plan: &mut Plan,
    relative: &Path,
    seen: &mut HashSet<String>,
    cancel: &AtomicBool,
    depth: usize,
    check_conflicts: bool,
) -> Result<()> {
    check_cancel(cancel)?;
    portable_relative(relative)?;

    if depth > 128 || seen.len() >= 1_000_000 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Selection exceeds traversal limits",
        ));
    }

    let key = relative.to_string_lossy().to_lowercase();
    if !seen.insert(key) {
        return Err(Error::new(
            ErrorKind::Conflict,
            "Duplicate or case-colliding selection",
        ));
    }

    let source = safe_existing(&plan.root.join(relative))?;
    if !source.starts_with(&plan.root) || identity(&source)?.volume != plan.source_id.volume {
        return Err(Error::new(
            ErrorKind::Validation,
            "Selection crosses the source volume boundary",
        ));
    }

    if check_conflicts {
        let target = plan.destination.join(relative);
        match fs::symlink_metadata(&target) {
            Ok(_) => {
                return Err(Error::new(
                    ErrorKind::Conflict,
                    "Destination entry already exists",
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::io("Check destination conflict", error)),
        }
    }

    let metadata =
        fs::metadata(&source).map_err(|error| Error::io("Read selected metadata", error))?;
    if metadata.is_dir() {
        plan.directories.push(relative.to_owned());

        let children =
            fs::read_dir(&source).map_err(|error| Error::io("Read selected directory", error))?;
        for entry in children {
            let entry = entry.map_err(|error| Error::io("Read selected entry", error))?;
            collect(
                plan,
                &relative.join(entry.file_name()),
                seen,
                cancel,
                depth + 1,
                check_conflicts,
            )?;
        }
    } else if metadata.is_file() {
        File::open(&source).map_err(|error| Error::io("Check source readability", error))?;

        plan.bytes = plan
            .bytes
            .checked_add(metadata.len())
            .ok_or_else(|| Error::new(ErrorKind::Validation, "Selection size overflow"))?;
        plan.files.push(PlannedFile {
            relative: relative.to_owned(),
            length: metadata.len(),
        });
    } else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Special files are not supported",
        ));
    }

    Ok(())
}

fn revalidate(backup_plan: &Plan) -> Result<()> {
    safe_existing(&backup_plan.root)?;
    safe_existing(&backup_plan.destination)?;

    let source_identity = identity(&backup_plan.root)?;
    let destination_identity = identity(&backup_plan.destination)?;

    if source_identity.volume != backup_plan.source_id.volume
        || destination_identity.volume != backup_plan.destination_id.volume
        || source_identity.physical_disks != backup_plan.source_id.physical_disks
        || destination_identity.physical_disks != backup_plan.destination_id.physical_disks
    {
        return Err(Error::new(
            ErrorKind::Validation,
            "Storage identity changed",
        ));
    }

    require_separate(&source_identity, &destination_identity)
}

fn prepare_destination_parent(backup_plan: &Plan, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| Error::new(ErrorKind::Validation, "Missing destination parent"))?;
    let relative = parent
        .strip_prefix(&backup_plan.destination)
        .map_err(|_| Error::new(ErrorKind::Validation, "Destination escaped root"))?;

    let mut current = backup_plan.destination.clone();
    for component in relative.components() {
        current.push(component);

        if !current
            .try_exists()
            .map_err(|error| Error::io("Probe destination parent", error))?
        {
            fs::create_dir(&current)
                .map_err(|error| Error::io("Create destination directory", error))?;
        }

        safe_existing(&current)?;
        if identity(&current)?.volume != backup_plan.destination_id.volume {
            return Err(Error::new(
                ErrorKind::Validation,
                "Destination crosses volume boundary",
            ));
        }
    }

    Ok(())
}

pub fn sha256(path: &Path, cancel: &AtomicBool) -> Result<String> {
    let mut file =
        File::open(safe_existing(path)?).map_err(|error| Error::io("Open checksum file", error))?;

    hash_reader(&mut file, cancel)
}

pub fn hash_reader(file: &mut File, cancel: &AtomicBool) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        check_cancel(cancel)?;
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|error| Error::io("Read checksum bytes", error))?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Progress is invoked only for bytes actually written. Cancellation is also checked during verification.
pub fn recover(
    root: &Path,
    selections: &[PathBuf],
    destination: &Path,
    cancel: &AtomicBool,
    progress: impl FnMut(u64, u64),
) -> BackupResult {
    recover_with_writer(root, selections, destination, cancel, progress, File::write)
}

fn recover_with_writer(
    root: &Path,
    selections: &[PathBuf],
    destination: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
    mut write_destination: impl FnMut(&mut File, &[u8]) -> io::Result<usize>,
) -> BackupResult {
    let mut result = BackupResult::default();
    result
        .operation
        .transition(State::Validating)
        .expect("initial transition");

    let outcome = (|| -> Result<()> {
        let backup_plan = plan(root, selections, destination, cancel)?;
        result.planned_bytes = backup_plan.bytes;

        result.operation.transition(State::Running)?;
        execute(
            &backup_plan,
            &mut result,
            cancel,
            &mut progress,
            &mut write_destination,
        )?;

        result.operation.transition(State::Verifying)?;
        verify(&backup_plan, &mut result, cancel)?;

        check_cancel(cancel)?;
        result.operation.transition(State::Completed)?;
        Ok(())
    })();

    if let Err(error) = outcome {
        result.operation.fail(error);
    }

    result
}

fn verify(backup_plan: &Plan, result: &mut BackupResult, cancel: &AtomicBool) -> Result<()> {
    revalidate(backup_plan)?;

    for file in &mut result.files {
        check_cancel(cancel)?;

        let expected_hash = file.sha256.as_deref().unwrap_or_default();
        let destination_hash = sha256(&backup_plan.destination.join(&file.relative), cancel)?;

        if destination_hash != expected_hash
            || sha256(&backup_plan.root.join(&file.relative), cancel)? != expected_hash
        {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Source/destination checksum mismatch",
            ));
        }

        file.verified = true;
    }

    if result.copied_bytes != result.planned_bytes || result.files.len() != backup_plan.files.len()
    {
        return Err(Error::new(
            ErrorKind::Incomplete,
            "Backup byte/file count mismatch",
        ));
    }

    Ok(())
}

fn execute(
    backup_plan: &Plan,
    result: &mut BackupResult,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
    write_destination: &mut impl FnMut(&mut File, &[u8]) -> io::Result<usize>,
) -> Result<()> {
    revalidate(backup_plan)?;

    for relative_directory in &backup_plan.directories {
        check_cancel(cancel)?;
        let target = backup_plan.destination.join(relative_directory);
        prepare_destination_parent(backup_plan, &target)?;
        fs::create_dir(&target).map_err(|error| Error::io("Create selected directory", error))?;

        result.directories.push(relative_directory.clone());
    }

    for planned_file in &backup_plan.files {
        check_cancel(cancel)?;
        revalidate(backup_plan)?;

        let source = safe_existing(&backup_plan.root.join(&planned_file.relative))?;
        if identity(&source)?.volume != backup_plan.source_id.volume {
            return Err(Error::new(
                ErrorKind::Validation,
                "Source file changed volume",
            ));
        }

        let mut input =
            File::open(&source).map_err(|error| Error::io("Open source file", error))?;
        let before = input
            .metadata()
            .map_err(|error| Error::io("Read source metadata", error))?;

        if before.len() != planned_file.length {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Source length changed after preflight",
            ));
        }

        let target = backup_plan.destination.join(&planned_file.relative);
        prepare_destination_parent(backup_plan, &target)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|error| Error::io("Create new destination file", error))?;

        result.files.push(FileResult {
            relative: planned_file.relative.clone(),
            bytes: 0,
            sha256: None,
            verified: false,
        });

        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut file_bytes_copied = 0u64;

        loop {
            check_cancel(cancel)?;
            let bytes_read = input
                .read(&mut buffer)
                .map_err(|error| Error::io("Read source bytes", error))?;

            if bytes_read == 0 {
                break;
            }
            if file_bytes_copied + bytes_read as u64 > planned_file.length {
                return Err(Error::new(ErrorKind::Integrity, "Source grew during copy"));
            }

            // write() accounts for short writes so partial-copy byte counts remain exact.
            let mut offset = 0;
            while offset < bytes_read {
                check_cancel(cancel)?;
                let bytes_written = write_destination(&mut output, &buffer[offset..bytes_read])
                    .map_err(|error| Error::io("Write destination bytes", error))?;
                if bytes_written == 0 {
                    return Err(Error::new(
                        ErrorKind::Io,
                        "Destination returned a zero-length write",
                    ));
                }

                offset += bytes_written;
                file_bytes_copied += bytes_written as u64;
                result.copied_bytes += bytes_written as u64;
                result.files.last_mut().unwrap().bytes = file_bytes_copied;
                progress(result.copied_bytes, backup_plan.bytes);
            }

            hash.update(&buffer[..bytes_read]);
        }

        output
            .sync_all()
            .map_err(|error| Error::io("Flush destination file", error))?;

        let after = input
            .metadata()
            .map_err(|error| Error::io("Recheck source metadata", error))?;

        if file_bytes_copied != planned_file.length
            || before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
        {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Source changed during copy",
            ));
        }

        result.files.last_mut().unwrap().sha256 = Some(format!("{:x}", hash.finalize()));
    }

    Ok(())
}

#[cfg(test)]
mod tests;
