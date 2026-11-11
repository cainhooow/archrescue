//! Native owner-drawn buttons and page summaries. Input and focus stay with Win32.

use windows_sys::Win32::UI::Controls::{ODS_DISABLED, ODS_FOCUS, ODS_SELECTED};

use super::{layout::Layout, theme::*, *};

pub unsafe fn fill(dc: HDC, rect: RECT, color: COLORREF) {
    SetDCBrushColor(dc, color);
    FillRect(dc, &rect, GetStockObject(DC_BRUSH));
}

unsafe fn outline(dc: HDC, rect: RECT, color: COLORREF) {
    SetDCBrushColor(dc, color);
    FrameRect(dc, &rect, GetStockObject(DC_BRUSH));
}

pub fn rect(x: i32, y: i32, width: i32, height: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    }
}

pub unsafe fn text(
    dc: HDC,
    font: HFONT,
    mut bounds: RECT,
    label: &str,
    color: COLORREF,
    flags: u32,
) {
    let label = wide(label);
    let previous = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, color);
    DrawTextW(dc, label.as_ptr(), -1, &mut bounds, flags);
    SelectObject(dc, previous);
}

const LINE: u32 = DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX;

pub unsafe fn button(item: &DRAWITEMSTRUCT, theme: &Theme, selected_page: usize) {
    let id = item.CtlID as usize;
    let bounds = item.rcItem;
    let nav = (100..100 + i18n::SECTIONS.len()).contains(&id);
    let selected = nav && id == 100 + selected_page;
    let primary = matches!(id, NEXT | PREVIEW | EXPORT) || deployment::primary(id);
    let write = id == COPY || id == RESUME;
    let disabled = item.itemState & ODS_DISABLED != 0;
    let pressed = item.itemState & ODS_SELECTED != 0;
    let color = if disabled {
        PANEL
    } else if pressed || selected {
        BORDER
    } else if primary {
        PRIMARY
    } else if nav {
        SIDEBAR
    } else {
        ELEVATED
    };

    fill(item.hDC, bounds, color);
    if !nav {
        outline(
            item.hDC,
            bounds,
            if write && !disabled { WARNING } else { BORDER },
        );
    }
    if selected {
        fill(
            item.hDC,
            rect(
                bounds.left,
                bounds.top + 3,
                3,
                bounds.bottom - bounds.top - 6,
            ),
            ACCENT,
        );
    }

    let mut label_area = bounds;
    label_area.left += if nav { 34 } else { 8 };
    label_area.right -= 8;
    if nav {
        icon(
            item.hDC,
            id - 100,
            bounds.left + 11,
            bounds.top + 10,
            if selected { ACCENT } else { MUTED },
        );
    }
    text(
        item.hDC,
        theme.body,
        label_area,
        &get(GetParent(item.hwndItem), id).replace('&', ""),
        if disabled {
            MUTED
        } else if write {
            WARNING
        } else {
            TEXT
        },
        LINE | if nav { DT_LEFT } else { DT_CENTER },
    );

    if item.itemState & ODS_FOCUS != 0 {
        outline(
            item.hDC,
            rect(
                bounds.left + 2,
                bounds.top + 2,
                bounds.right - bounds.left - 4,
                bounds.bottom - bounds.top - 4,
            ),
            ACCENT,
        );
    }
}

unsafe fn icon(dc: HDC, section: usize, x: i32, y: i32, color: COLORREF) {
    // Stroke coordinates use a 14px grid. The stock pen remains owned by GDI.
    let paths: &[&[(i32, i32)]] = match section {
        DASHBOARD => &[
            &[(0, 0), (6, 0), (6, 6), (0, 6), (0, 0)],
            &[(9, 0), (14, 0), (14, 14), (9, 14), (9, 0)],
            &[(0, 9), (6, 9), (6, 14), (0, 14), (0, 9)],
        ],
        DIAGNOSIS => &[&[(0, 7), (4, 7), (6, 2), (8, 12), (10, 7), (14, 7)]],
        INTEGRITY_PAGE => &[
            &[(7, 0), (13, 3), (13, 9), (7, 14), (1, 9), (1, 3), (7, 0)],
            &[(4, 7), (6, 9), (10, 5)],
        ],
        DISKS => &[
            &[(0, 2), (14, 2), (14, 12), (0, 12), (0, 2)],
            &[(0, 7), (14, 7)],
            &[(10, 9), (12, 9)],
        ],
        BACKUP => &[
            &[(1, 8), (1, 14), (13, 14), (13, 8)],
            &[(7, 0), (7, 10)],
            &[(3, 6), (7, 10), (11, 6)],
        ],
        REPAIR_PAGE => &[
            &[(7, 1), (7, 7), (13, 7)],
            &[(4, 2), (1, 5), (1, 10), (4, 13), (10, 13), (13, 10)],
        ],
        deployment::DEPLOY => &[
            &[(1, 1), (13, 1), (13, 10), (1, 10), (1, 1)],
            &[(7, 10), (7, 14)],
            &[(4, 14), (10, 14)],
        ],
        deployment::RESTORE => &[&[(0, 4), (5, 4), (7, 6), (14, 6), (14, 13), (0, 13), (0, 4)]],
        _ => &[
            &[(2, 0), (10, 0), (13, 3), (13, 14), (2, 14), (2, 0)],
            &[(5, 6), (10, 6)],
            &[(5, 10), (10, 10)],
        ],
    };

    let previous = SelectObject(dc, GetStockObject(DC_PEN));
    SetDCPenColor(dc, color);
    for path in paths {
        let points = path
            .iter()
            .map(|&(px, py)| POINT {
                x: x + px,
                y: y + py,
            })
            .collect::<Vec<_>>();
        Polyline(dc, points.as_ptr(), points.len() as i32);
    }
    SelectObject(dc, previous);
}

pub unsafe fn paint(hwnd: HWND, theme: &Theme, app: Option<&App>) {
    let mut paint = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    if !dc.is_null() {
        let area = Layout::read(hwnd);
        fill(dc, rect(0, 0, area.width, area.height), BACKGROUND);
        fill(dc, rect(0, 0, area.rail, area.height), SIDEBAR);
        fill(dc, rect(area.rail - 1, 0, 1, area.height), BORDER);

        fill(dc, rect(16, 23, 5, 25), ACCENT);
        text(
            dc,
            theme.emphasis,
            rect(29, 22, area.rail - 35, 26),
            "ArchRescue",
            TEXT,
            LINE,
        );
        text(
            dc,
            theme.body,
            rect(29, 47, area.rail - 35, 19),
            "Recuperação",
            MUTED,
            LINE,
        );

        if let Some(app) = app {
            page(dc, theme, &area, app);
        }
    }
    EndPaint(hwnd, &paint);
}

unsafe fn page(dc: HDC, theme: &Theme, area: &Layout, app: &App) {
    let x = area.left();
    let width = area.body_width();
    let title = if app.section == VERIFY_PAGE {
        "Verificação do backup"
    } else {
        i18n::SECTIONS[app.section]
    };
    text(
        dc,
        theme.heading,
        rect(x, 18, width - 168, 38),
        title,
        TEXT,
        LINE,
    );

    let source = app
        .report
        .as_ref()
        .and_then(|report| report.installations.installations.first())
        .map_or_else(
            || "Nenhuma instalação selecionada".into(),
            |installation| installation.path.display().to_string(),
        );
    let subtitle = if app.busy {
        format!("Operação em andamento   ·   {source}")
    } else {
        source
    };
    text(
        dc,
        theme.body,
        rect(x, 60, width, 24),
        &subtitle,
        MUTED,
        LINE,
    );
    fill(dc, rect(x, 94, width, 1), BORDER);

    if app.section == DASHBOARD {
        dashboard(dc, theme, area, app);
    } else {
        let top = Layout::body_top(app.section);
        let body = rect(x, top, width, area.body_bottom(app.section) - top);
        fill(dc, body, PANEL);
        outline(dc, body, BORDER);
    }

    let banner = rect(x, area.height - 58, width, 50);
    fill(dc, banner, PANEL);
    fill(
        dc,
        rect(x, area.height - 58, 3, 50),
        if app.error_status { CRITICAL } else { ACCENT },
    );
    if let Some(progress) = &app.progress {
        let percent = |bytes: u64| {
            ((bytes as u128 * width as u128) / progress.planned_bytes.max(1) as u128)
                .min(width as u128) as i32
        };
        fill(dc, rect(x, area.height - 60, width, 2), BORDER);
        fill(
            dc,
            rect(x, area.height - 60, percent(progress.acknowledged_bytes), 2),
            ACCENT,
        );
        fill(
            dc,
            rect(x, area.height - 60, percent(progress.verified_bytes), 2),
            theme::SUCCESS,
        );
    }
}

unsafe fn dashboard(dc: HDC, theme: &Theme, area: &Layout, app: &App) {
    let cards = summary(app);
    let x = area.left();
    let width = (area.body_width() - 12) / 2;
    for (index, (label, value, detail, color)) in cards.into_iter().enumerate() {
        let left = x + index as i32 % 2 * (width + 12);
        let top = 108 + index as i32 / 2 * 102;
        let bounds = rect(left, top, width, 90);
        fill(dc, bounds, PANEL);
        outline(dc, bounds, BORDER);
        text(
            dc,
            theme.body,
            rect(left + 14, top + 9, width - 28, 19),
            label,
            MUTED,
            LINE,
        );
        text(
            dc,
            theme.emphasis,
            rect(left + 14, top + 31, width - 28, 25),
            &value,
            color,
            LINE,
        );
        text(
            dc,
            theme.body,
            rect(left + 14, top + 59, width - 28, 20),
            &detail,
            MUTED,
            LINE,
        );
    }

    let bounds = rect(x, 316, area.body_width(), 87);
    fill(dc, bounds, ELEVATED);
    fill(dc, rect(x, 316, 3, 87), ACCENT);
    text(
        dc,
        theme.emphasis,
        rect(x + 14, 326, area.body_width() - 28, 25),
        "Próxima ação",
        TEXT,
        LINE,
    );
    text(
        dc,
        theme.body,
        rect(x + 14, 356, area.body_width() - 28, 39),
        recommendation(app),
        MUTED,
        DT_WORDBREAK | DT_NOPREFIX,
    );
}

type Card = (&'static str, String, String, COLORREF);

fn summary(app: &App) -> [Card; 4] {
    let report = app.report.as_ref();
    let installations = report.map(|report| &report.installations.installations);
    let disks = report.map(|report| &report.inventory.disks);
    let count = |count: Option<usize>| {
        count.map_or_else(|| "Consultando…".into(), |count| count.to_string())
    };
    let backup = report.and_then(|report| {
        report
            .transactions
            .iter()
            .rev()
            .find(|item| item.status.state == archrescue_core::State::Completed)
    });

    [
        (
            "Windows detectado",
            count(installations.map(Vec::len)),
            installations.and_then(|items| items.first()).map_or_else(
                || "Instalações offline corroboradas".into(),
                |item| item.path.display().to_string(),
            ),
            TEXT,
        ),
        (
            "Discos disponíveis",
            count(disks.map(Vec::len)),
            disks.and_then(|items| items.first()).map_or_else(
                || "Inventário de dispositivos".into(),
                |disk| disk.identity.clone(),
            ),
            TEXT,
        ),
        (
            "Integridade do Windows",
            "Não analisada".into(),
            "A análise verifica componentes do Windows offline".into(),
            MUTED,
        ),
        (
            "Backup",
            if app.progress.is_some() {
                "Em andamento"
            } else if backup.is_some() {
                "Verificado nesta sessão"
            } else {
                "Sem backup conhecido"
            }
            .into(),
            backup.map_or_else(
                || "Selecione os dados e um disco independente".into(),
                |backup| {
                    format!(
                        "{} arquivos · {} bytes verificados",
                        backup.progress.verified_files, backup.progress.verified_bytes
                    )
                },
            ),
            if backup.is_some() {
                theme::SUCCESS
            } else {
                TEXT
            },
        ),
    ]
}

fn recommendation(app: &App) -> &'static str {
    match &app.report {
        None => "Aguarde a identificação dos discos e instalações acessíveis.",
        Some(report) if report.installations.installations.is_empty() => "Confira os volumes acessíveis e atualize o inventário antes de selecionar uma instalação.",
        Some(_) => "Revise a instalação detectada e preserve os arquivos importantes em um disco independente.",
    }
}
