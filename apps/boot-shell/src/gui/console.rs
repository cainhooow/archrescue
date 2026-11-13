//! Bounded diagnostic runner with an explicit set of read-only actions.

use std::{
    fs::{self, File},
    io::{Read, Write},
    os::windows::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use windows_sys::Win32::{
    Globalization::{GetOEMCP, MultiByteToWideChar},
    System::Threading::CREATE_NO_WINDOW,
};

use super::*;

pub const PAGE: usize = 10;
const COMMAND: usize = 251;
const RUN: usize = 252;
pub const HISTORY: usize = 253;
const CLEAR: usize = 254;
const LOG: usize = 255;
const FALLBACK: usize = 256;
const COPY_OUTPUT: usize = 257;
const CAPTURE_LIMIT: usize = 128 * 1024;

#[derive(Clone, Copy)]
struct Diagnostic {
    title: &'static str,
    executable: &'static str,
    arguments: &'static [&'static str],
}

const ACTIONS: [Diagnostic; 3] = [
    Diagnostic {
        title: "Configuração de rede",
        executable: "ipconfig.exe",
        arguments: &["/all"],
    },
    Diagnostic {
        title: "Volumes e pontos de montagem",
        executable: "mountvol.exe",
        arguments: &[],
    },
    Diagnostic {
        title: "Processos em execução",
        executable: "tasklist.exe",
        arguments: &[],
    },
];

#[derive(Default)]
pub struct Console {
    output: String,
    history: Vec<(usize, String)>,
}

#[derive(Serialize)]
pub struct Outcome {
    command: String,
    started_unix: u64,
    exit_code: Option<i32>,
    cancelled: bool,
    error: Option<String>,
    #[serde(skip)]
    index: usize,
    #[serde(skip)]
    stdout: String,
    #[serde(skip)]
    stderr: String,
}

pub unsafe fn controls(hwnd: HWND, font: HFONT) -> Result<()> {
    add(hwnd, COMMAND, select::class(), "", WS_TABSTOP, font)?;
    add(hwnd, HISTORY, select::class(), "", WS_TABSTOP, font)?;
    add_option(hwnd, HISTORY, "Histórico de diagnósticos");
    SendMessageW(control(hwnd, HISTORY), CB_SETCURSEL, 0, 0);
    for action in ACTIONS {
        add_option(hwnd, COMMAND, action.title);
    }
    SendMessageW(control(hwnd, COMMAND), CB_SETCURSEL, 0, 0);
    for (id, label) in [
        (RUN, "Executar"),
        (COPY_OUTPUT, "Copiar"),
        (CLEAR, "Limpar"),
        (LOG, "Exportar log"),
        (FALLBACK, "Console externo de emergência"),
    ] {
        add(hwnd, id, "BUTTON", label, WS_TABSTOP, font)?;
    }

    Ok(())
}

unsafe fn add_option(hwnd: HWND, id: usize, text: &str) {
    SendMessageW(
        control(hwnd, id),
        CB_ADDSTRING,
        0,
        wide(text).as_ptr() as LPARAM,
    );
}

pub unsafe fn layout(hwnd: HWND, area: &layout::Layout) {
    let x = area.left() + 12;
    let width = area.body_width() - 24;
    let top = area.body_top(PAGE) + 12;
    let bottom = area.body_bottom(PAGE);

    place(hwnd, COMMAND, x, top, width - 111, 30);
    place(hwnd, RUN, x + width - 103, top, 103, 30);
    place(hwnd, HISTORY, x, top + 38, width, 28);
    place(hwnd, COPY_OUTPUT, x, bottom - 36, 78, 31);
    place(hwnd, CLEAR, x + 86, bottom - 36, 78, 31);
    place(hwnd, LOG, x + 172, bottom - 36, 105, 31);
    place(hwnd, FALLBACK, x + 285, bottom - 36, width - 285, 31);
    place(hwnd, DEST, x, bottom - 72, width, 28);
    place(hwnd, CONTENT, x, top + 76, width, bottom - top - 155);
}

pub unsafe fn render(hwnd: HWND, app: &App) -> bool {
    let active = app.section == PAGE;
    for id in [COMMAND, HISTORY, RUN, COPY_OUTPUT, CLEAR, LOG, FALLBACK] {
        show(hwnd, id, active && (id != FALLBACK || app.technician));
    }
    if active {
        EnableWindow(control(hwnd, RUN), (!app.busy) as i32);
        EnableWindow(
            control(hwnd, LOG),
            (!app.busy && !app.console.output.is_empty()) as i32,
        );
        EnableWindow(
            control(hwnd, HISTORY),
            (!app.console.history.is_empty()) as i32,
        );
        EnableWindow(control(hwnd, FALLBACK), (!app.busy) as i32);
        set(hwnd, CONTENT, &app.console.output);
    }

    active
}

pub unsafe fn action(hwnd: HWND, app: &mut App, id: usize) -> bool {
    if app.section != PAGE {
        return false;
    }
    match id {
        RUN => {
            let index = SendMessageW(control(hwnd, COMMAND), CB_GETCURSEL, 0, 0) as usize;
            let Some(diagnostic) = ACTIONS.get(index) else {
                return true;
            };
            let cancel = Arc::new(AtomicBool::new(false));
            app.cancel = cancel.clone();
            work(app, move || Event::Console(run(index, diagnostic, &cancel)));
            app.cancellable = true;
            set(hwnd, STATUS, "Diagnóstico em execução…");
        }
        HISTORY => {
            let index = SendMessageW(control(hwnd, HISTORY), CB_GETCURSEL, 0, 0) as usize;
            if let Some((action, text)) = index
                .checked_sub(1)
                .and_then(|index| app.console.history.get(index))
            {
                SendMessageW(control(hwnd, COMMAND), CB_SETCURSEL, *action, 0);
                app.console.output.clone_from(text);
            }
        }
        COPY_OUTPUT => {
            let output = control(hwnd, CONTENT);
            SendMessageW(output, windows_sys::Win32::UI::Controls::EM_SETSEL, 0, -1);
            SendMessageW(output, WM_COPY, 0, 0);
            SendMessageW(output, windows_sys::Win32::UI::Controls::EM_SETSEL, 0, 0);
            set(hwnd, STATUS, "Saída selecionada para cópia.");
        }
        CLEAR => app.console.output.clear(),
        LOG => {
            let Ok(destination) = app.session.destination(PAGE) else {
                session::request(app, picker::Purpose::ConsoleLog);
                return true;
            };
            let sources = app.session.sources(PAGE);
            let output = app.console.output.clone();
            work(app, move || {
                let result = (|| {
                    let path = destination.revalidate()?;
                    crate::resources::writable(&path, &sources, output.len() as u64)?;
                    export(&path, output.as_bytes())
                })();
                result.map_or_else(Event::Failure, Event::PlanExported)
            });
            set(hwnd, STATUS, "Exportando log do diagnóstico…");
        }
        FALLBACK if app.technician => match window::launch_console() {
            Ok(()) => {
                ShowWindow(hwnd, SW_MINIMIZE);
            }
            Err(error) => set(hwnd, STATUS, &error_message(&error)),
        },
        _ => return false,
    }
    super::render(hwnd, app);

    true
}

pub unsafe fn update(hwnd: HWND, app: &mut App, outcome: Outcome) {
    let status = if outcome.cancelled {
        "Comando cancelado.".into()
    } else if let Some(error) = &outcome.error {
        error.clone()
    } else {
        format!("Código de saída: {}", outcome.exit_code.unwrap_or_default())
    };
    let text = format!(
        "> {}\r\n{}\r\n\r\n{}{}",
        outcome.command,
        status,
        outcome.stdout,
        if outcome.stderr.is_empty() {
            String::new()
        } else {
            format!("\r\n\r\nErros\r\n{}", outcome.stderr)
        },
    );
    app.error_status = outcome.cancelled || outcome.error.is_some() || outcome.exit_code != Some(0);
    app.console.output.clone_from(&text);
    if app.console.history.len() == 20 {
        app.console.history.remove(0);
        SendMessageW(control(hwnd, HISTORY), CB_DELETESTRING, 1, 0);
    }
    let index = app.console.history.len() + 1;
    app.console.history.push((outcome.index, text));
    add_option(hwnd, HISTORY, ACTIONS[outcome.index].title);
    SendMessageW(control(hwnd, HISTORY), CB_SETCURSEL, index, 0);
    if let Some(report) = &mut app.report {
        if let Ok(audit) = serde_json::to_value(&outcome) {
            report.console_audit.push(audit);
        }
    }

    set(hwnd, STATUS, &status);
}

fn run(index: usize, action: &Diagnostic, cancel: &AtomicBool) -> Outcome {
    let mut outcome = Outcome {
        index,
        command: format!("{} {}", action.executable, action.arguments.join(" ")),
        started_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        exit_code: None,
        cancelled: false,
        error: None,
        stdout: String::new(),
        stderr: String::new(),
    };
    let result = (|| -> std::io::Result<()> {
        let root = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| {
                std::io::Error::other("Diretório do Windows em execução indisponível.")
            })?;
        if cancel.load(Ordering::Relaxed) {
            outcome.cancelled = true;
            return Ok(());
        }

        let mut child = Command::new(root.join("System32").join(action.executable))
            .args(action.arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;

        // Separate readers drain both pipes even after reaching their display limits.
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let result = thread::scope(|scope| {
            let stdout = scope.spawn(move || capture(stdout));
            let stderr = scope.spawn(move || capture(stderr));
            let status = loop {
                if cancel.load(Ordering::Relaxed) {
                    outcome.cancelled = true;
                    let _ = child.kill();
                    break child.wait();
                }
                match child.try_wait() {
                    Ok(Some(status)) => break Ok(status),
                    Ok(None) => thread::sleep(Duration::from_millis(50)),
                    Err(error) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break Err(error);
                    }
                }
            };
            outcome.stdout = stdout
                .join()
                .map_err(|_| std::io::Error::other("Leitor de saída interrompido."))??;
            outcome.stderr = stderr
                .join()
                .map_err(|_| std::io::Error::other("Leitor de erros interrompido."))??;
            status
        })?;
        outcome.exit_code = result.code();

        Ok(())
    })();
    outcome.error = result
        .err()
        .map(|error| format!("Falha no diagnóstico: {error}"));

    outcome
}

fn capture(mut pipe: impl Read) -> std::io::Result<String> {
    let mut buffer = [0u8; 8192];
    let mut captured = Vec::new();
    let mut truncated = false;
    loop {
        let count = pipe.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let keep = count.min(CAPTURE_LIMIT - captured.len());
        captured.extend_from_slice(&buffer[..keep]);
        truncated |= keep != count;
    }

    let mut text = decode(&captured)
        .replace("\r\n", "\n")
        .replace('\n', "\r\n");
    if truncated {
        text.push_str("\r\n[Saída limitada a 128 KiB]");
    }

    Ok(text)
}

fn decode(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.trim_start_matches('\u{feff}').into();
    }

    if let Some(units) = bytes.strip_prefix(&[0xff, 0xfe]) {
        let utf16 = units
            .as_chunks::<2>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16_lossy(&utf16);
    }
    let mut wide = vec![0; bytes.len()];
    // SAFETY: both buffers are valid and the bounded capture length fits i32.
    let written = unsafe {
        MultiByteToWideChar(
            GetOEMCP(),
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            wide.len() as i32,
        )
    };
    if written <= 0 {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    String::from_utf16_lossy(&wide[..written as usize])
}

fn export(destination: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let path = destination.join(format!(
        "archrescue-diagnostic-{}.txt",
        archrescue_core::Operation::new().id
    ));
    let mut file = File::create_new(&path).map_err(|error| Error::io("Criar log", error))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| Error::io("Gravar log", error))?;
    drop(file);
    if fs::read(&path).map_err(|error| Error::io("Reler log", error))? != bytes {
        return Err(Error::new(
            ErrorKind::Integrity,
            "O log gravado diverge do original.",
        ));
    }

    Ok(path)
}
