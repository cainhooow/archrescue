//! Selective, verified restoration to the current Windows user's known folders.

mod execution;
mod filesystem;
mod profile;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use archrescue_backup::{
    portable_relative,
    transaction::{verify_payload, ExpectedFile},
};
use archrescue_core::{Error, ErrorKind, Result};
use archrescue_storage::{safe_existing, space};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    filesystem::{exists, Directory},
    profile::PinnedProfile,
};

pub use execution::{execute, FileOutcome, FileReport, RestoreReport};
pub use profile::{Folder, Profile};

const ALLOCATION_ALLOWANCE: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConflictPolicy {
    Abort,
    Skip,
    Rename,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Action {
    Copy,
    Rename,
    Skip,
    Conflict,
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
pub struct Mapping {
    pub source: PathBuf,
    pub folder: Folder,
}

#[derive(Debug, Clone, Serialize)]
pub struct Request {
    pub backup: PathBuf,
    pub target_sid: String,
    pub mappings: Vec<Mapping>,
    pub conflicts: ConflictPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedFile {
    pub source: ExpectedFile,
    pub target_relative: Option<PathBuf>,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestorePlan {
    pub version: u32,
    pub backup_operation: String,
    pub manifest_version: u32,
    pub manifest_sha256: String,
    pub source_hint: PathBuf,
    pub target_sid: String,
    pub target_profile: PathBuf,
    pub profile_identity: String,
    pub known_folders: BTreeMap<Folder, (PathBuf, String)>,
    pub mappings: Vec<Mapping>,
    pub conflict_policy: ConflictPolicy,
    pub files: Vec<PlannedFile>,
    pub directories: Vec<PathBuf>,
    pub unsupported_directories: Vec<PathBuf>,
    pub planned_bytes: u64,
    pub required_bytes: u64,
    pub blocked_by_conflicts: bool,
    pub authorization_required: bool,
    pub fingerprint: String,
}

fn invalid(message: &str) -> Error {
    Error::new(ErrorKind::Validation, message)
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::new(ErrorKind::Cancelled, "Restauração cancelada"));
    }

    Ok(())
}

fn relative_key(path: &Path) -> String {
    path.to_string_lossy().to_uppercase()
}

fn validate_mappings(mappings: &[Mapping]) -> Result<()> {
    if mappings.is_empty() {
        return Err(invalid("Mapeie ao menos uma pasta da origem"));
    }

    let mut target_folders = BTreeSet::new();
    let mut source_folders: Vec<PathBuf> = Vec::new();
    for mapping in mappings {
        portable_relative(&mapping.source)?;
        let source = PathBuf::from(relative_key(&mapping.source));
        if !target_folders.insert(mapping.folder)
            || source_folders
                .iter()
                .any(|other| source.starts_with(other) || other.starts_with(&source))
        {
            return Err(invalid("Mapeamentos de pastas se sobrepõem"));
        }

        source_folders.push(source);
    }

    Ok(())
}

fn mapped(relative: &Path, mappings: &[Mapping], profile: &PinnedProfile) -> Option<PathBuf> {
    mappings.iter().find_map(|mapping| {
        let suffix = relative.strip_prefix(&mapping.source).ok()?;
        Some(profile.selected[&mapping.folder].path().join(suffix))
    })
}

fn inspect_parent(path: &Path, profile: &PinnedProfile) -> Result<()> {
    let relative = profile.relative(path)?;
    let mut current = profile.profile.path().to_owned();
    for component in relative.components() {
        current.push(component);
        if !exists(&current)? {
            break;
        }
        Directory::open(&current)?;
    }

    Ok(())
}

fn conflict_name(
    target: &Path,
    reserved: &BTreeSet<String>,
    profile: &PinnedProfile,
) -> Result<PathBuf> {
    let stem = target
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| invalid("Nome de arquivo inválido"))?;
    for index in 1..=10_000 {
        let mut name = format!("{stem} (ArchRescue {index})");
        if let Some(extension) = target.extension().and_then(|extension| extension.to_str()) {
            name.push('.');
            name.push_str(extension);
        }

        let path = target.with_file_name(name);
        if !reserved.contains(&relative_key(profile.relative(&path)?)) && !exists(&path)? {
            return Ok(path);
        }
    }

    Err(Error::new(
        ErrorKind::Conflict,
        "Nenhum nome alternativo disponível",
    ))
}

pub fn preview(request: &Request, cancel: &AtomicBool) -> Result<RestorePlan> {
    let profile = Profile::current()?;
    let (plan, _, _) = prepare(request, &profile, cancel)?;

    Ok(plan)
}

fn prepare(
    request: &Request,
    profile: &Profile,
    cancel: &AtomicBool,
) -> Result<(RestorePlan, PinnedProfile, Directory)> {
    validate_mappings(&request.mappings)?;
    let target = profile.pin(
        &request.target_sid,
        request.mappings.iter().map(|mapping| mapping.folder),
    )?;
    let _original_backup = Directory::open(&request.backup)?;
    let backup = safe_existing(&request.backup)?;
    if backup.starts_with(target.profile.path()) || target.profile.path().starts_with(&backup) {
        return Err(invalid(
            "O backup e o perfil devem ocupar árvores de diretórios distintas",
        ));
    }

    let payload = Directory::open(&backup.join("payload"))?;
    let manifest = verify_payload(&backup, cancel)?;
    for mapping in &request.mappings {
        if !manifest.directories.contains(&mapping.source) {
            return Err(invalid(
                "A origem mapeada deve ser um diretório do manifesto",
            ));
        }
    }

    let mut reserved = BTreeSet::new();
    let mut directories = Vec::new();
    let mut unsupported_directories = Vec::new();
    for directory in &manifest.directories {
        check_cancel(cancel)?;
        if let Some(path) = mapped(directory, &request.mappings, &target) {
            inspect_parent(&path, &target)?;
            let relative = target.relative(&path)?.to_owned();
            if !reserved.insert(relative_key(&relative)) {
                return Err(invalid("Dois diretórios ocupam o mesmo caminho no destino"));
            }
            directories.push(relative);
        } else {
            unsupported_directories.push(directory.clone());
        }
    }

    for entry in &manifest.files {
        if let Some(path) = mapped(&entry.relative, &request.mappings, &target) {
            if !reserved.insert(relative_key(target.relative(&path)?)) {
                return Err(invalid("Dois itens ocupam o mesmo caminho no destino"));
            }
        }
    }

    let mut files = Vec::new();
    let mut planned_bytes = 0u64;
    let mut required_bytes = 0u64;
    let mut blocked_by_conflicts = false;
    for entry in manifest.files {
        check_cancel(cancel)?;
        let Some(path) = mapped(&entry.relative, &request.mappings, &target) else {
            files.push(PlannedFile {
                source: entry,
                target_relative: None,
                action: Action::Unsupported,
            });
            continue;
        };
        let parent = path
            .parent()
            .ok_or_else(|| invalid("Arquivo sem diretório pai"))?;
        inspect_parent(parent, &target)?;

        let (path, action) = if exists(&path)? {
            match request.conflicts {
                ConflictPolicy::Abort => {
                    blocked_by_conflicts = true;
                    (path, Action::Conflict)
                }
                ConflictPolicy::Skip => (path, Action::Skip),
                ConflictPolicy::Rename => {
                    let path = conflict_name(&path, &reserved, &target)?;
                    reserved.insert(relative_key(target.relative(&path)?));
                    (path, Action::Rename)
                }
            }
        } else {
            (path, Action::Copy)
        };

        if matches!(action, Action::Copy | Action::Rename) {
            planned_bytes = planned_bytes
                .checked_add(entry.bytes)
                .ok_or_else(|| invalid("Tamanho total excedido"))?;
            required_bytes = required_bytes
                .checked_add(entry.bytes)
                .and_then(|bytes| bytes.checked_add(ALLOCATION_ALLOWANCE))
                .ok_or_else(|| invalid("Espaço necessário excedido"))?;
        }
        files.push(PlannedFile {
            source: entry,
            target_relative: Some(target.relative(&path)?.into()),
            action,
        });
    }
    required_bytes = required_bytes
        .checked_add((directories.len() as u64).saturating_mul(ALLOCATION_ALLOWANCE))
        .ok_or_else(|| invalid("Espaço necessário excedido"))?;
    require_capacity(target.profile.path(), required_bytes)?;

    let mut plan = RestorePlan {
        version: 1,
        backup_operation: manifest.operation_id,
        manifest_version: manifest.schema_version,
        manifest_sha256: manifest.plan_fingerprint,
        source_hint: manifest.source_hint,
        target_sid: request.target_sid.clone(),
        target_profile: target.profile.path().to_owned(),
        profile_identity: target.profile.key().into(),
        known_folders: target
            .selected
            .iter()
            .map(|(kind, directory)| (*kind, (directory.path().to_owned(), directory.key().into())))
            .collect(),
        mappings: request.mappings.clone(),
        conflict_policy: request.conflicts,
        files,
        directories,
        unsupported_directories,
        planned_bytes,
        required_bytes,
        blocked_by_conflicts,
        authorization_required: true,
        fingerprint: String::new(),
    };
    let bytes = serde_json::to_vec(&plan).map_err(|error| invalid(&error.to_string()))?;
    plan.fingerprint = format!("{:x}", Sha256::digest(bytes));

    Ok((plan, target, payload))
}

fn require_capacity(path: &Path, bytes: u64) -> Result<()> {
    if space(path)?.available < bytes {
        return Err(Error::new(
            ErrorKind::Environment,
            "Espaço insuficiente no perfil de destino",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests;
