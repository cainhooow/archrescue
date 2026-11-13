//! Bind explicit confirmation to the reviewed backup or checkpoint.

mod dialog;

use super::*;

pub(super) enum Confirmation {
    Start(Preview),
    Resume {
        source: Location,
        checkpoint: PathBuf,
        fingerprint: Option<String>,
        expected_destination: Option<archrescue_storage::RecoveryLocation>,
        recorded_state: Option<archrescue_core::State>,
    },
}

impl Confirmation {
    pub unsafe fn ask(self, hwnd: HWND, state: &WindowState) {
        let technician = state.app.borrow().technician;
        let (prompt, approved, refused) = match &self {
            Self::Start(preview) => (
                dialog::Prompt {
                    title: "Confirmar backup",
                    question: if preview.files == 1 {
                        "Iniciar backup de 1 arquivo?".into()
                    } else {
                        format!("Iniciar backup de {} arquivos?", preview.files)
                    },
                    paths: [
                        ("Origem", preview.source.label(technician)),
                        ("Destino", preview.destination.label(technician)),
                    ],
                    notice: if preview.files == 0 {
                        (
                            "Pasta vazia na seleção",
                            "A estrutura das pastas selecionadas será preservada.",
                        )
                    } else {
                        ("Cópia de conteúdo", "Os arquivos serão publicados em uma pasta exclusiva e conferidos por SHA-256.")
                    },
                    width: if [
                        preview.source.path.as_os_str(),
                        preview.destination.path.as_os_str(),
                    ]
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
                ..
            } => (
                dialog::Prompt {
                    title: "Confirmar retomada",
                    question: "Retomar a operação a partir do checkpoint?".into(),
                    paths: [
                        ("Checkpoint", paths::display(checkpoint, technician)),
                        ("Origem", source.label(technician)),
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
                if !matches_preview(&app, &preview) {
                    return;
                }
                begin_backup(hwnd, &mut app, move |cancel, progress| {
                    let source = preview.source.revalidate()?;
                    let destination = preview.destination.revalidate()?;
                    crate::resources::writable(&destination, &[preview.source], preview.bytes)?;
                    let checkpoint =
                        transaction::prepare(&source, &preview.selections, &destination, cancel)?;
                    transaction::resume(&source, &checkpoint, cancel, progress)
                });
            }
            Self::Resume {
                source,
                checkpoint,
                fingerprint,
                expected_destination,
                ..
            } => {
                if app.session.source.as_ref() != Some(&source) {
                    set(
                        hwnd,
                        STATUS,
                        "A origem foi alterada. Selecione o checkpoint novamente.",
                    );
                    return;
                }
                begin_backup(hwnd, &mut app, move |cancel, progress| {
                    let source = source.revalidate()?;
                    let manifest = transaction::manifest(&checkpoint)?;
                    if fingerprint.as_ref() != Some(&manifest.plan_fingerprint) {
                        return Err(Error::new(
                            ErrorKind::Conflict,
                            "O checkpoint selecionado mudou.",
                        ));
                    }
                    let parent = checkpoint.parent().ok_or_else(|| {
                        Error::new(ErrorKind::Validation, "Checkpoint sem pasta de destino.")
                    })?;
                    let destination = Location::capture(parent)?;
                    if destination.recorded_location() != expected_destination.as_ref() {
                        return Err(Error::new(
                            ErrorKind::Conflict,
                            "O destino original do checkpoint mudou.",
                        ));
                    }
                    transaction::resume(&source, &checkpoint, cancel, progress)
                });
            }
        }
    }
}

pub(super) fn matches_preview(app: &App, preview: &Preview) -> bool {
    app.session.destination(BACKUP).as_ref().ok() == Some(&preview.destination)
        && app.session.source.as_ref() == Some(&preview.source)
}
