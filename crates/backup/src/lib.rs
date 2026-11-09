//! Selective file copy with strict preflight and content verification.
//! No source write, deletion, overwrite, privilege escalation, or link traversal.
use archrescue_core::{Error, ErrorKind, Operation, Result, State};
use archrescue_storage::{identity, require_separate, safe_existing, space, VolumeIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};

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
    for child in fs::read_dir(directory).map_err(|e| Error::io("List directory", e))? {
        let child = child.map_err(|e| Error::io("Read directory entry", e))?;
        let path = child.path();
        match safe_existing(&path)
            .and_then(|p| fs::metadata(p).map_err(|e| Error::io("Read file metadata", e)))
        {
            Ok(m) => entries.push(Entry {
                name: child.file_name().into(),
                directory: m.is_dir(),
                bytes: m.len(),
                modified_unix: m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()),
                error: None,
            }),
            Err(e) => entries.push(Entry {
                name: child.file_name().into(),
                directory: false,
                bytes: 0,
                modified_unix: None,
                error: Some(e),
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
        Self{operation:Operation::new(),planned_bytes:0,copied_bytes:0,files:vec![],directories:vec![],limitations:vec!["Content-only copy: ACLs, timestamps, alternate streams and sparse allocation are not preserved".into(),"Partial destination files are retained on failure and must not be treated as verified".into()]}
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
fn portable_relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::new(
            ErrorKind::Validation,
            "Selection must be a relative path without traversal",
        ));
    }
    for c in path.components() {
        let name = c
            .as_os_str()
            .to_str()
            .ok_or_else(|| Error::new(ErrorKind::Unsupported, "Non-Unicode filename"))?;
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        if name.ends_with([' ', '.'])
            || name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c))
            || [
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ]
            .contains(&stem.as_str())
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
    let mut p = Plan {
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
            if selection != other && (selection.starts_with(other) || other.starts_with(selection))
            {
                return Err(Error::new(ErrorKind::Validation, "Overlapping selections"));
            }
        }
        collect(&mut p, selection, &mut seen, cancel, 0)?;
    }
    if p.bytes > space(&p.destination)?.available {
        return Err(Error::new(
            ErrorKind::Validation,
            "Insufficient destination free space",
        ));
    }
    Ok(p)
}
fn collect(
    plan: &mut Plan,
    relative: &Path,
    seen: &mut HashSet<String>,
    cancel: &AtomicBool,
    depth: usize,
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
    let target = plan.destination.join(relative);
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            return Err(Error::new(
                ErrorKind::Conflict,
                "Destination entry already exists",
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::io("Check destination conflict", e)),
    }
    let md = fs::metadata(&source).map_err(|e| Error::io("Read selected metadata", e))?;
    if md.is_dir() {
        plan.directories.push(relative.to_owned());
        for entry in fs::read_dir(&source).map_err(|e| Error::io("Read selected directory", e))? {
            let entry = entry.map_err(|e| Error::io("Read selected entry", e))?;
            collect(
                plan,
                &relative.join(entry.file_name()),
                seen,
                cancel,
                depth + 1,
            )?;
        }
    } else if md.is_file() {
        File::open(&source).map_err(|e| Error::io("Check source readability", e))?;
        plan.bytes = plan
            .bytes
            .checked_add(md.len())
            .ok_or_else(|| Error::new(ErrorKind::Validation, "Selection size overflow"))?;
        plan.files.push(PlannedFile {
            relative: relative.to_owned(),
            length: md.len(),
        });
    } else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Special files are not supported",
        ));
    }
    Ok(())
}
fn revalidate(p: &Plan) -> Result<()> {
    safe_existing(&p.root)?;
    safe_existing(&p.destination)?;
    let src = identity(&p.root)?;
    let dst = identity(&p.destination)?;
    if src.volume != p.source_id.volume
        || dst.volume != p.destination_id.volume
        || src.physical_disks != p.source_id.physical_disks
        || dst.physical_disks != p.destination_id.physical_disks
    {
        return Err(Error::new(
            ErrorKind::Validation,
            "Storage identity changed",
        ));
    }
    require_separate(&src, &dst)
}
fn parent(p: &Plan, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| Error::new(ErrorKind::Validation, "Missing destination parent"))?;
    let relative = parent
        .strip_prefix(&p.destination)
        .map_err(|_| Error::new(ErrorKind::Validation, "Destination escaped root"))?;
    let mut current = p.destination.clone();
    for c in relative.components() {
        current.push(c);
        if !current
            .try_exists()
            .map_err(|e| Error::io("Probe destination parent", e))?
        {
            fs::create_dir(&current).map_err(|e| Error::io("Create destination directory", e))?;
        }
        safe_existing(&current)?;
        if identity(&current)?.volume != p.destination_id.volume {
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
        File::open(safe_existing(path)?).map_err(|e| Error::io("Open checksum file", e))?;
    hash_reader(&mut file, cancel)
}
fn hash_reader(file: &mut File, cancel: &AtomicBool) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        check_cancel(cancel)?;
        let n = file
            .read(&mut buffer)
            .map_err(|e| Error::io("Read checksum bytes", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
/// Progress is invoked only for bytes actually written. Cancellation is also checked during verification.
pub fn recover(
    root: &Path,
    selections: &[PathBuf],
    destination: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> BackupResult {
    let mut result = BackupResult::default();
    result
        .operation
        .transition(State::Validating)
        .expect("initial transition");
    let run = (|| -> Result<()> {
        let p = plan(root, selections, destination, cancel)?;
        result.planned_bytes = p.bytes;
        result.operation.transition(State::Running)?;
        execute(&p, &mut result, cancel, &mut progress)?;
        result.operation.transition(State::Verifying)?;
        revalidate(&p)?;
        for f in &mut result.files {
            check_cancel(cancel)?;
            if sha256(&p.destination.join(&f.relative), cancel)?
                != f.sha256.clone().unwrap_or_default()
                || sha256(&p.root.join(&f.relative), cancel)?
                    != f.sha256.clone().unwrap_or_default()
            {
                return Err(Error::new(
                    ErrorKind::Integrity,
                    "Source/destination checksum mismatch",
                ));
            }
            f.verified = true;
        }
        if result.copied_bytes != result.planned_bytes || result.files.len() != p.files.len() {
            return Err(Error::new(
                ErrorKind::Incomplete,
                "Backup byte/file count mismatch",
            ));
        }
        check_cancel(cancel)?;
        result.operation.transition(State::Completed)?;
        Ok(())
    })();
    if let Err(e) = run {
        result.operation.fail(e);
    }
    result
}
fn execute(
    p: &Plan,
    result: &mut BackupResult,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
) -> Result<()> {
    revalidate(p)?;
    for dir in &p.directories {
        check_cancel(cancel)?;
        let target = p.destination.join(dir);
        parent(p, &target)?;
        fs::create_dir(&target).map_err(|e| Error::io("Create selected directory", e))?;
        result.directories.push(dir.clone());
    }
    for item in &p.files {
        check_cancel(cancel)?;
        revalidate(p)?;
        let source = safe_existing(&p.root.join(&item.relative))?;
        if identity(&source)?.volume != p.source_id.volume {
            return Err(Error::new(
                ErrorKind::Validation,
                "Source file changed volume",
            ));
        }
        let mut input = File::open(&source).map_err(|e| Error::io("Open source file", e))?;
        let before = input
            .metadata()
            .map_err(|e| Error::io("Read source metadata", e))?;
        if before.len() != item.length {
            return Err(Error::new(
                ErrorKind::Integrity,
                "Source length changed after preflight",
            ));
        }
        let target = p.destination.join(&item.relative);
        parent(p, &target)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| Error::io("Create new destination file", e))?;
        result.files.push(FileResult {
            relative: item.relative.clone(),
            bytes: 0,
            sha256: None,
            verified: false,
        });
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut copied = 0u64;
        loop {
            check_cancel(cancel)?;
            let n = input
                .read(&mut buffer)
                .map_err(|e| Error::io("Read source bytes", e))?;
            if n == 0 {
                break;
            }
            if copied + n as u64 > item.length {
                return Err(Error::new(ErrorKind::Integrity, "Source grew during copy"));
            }
            // write() accounts for short writes so partial-copy byte counts remain exact.
            let mut offset = 0;
            while offset < n {
                check_cancel(cancel)?;
                let wrote = output
                    .write(&buffer[offset..n])
                    .map_err(|e| Error::io("Write destination bytes", e))?;
                if wrote == 0 {
                    return Err(Error::new(
                        ErrorKind::Io,
                        "Destination returned a zero-length write",
                    ));
                }
                offset += wrote;
                copied += wrote as u64;
                result.copied_bytes += wrote as u64;
                result.files.last_mut().unwrap().bytes = copied;
                progress(result.copied_bytes, p.bytes);
            }
            hash.update(&buffer[..n]);
        }
        output
            .sync_all()
            .map_err(|e| Error::io("Flush destination file", e))?;
        let after = input
            .metadata()
            .map_err(|e| Error::io("Recheck source metadata", e))?;
        if copied != item.length
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
