//! Native Win32 shell. Long-running filesystem work is performed outside the UI thread.

mod deployment;
mod layout;
mod presentation;
mod theme;

use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};

use archrescue_backup::{
    transaction::{self, CheckpointEntry, Progress, TransactionReport},
    Entry,
};
use archrescue_core::{Error, ErrorKind, Result, VERSION};
use archrescue_evidence::{repair, Report};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Controls::{DRAWITEMSTRUCT, EM_SETMARGINS},
        Input::KeyboardAndMouse::{EnableWindow, SetFocus},
        WindowsAndMessaging::*,
    },
};

use crate::{i18n, views::state_label};

const REFRESH: usize = 200;
const ADDRESS: usize = 201;
const OPEN: usize = 202;
const UP: usize = 203;
const FILES: usize = 204;
const DEST: usize = 205;
const COPY: usize = 206;
const CANCEL: usize = 207;
const EXPORT: usize = 208;
const CONSOLE: usize = 209;
const CONTENT: usize = 210;
const STATUS: usize = 211;
const PREVIEW: usize = 212;
const CHECKPOINTS: usize = 213;
const RESUME: usize = 214;
const NEXT: usize = 216;
const TECHNICIAN: usize = 217;
const VERIFY: usize = 218;
const REPAIR: usize = 219;
const ESP: usize = 220;

const DASHBOARD: usize = 0;
const DIAGNOSIS: usize = 1;
const INTEGRITY_PAGE: usize = 2;
const DISKS: usize = 3;
const BACKUP: usize = 4;
const REPAIR_PAGE: usize = 5;
const REPORTS: usize = 8;
const VERIFY_PAGE: usize = 9;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn os_error(context: &str) -> Error {
    Error::io(context, std::io::Error::last_os_error())
}

enum Event {
    Snapshot(Box<Report>),
    Listing(PathBuf, Vec<Entry>),
    Progress(Progress),
    Preview(Preview),
    Checkpoints(Vec<CheckpointEntry>),
    Backup(Box<TransactionReport>),
    Repair(Box<repair::RepairPlan>),
    Exported(PathBuf),
    PlanExported(PathBuf),
    Deployment(deployment::Update),
    Failure(Error),
}

#[derive(Clone)]
struct Preview {
    source: PathBuf,
    destination: PathBuf,
    selections: Vec<PathBuf>,
    files: usize,
    bytes: u64,
}

struct App {
    deployment: deployment::Deployment,
    section: usize,
    technician: bool,
    preview: Option<Preview>,
    checkpoints: Option<Vec<CheckpointEntry>>,
    report: Option<Report>,
    entries: Vec<Entry>,
    directory: PathBuf,
    busy: bool,
    error_status: bool,
    progress: Option<Progress>,
    cancellable: bool,
    cancel: Arc<AtomicBool>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

struct WindowState {
    app: RefCell<App>,
    theme: theme::Theme,
    selected_page: Cell<usize>,
}

unsafe fn control(hwnd: HWND, id: usize) -> HWND {
    GetDlgItem(hwnd, id as i32)
}

unsafe fn set(hwnd: HWND, id: usize, text: &str) {
    let control = control(hwnd, id);
    let text = wide(&text.replace('\n', "\r\n"));

    SetWindowTextW(control, text.as_ptr());
}

unsafe fn get(hwnd: HWND, id: usize) -> String {
    let control = control(hwnd, id);
    let length = GetWindowTextLengthW(control);
    let mut buffer = vec![0u16; (length + 1).max(1) as usize];

    GetWindowTextW(control, buffer.as_mut_ptr(), buffer.len() as i32);

    String::from_utf16_lossy(&buffer[..length as usize])
}

unsafe fn add(
    hwnd: HWND,
    id: usize,
    class: &str,
    text: &str,
    style: u32,
    font: HFONT,
) -> Result<()> {
    let style = if class == "BUTTON" {
        style | BS_OWNERDRAW as u32
    } else {
        style
    };
    let child = CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        0,
        0,
        100,
        30,
        hwnd,
        id as HMENU,
        GetModuleHandleW(null()),
        null(),
    );

    if child.is_null() {
        return Err(os_error("Create native control"));
    }

    SendMessageW(child, WM_SETFONT, font as usize, 1);
    if class == "EDIT" {
        SendMessageW(child, EM_SETMARGINS, 3, (7 << 16) | 7);
    }

    Ok(())
}

unsafe fn place(hwnd: HWND, id: usize, x: i32, y: i32, width: i32, height: i32) {
    MoveWindow(control(hwnd, id), x, y, width.max(1), height.max(1), 1);
}

unsafe fn show(hwnd: HWND, id: usize, visible: bool) {
    ShowWindow(control(hwnd, id), if visible { SW_SHOW } else { SW_HIDE });
}

unsafe fn render(hwnd: HWND, app: &App) {
    let protect = app.section == BACKUP;
    let repair = app.section == REPAIR_PAGE;
    let verify = app.section == VERIFY_PAGE;
    let report_stage = app.section == REPORTS;
    let files = protect && app.preview.is_none();

    show(hwnd, FILES, files);
    show(hwnd, CONTENT, !files && app.section != DASHBOARD);
    show(hwnd, ESP, repair);
    show(
        hwnd,
        DEST,
        protect
            || repair
            || report_stage
            || matches!(app.section, deployment::DEPLOY | deployment::RESTORE),
    );
    for id in [ADDRESS, OPEN, UP, PREVIEW, COPY, CHECKPOINTS, RESUME] {
        show(hwnd, id, protect);
    }
    show(hwnd, EXPORT, report_stage);
    show(hwnd, VERIFY, verify);
    show(hwnd, REPAIR, repair);
    show(hwnd, NEXT, app.section == DASHBOARD);
    show(hwnd, CONSOLE, report_stage && app.technician);

    let area = layout::Layout::read(hwnd);
    for (index, label) in i18n::SECTIONS.iter().enumerate() {
        let name = if area.rail < 200 {
            i18n::COMPACT_SECTIONS[index]
        } else {
            label
        };
        set(hwnd, 100 + index, name);
        InvalidateRect(control(hwnd, 100 + index), null(), 0);
    }

    for id in [
        ADDRESS,
        DEST,
        ESP,
        OPEN,
        UP,
        CHECKPOINTS,
        REFRESH,
        REPAIR,
        FILES,
    ] {
        EnableWindow(control(hwnd, id), (!app.busy) as i32);
    }

    EnableWindow(
        control(hwnd, PREVIEW),
        (!app.busy && app.checkpoints.is_none() && app.preview.is_none()) as i32,
    );
    EnableWindow(
        control(hwnd, COPY),
        (!app.busy && app.preview.is_some()) as i32,
    );
    EnableWindow(
        control(hwnd, RESUME),
        (!app.busy && app.checkpoints.is_some()) as i32,
    );
    EnableWindow(control(hwnd, CANCEL), (app.busy && app.cancellable) as i32);
    EnableWindow(
        control(hwnd, EXPORT),
        (!app.busy && app.report.is_some()) as i32,
    );
    let has_backup = app
        .report
        .as_ref()
        .is_some_and(|report| !report.transactions.is_empty());
    EnableWindow(control(hwnd, VERIFY), (!app.busy && has_backup) as i32);
    set(
        hwnd,
        TECHNICIAN,
        if app.technician {
            "Visão assistida"
        } else {
            "Modo técnico"
        },
    );

    layout::arrange(hwnd, app.section);
    InvalidateRect(hwnd, null(), 0);

    if deployment::render(hwnd, app) {
        return;
    }

    if let Some(preview) = &app.preview {
        if protect {
            let text = format!(
                "PLANO DE BACKUP

Origem: {}
Destino independente: {}

{} arquivos | {} bytes

Seleções: {:?}

O destino receberá uma pasta exclusiva com checkpoint e arquivos verificados. Use Iniciar backup para confirmar.",
                preview.source.display(),
                preview.destination.display(),
                preview.files,
                preview.bytes,
                preview.selections,
            );
            set(hwnd, CONTENT, &text);
            return;
        }
    }

    let Some(report) = &app.report else {
        set(
            hwnd,
            CONTENT,
            if app.busy { i18n::LOADING } else { i18n::EMPTY },
        );
        return;
    };
    let text = crate::views::section(
        match app.section {
            DISKS => 0,
            DIAGNOSIS => 1,
            REPAIR_PAGE => 3,
            VERIFY_PAGE => 4,
            REPORTS => 5,
            _ => 2,
        },
        report,
        app.technician,
    );
    set(hwnd, CONTENT, &text);
}

fn work(app: &mut App, job: impl FnOnce() -> Event + Send + 'static) {
    app.busy = true;
    app.error_status = false;
    app.progress = None;
    app.cancellable = false;
    let sender = app.tx.clone();

    thread::spawn(move || {
        let event =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).unwrap_or_else(|_| {
                Event::Failure(Error::new(
                    ErrorKind::Incomplete,
                    "Worker stopped unexpectedly",
                ))
            });

        let _ = sender.send(event);
    });
}

unsafe fn refresh(hwnd: HWND, app: &mut App) {
    set(hwnd, STATUS, i18n::LOADING);
    work(app, || {
        let inventory = archrescue_storage::inventory();
        let roots = inventory
            .volumes
            .iter()
            .filter_map(|volume| volume.paths.first().cloned())
            .collect::<Vec<_>>();

        let report = Report::collect_with_inventory(&roots, inventory);

        Event::Snapshot(Box::new(report))
    });

    render(hwnd, app);
}

unsafe fn browse(hwnd: HWND, app: &mut App, path: PathBuf) {
    app.preview = None;
    app.checkpoints = None;
    set(hwnd, STATUS, i18n::LOADING);
    work(app, move || {
        let directory = match archrescue_storage::safe_existing(&path) {
            Ok(directory) => directory,
            Err(error) => return Event::Failure(error),
        };

        match archrescue_backup::browse(&directory) {
            Ok(entries) => Event::Listing(directory, entries),
            Err(error) => Event::Failure(error),
        }
    });

    render(hwnd, app);
}

unsafe fn selected_indices(hwnd: HWND) -> Vec<usize> {
    let list = control(hwnd, FILES);
    let selection_count = SendMessageW(list, LB_GETSELCOUNT, 0, 0);

    if selection_count <= 0 {
        return vec![];
    }

    let mut indices = vec![0i32; selection_count as usize];
    let count = SendMessageW(
        list,
        LB_GETSELITEMS,
        selection_count as usize,
        indices.as_mut_ptr() as isize,
    );

    if count < 0 {
        return vec![];
    }

    indices[..count as usize]
        .iter()
        .map(|index| *index as usize)
        .collect()
}

unsafe fn selected(hwnd: HWND, app: &App) -> Vec<PathBuf> {
    selected_indices(hwnd)
        .into_iter()
        .filter_map(|index| app.entries.get(index))
        .map(|entry| entry.name.clone())
        .collect()
}

unsafe fn stage(hwnd: HWND, app: &mut App, section: usize) {
    app.section = section;
    render(hwnd, app);
    // Keep focus on the selected navigation button so Tab continues in page order.
    SetFocus(control(hwnd, 100 + section.min(REPORTS)));
}

unsafe fn action(hwnd: HWND, app: &mut App, id: usize, notification: u32) {
    if (100..100 + i18n::SECTIONS.len()).contains(&id) {
        stage(hwnd, app, id - 100);
        return;
    }

    match id {
        NEXT => {
            stage(hwnd, app, DIAGNOSIS);
            return;
        }
        TECHNICIAN => {
            app.technician = !app.technician;
            render(hwnd, app);
            return;
        }
        CANCEL if app.busy && app.cancellable => {
            app.cancel.store(true, Ordering::Relaxed);
            set(hwnd, STATUS, "Cancelamento solicitado.");
            return;
        }
        _ => {}
    }

    if app.busy {
        return;
    }

    if deployment::action(hwnd, app, id, notification) {
        return;
    }

    match id {
        REFRESH => refresh(hwnd, app),
        OPEN => browse(hwnd, app, PathBuf::from(get(hwnd, ADDRESS))),
        UP => {
            if let Some(parent) = app.directory.parent() {
                browse(hwnd, app, parent.to_owned());
            }
        }
        FILES if notification == LBN_DBLCLK => {
            if app.checkpoints.is_some() {
                resume_backup(hwnd, app);
                return;
            }

            let selection = selected(hwnd, app);
            if selection.len() == 1 {
                let path = app.directory.join(&selection[0]);
                if app
                    .entries
                    .iter()
                    .any(|entry| entry.name == selection[0] && entry.directory)
                {
                    browse(hwnd, app, path);
                }
            }
        }
        PREVIEW => preview_backup(hwnd, app),
        COPY => start_backup(hwnd, app),
        CHECKPOINTS => list_checkpoints(hwnd, app),
        RESUME => resume_backup(hwnd, app),
        VERIFY => verify_backup(hwnd, app),
        REPAIR => preview_repair(hwnd, app),
        EXPORT => export_report(hwnd, app),
        CONSOLE => {
            if let Some(system) = std::env::var_os("SystemRoot") {
                match std::process::Command::new(PathBuf::from(system).join("System32/cmd.exe"))
                    .spawn()
                {
                    Ok(_) => set(hwnd, STATUS, "Console de diagnóstico aberto."),
                    Err(error) => set(hwnd, STATUS, &format!("Falha ao abrir console: {error}")),
                }
            }
        }
        _ => {}
    }
}

unsafe fn preview_backup(hwnd: HWND, app: &mut App) {
    let selections = selected(hwnd, app);
    if selections.is_empty() {
        set(hwnd, STATUS, "Selecione arquivos ou pastas na lista.");
        return;
    }

    let source = app.directory.clone();
    let destination = PathBuf::from(get(hwnd, DEST));
    app.cancel = Arc::new(AtomicBool::new(false));
    let cancel = app.cancel.clone();
    work(app, move || {
        match transaction::preview(&source, &selections, &destination, &cancel) {
            Ok((files, bytes)) => Event::Preview(Preview {
                source,
                destination,
                selections,
                files,
                bytes,
            }),
            Err(error) => Event::Failure(error),
        }
    });

    app.cancellable = true;
    set(hwnd, STATUS, "Calculando tamanho e validando discos…");
    render(hwnd, app);
}

unsafe fn start_backup(hwnd: HWND, app: &mut App) {
    let Some(preview) = app.preview.clone() else {
        return;
    };
    if preview.destination != PathBuf::from(get(hwnd, DEST)).as_path()
        || preview.source != PathBuf::from(get(hwnd, ADDRESS)).as_path()
    {
        set(
            hwnd,
            STATUS,
            "Origem ou destino editado. Abra a origem e gere um novo plano.",
        );
        return;
    }

    let prompt = format!(
        "Iniciar backup de {} arquivos de {} para {}?",
        preview.files,
        preview.source.display(),
        preview.destination.display()
    );
    if MessageBoxW(
        hwnd,
        wide(&prompt).as_ptr(),
        wide("Confirmar backup").as_ptr(),
        MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
    ) != IDYES
    {
        if let Some(report) = &mut app.report {
            report.decisions.push(format!("Backup recusado. {prompt}"));
        }
        return;
    }

    if let Some(report) = &mut app.report {
        report
            .decisions
            .push(format!("Backup autorizado. {prompt}"));
    }
    begin_backup(hwnd, app, move |cancel, progress| {
        let checkpoint = transaction::prepare(
            &preview.source,
            &preview.selections,
            &preview.destination,
            cancel,
        )?;
        transaction::resume(&preview.source, &checkpoint, cancel, progress)
    });
}

unsafe fn list_checkpoints(hwnd: HWND, app: &mut App) {
    let destination = PathBuf::from(get(hwnd, DEST));
    work(app, move || match transaction::checkpoints(&destination) {
        Ok(entries) => Event::Checkpoints(entries),
        Err(error) => Event::Failure(error),
    });
    set(hwnd, STATUS, "Procurando checkpoints no destino…");
    render(hwnd, app);
}

unsafe fn resume_backup(hwnd: HWND, app: &mut App) {
    let selection = selected_indices(hwnd);
    let Some(entry) = app.checkpoints.as_ref().and_then(|entries| {
        let [index] = selection.as_slice() else {
            return None;
        };
        entries.get(*index)
    }) else {
        set(hwnd, STATUS, "Selecione um único checkpoint.");
        return;
    };
    if let Some(error) = &entry.error {
        set(hwnd, STATUS, &error_message(error));
        return;
    }

    let source = PathBuf::from(get(hwnd, ADDRESS));
    let checkpoint = entry.directory.clone();
    let prompt = format!(
        "Retomar {} usando a origem {}?",
        checkpoint.display(),
        source.display()
    );
    if MessageBoxW(
        hwnd,
        wide(&prompt).as_ptr(),
        wide("Confirmar retomada").as_ptr(),
        MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
    ) != IDYES
    {
        if let Some(report) = &mut app.report {
            report
                .decisions
                .push(format!("Retomada recusada. {prompt}"));
        }
        return;
    }

    if let Some(report) = &mut app.report {
        report
            .decisions
            .push(format!("Retomada autorizada. {prompt}"));
    }
    begin_backup(hwnd, app, move |cancel, progress| {
        transaction::resume(&source, &checkpoint, cancel, progress)
    });
}

unsafe fn begin_backup(
    hwnd: HWND,
    app: &mut App,
    job: impl FnOnce(&AtomicBool, &mut dyn FnMut(&Progress)) -> Result<TransactionReport>
        + Send
        + 'static,
) {
    app.cancel = Arc::new(AtomicBool::new(false));
    let cancel = app.cancel.clone();
    let sender = app.tx.clone();
    work(app, move || {
        let mut last_update = std::time::Instant::now();
        let mut progress = |snapshot: &Progress| {
            if last_update.elapsed().as_millis() >= 100 {
                let _ = sender.send(Event::Progress(snapshot.clone()));
                last_update = std::time::Instant::now();
            }
        };

        match job(&cancel, &mut progress) {
            Ok(result) => Event::Backup(Box::new(result)),
            Err(error) => Event::Failure(error),
        }
    });

    app.cancellable = true;
    set(
        hwnd,
        STATUS,
        "Validando seleção e hashes. Aguarde a leitura inicial…",
    );
    render(hwnd, app);
}

unsafe fn verify_backup(hwnd: HWND, app: &mut App) {
    let Some(backup) = app
        .report
        .as_ref()
        .and_then(|report| report.transactions.last())
    else {
        return;
    };
    let directory = backup.directory.clone();
    let source = backup.source_root.clone();
    begin_backup(hwnd, app, move |cancel, _| {
        transaction::verify(&source, &directory, cancel)
    });
    set(
        hwnd,
        STATUS,
        "Relendo origem, checkpoint e arquivos publicados…",
    );
}

unsafe fn preview_repair(hwnd: HWND, app: &mut App) {
    let Some(report) = &app.report else {
        return;
    };
    let [installation] = report.installations.installations.as_slice() else {
        set(
            hwnd,
            STATUS,
            "O plano assistido exige um único candidato Windows offline.",
        );
        return;
    };
    let windows_root = installation.volume.clone();
    let esp = PathBuf::from(get(hwnd, ESP));
    let destination = PathBuf::from(get(hwnd, DEST));
    work(app, move || {
        match repair::plan(&windows_root, &esp, &destination, None, None) {
            Ok(plan) => Event::Repair(Box::new(plan)),
            Err(error) => Event::Failure(error),
        }
    });
    set(
        hwnd,
        STATUS,
        "Identificando a ESP e os requisitos do plano…",
    );
    render(hwnd, app);
}

unsafe fn export_report(hwnd: HWND, app: &mut App) {
    let Some(report) = app.report.clone() else {
        return;
    };
    let destination = PathBuf::from(get(hwnd, DEST));
    work(app, move || {
        let output = (|| {
            let destination = archrescue_storage::safe_existing(&destination)?;
            let destination_id = archrescue_storage::identity(&destination)?;
            for installation in &report.installations.installations {
                let source_id = archrescue_storage::identity(&installation.volume)?;
                archrescue_storage::require_separate(&source_id, &destination_id)?;
            }
            for transaction in &report.transactions {
                let source_id = archrescue_storage::identity(&transaction.source_root)?;
                archrescue_storage::require_separate(&source_id, &destination_id)?;
            }

            archrescue_evidence::export(&report, &destination)
        })();
        match output {
            Ok(path) => Event::Exported(path),
            Err(error) => Event::Failure(error),
        }
    });
    set(hwnd, STATUS, "Exportando relatório da sessão…");
    render(hwnd, app);
}

fn error_message(error: &Error) -> String {
    let label = match error.kind {
        ErrorKind::Permission => "Acesso negado. Verifique permissões e bloqueio do volume.",
        ErrorKind::Integrity => "Falha de integridade. Confira o checkpoint e os hashes.",
        ErrorKind::Conflict => "Conflito no destino. Preserve o conteúdo existente.",
        ErrorKind::Unsupported => "Operação indisponível para esta configuração.",
        ErrorKind::Cancelled => "Operação cancelada.",
        ErrorKind::Validation => "Seleção, espaço ou identidade de disco inválidos.",
        ErrorKind::Io | ErrorKind::Environment => "Falha de acesso. Confira mídia e conexão.",
        ErrorKind::Incomplete => "Operação incompleta.",
    };
    format!("{label} {}", error.context)
}

unsafe fn events(hwnd: HWND, app: &mut App) {
    while let Ok(event) = app.rx.try_recv() {
        app.error_status = false;
        match event {
            Event::Progress(progress) => {
                let message = format!(
                    "{} | {}
Gravados {}/{} bytes | Verificados {} bytes ({} arquivos)",
                    progress.operation_id,
                    progress.stage,
                    progress.acknowledged_bytes,
                    progress.planned_bytes,
                    progress.verified_bytes,
                    progress.verified_files,
                );
                set(hwnd, STATUS, &message);
                app.progress = Some(progress);
                InvalidateRect(hwnd, null(), 0);
                continue;
            }
            Event::Deployment(deployment::Update::Progress(read, total)) => {
                deployment::update(hwnd, app, deployment::Update::Progress(read, total));
                continue;
            }
            Event::Deployment(update) => deployment::update(hwnd, app, update),
            Event::PlanExported(path) => set(
                hwnd,
                STATUS,
                &format!("Arquivo exportado: {}", path.display()),
            ),
            Event::Snapshot(mut report) => {
                deployment::refresh_disks(hwnd, app, &report);
                if let Some(old) = app.report.take() {
                    report.operations = old.operations;
                    report.transactions = old.transactions;
                    report.decisions = old.decisions;
                    report.repair_plans = old.repair_plans;
                    report.repair_attempts = old.repair_attempts;
                    report.session_errors = old.session_errors;
                }

                if app.directory.as_os_str().is_empty() {
                    let root = report
                        .installations
                        .installations
                        .first()
                        .map(|item| &item.volume)
                        .or_else(|| {
                            report
                                .inventory
                                .volumes
                                .iter()
                                .flat_map(|volume| &volume.paths)
                                .next()
                        });
                    if let Some(root) = root {
                        app.directory = root.clone();
                        set(hwnd, ADDRESS, &root.to_string_lossy());
                    }
                }
                if get(hwnd, ESP).is_empty() {
                    if let Some(esp) = report
                        .diagnosis
                        .associations
                        .iter()
                        .find_map(|item| item.mounted_efi.first())
                    {
                        set(hwnd, ESP, &esp.to_string_lossy());
                    }
                }

                app.report = Some(*report);
                set(hwnd, STATUS, "Inventário e diagnóstico atualizados.");
            }
            Event::Listing(path, entries) => show_listing(hwnd, app, path, entries),
            Event::Preview(preview) => {
                app.preview = Some(preview);
                set(hwnd, STATUS, "Revise o plano e confirme em Iniciar backup.");
            }
            Event::Checkpoints(entries) => show_checkpoints(hwnd, app, entries),
            Event::Backup(result) => {
                app.preview = None;
                app.section = VERIFY_PAGE;
                app.error_status = result.status.error.is_some();
                let message = result.status.error.as_ref().map_or_else(
                    || {
                        format!(
                            "{} | {} arquivos verificados",
                            state_label(result.status.state),
                            result.progress.verified_files
                        )
                    },
                    error_message,
                );
                set(hwnd, STATUS, &message);

                if let Some(report) = &mut app.report {
                    report.transactions.push(*result);
                }
            }
            Event::Repair(plan) => {
                if let Some(report) = &mut app.report {
                    report.repair_plans.push(*plan);
                }
                set(hwnd, STATUS, "Prévia do reparo disponível.");
            }
            Event::Exported(path) => set(
                hwnd,
                STATUS,
                &format!("Relatório gravado e verificado: {}", path.display()),
            ),
            Event::Failure(error) => {
                app.error_status = true;
                set(hwnd, STATUS, &error_message(&error));
                if let Some(report) = &mut app.report {
                    report.session_errors.push(error);
                }
            }
        }

        app.busy = false;
        app.progress = None;
        render(hwnd, app);
    }
}

unsafe fn show_checkpoints(hwnd: HWND, app: &mut App, entries: Vec<CheckpointEntry>) {
    app.preview = None;
    let list = control(hwnd, FILES);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    for entry in &entries {
        let state = match (&entry.error, &entry.status) {
            (Some(_), _) => "Inacessível ou corrompido",
            (_, Some(status)) if status.state == archrescue_core::State::Completed => {
                "Conclusão registrada"
            }
            (_, Some(status)) => state_label(status.state),
            _ => "Planejado",
        };
        let label = format!(
            "{} | {}",
            entry
                .directory
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            state
        );
        SendMessageW(list, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
    }

    set(
        hwnd,
        STATUS,
        if entries.is_empty() {
            "Nenhum checkpoint neste destino."
        } else {
            "Informe a origem no campo superior, selecione um checkpoint e use Retomar."
        },
    );
    app.checkpoints = Some(entries);
}

unsafe fn show_listing(hwnd: HWND, app: &mut App, path: PathBuf, entries: Vec<Entry>) {
    app.directory = path;
    app.entries = entries;

    set(hwnd, ADDRESS, &app.directory.to_string_lossy());
    SendMessageW(control(hwnd, FILES), LB_RESETCONTENT, 0, 0);

    for entry in &app.entries {
        let kind = if entry.directory {
            "[Pasta]"
        } else {
            "[Arquivo]"
        };
        let detail = if let Some(error) = &entry.error {
            format!("Indisponível: {error}")
        } else {
            format!("{} bytes", entry.bytes)
        };
        let text = format!("{kind}  {}  {detail}", entry.name.display());

        SendMessageW(
            control(hwnd, FILES),
            LB_ADDSTRING,
            0,
            wide(&text).as_ptr() as isize,
        );
    }

    set(
        hwnd,
        STATUS,
        if app.entries.is_empty() {
            "Pasta vazia."
        } else {
            "Listagem concluída. Ctrl/Shift para selecionar; clique duplo para abrir uma pasta."
        },
    );
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;

    if msg == WM_NCCREATE {
        let create = &*(lp as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wp, lp);
    }

    if state_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }

    let state = &*state_ptr;
    if matches!(
        msg,
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX
    ) {
        SetTextColor(wp as HDC, theme::TEXT);
        SetBkColor(wp as HDC, theme::PANEL);
        return state.theme.panel as isize;
    }

    // Native window operations can send these messages synchronously while
    // application state is borrowed. They only need the window and must still run.
    match msg {
        WM_PAINT => {
            let app = state.app.try_borrow().ok();
            presentation::paint(hwnd, &state.theme, app.as_deref());
            return 0;
        }
        WM_DRAWITEM if lp != 0 => {
            // The notification supplies a live DRAWITEMSTRUCT and child paint DC.
            let item = &*(lp as *const DRAWITEMSTRUCT);
            let selected = state.selected_page.get();
            presentation::button(item, &state.theme, selected);
            return 1;
        }
        WM_GETMINMAXINFO => {
            let info = &mut *(lp as *mut MINMAXINFO);
            info.ptMinTrackSize = POINT {
                x: 800,
                y: 600,
            };
            return 0;
        }
        WM_DESTROY => {
            KillTimer(hwnd, 1);
            PostQuitMessage(0);
            return 0;
        }
        _ => {}
    }

    // RefCell prevents overlapping mutable access when native controls reenter.
    let Ok(mut app_borrow) = state.app.try_borrow_mut() else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    let app = &mut *app_borrow;

    match msg {
        WM_SIZE => {
            if !control(hwnd, STATUS).is_null() {
                render(hwnd, app);
            }
            0
        }
        WM_COMMAND => {
            let id = wp & 0xffff;
            action(
                hwnd,
                app,
                if id == IDCANCEL as usize { CANCEL } else { id },
                ((wp >> 16) & 0xffff) as u32,
            );
            state.selected_page.set(app.section);
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_TIMER => {
            events(hwnd, app);
            state.selected_page.set(app.section);
            0
        }
        WM_CLOSE => {
            if app.busy {
                app.cancel.store(true, Ordering::Relaxed);
                set(
                    hwnd,
                    STATUS,
                    "Aguarde a operação terminar ou confirmar cancelamento antes de fechar.",
                );
            } else {
                DestroyWindow(hwnd);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

struct OwnedWindow(HWND);

impl Drop for OwnedWindow {
    fn drop(&mut self) {
        // SAFETY: the window belongs to the calling GUI thread and state outlives it.
        unsafe {
            if IsWindow(self.0) != 0 {
                DestroyWindow(self.0);
            }
        }
    }
}

pub fn run() -> Result<()> {
    unsafe {
        let (tx, rx) = mpsc::channel();
        let theme = theme::Theme::new()?;
        let font = theme.body;
        let app = Box::new(WindowState {
            app: RefCell::new(App {
                deployment: deployment::Deployment::default(),
                section: DASHBOARD,
                technician: false,
                preview: None,
                checkpoints: None,
                report: None,
                entries: vec![],
                directory: PathBuf::new(),
                busy: false,
                error_status: false,
                progress: None,
                cancellable: false,
                cancel: Arc::new(AtomicBool::new(false)),
                tx,
                rx,
            }),
            theme,
            selected_page: Cell::new(DASHBOARD),
        });

        let class = wide("ArchRescueRecovery");
        let instance = GetModuleHandleW(null());
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszClassName: class.as_ptr(),
            ..Default::default()
        };

        if RegisterClassW(&window_class) == 0 {
            return Err(os_error("Register native window"));
        }

        let title = format!("ArchRescue {VERSION} — Recuperação e diagnóstico");
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            class.as_ptr(),
            wide(&title).as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            GetSystemMetrics(SM_CXSCREEN).min(1280),
            GetSystemMetrics(SM_CYSCREEN).min(820),
            null_mut(),
            null_mut(),
            instance,
            (&*app as *const WindowState).cast(),
        );

        if hwnd.is_null() {
            return Err(os_error("Create ArchRescue window"));
        }

        let window = OwnedWindow(hwnd);
        for (index, label) in i18n::SECTIONS.iter().enumerate() {
            add(hwnd, 100 + index, "BUTTON", label, WS_TABSTOP, font)?;
        }

        for (id, label) in [
            (REFRESH, "Atualizar"),
            (OPEN, "Abrir"),
            (UP, "Subir"),
            (PREVIEW, "Plane&jar"),
            (COPY, "&Iniciar backup"),
            (CHECKPOINTS, "Checkpoints"),
            (RESUME, "&Retomar"),
            (NEXT, "Revisar diagnóstico"),
            (TECHNICIAN, "Modo &técnico"),
            (VERIFY, "Verificar novamente"),
            (REPAIR, "Gerar prévia do reparo"),
            (CANCEL, "Cancelar"),
            (EXPORT, "Exportar relatório"),
            (CONSOLE, "Console de diagnóstico"),
        ] {
            add(hwnd, id, "BUTTON", label, WS_TABSTOP, font)?;
        }

        add(
            hwnd,
            ADDRESS,
            "EDIT",
            "",
            WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            font,
        )?;
        add(
            hwnd,
            DEST,
            "EDIT",
            "",
            WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            font,
        )?;
        add(
            hwnd,
            ESP,
            "EDIT",
            "",
            WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            font,
        )?;
        for (id, hint) in [
            (DEST, "Destino: pasta em outro disco"),
            (ADDRESS, "Origem: pasta que contém os dados"),
            (ESP, "ESP: raiz da partição EFI acessível"),
        ] {
            SendMessageW(control(hwnd, id), 0x1501, 1, wide(hint).as_ptr() as isize);
        }

        add(
            hwnd,
            FILES,
            "LISTBOX",
            "",
            WS_TABSTOP
                | WS_BORDER
                | WS_VSCROLL
                | WS_HSCROLL
                | LBS_EXTENDEDSEL as u32
                | LBS_NOTIFY as u32,
            font,
        )?;
        add(
            hwnd,
            CONTENT,
            "EDIT",
            i18n::LOADING,
            WS_TABSTOP
                | WS_VSCROLL
                | ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32,
            font,
        )?;
        add(hwnd, STATUS, "STATIC", i18n::LOADING, 0, font)?;

        deployment::controls(hwnd, font)?;
        layout::arrange(hwnd, DASHBOARD);
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        SetTimer(hwnd, 1, 100, None);
        refresh(hwnd, &mut app.app.borrow_mut());

        let mut msg = MSG::default();
        loop {
            let result = GetMessageW(&mut msg, null_mut(), 0, 0);
            if result == 0 {
                break;
            }
            if result == -1 {
                return Err(os_error("Native message loop"));
            }
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        drop(window);

        Ok(())
    }
}
