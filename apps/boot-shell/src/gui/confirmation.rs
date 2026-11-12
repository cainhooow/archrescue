//! Bind explicit confirmation to the reviewed backup or checkpoint.

mod dialog;

use std::path::Path;

use super::*;

pub(super) enum Confirmation {
    Start(Preview),
    Resume {
        source: PathBuf,
        checkpoint: PathBuf,
        recorded_state: Option<archrescue_core::State>,
    },
}

impl Confirmation {
    pub unsafe fn ask(self, hwnd: HWND, state: &WindowState) {
        let (prompt, approved, refused) = match &self {
            Self::Start(preview) => (
                dialog::Prompt {
                    title: "Confirmar backup",
                    question: format!("Iniciar backup de {} arquivos?", preview.files),
                    paths: [
                        ("Origem", preview.source.display().to_string()),
                        ("Destino", preview.destination.display().to_string()),
                    ],
                    notice: if preview.files == 0 {
                        (
                            "Pasta vazia na seleção",
                            "A estrutura das pastas selecionadas será preservada.",
                        )
                    } else {
                        ("Cópia de conteúdo", "Os arquivos serão publicados em uma pasta exclusiva e conferidos por SHA-256.")
                    },
                    width: if [preview.source.as_os_str(), preview.destination.as_os_str()]
                        .iter()
                        .any(|path| path.len() > 60)
                    {
                        730
                    } else {
                        600
                    },
                },
                "Backup autorizado",
                "Backup recusado",
            ),
            Self::Resume {
                source,
                checkpoint,
                recorded_state,
            } => (
                dialog::Prompt {
                    title: "Confirmar retomada",
                    question: "Retomar a operação a partir do checkpoint?".into(),
                    paths: [
                        ("Checkpoint", checkpoint.display().to_string()),
                        ("Origem", source.display().to_string()),
                    ],
                    notice: (
                        match recorded_state {
                            Some(archrescue_core::State::Running) => {
                                "Estado registrado · Em execução"
                            }
                            Some(archrescue_core::State::Completed) => {
                                "Estado registrado · Conclusão registrada"
                            }
                            Some(archrescue_core::State::Cancelled) => {
                                "Estado registrado · Cancelado"
                            }
                            Some(archrescue_core::State::Pending) => {
                                "Estado registrado · Planejado"
                            }
                            Some(state) => state_label(*state),
                            None => "Validar checkpoint",
                        },
                        "A retomada valida a origem e confere o conteúdo já gravado.",
                    ),
                    width: 600,
                },
                "Retomada autorizada",
                "Retomada recusada",
            ),
        };

        // The modal dialog dispatches messages. App must be available to paint the owner window.
        state.confirming.set(true);
        let accepted = dialog::show(hwnd, state, &prompt);
        state.confirming.set(false);

        let Ok(mut app) = state.app.try_borrow_mut() else {
            return;
        };
        if let Some(report) = &mut app.report {
            let decision = if accepted { approved } else { refused };
            let [(first, first_path), (second, second_path)] = &prompt.paths;
            report.decisions.push(format!(
                "{decision}. {}\n{first}: {first_path}\n{second}: {second_path}",
                prompt.question,
            ));
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
                ..
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
