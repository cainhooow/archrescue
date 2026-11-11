//! Structured recovery pages with native list selection and readable details.

use std::fmt::Write;

use archrescue_integrity::{IntegrityReport, Observation, Severity};
use archrescue_windows_detection::ProbeState;

use super::*;

pub const ROWS: usize = 240;
pub const ROOT: usize = 241;
pub const SCAN: usize = 242;
pub const PROTECT: usize = 243;
pub const REPORT_ACTION: usize = 244;
pub const PAGE_HINT: usize = 245;

pub fn structured(section: usize) -> bool {
    matches!(section, DIAGNOSIS | DISKS | INTEGRITY_PAGE)
}

pub struct Row {
    label: String,
    detail: String,
}

impl Row {
    fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
        }
    }
}

pub unsafe fn controls(hwnd: HWND, font: HFONT) -> Result<()> {
    add(
        hwnd,
        ROOT,
        "COMBOBOX",
        "",
        WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
        font,
    )?;
    add(
        hwnd,
        ROWS,
        "LISTBOX",
        "",
        WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY as u32,
        font,
    )?;
    add(hwnd, PAGE_HINT, "STATIC", "", 0, font)?;

    for (id, label) in [
        (SCAN, "Analisar"),
        (PROTECT, "Proteger dados"),
        (REPORT_ACTION, "Relatórios"),
    ] {
        add(hwnd, id, "BUTTON", label, WS_TABSTOP, font)?;
    }

    Ok(())
}

pub unsafe fn layout(hwnd: HWND, area: &layout::Layout, section: usize) {
    let x = area.left();
    let width = area.body_width();
    let top = layout::Layout::body_top(section);
    let bottom = area.body_bottom(section);

    place(hwnd, ROOT, x, 128, width - 140, 252);
    place(hwnd, SCAN, x + width - 130, 128, 130, 30);
    place(hwnd, PROTECT, x, area.actions(), 142, 32);
    place(hwnd, REPORT_ACTION, x + 152, area.actions(), 122, 32);

    if width >= 880 {
        let list_width = width * 43 / 100;
        place(hwnd, PAGE_HINT, x + 12, top + 8, list_width - 24, 23);
        place(
            hwnd,
            ROWS,
            x + 9,
            top + 38,
            list_width - 18,
            bottom - top - 46,
        );
        place(
            hwnd,
            CONTENT,
            x + list_width + 12,
            top + 38,
            width - list_width - 22,
            bottom - top - 46,
        );
    } else {
        place(hwnd, PAGE_HINT, x + 12, top + 8, width - 24, 23);
        let rows_height = ((bottom - top - 48) * 43 / 100).clamp(50, 190);
        place(hwnd, ROWS, x + 9, top + 35, width - 18, rows_height);
        let details_top = top + 44 + rows_height;
        place(
            hwnd,
            CONTENT,
            x + 9,
            details_top,
            width - 18,
            bottom - details_top - 8,
        );
    }
}

pub unsafe fn refresh_sources(hwnd: HWND, app: &mut App, report: &Report) {
    let previous = app.roots.get(app.root_index).cloned();
    app.roots = report
        .installations
        .probes
        .iter()
        .filter(|probe| {
            matches!(
                probe.state,
                ProbeState::Candidate | ProbeState::Incomplete | ProbeState::Inaccessible
            )
        })
        .map(|probe| probe.root.clone())
        .collect();
    let combo = control(hwnd, ROOT);
    SendMessageW(combo, CB_RESETCONTENT, 0, 0);
    for root in &app.roots {
        let label = wide(&format!("Origem  ·  {}", root.display()));
        SendMessageW(combo, CB_ADDSTRING, 0, label.as_ptr() as isize);
    }
    app.root_index = previous
        .and_then(|root| app.roots.iter().position(|item| item == &root))
        .unwrap_or(0);
    SendMessageW(combo, CB_SETCURSEL, app.root_index, 0);
}

pub fn selected_scan(app: &App) -> Option<&IntegrityReport> {
    let root = app.roots.get(app.root_index)?;
    app.report
        .as_ref()?
        .integrity
        .iter()
        .rev()
        .find(|scan| &scan.root == root)
}

pub fn integrity_status(app: &App) -> &'static str {
    if app.scanning {
        "Analisando…"
    } else if let Some(scan) = selected_scan(app) {
        scan.status.label()
    } else {
        "Não analisada"
    }
}

pub unsafe fn action(hwnd: HWND, app: &mut App, id: usize, notification: u32) -> bool {
    match id {
        ROWS if notification == LBN_SELCHANGE => {
            app.row_index = SendMessageW(control(hwnd, ROWS), LB_GETCURSEL, 0, 0) as usize;
            show_detail(hwnd, app);
        }
        ROOT if notification == CBN_SELCHANGE && !app.busy => {
            app.root_index = SendMessageW(control(hwnd, ROOT), CB_GETCURSEL, 0, 0) as usize;
            app.row_index = 0;
            render(hwnd, app);
        }
        SCAN if !app.busy => start(hwnd, app),
        PROTECT if !app.busy => {
            stage(hwnd, app, BACKUP);
            if let Some(root) = app.roots.get(app.root_index).cloned() {
                browse(hwnd, app, root);
            }
        }
        REPORT_ACTION => stage(hwnd, app, REPORTS),
        _ => return false,
    }

    true
}

unsafe fn start(hwnd: HWND, app: &mut App) {
    let Some(root) = app.roots.get(app.root_index).cloned() else {
        return;
    };
    let reference = deployment::reference(hwnd, app);
    let cancel = Arc::new(AtomicBool::new(false));
    app.cancel = cancel.clone();
    let sender = app.tx.clone();
    work(app, move || {
        let mut scan = archrescue_integrity::scan(
            &root,
            archrescue_integrity::ScanMode::PassiveOffline,
            &cancel,
            |progress| {
                let _ = sender.send(Event::IntegrityProgress(progress));
            },
        );
        if let Some((media, image)) = reference {
            crate::integrity::add_reference(&mut scan, &media, &image);
        }
        Event::Integrity(Box::new(scan))
    });

    app.cancellable = true;
    app.scanning = true;
    set(hwnd, STATUS, "Preparando inspeção somente leitura…");
    render(hwnd, app);
}

pub unsafe fn render_page(hwnd: HWND, app: &App) -> bool {
    let integrity = app.section == INTEGRITY_PAGE;
    let structure = structured(app.section);
    show(hwnd, ROWS, structure);
    show(hwnd, PAGE_HINT, structure);
    for id in [ROOT, SCAN, PROTECT, REPORT_ACTION] {
        show(hwnd, id, integrity);
    }
    EnableWindow(
        control(hwnd, ROOT),
        (!app.busy && !app.roots.is_empty()) as i32,
    );
    EnableWindow(
        control(hwnd, SCAN),
        (!app.busy && !app.roots.is_empty()) as i32,
    );
    EnableWindow(control(hwnd, PROTECT), (!app.busy) as i32);

    if !structure {
        return false;
    }

    let hint = match app.section {
        DISKS => "Dispositivos físicos e volumes   ·   Selecione para ver os detalhes",
        DIAGNOSIS => "Evidências de inicialização   ·   Selecione para ver os detalhes",
        _ => integrity_status(app),
    };
    set(hwnd, PAGE_HINT, hint);

    let list = control(hwnd, ROWS);
    SendMessageW(list, WM_SETREDRAW, 0, 0);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    let rows = rows(app);
    for row in &rows {
        SendMessageW(list, LB_ADDSTRING, 0, wide(&row.label).as_ptr() as isize);
    }
    SendMessageW(
        list,
        LB_SETCURSEL,
        app.row_index.min(rows.len().saturating_sub(1)),
        0,
    );
    SendMessageW(list, WM_SETREDRAW, 1, 0);
    InvalidateRect(list, null(), 0);
    set(hwnd, CONTENT, &detail(app, &rows));

    true
}

unsafe fn show_detail(hwnd: HWND, app: &App) {
    set(hwnd, CONTENT, &detail(app, &rows(app)));
}

fn detail(app: &App, rows: &[Row]) -> String {
    rows.get(app.row_index).or_else(|| rows.first()).map_or_else(|| {
        if app.busy { i18n::LOADING.into() } else if app.section == INTEGRITY_PAGE {
            "Selecione a origem offline e inicie a análise. A inspeção lê executáveis, metadados de manutenção e o histórico CBS disponível.".into()
        } else { i18n::EMPTY.into() }
    }, |row| row.detail.clone())
}

fn rows(app: &App) -> Vec<Row> {
    let Some(report) = &app.report else {
        return vec![];
    };
    match app.section {
        DISKS => disk_rows(report, app.technician),
        DIAGNOSIS => diagnosis_rows(report, app.technician),
        INTEGRITY_PAGE => selected_scan(app)
            .map(|scan| integrity_rows(scan, app.technician))
            .unwrap_or_default(),
        _ => vec![],
    }
}

fn diagnosis_rows(report: &Report, technician: bool) -> Vec<Row> {
    let mut rows: Vec<_> = report
        .diagnosis
        .findings
        .iter()
        .map(|finding| {
            let state = match finding.confidence {
                archrescue_diagnostics::recovery::Confidence::Observed => "Observado",
                archrescue_diagnostics::recovery::Confidence::ConfirmedFailure => {
                    "Falha confirmada"
                }
                archrescue_diagnostics::recovery::Confidence::SuspectedFailure => "Indício",
                _ => "Evidência limitada",
            };
            let mut detail = format!(
                "{state}\n{}\n\n{}\n\nPróxima ação\n{}",
                finding.affected.display(),
                finding.evidence,
                finding.next_action
            );
            if technician {
                let _ = write!(
                    detail,
                    "\n\n{} · Bloqueia reparo: {}",
                    finding.id,
                    if finding.blocks_repair { "sim" } else { "não" }
                );
            }
            Row::new(
                format!("{state}  ·  {}", finding.affected.display()),
                detail,
            )
        })
        .collect();
    if technician {
        for association in &report.diagnosis.associations {
            rows.push(Row::new(
                format!("Associação GPT · {}", association.installation.display()),
                format!("{association:#?}"),
            ));
        }
        for error in report
            .inventory
            .errors
            .iter()
            .chain(&report.installations.errors)
            .chain(&report.boot.errors)
        {
            rows.push(Row::new(
                "Erro da coleta",
                format!(
                    "{error}
Código: {:?}",
                    error.os_code
                ),
            ));
        }
    }

    rows
}

fn integrity_rows(scan: &IntegrityReport, technician: bool) -> Vec<Row> {
    let version = scan.windows.as_ref().map_or_else(
        || "Versão indisponível".into(),
        |item| {
            format!(
                "{} · Versão de arquivo {}",
                item.architecture, item.file_version
            )
        },
    );
    let evaluated = scan
        .checks
        .iter()
        .filter(|check| {
            !matches!(
                check.observation,
                Observation::Unsupported | Observation::Inconclusive
            )
        })
        .count();
    let mut summary = format!(
        "{}\n{}\n{}\n\n{} verificações com evidência · {} registradas\n{}\n\n{}",
        scan.status.label(),
        scan.windows_directory.display(),
        version,
        evaluated,
        scan.checks.len(),
        scan.recommendation(),
        scan.limitations.join("\n\n"),
    );
    for error in &scan.errors {
        let _ = write!(summary, "\n\n{}", error.context);
    }
    if let Some(reference) = &scan.reference {
        let _ = write!(
            summary,
            "\n\nMídia de referência · índice {}\n{}\n{}",
            reference.index,
            reference.path.display(),
            reference.reason
        );
    }
    if technician {
        let _ = write!(
            summary,
            "\n\nVerificações não executadas\n{}",
            scan.not_executed.join("\n")
        );
    }
    let mut rows = vec![Row::new("Resumo da análise", summary)];
    let mut checks = scan
        .checks
        .iter()
        .filter(|check| {
            technician
                || !matches!(
                    check.observation,
                    Observation::VerifiedStructure | Observation::Unsupported
                )
        })
        .collect::<Vec<_>>();
    checks.sort_by_key(|check| match check.severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    });
    for check in checks {
        let state = match (check.severity, check.observation) {
            (Severity::Error, _) => "CRÍTICO",
            (Severity::Warning, _) => "ATENÇÃO",
            (_, Observation::HistoricalFailure) => "HISTÓRICO",
            _ => "EVIDÊNCIA",
        };
        let mut detail = format!(
            "{}\n{} · {}\n{}\n\nEvidência\n{}\n\nPróxima ação\n{}",
            check.expected_component,
            check.category.label(),
            check.observation.label(),
            check.path.display(),
            check.evidence,
            check.next_action,
        );
        if technician {
            let _ = write!(detail, "\n\n{}", check.identifier);
            if let Some(error) = &check.error {
                let _ = write!(detail, "\n{}", error);
            }
        }
        rows.push(Row::new(
            format!("{state}  ·  {}", check.expected_component),
            detail,
        ));
    }

    rows
}

fn disk_rows(report: &Report, technician: bool) -> Vec<Row> {
    let mut rows = vec![];
    for disk in &report.inventory.disks {
        let name = disk.number.map_or_else(
            || "Disco sem número".into(),
            |number| format!("Disco {number}"),
        );
        let capacity = disk
            .capacity
            .map_or_else(|| "Capacidade indisponível".into(), size);
        let style = disk.style.as_deref().unwrap_or("Estilo indisponível");
        let access = match disk.read_only {
            Some(true) => "Protegido contra gravação",
            Some(false) => "Dispositivo aceita gravação",
            None => "Proteção contra gravação indeterminada",
        };
        let mut detail = format!(
            "{name} · {capacity} · {style}\n{}\n{access}\n{} partições observadas\n",
            disk.identity,
            disk.partitions.len(),
        );
        for partition in &disk.partitions {
            let _ = writeln!(
                detail,
                "Partição {} · {} · {}",
                partition.number,
                size(partition.length),
                if partition.efi {
                    "EFI"
                } else {
                    &partition.kind
                }
            );
        }
        for volume in report.inventory.volumes.iter().filter(|volume| {
            disk.number
                .is_some_and(|number| volume.physical_disks.contains(&number))
        }) {
            let _ = write!(detail, "\n{}", volume_detail(volume, technician));
        }
        if !disk.errors.is_empty() {
            let _ = write!(
                detail,
                "\nConsulta parcial · {} erros de dispositivo",
                disk.errors.len()
            );
        }
        if technician {
            let _ = write!(detail, "\n{}\n{:#?}", disk.device_path, disk.errors);
        }
        rows.push(Row::new(
            format!("{name}  ·  {capacity}  ·  {style}"),
            detail,
        ));
    }
    for volume in &report.inventory.volumes {
        let name = volume.paths.first().map_or_else(
            || "Sem caminho de acesso".into(),
            |path| path.display().to_string(),
        );
        let filesystem = volume.filesystem.as_deref().unwrap_or("FS indisponível");
        rows.push(Row::new(
            format!("Volume  ·  {name}  ·  {filesystem}"),
            volume_detail(volume, technician),
        ));
    }

    rows
}

fn volume_detail(volume: &archrescue_storage::Volume, technician: bool) -> String {
    let paths = volume
        .paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let mut detail = format!(
        "{} · {}\n{}\n{}\n",
        if paths.is_empty() {
            "Sem caminho de acesso"
        } else {
            &paths
        },
        volume
            .filesystem
            .as_deref()
            .unwrap_or("Sistema de arquivos indisponível"),
        volume.space.as_ref().map_or_else(
            || "Espaço disponível indeterminado".into(),
            |space| format!(
                "{} disponíveis de {}",
                size(space.available),
                size(space.total)
            )
        ),
        volume.identity,
    );
    if !volume.errors.is_empty() {
        let _ = writeln!(
            detail,
            "{} restrições na consulta do volume",
            volume.errors.len()
        );
    }
    if technician {
        let _ = write!(
            detail,
            "\nDiscos associados: {:?}\n{:#?}",
            volume.physical_disks, volume.errors
        );
    }

    detail
}

fn size(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        format!("{:.1} GiB", bytes as f64 / (1_u64 << 30) as f64)
    } else {
        format!("{:.1} MiB", bytes as f64 / (1_u64 << 20) as f64)
    }
}
