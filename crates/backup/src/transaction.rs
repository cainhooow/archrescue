//! Durable selective backups. Data becomes visible in payload/ only after a full readback.

mod journal;

use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};

use archrescue_storage::{recovery_location, RecoveryLocation};

use super::*;

const SCHEMA: u32 = 1;
const PREFIX: &str = "archrescue-backup-";
const INTERVAL: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedFile {
    pub relative: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub schema_version: u32,
    pub operation_id: String,
    pub created_unix: u64,
    pub source_identity: RecoveryLocation,
    pub destination_identity: RecoveryLocation,
    pub source_hint: PathBuf,
    pub source_at_creation: VolumeIdentity,
    pub destination_at_creation: VolumeIdentity,
    pub selections: Vec<PathBuf>,
    pub directories: Vec<PathBuf>,
    pub files: Vec<ExpectedFile>,
    pub planned_bytes: u64,
    pub plan_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileState {
    Pending,
    Copying,
    Partial,
    ReadyToPublish,
    Verified,
    Failed,
    Cancelled,
    SourceChanged,
    DestinationConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCheckpoint {
    pub plan_fingerprint: String,
    pub index: usize,
    pub updated_unix: u64,
    pub state: FileState,
    pub acknowledged_bytes: u64,
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatus {
    pub operation_id: String,
    pub plan_fingerprint: String,
    pub updated_unix: u64,
    pub state: State,
    pub resumed: bool,
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    pub operation_id: String,
    pub stage: String,
    pub elapsed_seconds: u64,
    pub planned_bytes: u64,
    pub acknowledged_bytes: u64,
    pub verified_bytes: u64,
    pub verified_files: usize,
    pub skipped_files: usize,
    pub failed_files: usize,
    pub partial_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReport {
    pub directory: PathBuf,
    pub source_root: PathBuf,
    pub manifest: BackupManifest,
    pub status: TransactionStatus,
    pub progress: Progress,
    pub files: Vec<FileCheckpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointEntry {
    pub directory: PathBuf,
    pub manifest: Option<BackupManifest>,
    pub status: Option<TransactionStatus>,
    pub error: Option<Error>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn changed(message: &str) -> Error {
    Error::new(ErrorKind::Integrity, message)
}

fn bad_destination(message: &str) -> Error {
    Error::new(ErrorKind::Conflict, message)
}

fn operation_name(id: &str) -> String {
    format!("{PREFIX}{id}")
}

fn fingerprint(manifest: &BackupManifest) -> Result<String> {
    let mut plan = manifest.clone();
    plan.plan_fingerprint.clear();

    journal::fingerprint(&plan)
}

fn directories(plan: &Plan) -> Vec<PathBuf> {
    let mut directories = BTreeSet::new();
    for path in plan
        .directories
        .iter()
        .chain(plan.files.iter().map(|file| &file.relative))
    {
        for parent in path
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
        {
            directories.insert(parent.to_owned());
        }
    }
    directories.extend(plan.directories.iter().cloned());

    directories.into_iter().collect()
}

fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir(path).map_err(|error| Error::io("Create transaction directory", error))?;
    journal::sync_directory(path.parent().expect("transaction directory has a parent"))
}

fn location_changed() -> Error {
    Error::new(
        ErrorKind::Validation,
        "Identidade da origem ou do destino mudou",
    )
}

pub fn preview(
    source: &Path,
    selections: &[PathBuf],
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<(usize, u64)> {
    let plan = scan(source, selections, destination, cancel, false)?;
    recovery_location(&plan.root)?;
    recovery_location(&plan.destination)?;
    if plan.bytes > space(&plan.destination)?.available {
        return Err(Error::new(
            ErrorKind::Validation,
            "Espaço insuficiente no destino",
        ));
    }

    Ok((plan.files.len(), plan.bytes))
}

/// Hash the complete selection before creating a transaction on the independent destination.
pub fn prepare(
    source: &Path,
    selections: &[PathBuf],
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    let mut plan = scan(source, selections, destination, cancel, false)?;
    let source_identity = recovery_location(&plan.root)?;
    let destination_identity = recovery_location(&plan.destination)?;

    if plan.bytes > space(&plan.destination)?.available {
        return Err(Error::new(
            ErrorKind::Validation,
            "Espaço insuficiente no destino",
        ));
    }

    plan.files
        .sort_by(|left, right| left.relative.cmp(&right.relative));
    let mut files = Vec::with_capacity(plan.files.len());
    for file in &plan.files {
        files.push(ExpectedFile {
            relative: file.relative.clone(),
            bytes: file.length,
            sha256: hash_source(&plan, &file.relative, file.length, cancel)?,
        });
    }

    let id = Operation::new().id;
    let directory = plan.destination.join(operation_name(&id));
    let mut manifest = BackupManifest {
        schema_version: SCHEMA,
        operation_id: id,
        created_unix: now(),
        source_identity,
        destination_identity,
        source_hint: plan.root.clone(),
        source_at_creation: plan.source_id.clone(),
        destination_at_creation: plan.destination_id.clone(),
        selections: selections.to_vec(),
        directories: directories(&plan),
        files,
        planned_bytes: plan.bytes,
        plan_fingerprint: String::new(),
    };
    manifest.plan_fingerprint = fingerprint(&manifest)?;

    check_cancel(cancel)?;
    revalidate(&plan)?;
    if recovery_location(&plan.root)? != manifest.source_identity
        || recovery_location(&plan.destination)? != manifest.destination_identity
    {
        return Err(location_changed());
    }

    create_directory(&directory)?;
    create_directory(&directory.join("payload"))?;
    create_directory(&directory.join("journal"))?;
    journal::save(&directory.join("manifest.json"), &manifest, false)?;

    Ok(directory)
}

pub fn manifest(directory: &Path) -> Result<BackupManifest> {
    let directory = safe_existing(directory)?;
    let manifest: BackupManifest = journal::read(&directory.join("manifest.json"))?;
    let expected_name = operation_name(&manifest.operation_id);
    if manifest.schema_version != SCHEMA
        || directory.file_name().and_then(|name| name.to_str()) != Some(expected_name.as_str())
        || fingerprint(&manifest)? != manifest.plan_fingerprint
    {
        return Err(changed(
            "Versão, identificador ou plano do checkpoint inválido",
        ));
    }

    for relative in manifest
        .selections
        .iter()
        .chain(&manifest.directories)
        .chain(manifest.files.iter().map(|file| &file.relative))
    {
        portable_relative(relative)?;
    }

    Ok(manifest)
}

/// Verify the selected payload against its manifest without opening the original source.
pub fn verify_payload(directory: &Path, cancel: &AtomicBool) -> Result<BackupManifest> {
    let directory = safe_existing(directory)?;
    let manifest = manifest(&directory)?;
    let payload = safe_existing(&directory.join("payload"))?;
    let volume = identity(&directory)?;

    if !payload.is_dir() || identity(&payload)?.volume != volume.volume {
        return Err(location_changed());
    }

    let mut total = 0u64;
    for entry in &manifest.files {
        check_cancel(cancel)?;
        verify_file(&payload.join(&entry.relative), entry, &volume, cancel)?;
        total = total
            .checked_add(entry.bytes)
            .ok_or_else(|| changed("Tamanho do backup excedido"))?;
    }

    for entry in &manifest.directories {
        check_cancel(cancel)?;
        let directory = safe_existing(&payload.join(entry))?;
        if !directory.is_dir() || identity(&directory)?.volume != volume.volume {
            return Err(location_changed());
        }
    }

    if total != manifest.planned_bytes {
        return Err(changed("Tamanho do backup diverge do manifesto"));
    }

    Ok(manifest)
}

pub fn checkpoints(destination: &Path) -> Result<Vec<CheckpointEntry>> {
    let destination = safe_existing(destination)?;
    let mut entries = Vec::new();
    for child in fs::read_dir(destination).map_err(|error| Error::io("List backups", error))? {
        let child = child.map_err(|error| Error::io("Read backup entry", error))?;
        if !child.file_name().to_string_lossy().starts_with(PREFIX) {
            continue;
        }

        let directory = child.path();
        match manifest(&directory) {
            Ok(manifest) => {
                let status = read_status(&directory, &manifest);
                let (status, error) = match status {
                    Ok(status) => (status, None),
                    Err(error) => (None, Some(error)),
                };

                entries.push(CheckpointEntry {
                    directory,
                    manifest: Some(manifest),
                    status,
                    error,
                });
            }
            Err(error) => entries.push(CheckpointEntry {
                directory,
                manifest: None,
                status: None,
                error: Some(error),
            }),
        }
    }

    entries.sort_by(|left, right| left.directory.cmp(&right.directory));

    Ok(entries)
}

fn read_status(directory: &Path, manifest: &BackupManifest) -> Result<Option<TransactionStatus>> {
    let path = directory.join("status.json");
    if !journal::exists(&path)? {
        return Ok(None);
    }

    let status: TransactionStatus = journal::read(&path)?;
    if status.operation_id != manifest.operation_id
        || status.plan_fingerprint != manifest.plan_fingerprint
    {
        return Err(changed("Estado pertence a outra operação"));
    }

    Ok(Some(status))
}

fn file_checkpoint(
    directory: &Path,
    manifest: &BackupManifest,
    index: usize,
) -> Result<FileCheckpoint> {
    let path = checkpoint_path(directory, index);
    if !journal::exists(&path)? {
        return Ok(FileCheckpoint {
            plan_fingerprint: manifest.plan_fingerprint.clone(),
            index,
            updated_unix: now(),
            state: FileState::Pending,
            acknowledged_bytes: 0,
            error: None,
        });
    }

    let checkpoint: FileCheckpoint = journal::read(&path)?;
    let length = manifest.files[index].bytes;
    if checkpoint.index != index
        || checkpoint.plan_fingerprint != manifest.plan_fingerprint
        || checkpoint.acknowledged_bytes > length
        || (matches!(
            checkpoint.state,
            FileState::ReadyToPublish | FileState::Verified
        ) && checkpoint.acknowledged_bytes != length)
    {
        return Err(changed("Registro de arquivo incompatível com o plano"));
    }

    Ok(checkpoint)
}

fn checkpoint_path(directory: &Path, index: usize) -> PathBuf {
    directory.join("journal").join(format!("{index}.json"))
}

fn staging_path(directory: &Path, index: usize) -> PathBuf {
    directory.join("journal").join(format!("{index}.part"))
}

fn save_file(directory: &Path, checkpoint: &mut FileCheckpoint, state: FileState) -> Result<()> {
    checkpoint.state = state;
    checkpoint.updated_unix = now();

    journal::save(
        &checkpoint_path(directory, checkpoint.index),
        checkpoint,
        true,
    )
}

fn hash_source(plan: &Plan, relative: &Path, bytes: u64, cancel: &AtomicBool) -> Result<String> {
    let path = safe_existing(&plan.root.join(relative))?;
    if identity(&path)?.volume != plan.source_id.volume {
        return Err(location_changed());
    }

    hash_file(&path, bytes, cancel)
}

fn hash_file(path: &Path, bytes: u64, cancel: &AtomicBool) -> Result<String> {
    let mut file = File::open(safe_existing(path)?)
        .map_err(|error| Error::io("Open file to verify", error))?;
    let before = file
        .metadata()
        .map_err(|error| Error::io("Read file metadata", error))?;
    if !before.is_file() || before.len() != bytes {
        return Err(changed("Tamanho ou tipo de arquivo mudou"));
    }

    let hash = hash_reader(&mut file, cancel)?;
    let after = file
        .metadata()
        .map_err(|error| Error::io("Recheck file metadata", error))?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(changed("Arquivo mudou durante a leitura"));
    }

    Ok(hash)
}

fn verify_file(
    path: &Path,
    file: &ExpectedFile,
    volume: &VolumeIdentity,
    cancel: &AtomicBool,
) -> Result<()> {
    let path = safe_existing(path)?;
    if identity(&path)?.volume != volume.volume {
        return Err(location_changed());
    }

    if hash_file(&path, file.bytes, cancel)? != file.sha256 {
        return Err(changed("SHA-256 do destino diverge do plano"));
    }

    Ok(())
}

fn check_tree(
    plan: &Plan,
    directory: &Path,
    files: &BTreeSet<&Path>,
    directories: &BTreeSet<&Path>,
    cancel: &AtomicBool,
) -> Result<()> {
    check_cancel(cancel)?;
    for entry in fs::read_dir(directory).map_err(|error| Error::io("Read backup payload", error))? {
        let entry = entry.map_err(|error| Error::io("Read payload entry", error))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(&plan.destination)
            .expect("entry is under payload");
        let path = safe_existing(&path)?;
        if identity(&path)?.volume != plan.destination_id.volume {
            return Err(location_changed());
        }

        if path.is_dir() && directories.contains(relative) {
            check_tree(plan, &path, files, directories, cancel)?;
        } else if !path.is_file() || !files.contains(relative) {
            return Err(bad_destination("Entrada inesperada no diretório de backup"));
        }
    }

    Ok(())
}

/// Resume the exact operation selected by directory. Completed data is reread before reuse.
pub fn resume(
    source: &Path,
    directory: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(&Progress),
) -> Result<TransactionReport> {
    run(source, directory, cancel, &mut progress, false)
}

/// Independently recheck a completed transaction. This does not write to either device.
pub fn verify(source: &Path, directory: &Path, cancel: &AtomicBool) -> Result<TransactionReport> {
    run(source, directory, cancel, &mut |_| {}, true)
}

fn run(
    source: &Path,
    directory: &Path,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(&Progress),
    verify_only: bool,
) -> Result<TransactionReport> {
    check_cancel(cancel)?;
    let directory = safe_existing(directory)?;
    let manifest = manifest(&directory)?;
    let destination = directory.parent().expect("operation has a parent");
    let plan = scan(
        source,
        &manifest.selections,
        &directory.join("payload"),
        cancel,
        false,
    )?;

    if recovery_location(&plan.root)? != manifest.source_identity
        || recovery_location(destination)? != manifest.destination_identity
    {
        return Err(location_changed());
    }

    let mut names: Vec<_> = plan
        .files
        .iter()
        .map(|file| (&file.relative, file.length))
        .collect();
    names.sort_unstable();
    if !names.into_iter().eq(manifest
        .files
        .iter()
        .map(|file| (&file.relative, file.bytes)))
        || directories(&plan) != manifest.directories
        || plan.bytes != manifest.planned_bytes
    {
        return Err(changed(
            "A seleção de origem mudou desde a criação do plano",
        ));
    }

    let journal_dir = safe_existing(&directory.join("journal"))?;
    for path in [destination, &directory, &journal_dir] {
        if identity(path)?.volume != plan.destination_id.volume {
            return Err(location_changed());
        }
    }

    let _lock = if verify_only {
        None
    } else {
        Some(journal::lock(&journal_dir.join("operation.lock"))?)
    };
    let previous = read_status(&directory, &manifest)?;
    if verify_only
        && !previous
            .as_ref()
            .is_some_and(|status| status.state == State::Completed)
    {
        return Err(Error::new(
            ErrorKind::Incomplete,
            "O backup ainda não foi concluído",
        ));
    }

    let files = (0..manifest.files.len())
        .map(|index| file_checkpoint(&directory, &manifest, index))
        .collect::<Result<Vec<_>>>()?;
    let mut report = TransactionReport {
        directory,
        source_root: plan.root.clone(),
        status: TransactionStatus {
            operation_id: manifest.operation_id.clone(),
            plan_fingerprint: manifest.plan_fingerprint.clone(),
            updated_unix: now(),
            state: State::Validating,
            resumed: previous.is_some(),
            error: None,
        },
        progress: Progress {
            operation_id: manifest.operation_id.clone(),
            planned_bytes: manifest.planned_bytes,
            stage: "Validando origem e destino".into(),
            ..Progress::default()
        },
        manifest,
        files,
    };
    let started = std::time::Instant::now();
    progress(&report.progress);

    let outcome = process(&plan, &mut report, cancel, progress, started, verify_only);
    if verify_only {
        outcome?;
        report.status = previous.expect("verified completed status exists");

        return Ok(report);
    }

    report.status.state = match &outcome {
        Ok(()) => State::Completed,
        Err(error) if error.kind == ErrorKind::Cancelled => State::Cancelled,
        Err(_) => State::Failed,
    };
    report.status.error = outcome.err();
    report.status.updated_unix = now();
    save_status(&report)?;

    Ok(report)
}

fn save_status(report: &TransactionReport) -> Result<()> {
    journal::save(&report.directory.join("status.json"), &report.status, true)
}

fn process(
    plan: &Plan,
    report: &mut TransactionReport,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(&Progress),
    started: std::time::Instant,
    verify_only: bool,
) -> Result<()> {
    let manifest = &report.manifest;
    let directory = &report.directory;
    let allowed_files = manifest
        .files
        .iter()
        .map(|file| file.relative.as_path())
        .collect();
    let allowed_directories = manifest.directories.iter().map(PathBuf::as_path).collect();
    check_tree(
        plan,
        &plan.destination,
        &allowed_files,
        &allowed_directories,
        cancel,
    )?;

    let mut remaining = 0;
    let mut retained = vec![false; manifest.files.len()];
    for (index, (file, checkpoint)) in manifest.files.iter().zip(&mut report.files).enumerate() {
        check_cancel(cancel)?;
        if hash_source(plan, &file.relative, file.bytes, cancel)? != file.sha256 {
            checkpoint.state = FileState::SourceChanged;
            return Err(changed("SHA-256 da origem mudou desde a criação do plano"));
        }

        let target = plan.destination.join(&file.relative);
        let staging = staging_path(directory, index);
        let staging_exists = journal::exists(&staging)?;
        let published = journal::exists(&target)?;
        if published {
            if !matches!(
                checkpoint.state,
                FileState::Verified | FileState::ReadyToPublish
            ) || staging_exists
                || (verify_only && checkpoint.state != FileState::Verified)
            {
                checkpoint.state = FileState::DestinationConflict;
                return Err(bad_destination("Conflito com arquivo publicado"));
            }

            verify_file(&target, file, &plan.destination_id, cancel)?;
            retained[index] = true;
            report.progress.acknowledged_bytes += file.bytes;
            report.progress.verified_bytes += file.bytes;
            report.progress.verified_files += 1;
            report.progress.skipped_files += 1;
        } else {
            if verify_only || checkpoint.state == FileState::Verified {
                return Err(changed("Arquivo verificado desapareceu do destino"));
            }

            if staging_exists {
                let staging = safe_existing(&staging)?;
                if !staging.is_file()
                    || identity(&staging)?.volume != plan.destination_id.volume
                    || checkpoint.state == FileState::Pending
                {
                    return Err(bad_destination(
                        "Arquivo temporário sem checkpoint correspondente",
                    ));
                }
            }

            remaining += file.bytes;
        }

        report.progress.elapsed_seconds = started.elapsed().as_secs();
        progress(&report.progress);
    }

    if verify_only {
        for relative in &manifest.directories {
            if !safe_existing(&plan.destination.join(relative))?.is_dir() {
                return Err(changed("Diretório do backup não está disponível"));
            }
        }

        return Ok(());
    }

    // Revalidate before deleting any owned staging files or creating payload entries.
    revalidate(plan)?;
    report.status.state = State::Running;
    report.status.updated_unix = now();
    save_status(report)?;

    for (index, &retained) in retained.iter().enumerate() {
        if !retained {
            check_cancel(cancel)?;
            let staging = staging_path(&report.directory, index);
            if journal::exists(&staging)? {
                fs::remove_file(staging)
                    .map_err(|error| Error::io("Discard interrupted staging file", error))?;
            }
        }
    }

    if remaining > space(&plan.destination)?.available {
        return Err(Error::new(
            ErrorKind::Validation,
            "Espaço insuficiente para retomar",
        ));
    }

    for relative in &report.manifest.directories {
        check_cancel(cancel)?;
        let path = plan.destination.join(relative);
        if !journal::exists(&path)? {
            create_directory(&path)?;
        }
    }

    for (index, retained) in retained.into_iter().enumerate() {
        check_cancel(cancel)?;
        revalidate(plan)?;

        let file = &report.manifest.files[index];
        let checkpoint = &mut report.files[index];
        if retained {
            save_file(&report.directory, checkpoint, FileState::Verified)?;
            continue;
        }

        let outcome = copy_file(
            plan,
            &report.directory,
            file,
            checkpoint,
            cancel,
            |written| {
                report.progress.stage = format!("Copiando {}", file.relative.display());
                report.progress.acknowledged_bytes += written;
                report.progress.elapsed_seconds = started.elapsed().as_secs();
                progress(&report.progress);
            },
        );

        if let Err(error) = outcome {
            // A prepared publication may already be visible. Keep its recovery state intact.
            if checkpoint.state != FileState::ReadyToPublish {
                checkpoint.error = Some(error.clone());
                let state = if error.kind == ErrorKind::Cancelled {
                    FileState::Cancelled
                } else if checkpoint.acknowledged_bytes > 0 {
                    FileState::Partial
                } else {
                    FileState::Failed
                };

                report.progress.failed_files = usize::from(state == FileState::Failed);
                report.progress.partial_files = usize::from(checkpoint.acknowledged_bytes > 0);
                save_file(&report.directory, checkpoint, state)?;
            }

            return Err(error);
        }

        report.progress.stage = "Verificando arquivos publicados".into();
        report.progress.verified_bytes += file.bytes;
        report.progress.verified_files += 1;
        progress(&report.progress);
    }

    revalidate(plan)?;
    report.status.state = State::Verifying;
    report.status.updated_unix = now();
    save_status(report)?;

    if report.progress.verified_bytes != report.manifest.planned_bytes
        || report.progress.verified_files != report.manifest.files.len()
    {
        return Err(changed(
            "Contagem de arquivos ou bytes verificados divergente",
        ));
    }

    for file in &report.manifest.files {
        check_cancel(cancel)?;
        verify_file(
            &plan.destination.join(&file.relative),
            file,
            &plan.destination_id,
            cancel,
        )?;
        if hash_source(plan, &file.relative, file.bytes, cancel)? != file.sha256 {
            return Err(changed("SHA-256 da origem mudou antes da conclusão"));
        }
    }

    revalidate(plan)?;
    check_cancel(cancel)
}

fn copy_file(
    plan: &Plan,
    directory: &Path,
    file: &ExpectedFile,
    checkpoint: &mut FileCheckpoint,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<()> {
    let source = safe_existing(&plan.root.join(&file.relative))?;
    if identity(&source)?.volume != plan.source_id.volume {
        return Err(location_changed());
    }

    let target = plan.destination.join(&file.relative);
    prepare_destination_parent(plan, &target)?;
    let staging = staging_path(directory, checkpoint.index);
    let mut input = File::open(&source).map_err(|error| Error::io("Open backup source", error))?;

    checkpoint.acknowledged_bytes = 0;
    checkpoint.error = None;
    save_file(directory, checkpoint, FileState::Copying)?;

    let mut output =
        File::create_new(&staging).map_err(|error| Error::io("Create staging file", error))?;
    let mut buffer = [0u8; 65536];
    let mut last_checkpoint = 0;
    loop {
        check_cancel(cancel)?;
        let read = input
            .read(&mut buffer)
            .map_err(|error| Error::io("Read backup data", error))?;
        if read == 0 {
            break;
        }
        if checkpoint.acknowledged_bytes + read as u64 > file.bytes {
            return Err(changed("A origem cresceu durante a cópia"));
        }

        let mut offset = 0;
        while offset < read {
            check_cancel(cancel)?;
            let written = output
                .write(&buffer[offset..read])
                .map_err(|error| Error::io("Write staging bytes", error))?;
            if written == 0 {
                return Err(Error::new(ErrorKind::Io, "Destino aceitou zero bytes"));
            }

            offset += written;
            checkpoint.acknowledged_bytes += written as u64;
            progress(written as u64);
            if checkpoint.acknowledged_bytes - last_checkpoint >= INTERVAL {
                save_file(directory, checkpoint, FileState::Copying)?;
                last_checkpoint = checkpoint.acknowledged_bytes;
            }
        }
    }

    output
        .sync_all()
        .map_err(|error| Error::io("Flush staged backup file", error))?;
    drop(output);
    verify_file(&staging, file, &plan.destination_id, cancel)?;
    if hash_source(plan, &file.relative, file.bytes, cancel)? != file.sha256 {
        return Err(changed("O conteúdo da origem mudou durante a cópia"));
    }

    revalidate(plan)?;
    save_file(directory, checkpoint, FileState::ReadyToPublish)?;
    journal::publish(&staging, &target, false)?;
    verify_file(&target, file, &plan.destination_id, cancel)?;
    save_file(directory, checkpoint, FileState::Verified)
}
