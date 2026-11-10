//! Native Win32 shell. Long-running filesystem work is performed outside the UI thread.

use std::{
    cell::RefCell,
    path::PathBuf,
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};

use archrescue_backup::{BackupResult, Entry};
use archrescue_core::{Error, ErrorKind, Result, VERSION};
use archrescue_evidence::Report;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
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

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn os_error(context: &str) -> Error {
    Error::io(context, std::io::Error::last_os_error())
}

enum Event {
    Snapshot(Box<Report>),
    Listing(PathBuf, Vec<Entry>),
    Progress(u64, u64),
    Backup(BackupResult),
    Exported(PathBuf),
    Failure(Error),
}

struct App {
    section: usize,
    report: Option<Report>,
    entries: Vec<Entry>,
    directory: PathBuf,
    busy: bool,
    cancellable: bool,
    cancel: Arc<AtomicBool>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

struct WindowState {
    app: RefCell<App>,
    brush: HBRUSH,
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

    Ok(())
}

unsafe fn place(hwnd: HWND, id: usize, x: i32, y: i32, width: i32, height: i32) {
    MoveWindow(control(hwnd, id), x, y, width.max(1), height.max(1), 1);
}

unsafe fn layout(hwnd: HWND) {
    let mut client_rect = RECT::default();
    GetClientRect(hwnd, &mut client_rect);
    let width = client_rect.right;
    let height = client_rect.bottom;

    for index in 0..7 {
        place(hwnd, 100 + index, 12, 58 + index as i32 * 43, 192, 36);
    }

    place(hwnd, REFRESH, 12, 370, 192, 34);
    place(hwnd, CONSOLE, 12, 413, 192, 34);

    place(hwnd, ADDRESS, 224, 58, width - 430, 30);
    place(hwnd, OPEN, width - 194, 58, 90, 30);
    place(hwnd, UP, width - 98, 58, 86, 30);

    place(hwnd, FILES, 224, 100, width - 236, height - 290);
    place(hwnd, CONTENT, 224, 100, width - 236, height - 290);

    place(hwnd, DEST, 224, height - 176, width - 236, 30);
    place(hwnd, COPY, 224, height - 132, 195, 34);
    place(hwnd, CANCEL, 429, height - 132, 110, 34);
    place(hwnd, EXPORT, 549, height - 132, 190, 34);
    place(hwnd, STATUS, 224, height - 85, width - 236, 65);
}

unsafe fn render(hwnd: HWND, app: &App) {
    ShowWindow(
        control(hwnd, FILES),
        if app.section == 4 { SW_SHOW } else { SW_HIDE },
    );
    ShowWindow(
        control(hwnd, CONTENT),
        if app.section == 4 { SW_HIDE } else { SW_SHOW },
    );

    for id in [OPEN, UP, ADDRESS] {
        ShowWindow(
            control(hwnd, id),
            if app.section == 4 { SW_SHOW } else { SW_HIDE },
        );
    }

    EnableWindow(control(hwnd, COPY), (!app.busy && app.section == 4) as i32);
    EnableWindow(control(hwnd, CANCEL), (app.busy && app.cancellable) as i32);
    EnableWindow(control(hwnd, REFRESH), (!app.busy) as i32);
    EnableWindow(
        control(hwnd, EXPORT),
        (!app.busy && app.report.is_some()) as i32,
    );

    let Some(report) = &app.report else {
        set(
            hwnd,
            CONTENT,
            if app.busy { i18n::LOADING } else { i18n::EMPTY },
        );
        return;
    };

    let text = crate::views::section(app.section, report);
    set(hwnd, CONTENT, &text);
}

fn work(app: &mut App, job: impl FnOnce() -> Event + Send + 'static) {
    app.busy = true;
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

        let mut report = Report::collect(&roots);
        report.inventory = inventory;

        Event::Snapshot(Box::new(report))
    });

    render(hwnd, app);
}

unsafe fn browse(hwnd: HWND, app: &mut App, path: PathBuf) {
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

unsafe fn selected(hwnd: HWND, app: &App) -> Vec<PathBuf> {
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
        .filter_map(|index| app.entries.get(*index as usize))
        .map(|entry| entry.name.clone())
        .collect()
}

unsafe fn action(hwnd: HWND, app: &mut App, id: usize, notification: u32) {
    if (100..107).contains(&id) {
        app.section = id - 100;
        render(hwnd, app);
        return;
    }

    if id == CANCEL {
        app.cancel.store(true, Ordering::Relaxed);
        set(
            hwnd,
            STATUS,
            "Cancelamento solicitado; aguardando confirmação da operação.",
        );
        return;
    }

    if app.busy {
        return;
    }

    match id {
        REFRESH => refresh(hwnd, app),
        OPEN => {
            let path = PathBuf::from(get(hwnd, ADDRESS));
            browse(hwnd, app, path);
        }
        UP => {
            if let Some(parent) = app.directory.parent() {
                browse(hwnd, app, parent.to_owned());
            }
        }
        FILES if notification == LBN_DBLCLK => {
            let selection = selected(hwnd, app);
            if selection.len() == 1 {
                let path = app.directory.join(&selection[0]);
                if path.is_dir() {
                    browse(hwnd, app, path);
                }
            }
        }
        COPY => start_backup(hwnd, app),
        EXPORT => {
            let Some(report) = app.report.clone() else {
                return;
            };

            let destination = PathBuf::from(get(hwnd, DEST));
            work(app, move || {
                match archrescue_evidence::export(&report, &destination) {
                    Ok(path) => Event::Exported(path),
                    Err(error) => Event::Failure(error),
                }
            });

            set(hwnd, STATUS, "Exportando e verificando relatório…");
            render(hwnd, app);
        }
        CONSOLE => {
            if let Some(system) = std::env::var_os("SystemRoot") {
                match std::process::Command::new(PathBuf::from(system).join("System32/cmd.exe"))
                    .spawn()
                {
                    Ok(_) => set(
                        hwnd,
                        STATUS,
                        "Console aberto. Comandos manuais não são limitados pela aplicação.",
                    ),
                    Err(error) => set(hwnd, STATUS, &format!("Falha ao abrir console: {error}")),
                }
            }
        }
        _ => {}
    }
}

unsafe fn start_backup(hwnd: HWND, app: &mut App) {
    let selections = selected(hwnd, app);
    if selections.is_empty() {
        set(
            hwnd,
            STATUS,
            "Selecione pelo menos um arquivo ou diretório acessível.",
        );
        return;
    }

    let source_root = app.directory.clone();
    let destination = PathBuf::from(get(hwnd, DEST));
    let message = format!(
        "Copiar {} seleções de {} para {}?\n\n\
         Apenas conteúdo; sem ACLs ou fluxos alternativos. Conflitos serão rejeitados.",
        selections.len(),
        source_root.display(),
        destination.display(),
    );

    if MessageBoxW(
        hwnd,
        wide(&message).as_ptr(),
        wide("Confirmar destino do backup").as_ptr(),
        MB_YESNO | MB_ICONQUESTION,
    ) != IDYES
    {
        return;
    }

    app.cancel = Arc::new(AtomicBool::new(false));
    let cancel = app.cancel.clone();
    let sender = app.tx.clone();
    work(app, move || {
        let mut last_update = std::time::Instant::now();
        let report_progress = |copied_bytes, planned_bytes| {
            if last_update.elapsed().as_millis() >= 100 || copied_bytes == planned_bytes {
                let _ = sender.send(Event::Progress(copied_bytes, planned_bytes));
                last_update = std::time::Instant::now();
            }
        };

        let result = archrescue_backup::recover(
            &source_root,
            &selections,
            &destination,
            &cancel,
            report_progress,
        );

        Event::Backup(result)
    });

    app.cancellable = true;
    app.section = 5;
    set(hwnd, STATUS, "Validando origem, destino, espaço e seleção…");
    render(hwnd, app);
}

unsafe fn events(hwnd: HWND, app: &mut App) {
    while let Ok(event) = app.rx.try_recv() {
        match event {
            Event::Progress(copied_bytes, planned_bytes) => {
                let message = format!(
                    "Copiados: {copied_bytes} / {planned_bytes} bytes. \
                     A conclusão depende da verificação SHA-256."
                );

                set(hwnd, STATUS, &message);
                continue;
            }
            Event::Snapshot(mut report) => {
                if let Some(old) = &app.report {
                    report.operations = old.operations.clone();
                }

                if app.directory.as_os_str().is_empty() {
                    if let Some(root) = report
                        .inventory
                        .volumes
                        .iter()
                        .flat_map(|volume| &volume.paths)
                        .next()
                    {
                        app.directory = root.clone();
                        set(hwnd, ADDRESS, &root.to_string_lossy());
                    }
                }

                app.report = Some(*report);
                set(
                    hwnd,
                    STATUS,
                    "Consulta concluída. Confira erros e limitações nas seções.",
                );
            }
            Event::Listing(path, entries) => show_listing(hwnd, app, path, entries),
            Event::Backup(result) => {
                let message = format!(
                    "{} — {} / {} bytes",
                    state_label(result.operation.state),
                    result.copied_bytes,
                    result.planned_bytes,
                );

                set(hwnd, STATUS, &message);

                if let Some(report) = &mut app.report {
                    report.operations.push(result);
                }
            }
            Event::Exported(path) => set(
                hwnd,
                STATUS,
                &format!("Relatório gravado e verificado: {}", path.display()),
            ),
            Event::Failure(error) => set(hwnd, STATUS, &format!("Falha: {error}")),
        }

        app.busy = false;
        render(hwnd, app);
    }
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
        SetTextColor(wp as HDC, 0x00eeeae5);
        SetBkColor(wp as HDC, 0x0025221d);
        return state.brush as isize;
    }

    // Native window operations can send these messages synchronously while
    // application state is borrowed. They only need the window and must still run.
    match msg {
        WM_SIZE => {
            layout(hwnd);
            return 0;
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
        WM_COMMAND => {
            action(hwnd, app, wp & 0xffff, ((wp >> 16) & 0xffff) as u32);
            0
        }
        WM_TIMER => {
            events(hwnd, app);
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

pub fn run() -> Result<()> {
    unsafe {
        let (tx, rx) = mpsc::channel();
        let brush = CreateSolidBrush(0x0025221d);
        let font = CreateFontW(
            -17,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            DEFAULT_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide("Segoe UI").as_ptr(),
        );

        let app = Box::new(WindowState {
            app: RefCell::new(App {
                section: 0,
                report: None,
                entries: vec![],
                directory: PathBuf::new(),
                busy: false,
                cancellable: false,
                cancel: Arc::new(AtomicBool::new(false)),
                tx,
                rx,
            }),
            brush,
        });

        let class = wide("ArchRescueP0");
        let instance = GetModuleHandleW(null());
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: brush,
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
            1100,
            740,
            null_mut(),
            null_mut(),
            instance,
            (&*app as *const WindowState).cast(),
        );

        if hwnd.is_null() {
            return Err(os_error("Create ArchRescue window"));
        }

        for (index, label) in i18n::SECTIONS.iter().enumerate() {
            add(hwnd, 100 + index, "BUTTON", label, WS_TABSTOP, font)?;
        }

        for (id, label) in [
            (REFRESH, "Atualizar diagnóstico"),
            (OPEN, "Abrir"),
            (UP, "Subir"),
            (COPY, "Copiar seleção"),
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
        SendMessageW(
            control(hwnd, DEST),
            0x1501,
            1,
            wide("Destino: diretório absoluto em volume persistente autorizado").as_ptr() as isize,
        );

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

        layout(hwnd);
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

        DeleteObject(font);
        DeleteObject(brush);

        Ok(())
    }
}
