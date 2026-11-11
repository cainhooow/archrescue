//! Bind an explicit native confirmation to the reviewed backup or checkpoint.

use std::path::Path;

use super::*;

pub(super) enum Confirmation {
    Start(Preview),
    Resume {
        source: PathBuf,
        checkpoint: PathBuf,
    },
}

impl Confirmation {
    pub unsafe fn ask(self, hwnd: HWND, state: &WindowState) {
        let (title, prompt, approved, refused) = match &self {
            Self::Start(preview) => (
                "Confirmar backup",
                format!(
                    "Iniciar backup de {} arquivos?\n\nOrigem: {}\nDestino: {}",
                    preview.files,
                    preview.source.display(),
                    preview.destination.display(),
                ),
                "Backup autorizado",
                "Backup recusado",
            ),
            Self::Resume {
                source,
                checkpoint,
            } => (
                "Confirmar retomada",
                format!(
                    "Retomar a operação?\n\nCheckpoint: {}\nOrigem: {}",
                    checkpoint.display(),
                    source.display()
                ),
                "Retomada autorizada",
                "Retomada recusada",
            ),
        };

        // The modal dialog dispatches messages. App must be available to paint the owner window.
        state.confirming.set(true);
        let accepted = MessageBoxW(
            hwnd,
            wide(&prompt).as_ptr(),
            wide(title).as_ptr(),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES;
        state.confirming.set(false);

        let Ok(mut app) = state.app.try_borrow_mut() else {
            return;
        };
        if let Some(report) = &mut app.report {
            let decision = if accepted { approved } else { refused };
            report.decisions.push(format!("{decision}. {prompt}"));
        }
        if !accepted || app.busy {
            return;
        }

        match self {
            Self::Start(preview) => {
                if !matches_preview(hwnd, &preview) {
                    return;
                }
                begin_backup(hwnd, &mut app, move |cancel, progress| {
                    let checkpoint = transaction::prepare(
                        &preview.source,
                        &preview.selections,
                        &preview.destination,
                        cancel,
                    )?;
                    transaction::resume(&preview.source, &checkpoint, cancel, progress)
                });
            }
            Self::Resume {
                source,
                checkpoint,
            } => {
                if source != Path::new(&get(hwnd, ADDRESS)) {
                    set(
                        hwnd,
                        STATUS,
                        "A origem foi alterada. Selecione o checkpoint novamente.",
                    );
                    return;
                }
                begin_backup(hwnd, &mut app, move |cancel, progress| {
                    transaction::resume(&source, &checkpoint, cancel, progress)
                });
            }
        }
    }
}

pub(super) unsafe fn matches_preview(hwnd: HWND, preview: &Preview) -> bool {
    if preview.destination != Path::new(&get(hwnd, DEST))
        || preview.source != Path::new(&get(hwnd, ADDRESS))
    {
        set(
            hwnd,
            STATUS,
            "Origem ou destino editado. Abra a origem e gere um novo plano.",
        );
        return false;
    }

    true
}
