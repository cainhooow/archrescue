//! Native Win32 shell. Long-running filesystem work is performed outside the UI thread.

mod confirmation;
mod console;
mod deployment;
mod layout;
mod navigation;
mod pages;
mod picker;
mod presentation;
mod renderer;
mod scroll;
mod select;
mod session;
mod theme;
mod window;

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
        Controls::{
            DRAWITEMSTRUCT, EM_SETMARGINS, MEASUREITEMSTRUCT, ODT_BUTTON, ODT_LISTBOX,
            WM_MOUSELEAVE,
        },
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{
            EnableWindow, GetFocus, IsWindowEnabled, SetFocus, TrackMouseEvent, TME_LEAVE,
            TRACKMOUSEEVENT, VK_F11, VK_RETURN,
        },
        WindowsAndMessaging::*,
    },
};

use crate::{
    i18n, paths,
    resources::{Catalog, Location},
    views::state_label,
};
use session::{CHOOSE_BACKUP, CHOOSE_WINDOWS, SELECT_RECOMMENDED};

const REFRESH: usize = 200;
const ADDRESS: usize = 201;
const FILES: usize = 204;
const DEST: usize = 205;
const COPY: usize = 206;
const CANCEL: usize = 207;
const EXPORT: usize = 208;
const CONTENT: usize = 210;
const STATUS: usize = 211;
const PREVIEW: usize = 212;
const CHECKPOINTS: usize = 213;
const RESUME: usize = 214;
const PREVIOUS: usize = 215;
const NEXT: usize = 216;
const TECHNICIAN: usize = 217;
const VERIFY: usize = 218;
const REPAIR: usize = 219;
const ESP: usize = 220;
const DISK_DETAILS: usize = 221;
const SOURCE_LABEL: usize = 222;
const DESTINATION_LABEL: usize = 223;
const ESP_LABEL: usize = 224;
const REPAIR_UNAVAILABLE: usize = 225;
const BACKUP_SELECTION: usize = 226;

const RENDER_FAILURE: u32 = WM_APP + 1;
const BUTTON_CLASS: [u16; 6] = [
    b'B' as u16,
    b'u' as u16,
    b't' as u16,
    b't' as u16,
    b'o' as u16,
    b'n' as u16,
];

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
    Snapshot(Box<(Report, Catalog)>),
    Listing(Location, Vec<Entry>, bool),
    BackupVerified(Result<()>),
    Progress(Progress),
    Preview(Preview),
    Checkpoints(Vec<CheckpointEntry>),
    Backup(Box<TransactionReport>),
    Repair(Box<repair::RepairPlan>),
    Exported(PathBuf),
    PlanExported(PathBuf),
    HandoffExported(PathBuf),
    Deployment(deployment::Update),
    Console(console::Outcome),
    Integrity(Box<archrescue_integrity::IntegrityReport>),
    IntegrityProgress(archrescue_integrity::Progress),
    Failure(Error),
}

#[derive(Clone)]
struct Preview {
    source: Location,
    destination: Location,
    available_bytes: u64,
    selections: Vec<PathBuf>,
    files: usize,
    bytes: u64,
}

struct App {
    deployment: deployment::Deployment,
    console: console::Console,
    session: session::Session,
    picker: Option<picker::Purpose>,
    recommended: bool,
    refresh_pending: bool,
    section: usize,
    technician: bool,
    preview: Option<Preview>,
    current_repair: Option<repair::RepairPlan>,
    checkpoints: Option<Vec<CheckpointEntry>>,
    confirmation: Option<confirmation::Confirmation>,
    report: Option<Report>,
    entries: Vec<Entry>,
    listing_loaded: bool,
    directory: PathBuf,
    busy: bool,
    error_status: bool,
    operation_page: usize,
    progress: Option<Progress>,
    media_progress: Option<(u64, u64)>,
    report_export: Option<PathBuf>,
    handoff_export: Option<PathBuf>,
    scanning: bool,
    roots: Vec<PathBuf>,
    root_index: usize,
    row_index: usize,
    cancellable: bool,
    cancel: Arc<AtomicBool>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

struct WindowState {
    app: RefCell<App>,
    theme: theme::Theme,
    frame: RefCell<renderer::Frame>,
    control_frame: RefCell<renderer::Frame>,
    scene_dirty: Cell<bool>,
    render_failed: Cell<bool>,
    mode: window::Mode,
    hovered: Cell<HWND>,
    selected_page: Cell<usize>,
    confirming: Cell<bool>,
}

unsafe fn invalidate_scene(hwnd: HWND) {
    // The HWND belongs to this thread and WindowState outlives it.
    let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
    if let Some(state) = state.as_ref() {
        state.scene_dirty.set(true);
    }

    InvalidateRect(hwnd, null(), 0);
}

unsafe fn control(hwnd: HWND, id: usize) -> HWND {
    GetDlgItem(hwnd, id as i32)
}

unsafe fn set(hwnd: HWND, id: usize, text: &str) {
    let control = control(hwnd, id);
    let text = wide(&text.replace("\r\n", "\n").replace('\n', "\r\n"));

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
    let style = style
        | match class {
            "BUTTON" => BS_OWNERDRAW as u32,
            "LISTBOX" => (LBS_OWNERDRAWFIXED | LBS_HASSTRINGS | LBS_NOINTEGRALHEIGHT) as u32,
            _ => 0,
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
    if matches!(id, FILES | CONTENT | BACKUP_SELECTION | pages::ROWS) {
        scroll::attach(hwnd, id, child)?;
    }
    if class == "EDIT" {
        SendMessageW(child, EM_SETMARGINS, 3, (7 << 16) | 7);
    }

    Ok(())
}

unsafe fn is_button(hwnd: HWND) -> bool {
    let mut class = [0; 16];
    let length = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);

    length == BUTTON_CLASS.len() as i32 && class[..BUTTON_CLASS.len()] == BUTTON_CLASS
}

unsafe fn track_hover(hwnd: HWND, state: &WindowState, message: &MSG) {
    let hovered = state.hovered.get();
    if message.message == WM_MOUSELEAVE && message.hwnd == hovered {
        state.hovered.set(null_mut());
        InvalidateRect(hovered, null(), 0);
        return;
    }
    if message.message != WM_MOUSEMOVE || message.hwnd == hovered {
        return;
    }
    if GetParent(message.hwnd) != hwnd || !is_button(message.hwnd) {
        return;
    }

    let mut tracking = TRACKMOUSEEVENT {
        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE,
        hwndTrack: message.hwnd,
        dwHoverTime: 0,
    };
    if TrackMouseEvent(&mut tracking) != 0 {
        state.hovered.set(message.hwnd);
        if !hovered.is_null() {
            InvalidateRect(hovered, null(), 0);
        }
        InvalidateRect(message.hwnd, null(), 0);
    }
}

unsafe fn place(hwnd: HWND, id: usize, x: i32, y: i32, width: i32, height: i32) {
    let width = width.max(1);
    let height = height.max(1);
    MoveWindow(control(hwnd, id), x, y, width, height, 1);
    scroll::place(hwnd, id, x, y, width, height);
}

unsafe fn show(hwnd: HWND, id: usize, visible: bool) {
    ShowWindow(control(hwnd, id), if visible { SW_SHOW } else { SW_HIDE });
    scroll::show(hwnd, id, visible);
}

unsafe fn render(hwnd: HWND, app: &App) {
    session::render_fields(hwnd, app);
    let protect = app.section == BACKUP;
    let repair = app.section == REPAIR_PAGE;
    let verify = app.section == VERIFY_PAGE;
    let report_stage = app.section == REPORTS;
    let has_selection =
        protect && app.preview.is_none() && (app.listing_loaded || app.checkpoints.is_some());
    show(hwnd, BACKUP_SELECTION, has_selection);
    let has_entries = app
        .checkpoints
        .as_ref()
        .map_or(!app.entries.is_empty(), |entries| !entries.is_empty());
    show(hwnd, FILES, protect && app.preview.is_none() && has_entries);
    show(
        hwnd,
        CONTENT,
        if protect {
            app.preview.is_some()
        } else {
            !matches!(app.section, DASHBOARD | deployment::RESTORE)
        },
    );
    show(hwnd, DISK_DETAILS, app.section == DASHBOARD);
    show(
        hwnd,
        CHOOSE_WINDOWS,
        protect || repair || app.section == DIAGNOSIS,
    );
    show(hwnd, CHOOSE_BACKUP, verify);
    show(
        hwnd,
        SELECT_RECOMMENDED,
        protect && app.recommended && app.preview.is_none() && app.checkpoints.is_none(),
    );
    show(hwnd, ESP, repair);
    show(hwnd, ESP_LABEL, repair);
    show(hwnd, SOURCE_LABEL, protect);
    let destination = protect
        || repair
        || report_stage
        || matches!(
            app.section,
            deployment::DEPLOY | deployment::RESTORE | console::PAGE
        );
    let destination_label = match app.section {
        REPORTS => "Pasta em disco separado",
        REPAIR_PAGE => "Destino de preservação",
        deployment::RESTORE => "Pasta para exportação",
        _ => "Destino",
    };
    set(hwnd, DESTINATION_LABEL, destination_label);
    for id in [DESTINATION_LABEL, DEST] {
        show(
            hwnd,
            id,
            destination && (id != DESTINATION_LABEL || app.section != console::PAGE),
        );
    }
    for id in [ADDRESS, PREVIEW, COPY, CHECKPOINTS, RESUME] {
        show(hwnd, id, protect);
    }
    show(hwnd, EXPORT, report_stage);
    show(hwnd, VERIFY, verify);
    show(hwnd, REPAIR, repair);
    show(hwnd, REPAIR_UNAVAILABLE, repair);
    EnableWindow(control(hwnd, REPAIR_UNAVAILABLE), 0);
    EnableWindow(
        control(hwnd, PREVIOUS),
        navigation::adjacent(app.section, -1).is_some() as i32,
    );
    EnableWindow(
        control(hwnd, NEXT),
        navigation::adjacent(app.section, 1).is_some() as i32,
    );

    let area = layout::Layout::read(hwnd);
    for page in navigation::ORDER {
        set(
            hwnd,
            100 + page,
            navigation::label(page, area.rail < 224, app.technician),
        );
        InvalidateRect(control(hwnd, 100 + page), null(), 0);
    }
    for (id, full, compact) in [
        (REFRESH, "Atualizar diagnóstico", "Atualizar"),
        (CANCEL, "Cancelar operação", "Cancelar"),
    ] {
        set(hwnd, id, if area.rail < 176 { compact } else { full });
    }

    for id in [
        CHOOSE_WINDOWS,
        CHOOSE_BACKUP,
        SELECT_RECOMMENDED,
        ADDRESS,
        DEST,
        ESP,
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
        (!app.busy && app.report.is_some() && app.session.destination(REPORTS).is_ok()) as i32,
    );
    let has_backup = app
        .report
        .as_ref()
        .is_some_and(|report| !report.transactions.is_empty());
    EnableWindow(
        control(hwnd, VERIFY),
        (!app.busy && (has_backup || app.session.backup.is_some())) as i32,
    );
    set(
        hwnd,
        TECHNICIAN,
        if app.technician {
            "Visão assistida"
        } else {
            "Modo técnico"
        },
    );

    layout::arrange(hwnd, app);
    if has_selection {
        update_backup_selection(hwnd, app);
    }
    invalidate_scene(hwnd);

    let structured = pages::render_page(hwnd, app);
    let deployment = deployment::render(hwnd, app);
    let console = console::render(hwnd, app);
    if console || deployment || structured {
        return;
    }

    if let Some(preview) = app.preview.as_ref().filter(|_| protect) {
        let selection = preview
            .selections
            .iter()
            .map(|path| {
                if app.recommended {
                    session::folder_label(path)
                } else {
                    paths::display(path, app.technician)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        set(hwnd, CONTENT, &selection);
        return;
    }

    let Some(report) = &app.report else {
        set(
            hwnd,
            CONTENT,
            if app.busy { i18n::LOADING } else { i18n::EMPTY },
        );
        return;
    };
    let text = match app.section {
        REPAIR_PAGE => crate::views::repair(app.current_repair.as_ref(), app.technician),
        VERIFY_PAGE if app.session.backup.is_some() => {
            let backup = app.session.backup.as_ref().unwrap();
            let status = match &app.session.backup_verification {
                None => "Manifesto validado. Conteúdo aguarda verificação.".into(),
                Some(Ok(())) => "Conteúdo conferido por SHA-256 nesta sessão.".into(),
                Some(Err(error)) => error_message(error),
            };
            format!("{status}\r\n{}", backup.location.label(app.technician))
        }
        VERIFY_PAGE => crate::views::verify(report, app.technician),
        REPORTS => crate::views::session(report, app.technician),
        _ => String::new(),
    };
    set(hwnd, CONTENT, &text);
}

fn work(app: &mut App, job: impl FnOnce() -> Event + Send + 'static) {
    app.busy = true;
    app.error_status = false;
    app.operation_page = app.section;
    app.progress = None;
    app.media_progress = None;
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
    if app.report.is_none() {
        set(hwnd, STATUS, i18n::LOADING);
    }
    let generation = app.session.catalog.generation + 1;
    let registered = app.session.registered.clone();
    work(app, move || {
        let inventory = archrescue_storage::inventory();
        let roots = inventory
            .volumes
            .iter()
            .filter_map(crate::resources::volume_root)
            .collect::<Vec<_>>();

        let report = Report::collect_with_inventory(&roots, inventory);

        let catalog = Catalog::discover(&report, &registered, generation);
        Event::Snapshot(Box::new((report, catalog)))
    });

    render(hwnd, app);
}

unsafe fn browse(hwnd: HWND, app: &mut App, source: Location) {
    app.preview = None;
    app.checkpoints = None;
    app.listing_loaded = false;
    set(hwnd, STATUS, i18n::LOADING);
    work(app, move || {
        match source
            .revalidate()
            .and_then(|directory| archrescue_backup::browse(&directory))
        {
            Ok(entries) => Event::Listing(source, entries, false),
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
        .filter(|entry| entry.error.is_none())
        .map(|entry| entry.name.clone())
        .collect()
}

unsafe fn update_backup_selection(hwnd: HWND, app: &App) {
    let indices = selected_indices(hwnd);
    let summary = if let Some(checkpoints) = &app.checkpoints {
        match indices.as_slice() {
            [] => "Selecione um checkpoint e use Retomar.".to_owned(),
            [index] => checkpoints.get(*index).map_or_else(String::new, |entry| {
                let path = paths::display(&entry.directory, app.technician);
                let status = entry
                    .status
                    .as_ref()
                    .map_or("Planejado", |status| state_label(status.state));
                format!("{status}\n{path}")
            }),
            _ => "Selecione um único checkpoint para retomar.".to_owned(),
        }
    } else if indices.is_empty() {
        "Use Ctrl ou Shift para selecionar vários itens.".to_owned()
    } else {
        indices
            .into_iter()
            .filter_map(|index| app.entries.get(index))
            .map(|entry| {
                if app.recommended {
                    session::folder_label(&entry.name)
                } else {
                    paths::display(&entry.name, app.technician)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    set(hwnd, BACKUP_SELECTION, &summary);
}

unsafe fn stage(hwnd: HWND, app: &mut App, section: usize) {
    app.section = section;
    app.row_index = 0;
    if section == BACKUP && app.session.source.is_none() && !app.busy {
        if let Some(windows) = app.session.available_windows().cloned() {
            session::recommended(hwnd, app, windows);
        }
    }
    render(hwnd, app);
    // Keep focus on the selected navigation button so Tab continues in page order.
    SetFocus(control(hwnd, 100 + navigation::owner(section)));
}

unsafe fn action(hwnd: HWND, app: &mut App, id: usize, notification: u32) {
    if let Some(page) = id
        .checked_sub(100)
        .filter(|page| navigation::ORDER.contains(page))
    {
        stage(hwnd, app, page);
        return;
    }

    match id {
        PREVIOUS | NEXT => {
            let offset = if id == PREVIOUS { -1 } else { 1 };
            if let Some(page) = navigation::adjacent(app.section, offset) {
                stage(hwnd, app, page);
            }
            return;
        }
        DISK_DETAILS => {
            stage(hwnd, app, DISKS);
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

    if pages::action(hwnd, app, id, notification) || app.busy {
        return;
    }

    if console::action(hwnd, app, id) || deployment::action(hwnd, app, id, notification) {
        return;
    }

    match id {
        REFRESH => refresh(hwnd, app),
        CHOOSE_WINDOWS => session::request(app, picker::Purpose::Windows),
        CHOOSE_BACKUP => session::request(app, picker::Purpose::Backup),
        ADDRESS => session::request(app, picker::Purpose::Source),
        ESP => session::request(app, picker::Purpose::Efi),
        DEST => {
            let purpose = match app.section {
                BACKUP => picker::Purpose::SaveFiles,
                REPAIR_PAGE => picker::Purpose::Preserve,
                deployment::DEPLOY => picker::Purpose::Plan,
                deployment::RESTORE => picker::Purpose::Handoff,
                console::PAGE => picker::Purpose::ConsoleLog,
                _ => picker::Purpose::Report,
            };
            session::request(app, purpose);
        }
        SELECT_RECOMMENDED => {
            SendMessageW(control(hwnd, FILES), LB_SETSEL, 1, -1);
            update_backup_selection(hwnd, app);
        }
        FILES if notification == LBN_SELCHANGE => update_backup_selection(hwnd, app),
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
                    if let Some(source) = app.session.source.clone() {
                        app.preview = None;
                        set(hwnd, STATUS, i18n::LOADING);
                        work(app, move || {
                            let listing = (|| {
                                source.revalidate()?;
                                let location = Location::capture(&path)?;
                                let entries = archrescue_backup::browse(&location.path)?;
                                Ok(Event::Listing(location, entries, false))
                            })();
                            listing.unwrap_or_else(Event::Failure)
                        });
                        render(hwnd, app);
                    }
                }
            }
        }
        PREVIEW => preview_backup(hwnd, app),
        COPY => start_backup(app),
        CHECKPOINTS => list_checkpoints(hwnd, app),
        RESUME => resume_backup(hwnd, app),
        VERIFY => verify_backup(hwnd, app),
        REPAIR => preview_repair(hwnd, app),
        EXPORT => export_report(hwnd, app),
        _ => {}
    }
}

unsafe fn preview_backup(hwnd: HWND, app: &mut App) {
    let selections = selected(hwnd, app);
    if selections.is_empty() {
        set(hwnd, STATUS, "Selecione arquivos ou pastas na lista.");
        return;
    }

    let Some(source) = app.session.source.clone() else {
        set(
            hwnd,
            STATUS,
            "Escolha a instalação do Windows ou uma pasta de origem.",
        );
        return;
    };
    let Ok(destination) = app.session.destination(BACKUP) else {
        set(hwnd, STATUS, "Escolha onde salvar os arquivos.");
        return;
    };
    app.cancel = Arc::new(AtomicBool::new(false));
    let cancel = app.cancel.clone();
    work(app, move || {
        let result = (|| {
            let source_path = source.revalidate()?;
            let destination_path = destination.revalidate()?;
            let space =
                crate::resources::writable(&destination_path, std::slice::from_ref(&source), 0)?;
            let (files, bytes) =
                transaction::preview(&source_path, &selections, &destination_path, &cancel)?;
            Ok(Event::Preview(Preview {
                source,
                destination,
                available_bytes: space.available,
                selections,
                files,
                bytes,
            }))
        })();
        result.unwrap_or_else(Event::Failure)
    });

    app.cancellable = true;
    set(hwnd, STATUS, "Calculando tamanho e validando discos…");
    render(hwnd, app);
}

fn start_backup(app: &mut App) {
    let Some(preview) = app.preview.clone() else {
        return;
    };
    if confirmation::matches_preview(app, &preview) {
        app.confirmation = Some(confirmation::Confirmation::Start(preview));
    }
}

unsafe fn list_checkpoints(hwnd: HWND, app: &mut App) {
    let Ok(destination) = app.session.destination(BACKUP) else {
        session::request(app, picker::Purpose::Backup);
        return;
    };
    work(app, move || {
        match destination
            .revalidate()
            .and_then(|path| transaction::checkpoints(&path))
        {
            Ok(entries) => Event::Checkpoints(entries),
            Err(error) => Event::Failure(error),
        }
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

    let Some(source) = app.session.source.clone() else {
        set(hwnd, STATUS, "Escolha a pasta original para retomar.");
        return;
    };
    let checkpoint = entry.directory.clone();
    let expected_destination = entry
        .manifest
        .as_ref()
        .map(|manifest| manifest.destination_identity.clone());
    let fingerprint = entry
        .manifest
        .as_ref()
        .map(|manifest| manifest.plan_fingerprint.clone());
    app.confirmation = Some(confirmation::Confirmation::Resume {
        source,
        checkpoint,
        fingerprint,
        expected_destination,
        recorded_state: entry.status.as_ref().map(|status| status.state),
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
    if let Some(backup) = app.session.backup.clone() {
        let cancel = Arc::new(AtomicBool::new(false));
        app.cancel = cancel.clone();
        app.session.backup_verification = None;
        work(app, move || {
            let result = backup
                .revalidate()
                .and_then(|path| transaction::verify_payload(&path, &cancel))
                .and_then(|manifest| {
                    if manifest.plan_fingerprint == backup.fingerprint {
                        Ok(())
                    } else {
                        Err(Error::new(
                            ErrorKind::Conflict,
                            "O manifesto mudou durante a verificação.",
                        ))
                    }
                });
            Event::BackupVerified(result)
        });
        app.cancellable = true;
        set(hwnd, STATUS, "Relendo o conteúdo publicado do backup…");
        render(hwnd, app);
        return;
    }

    let Some(backup) = app
        .report
        .as_ref()
        .and_then(|report| report.transactions.last())
    else {
        return;
    };
    let directory = backup.directory.clone();
    let source = backup.source_root.clone();
    let expected = backup.manifest.plan_fingerprint.clone();
    begin_backup(hwnd, app, move |cancel, _| {
        let manifest = transaction::manifest(&directory)?;
        if manifest.plan_fingerprint != expected {
            return Err(Error::new(
                ErrorKind::Conflict,
                "A operação de backup mudou.",
            ));
        }
        transaction::verify(&source, &directory, cancel)
    });
    set(
        hwnd,
        STATUS,
        "Relendo origem, checkpoint e arquivos publicados…",
    );
}

unsafe fn preview_repair(hwnd: HWND, app: &mut App) {
    let Some(windows) = app.session.available_windows().cloned() else {
        set(hwnd, STATUS, "Escolha uma instalação Windows disponível.");
        return;
    };
    let Ok(destination) = app.session.destination(REPAIR_PAGE) else {
        set(
            hwnd,
            STATUS,
            "Escolha onde preservar os arquivos EFI e os dados.",
        );
        return;
    };
    let Some(efi) = app
        .session
        .efi
        .clone()
        .or_else(|| app.session.suggested_efi.clone())
    else {
        set(hwnd, STATUS, "Não foi possível identificar com segurança a partição de inicialização. Consulte as candidatas.");
        return;
    };
    work(app, move || {
        let result = (|| {
            let windows_root = windows.revalidate()?;
            let esp = efi.revalidate()?;
            let destination = destination.revalidate()?;
            crate::resources::writable(&destination, &[windows.location, efi], 64 * 1024 * 1024)?;
            repair::plan(&windows_root, &esp, &destination, None, None)
        })();
        match result {
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
    let Ok(destination) = app.session.destination(REPORTS) else {
        return;
    };
    let sources = app.session.sources(REPORTS);
    work(app, move || {
        let output = (|| {
            let destination = destination.revalidate()?;
            crate::resources::writable(&destination, &sources, 0)?;
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
                    "{}
Gravados {}/{} bytes · Verificados {} bytes · {} arquivos · {} falhas",
                    progress.stage,
                    progress.acknowledged_bytes,
                    progress.planned_bytes,
                    progress.verified_bytes,
                    progress.verified_files,
                    progress.failed_files,
                );
                set(hwnd, STATUS, &message);
                app.progress = Some(progress);
                invalidate_scene(hwnd);
                continue;
            }
            Event::Deployment(deployment::Update::Progress(read, total)) => {
                app.media_progress = Some((read, total));
                deployment::update(hwnd, app, deployment::Update::Progress(read, total));
                invalidate_scene(hwnd);
                continue;
            }
            Event::Deployment(update) => deployment::update(hwnd, app, update),
            Event::PlanExported(path) => set(
                hwnd,
                STATUS,
                &format!(
                    "Arquivo exportado: {}",
                    paths::display(&path, app.technician)
                ),
            ),
            Event::Snapshot(snapshot) => {
                let (mut report, catalog) = *snapshot;
                let old_volumes = app
                    .report
                    .as_ref()
                    .map_or(0, |old| old.inventory.volumes.len());
                let stale = app.session.replace_catalog(catalog);
                app.session.suggest_efi(&report);
                pages::refresh_sources(hwnd, app, &report);
                if let Some(old) = app.report.take() {
                    report.integrity = old.integrity;
                    report.operations = old.operations;
                    report.transactions = old.transactions;
                    report.decisions = old.decisions;
                    report.repair_plans = old.repair_plans;
                    report.repair_attempts = old.repair_attempts;
                    report.session_errors = old.session_errors;
                    report.console_audit = old.console_audit;
                }
                let new_volumes = report.inventory.volumes.len();
                app.report = Some(report);
                app.session.backup_verification = None;
                let status = if stale {
                    "O Windows selecionado está indisponível. Atualize ou escolha outro."
                } else if new_volumes > old_volumes {
                    "Volumes encontrados. Inventário e diagnóstico atualizados."
                } else if new_volumes < old_volumes {
                    "Um ou mais volumes foram removidos. Confira suas seleções."
                } else {
                    "Inventário e diagnóstico atualizados."
                };
                let warning = app.session.catalog.warnings.first();
                if old_volumes != new_volumes
                    || stale
                    || warning.is_some()
                    || app
                        .report
                        .as_ref()
                        .is_some_and(|report| report.transactions.is_empty())
                {
                    set(hwnd, STATUS, warning.map_or(status, String::as_str));
                }
            }
            Event::Listing(location, entries, recommended) => {
                app.recommended = recommended;
                show_listing(hwnd, app, location, entries);
            }
            Event::BackupVerified(result) => {
                let status = match &result {
                    Ok(()) => "Conteúdo do backup conferido por SHA-256.".into(),
                    Err(error) => error_message(error),
                };
                app.error_status = result.is_err();
                app.session.backup_verification = Some(result);
                set(hwnd, STATUS, &status);
            }
            Event::Console(result) => console::update(hwnd, app, result),
            Event::Preview(preview) => {
                app.preview = Some(preview);
                set(hwnd, STATUS, "Revise o plano e confirme em Iniciar backup.");
            }
            Event::Checkpoints(entries) => show_checkpoints(hwnd, app, entries),
            Event::Backup(result) => {
                app.preview = None;
                app.section = VERIFY_PAGE;
                app.session.backup = None;
                app.session.backup_verification = None;
                if let Some(parent) = result.directory.parent() {
                    let parent = parent.to_owned();
                    if !app.session.registered.contains(&parent) {
                        app.session.registered.push(parent);
                    }
                }
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

                app.refresh_pending = true;
                if let Some(report) = &mut app.report {
                    report.transactions.push(*result);
                }
            }
            Event::Repair(plan) => {
                app.current_repair = Some((*plan).clone());
                if let Some(report) = &mut app.report {
                    report.repair_plans.push(*plan);
                }
                set(hwnd, STATUS, "Prévia do reparo disponível.");
            }
            Event::Exported(path) => {
                set(
                    hwnd,
                    STATUS,
                    &format!(
                        "Relatório gravado e verificado: {}",
                        paths::display(&path, app.technician)
                    ),
                );
                app.report_export = Some(path);
            }
            Event::HandoffExported(path) => {
                set(
                    hwnd,
                    STATUS,
                    &format!(
                        "Referência exportada: {}",
                        paths::display(&path, app.technician)
                    ),
                );
                app.handoff_export = Some(path);
            }
            Event::IntegrityProgress(progress) => {
                set(
                    hwnd,
                    STATUS,
                    &format!(
                        "{} · {} verificações registradas",
                        progress.phase, progress.completed_checks
                    ),
                );
                continue;
            }
            Event::Integrity(scan) => {
                app.scanning = false;
                app.row_index = 0;
                app.error_status = matches!(scan.status, archrescue_integrity::Status::Blocked);
                set(hwnd, STATUS, scan.status.label());
                if let Some(report) = &mut app.report {
                    report
                        .integrity
                        .retain(|previous| previous.root != scan.root);
                    report.integrity.push(*scan);
                }
            }
            Event::Failure(error) => {
                app.scanning = false;
                app.error_status = true;
                set(hwnd, STATUS, &error_message(&error));
                if let Some(report) = &mut app.report {
                    report.session_errors.push(error);
                }
            }
        }

        app.busy = false;
        app.progress = None;
        app.media_progress = None;
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
            "{}\t{}",
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
            "Escolha a pasta original, selecione um checkpoint e use Retomar."
        },
    );
    app.checkpoints = Some(entries);
}

unsafe fn show_listing(hwnd: HWND, app: &mut App, location: Location, entries: Vec<Entry>) {
    app.directory = location.path.clone();
    app.session.source = Some(location);
    app.entries = entries;
    app.listing_loaded = true;

    SendMessageW(control(hwnd, FILES), LB_RESETCONTENT, 0, 0);

    for entry in &app.entries {
        let kind = if entry.directory { "Pasta" } else { "Arquivo" };
        let detail = if let Some(error) = &entry.error {
            format!("Indisponível: {error}")
        } else if entry.directory {
            "Calcular na prévia".into()
        } else {
            format!("{} bytes", entry.bytes)
        };
        let name = if app.recommended {
            session::folder_label(&entry.name)
        } else {
            paths::friendly(&entry.name)
        };
        let text = format!("{name}\t{kind}\t{detail}");

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
        let id = GetDlgCtrlID(lp as HWND) as usize;
        let is_form_label = matches!(id, SOURCE_LABEL | DESTINATION_LABEL | ESP_LABEL)
            || deployment::form_label(id);
        SetTextColor(
            wp as HDC,
            if is_form_label {
                theme::MUTED
            } else {
                theme::TEXT
            },
        );
        if matches!(id, ADDRESS | DEST | ESP) || deployment::form_field(id) {
            SetBkColor(wp as HDC, theme::INPUT);
            return state.theme.input as isize;
        }
        if id == STATUS {
            SetBkColor(wp as HDC, theme::FOOTER);
            return state.theme.footer as isize;
        }
        SetBkColor(wp as HDC, theme::PANEL);
        return state.theme.panel as isize;
    }

    // Native window operations can send these messages synchronously while
    // application state is borrowed. They only need the window and must still run.
    match msg {
        WM_DEVICECHANGE
            if matches!(
                wp as u32,
                DBT_DEVICEARRIVAL | DBT_DEVICEREMOVECOMPLETE | DBT_DEVNODES_CHANGED
            ) =>
        {
            if let Ok(mut app) = state.app.try_borrow_mut() {
                app.refresh_pending = true;
            }
            return 1;
        }
        WM_PAINT => {
            presentation::paint(hwnd, state);
            return 0;
        }
        WM_DRAWITEM if lp != 0 => {
            // The notification supplies a live DRAWITEMSTRUCT and child paint DC.
            let item = &*(lp as *const DRAWITEMSTRUCT);
            if !matches!(item.CtlType, ODT_BUTTON | ODT_LISTBOX) {
                return DefWindowProcW(hwnd, msg, wp, lp);
            }
            if let Ok(mut frame) = state.control_frame.try_borrow_mut() {
                presentation::control_frame(
                    &mut frame,
                    item,
                    &state.theme,
                    state.selected_page.get(),
                    state.hovered.get() == item.hwndItem,
                );
            }
            return 1;
        }
        WM_MEASUREITEM if lp != 0 => {
            // The owner-drawn row height is in client pixels, like the layout.
            let item = &mut *(lp as *mut MEASUREITEMSTRUCT);
            item.itemHeight = 27;
            return 1;
        }
        WM_ERASEBKGND => return 1,
        WM_GETMINMAXINFO => {
            let info = &mut *(lp as *mut MINMAXINFO);
            if !state.mode.is_fullscreen() {
                if let Ok(display) = window::monitor(hwnd) {
                    let work = display.rcWork;
                    info.ptMinTrackSize = POINT {
                        x: (work.right - work.left).min(800),
                        y: (work.bottom - work.top).min(565),
                    };
                }
            }
            return 0;
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            // The suggested rectangle is valid for the duration of WM_DPICHANGED.
            let suggested = (msg == WM_DPICHANGED && lp != 0).then(|| *(lp as *const RECT));
            if let Err(error) = state.mode.reflow(hwnd, suggested) {
                eprintln!("{error}");
            }
            if let Err(error) =
                state
                    .theme
                    .rescale(hwnd, GetDpiForWindow(hwnd), &layout::Layout::read(hwnd))
            {
                eprintln!("{error}");
            }
            invalidate_scene(hwnd);
            return 0;
        }
        RENDER_FAILURE => {
            eprintln!("Falha ao apresentar a interface. Retorne ao console de recuperação.");
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
            return 0;
        }
        WM_DESTROY => {
            KillTimer(hwnd, 1);
            PostQuitMessage(0);
            return 0;
        }
        WM_SIZE | WM_COMMAND | WM_TIMER | WM_CLOSE => {}
        // DefWindowProc sends WM_SIZE synchronously from WM_WINDOWPOSCHANGED.
        // Forward default messages before taking the application borrow.
        _ => return DefWindowProcW(hwnd, msg, wp, lp),
    }

    if state.confirming.get() && matches!(msg, WM_COMMAND | WM_CLOSE) {
        return 0;
    }

    // RefCell prevents overlapping mutable access when native controls reenter.
    let Ok(mut app_borrow) = state.app.try_borrow_mut() else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    let app = &mut *app_borrow;

    match msg {
        WM_SIZE => {
            if wp != SIZE_MINIMIZED as usize && !control(hwnd, STATUS).is_null() {
                if let Err(error) =
                    state
                        .theme
                        .rescale(hwnd, GetDpiForWindow(hwnd), &layout::Layout::read(hwnd))
                {
                    eprintln!("{error}");
                }
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
            let confirmation = app.confirmation.take();
            let picker = app.picker.take();
            drop(app_borrow);
            if let Some(confirmation) = confirmation {
                confirmation.ask(hwnd, state);
            } else if let Some(purpose) = picker {
                session::run_picker(hwnd, state, purpose);
            }
            0
        }
        WM_TIMER => {
            events(hwnd, app);
            if app.refresh_pending && !app.busy && !state.confirming.get() {
                app.refresh_pending = false;
                refresh(hwnd, app);
            }
            state.selected_page.set(app.section);
            if state.render_failed.get() && !app.busy {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
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
        let _dpi_context = window::DpiContext::enter();
        let display = window::monitor(GetForegroundWindow())?.rcMonitor;
        let (tx, rx) = mpsc::channel();
        let theme = theme::Theme::new()?;
        let app = Box::new(WindowState {
            app: RefCell::new(App {
                deployment: deployment::Deployment::default(),
                session: session::Session::default(),
                picker: None,
                recommended: false,
                refresh_pending: false,
                console: console::Console::default(),
                section: DASHBOARD,
                technician: false,
                preview: None,
                current_repair: None,
                checkpoints: None,
                confirmation: None,
                report: None,
                entries: vec![],
                listing_loaded: false,
                directory: PathBuf::new(),
                busy: false,
                error_status: false,
                operation_page: DASHBOARD,
                progress: None,
                media_progress: None,
                report_export: None,
                handoff_export: None,
                scanning: false,
                roots: vec![],
                root_index: 0,
                row_index: 0,
                cancellable: false,
                cancel: Arc::new(AtomicBool::new(false)),
                tx,
                rx,
            }),
            theme,
            frame: RefCell::default(),
            control_frame: RefCell::default(),
            scene_dirty: Cell::new(true),
            render_failed: Cell::new(false),
            mode: window::Mode::fullscreen(),
            hovered: Cell::new(null_mut()),
            selected_page: Cell::new(DASHBOARD),
            confirming: Cell::new(false),
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

        select::register(instance)?;
        scroll::register(instance)?;
        if RegisterClassW(&window_class) == 0 {
            return Err(os_error("Register native window"));
        }

        let title = format!("ArchRescue {VERSION} — Recuperação e diagnóstico");
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            class.as_ptr(),
            wide(&title).as_ptr(),
            window::FULLSCREEN_STYLE,
            display.left,
            display.top,
            display.right - display.left,
            display.bottom - display.top,
            null_mut(),
            null_mut(),
            instance,
            (&*app as *const WindowState).cast(),
        );

        if hwnd.is_null() {
            return Err(os_error("Create ArchRescue window"));
        }

        let window = OwnedWindow(hwnd);
        app.theme
            .rescale(hwnd, GetDpiForWindow(hwnd), &layout::Layout::read(hwnd))?;
        let font = app.theme.body();
        let screen = GetDC(hwnd);
        let area = layout::Layout::read(hwnd);
        let reserved = !screen.is_null()
            && app
                .frame
                .borrow_mut()
                .reserve(screen, area.width, area.height);
        if !screen.is_null() {
            ReleaseDC(hwnd, screen);
        }
        if !reserved {
            return Err(Error::new(
                ErrorKind::Environment,
                "Create software framebuffer",
            ));
        }

        for page in navigation::ORDER {
            add(
                hwnd,
                100 + page,
                "BUTTON",
                navigation::label(page, false, false),
                WS_TABSTOP,
                font,
            )?;
        }

        for (id, label) in [
            (REFRESH, "Atualizar"),
            (DISK_DETAILS, "Detalhes dos discos"),
            (PREVIEW, "Plane&jar"),
            (COPY, "&Iniciar backup"),
            (CHECKPOINTS, "Checkpoints"),
            (RESUME, "&Retomar"),
            (PREVIOUS, "Voltar"),
            (NEXT, "Avançar"),
            (TECHNICIAN, "Modo &técnico"),
            (VERIFY, "Verificar novamente"),
            (REPAIR, "Gerar prévia do reparo"),
            (REPAIR_UNAVAILABLE, "Executar BCDBoot"),
            (CANCEL, "Cancelar"),
            (EXPORT, "Exportar relatório"),
        ] {
            add(hwnd, id, "BUTTON", label, WS_TABSTOP, font)?;
        }

        add(hwnd, SOURCE_LABEL, "STATIC", "Origem", 0, font)?;
        add(hwnd, DESTINATION_LABEL, "STATIC", "Destino", 0, font)?;
        add(
            hwnd,
            ESP_LABEL,
            "STATIC",
            "Partição de inicialização",
            0,
            font,
        )?;
        for (id, label) in [
            (ADDRESS, "Escolher pasta"),
            (DEST, "Escolher destino"),
            (ESP, "Escolher partição EFI"),
            (CHOOSE_WINDOWS, "Escolher Windows"),
            (CHOOSE_BACKUP, "Escolher backup"),
            (SELECT_RECOMMENDED, "Selecionar recomendadas"),
        ] {
            add(hwnd, id, "BUTTON", label, WS_TABSTOP, font)?;
        }

        add(
            hwnd,
            FILES,
            "LISTBOX",
            "",
            WS_TABSTOP | WS_VSCROLL | LBS_EXTENDEDSEL as u32 | LBS_NOTIFY as u32,
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
        add(
            hwnd,
            STATUS,
            "EDIT",
            i18n::LOADING,
            WS_TABSTOP | ES_MULTILINE as u32 | ES_READONLY as u32,
            font,
        )?;

        add(
            hwnd,
            BACKUP_SELECTION,
            "EDIT",
            "",
            WS_TABSTOP | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
            font,
        )?;
        pages::controls(hwnd, font)?;
        console::controls(hwnd, font)?;
        deployment::controls(hwnd, font)?;
        if SetTimer(hwnd, 1, 100, None) == 0 {
            return Err(os_error("Start recovery event timer"));
        }
        refresh(hwnd, &mut app.app.borrow_mut());
        ShowWindow(hwnd, SW_SHOW);
        SetFocus(control(hwnd, 100 + DASHBOARD));
        UpdateWindow(hwnd);

        let mut msg = MSG::default();
        loop {
            let result = GetMessageW(&mut msg, null_mut(), 0, 0);
            if result == 0 {
                break;
            }
            if result == -1 {
                return Err(os_error("Native message loop"));
            }
            track_hover(hwnd, &app, &msg);
            if msg.message == WM_KEYDOWN && msg.wParam == VK_F11 as usize {
                if msg.lParam & (1 << 30) == 0 {
                    if let Err(error) = app.mode.toggle(hwnd) {
                        set(
                            hwnd,
                            STATUS,
                            &format!("Falha ao alternar tela cheia: {error}"),
                        );
                    }
                }
                continue;
            }
            if msg.message == WM_KEYDOWN && msg.wParam == VK_RETURN as usize {
                let focused = GetFocus();
                let id = GetDlgCtrlID(focused);
                if is_button(focused) {
                    SendMessageW(focused, BM_CLICK, 0, 0);
                    continue;
                }
                if id == FILES as i32 {
                    SendMessageW(
                        hwnd,
                        WM_COMMAND,
                        FILES | ((LBN_DBLCLK as usize) << 16),
                        focused as isize,
                    );
                    continue;
                }
            }
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if matches!(
                msg.message,
                WM_KEYDOWN
                    | WM_MOUSEWHEEL
                    | WM_LBUTTONDOWN
                    | WM_LBUTTONUP
                    | WM_MOUSEMOVE
                    | WM_TIMER
            ) {
                scroll::invalidate(hwnd);
            }
        }

        drop(window);

        if app.render_failed.get() {
            return Err(Error::new(
                ErrorKind::Environment,
                "Present software framebuffer",
            ));
        }

        Ok(())
    }
}
