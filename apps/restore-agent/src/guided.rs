//! Installed-account restore with native Windows selection and review.

use std::{
    ffi::OsString,
    fmt::Write,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::atomic::AtomicBool,
};

use archrescue_backup::transaction::{self, BackupManifest};
use archrescue_core::{presentation, Error, ErrorKind, Result};
use archrescue_restore::{ConflictPolicy, FileOutcome, Folder, Mapping, Profile, Request};
use windows_sys::Win32::UI::{Controls::Dialogs::*, WindowsAndMessaging::*};

const TITLE: &str = "ArchRescue · Restaurar arquivos";

pub fn run() -> Result<()> {
    if let Err(error) = restore() {
        message(
            &presentation::text(&error.to_string(), false),
            MB_OK | MB_ICONERROR,
        );
        return Err(error);
    }

    Ok(())
}

fn restore() -> Result<()> {
    let profile = Profile::current()?;
    let Some(manifest_path) = choose_file(
        "Escolha o manifesto na pasta do backup",
        "Manifesto ArchRescue\0manifest.json\0\0",
        None,
    )?
    else {
        return Ok(());
    };
    let backup = manifest_path
        .parent()
        .ok_or_else(|| invalid("Pasta do backup indisponível."))?;
    let manifest = transaction::manifest(backup)?;

    let recovery = match message(
        "Este backup faz parte de uma implantação ArchRescue?\n\nSim · Escolher o registro final de implantação.\nNão · Restaurar um backup independente.",
        MB_YESNOCANCEL | MB_ICONQUESTION | MB_DEFBUTTON3,
    ) {
        IDYES => {
            let Some(record) = choose_file(
                "Escolha o registro final da implantação",
                "Registro de implantação\0execution-*.json;execution.json\0\0",
                None,
            )? else {
                return Ok(());
            };
            let path = archrescue_storage::safe_existing(&record)?;
            Some(super::read_binding(&path)?)
        }
        IDNO => None,
        _ => return Ok(()),
    };

    let mappings = review_mappings(&manifest, &profile)?;
    if mappings.is_empty() {
        return Ok(());
    }
    let Some(conflicts) = choose_conflicts() else {
        return Ok(());
    };
    let request = Request {
        backup: backup.to_owned(),
        target_sid: profile.account_sid.clone(),
        mappings,
        conflicts,
    };

    let cancel = AtomicBool::new(false);
    let plan = archrescue_restore::preview_recovery(&request, recovery.as_ref(), &cancel)?;
    let mut review = format!(
        "Conta: {}\nBackup: {}\n{} arquivos, {} bytes\nPolítica: {}\n\n",
        presentation::path(&profile.root),
        presentation::path(backup),
        plan.files.len(),
        plan.planned_bytes,
        conflict_label(conflicts),
    );
    for mapping in &plan.mappings {
        let destination = &plan.known_folders[&mapping.folder].0;
        let _ = writeln!(
            review,
            "{} → {}",
            presentation::path(&mapping.source),
            presentation::path(destination)
        );
    }

    let renamed = plan
        .files
        .iter()
        .filter(|file| file.action == archrescue_restore::Action::Rename)
        .count();
    let skipped = plan
        .files
        .iter()
        .filter(|file| file.action == archrescue_restore::Action::Skip)
        .count();
    let unsupported = plan
        .files
        .iter()
        .filter(|file| file.action == archrescue_restore::Action::Unsupported)
        .count();
    let _ = write!(review, "\n{renamed} com novo nome · {skipped} ignorados · {unsupported} sem mapeamento\n{} pastas sem mapeamento.", plan.unsupported_directories.len());
    if plan.blocked_by_conflicts {
        review.push_str("\n\nHá arquivos no destino. Reabra o assistente e escolha outra política para prosseguir.");
        message(&review, MB_OK | MB_ICONWARNING);
        return Ok(());
    }

    review.push_str("\n\nRestaurar com esta seleção?");
    if message(&review, MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2) != IDYES {
        return Ok(());
    }

    let authorization = format!("AUTORIZAR-RESTAURACAO:{}", plan.fingerprint);
    let result = archrescue_restore::execute_recovery(
        &request,
        recovery.as_ref(),
        &authorization,
        &cancel,
        |_, _, _| {},
    )?;
    let verified = result
        .files
        .iter()
        .filter(|file| file.outcome == FileOutcome::Verified)
        .count();
    let skipped = result
        .files
        .iter()
        .filter(|file| file.outcome == FileOutcome::Skipped)
        .count();
    let summary = format!(
        "{verified} arquivos restaurados e verificados por SHA-256.\n{skipped} arquivos ignorados.\n{} pastas preparadas.\n{}",
        result.prepared_directories.len(),
        presentation::path(&result.target_profile),
    );
    if let Some(error) = result.error {
        return Err(Error::new(error.kind, format!("{summary}\n\n{error}")));
    }
    message(&summary, MB_OK | MB_ICONINFORMATION);
    Ok(())
}

fn review_mappings(manifest: &BackupManifest, profile: &Profile) -> Result<Vec<Mapping>> {
    let mut mappings = Vec::new();
    let mut suggestions = 0;
    for (name, label, folder) in [
        ("Desktop", "Área de Trabalho", Folder::Desktop),
        ("Documents", "Documentos", Folder::Documents),
        ("Downloads", "Downloads", Folder::Downloads),
        ("Pictures", "Imagens", Folder::Pictures),
    ] {
        let Some(destination) = profile.folders.get(&folder) else {
            continue;
        };
        for source in suggested_folders(&manifest.directories, name) {
            suggestions += 1;
            let choice = message(
                &format!(
                    "Restaurar esta pasta para {label}?\n\n{}\n→ {}\n\nNão permite consultar a próxima pasta sugerida.",
                    presentation::path(source),
                    presentation::path(destination),
                ),
                MB_YESNOCANCEL | MB_ICONQUESTION | MB_DEFBUTTON3,
            );
            if choice == IDYES {
                mappings.push(Mapping {
                    source: source.clone(),
                    folder,
                });
                break;
            }
            if choice == IDCANCEL {
                return Ok(Vec::new());
            }
        }
    }

    if suggestions == 0 {
        return Err(invalid("O manifesto não contém pastas reconhecidas neste perfil. Use os mapeamentos explícitos do agente para este backup."));
    }
    Ok(mappings)
}

fn suggested_folders<'a>(
    directories: &'a [PathBuf],
    name: &'a str,
) -> impl Iterator<Item = &'a PathBuf> {
    directories.iter().filter(move |path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|part| part.eq_ignore_ascii_case(name))
    })
}

fn choose_conflicts() -> Option<ConflictPolicy> {
    match message(
        "Se um arquivo já existir, salvar a cópia restaurada com outro nome?\n\nSim · Preservar ambos os arquivos.\nNão · Consultar outras opções.",
        MB_YESNOCANCEL | MB_ICONQUESTION | MB_DEFBUTTON3,
    ) {
        IDYES => Some(ConflictPolicy::Rename),
        IDNO => match message(
            "Ignorar arquivos que já existem?\n\nSim · Ignorar conflitos.\nNão · Bloquear a operação se houver conflito.",
            MB_YESNOCANCEL | MB_ICONQUESTION | MB_DEFBUTTON3,
        ) {
            IDYES => Some(ConflictPolicy::Skip),
            IDNO => Some(ConflictPolicy::Abort),
            _ => None,
        },
        _ => None,
    }
}

fn conflict_label(policy: ConflictPolicy) -> &'static str {
    match policy {
        ConflictPolicy::Abort => "Bloquear se houver conflito",
        ConflictPolicy::Skip => "Ignorar arquivos existentes",
        ConflictPolicy::Rename => "Preservar os dois arquivos",
    }
}

fn choose_file(title: &str, filter: &str, initial: Option<&Path>) -> Result<Option<PathBuf>> {
    use std::os::windows::ffi::OsStrExt;

    let title = wide(title);
    let filter: Vec<_> = filter.encode_utf16().collect();
    let initial = initial.map(|path| {
        path.as_os_str()
            .encode_wide()
            .chain([0])
            .collect::<Vec<_>>()
    });
    let mut path = vec![0u16; 32768];
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: filter.as_ptr(),
        lpstrFile: path.as_mut_ptr(),
        nMaxFile: path.len() as u32,
        lpstrTitle: title.as_ptr(),
        lpstrInitialDir: initial
            .as_ref()
            .map_or(std::ptr::null(), |path| path.as_ptr()),
        Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    // SAFETY: all string and output buffers live through the synchronous dialog.
    if unsafe { GetOpenFileNameW(&mut dialog) } == 0 {
        // SAFETY: GetOpenFileNameW returned failure or cancellation on this thread.
        let error = unsafe { CommDlgExtendedError() };
        return if error == 0 {
            Ok(None)
        } else {
            Err(Error::new(
                ErrorKind::Io,
                format!("O seletor de arquivos falhou ({error:#x})."),
            ))
        };
    }

    let end = path
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(path.len());
    Ok(Some(OsString::from_wide(&path[..end]).into()))
}

fn message(value: &str, style: MESSAGEBOX_STYLE) -> i32 {
    let title = wide(TITLE);
    let value = wide(value);
    // SAFETY: title and value are NUL-terminated and live through the call.
    unsafe {
        MessageBoxW(
            null_mut(),
            value.as_ptr(),
            title.as_ptr(),
            style | MB_SETFOREGROUND,
        )
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn invalid(message: &str) -> Error {
    Error::new(ErrorKind::Validation, message)
}
