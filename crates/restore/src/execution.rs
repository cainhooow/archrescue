use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use archrescue_backup::hash_reader;
use archrescue_core::{Error, ErrorKind, Operation, Result};
use serde::Serialize;

use crate::{
    check_cancel, filesystem, invalid, prepare, require_capacity, Action, Directory, PlannedFile,
    Profile, Request,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FileOutcome {
    NotAttempted,
    Verified,
    Skipped,
    Conflicted,
    Failed,
    Unsupported,
}

#[derive(Debug, Serialize)]
pub struct FileReport {
    pub source_relative: PathBuf,
    pub target_relative: Option<PathBuf>,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
    pub bytes_copied: u64,
    pub published: bool,
    pub action: Action,
    pub outcome: FileOutcome,
    pub error: Option<Error>,
}

#[derive(Debug, Serialize)]
pub struct RestoreReport {
    pub operation_id: String,
    pub backup_operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<crate::RecoveryBinding>,
    pub plan_sha256: String,
    pub target_sid: String,
    pub target_profile: PathBuf,
    pub files: Vec<FileReport>,
    pub prepared_directories: Vec<PathBuf>,
    pub error: Option<Error>,
}

pub fn execute(
    request: &Request,
    authorization: &str,
    cancel: &AtomicBool,
    progress: impl FnMut(usize, usize, &FileReport),
) -> Result<RestoreReport> {
    execute_recovery(request, None, authorization, cancel, progress)
}

pub fn execute_recovery(
    request: &Request,
    recovery: Option<&crate::RecoveryBinding>,
    authorization: &str,
    cancel: &AtomicBool,
    progress: impl FnMut(usize, usize, &FileReport),
) -> Result<RestoreReport> {
    execute_for(
        request,
        &Profile::current()?,
        recovery,
        authorization,
        cancel,
        progress,
    )
}

pub(crate) fn execute_for(
    request: &Request,
    profile: &Profile,
    recovery: Option<&crate::RecoveryBinding>,
    authorization: &str,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize, &FileReport),
) -> Result<RestoreReport> {
    let (plan, target, payload) = prepare(request, profile, recovery, cancel)?;
    if authorization != format!("AUTORIZAR-RESTAURACAO:{}", plan.fingerprint) {
        return Err(Error::new(
            ErrorKind::Permission,
            "A autorização não corresponde ao plano atual",
        ));
    }

    let mut report = RestoreReport {
        operation_id: Operation::new().id,
        backup_operation: plan.backup_operation,
        recovery: plan.recovery,
        plan_sha256: plan.fingerprint,
        target_sid: plan.target_sid,
        target_profile: plan.target_profile,
        files: plan.files.iter().map(initial_file_report).collect(),
        prepared_directories: vec![],
        error: None,
    };
    if plan.blocked_by_conflicts {
        report.error = Some(Error::new(
            ErrorKind::Conflict,
            "Conflitos bloquearam a restauração",
        ));
        return Ok(report);
    }

    let total = plan.files.len();
    let root = target.profile.path();
    require_capacity(root, plan.required_bytes)?;
    for directory in plan.directories {
        let result = check_cancel(cancel).and_then(|_| Directory::create(&root.join(&directory)));
        if let Err(error) = result {
            report.error = Some(error);
            return Ok(report);
        }

        report.prepared_directories.push(directory);
    }

    for (index, (planned, entry)) in plan.files.iter().zip(&mut report.files).enumerate() {
        if let Err(error) = check_cancel(cancel) {
            report.error = Some(error);
            break;
        }
        if matches!(planned.action, Action::Copy | Action::Rename) {
            let target_relative = planned
                .target_relative
                .as_ref()
                .ok_or_else(|| invalid("Plano sem caminho de destino"))?;
            let target_path = root.join(target_relative);
            let staged_name = format!(".archrescue-{}-{index}.part", report.operation_id);

            if let Err(error) = restore_file(
                planned,
                payload.path(),
                &target_path,
                &staged_name,
                entry,
                cancel,
            ) {
                entry.outcome = if matches!(error.kind, ErrorKind::Conflict) {
                    FileOutcome::Conflicted
                } else {
                    FileOutcome::Failed
                };
                entry.error = Some(error.clone());
                report.error = Some(error);
                progress(index + 1, total, entry);
                break;
            }
        }

        progress(index + 1, total, entry);
    }

    Ok(report)
}

fn initial_file_report(planned: &PlannedFile) -> FileReport {
    let outcome = match planned.action {
        Action::Copy | Action::Rename => FileOutcome::NotAttempted,
        Action::Skip => FileOutcome::Skipped,
        Action::Conflict => FileOutcome::Conflicted,
        Action::Unsupported => FileOutcome::Unsupported,
    };

    FileReport {
        source_relative: planned.source.relative.clone(),
        target_relative: planned.target_relative.clone(),
        expected_sha256: planned.source.sha256.clone(),
        actual_sha256: None,
        bytes_copied: 0,
        published: false,
        action: planned.action,
        outcome,
        error: None,
    }
}

fn restore_file(
    planned: &PlannedFile,
    payload: &Path,
    target: &Path,
    staged_name: &str,
    report: &mut FileReport,
    cancel: &AtomicBool,
) -> Result<()> {
    let source = payload.join(&planned.source.relative);
    let _source_parent = Directory::open(
        source
            .parent()
            .ok_or_else(|| invalid("Origem sem diretório pai"))?,
    )?;
    let mut input = filesystem::open_input(&source)?;

    let parent = target
        .parent()
        .ok_or_else(|| invalid("Destino sem diretório pai"))?;
    let _target_parent = Directory::create(parent)?;
    if filesystem::exists(target)? {
        return Err(Error::new(
            ErrorKind::Conflict,
            "Novo conflito após a revisão",
        ));
    }

    let staged = parent.join(staged_name);
    let mut output = filesystem::stage(&staged)?;
    let result = (|| {
        copy_expected(
            &mut input,
            &mut output,
            planned.source.bytes,
            report,
            cancel,
        )?;
        output
            .sync_all()
            .map_err(|error| Error::io("Sincronizar cópia temporária", error))?;

        let mut readback = filesystem::read_staged(&staged)?;
        let hash = hash_reader(&mut readback, cancel)?;
        drop(readback);
        report.actual_sha256 = Some(hash.clone());
        if hash != planned.source.sha256 {
            return Err(Error::new(
                ErrorKind::Integrity,
                "SHA-256 relido diverge do manifesto",
            ));
        }

        check_cancel(cancel)?;
        filesystem::publish(&output, &staged, target)?;
        report.published = true;
        report.outcome = FileOutcome::Verified;

        Ok(())
    })();

    #[cfg(windows)]
    let cleanup = if result.is_err() {
        filesystem::discard(&output, &staged)
    } else {
        Ok(())
    };
    #[cfg(not(windows))]
    let cleanup = filesystem::discard(&output, &staged);

    match (result, cleanup) {
        (Err(mut error), Err(cleanup)) => {
            error.context.push_str(&format!(" | {cleanup}"));
            Err(error)
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
        _ => Ok(()),
    }
}

fn copy_expected(
    input: &mut impl Read,
    output: &mut impl Write,
    bytes: u64,
    report: &mut FileReport,
    cancel: &AtomicBool,
) -> Result<()> {
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        check_cancel(cancel)?;
        let count = input
            .read(&mut buffer)
            .map_err(|error| Error::io("Ler conteúdo do backup", error))?;
        if count == 0 {
            break;
        }
        if count as u64 > bytes - report.bytes_copied {
            return Err(Error::new(
                ErrorKind::Integrity,
                "O arquivo de backup cresceu após a verificação",
            ));
        }

        let mut remaining = &buffer[..count];
        while !remaining.is_empty() {
            check_cancel(cancel)?;
            let written = output
                .write(remaining)
                .map_err(|error| Error::io("Gravar conteúdo temporário", error))?;
            if written == 0 {
                return Err(Error::new(
                    ErrorKind::Io,
                    "Gravação temporária sem progresso",
                ));
            }

            report.bytes_copied += written as u64;
            remaining = &remaining[written..];
        }
    }
    if report.bytes_copied != bytes {
        return Err(Error::new(
            ErrorKind::Integrity,
            "O tamanho copiado diverge do manifesto",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_write_failure_retains_the_actual_written_count() {
        struct FailingWriter {
            written: bool,
        }
        impl Write for FailingWriter {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                if self.written {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Injected after progress",
                    ));
                }
                self.written = true;
                Ok(data.len().min(2))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let planned = PlannedFile {
            source: archrescue_backup::transaction::ExpectedFile {
                relative: "file.txt".into(),
                bytes: 4,
                sha256: "expected".into(),
            },
            target_relative: Some("Documents/file.txt".into()),
            action: Action::Copy,
        };
        let mut report = initial_file_report(&planned);
        let mut writer = FailingWriter {
            written: false,
        };

        let error = copy_expected(
            &mut &b"data"[..],
            &mut writer,
            4,
            &mut report,
            &AtomicBool::new(false),
        )
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::Permission);
        assert_eq!(report.bytes_copied, 2);
        assert!(!report.published);
        assert_ne!(report.outcome, FileOutcome::Verified);
    }

    #[test]
    fn short_copy_records_actual_bytes_without_verification() {
        let planned = PlannedFile {
            source: archrescue_backup::transaction::ExpectedFile {
                relative: "file.txt".into(),
                bytes: 10,
                sha256: "expected".into(),
            },
            target_relative: Some("Documents/file.txt".into()),
            action: Action::Copy,
        };
        let mut report = initial_file_report(&planned);
        let mut bytes = Vec::new();

        let error = copy_expected(
            &mut &b"part"[..],
            &mut bytes,
            10,
            &mut report,
            &AtomicBool::new(false),
        )
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::Integrity);
        assert_eq!(report.bytes_copied, 4);
        assert_eq!(report.actual_sha256, None);
        assert!(!report.published);
    }
}
