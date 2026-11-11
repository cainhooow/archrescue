//! Native owner-drawn buttons and page summaries. Input and focus stay with Win32.

use windows_sys::Win32::UI::Controls::{ODS_DISABLED, ODS_FOCUS, ODS_SELECTED};

use super::{layout::Layout, theme::*, *};

mod cards;

unsafe fn fill(dc: HDC, rect: RECT, color: COLORREF) {
    SetDCBrushColor(dc, color);
    FillRect(dc, &rect, GetStockObject(DC_BRUSH));
}

unsafe fn outline(dc: HDC, rect: RECT, color: COLORREF) {
    SetDCBrushColor(dc, color);
    FrameRect(dc, &rect, GetStockObject(DC_BRUSH));
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    }
}

unsafe fn text(dc: HDC, font: HFONT, mut bounds: RECT, label: &str, color: COLORREF, flags: u32) {
    let label = wide(label);
    let previous = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, color);
    DrawTextW(dc, label.as_ptr(), -1, &mut bounds, flags);
    SelectObject(dc, previous);
}

const LINE: u32 = DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX;

unsafe fn surface(dc: HDC, bounds: RECT, background: COLORREF, border: COLORREF, radius: i32) {
    let previous_pen = SelectObject(dc, GetStockObject(DC_PEN));
    let previous_brush = SelectObject(dc, GetStockObject(DC_BRUSH));
    SetDCPenColor(dc, border);
    SetDCBrushColor(dc, background);
    RoundRect(
        dc,
        bounds.left,
        bounds.top,
        bounds.right,
        bounds.bottom,
        radius,
        radius,
    );
    SelectObject(dc, previous_brush);
    SelectObject(dc, previous_pen);
}

pub unsafe fn button(item: &DRAWITEMSTRUCT, theme: &Theme, selected_page: usize) {
    let id = item.CtlID as usize;
    let bounds = item.rcItem;
    let page = id
        .checked_sub(100)
        .filter(|page| navigation::ORDER.contains(page));
    let utility = matches!(id, TECHNICIAN | REFRESH | CONSOLE | CANCEL);
    let nav = page.is_some() || utility;
    let selected = page.is_some_and(|page| page == navigation::owner(selected_page));
    let primary = matches!(
        id,
        PREVIEW | COPY | RESUME | EXPORT | VERIFY | REPAIR | pages::SCAN
    ) || deployment::primary(id);
    let disabled = item.itemState & ODS_DISABLED != 0;
    let pressed = item.itemState & ODS_SELECTED != 0;
    let accent = primary && !disabled;
    let backdrop = if nav { SIDEBAR } else { PANEL };
    let color = if disabled {
        backdrop
    } else if selected || pressed && !accent {
        SELECTED
    } else if accent {
        ACCENT
    } else if nav {
        SIDEBAR
    } else {
        ELEVATED
    };

    fill(item.hDC, bounds, backdrop);
    if !nav || selected || pressed {
        let border = if accent { ACCENT } else { BORDER };
        surface(item.hDC, bounds, color, border, 8);
    }
    if selected {
        fill(
            item.hDC,
            rect(
                bounds.left,
                bounds.top + 7,
                3,
                bounds.bottom - bounds.top - 14,
            ),
            ACCENT,
        );
    }

    let mut label_area = bounds;
    label_area.left += if nav { 34 } else { 7 };
    label_area.right -= 7;
    if nav {
        let symbol = page.unwrap_or(match id {
            TECHNICIAN => INTEGRITY_PAGE,
            REFRESH => deployment::RESTORE,
            CONSOLE => deployment::DEPLOY,
            _ => VERIFY_PAGE,
        });
        icon(
            item.hDC,
            symbol,
            bounds.left + 10,
            (bounds.top + bounds.bottom - 14) / 2,
            if selected { ACCENT } else { MUTED },
        );
    }
    let foreground = if accent {
        ON_ACCENT
    } else if disabled || nav && !selected {
        MUTED
    } else {
        TEXT
    };
    text(
        item.hDC,
        if accent { theme.emphasis } else { theme.body },
        label_area,
        &get(GetParent(item.hwndItem), id).replace('&', ""),
        foreground,
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
            if accent { ON_ACCENT } else { ACCENT },
        );
    }
}

pub unsafe fn list_item(item: &DRAWITEMSTRUCT, theme: &Theme) {
    let combo = item.CtlType == ODT_COMBOBOX;
    let selected = item.itemState & ODS_SELECTED != 0;
    let background = if selected {
        SELECTED
    } else if !combo && item.itemID % 2 == 1 {
        ELEVATED
    } else if combo {
        INPUT
    } else {
        PANEL
    };
    fill(item.hDC, item.rcItem, background);

    let label = if item.itemID == u32::MAX {
        match item.CtlID as usize {
            pages::ROOT => "Selecione a origem".to_owned(),
            id => deployment::empty_choice(id).to_owned(),
        }
    } else {
        let (measure, read) = if combo {
            (CB_GETLBTEXTLEN, CB_GETLBTEXT)
        } else {
            (LB_GETTEXTLEN, LB_GETTEXT)
        };
        let length = SendMessageW(item.hwndItem, measure, item.itemID as usize, 0);
        if length < 0 {
            return;
        }
        let mut buffer = vec![0u16; length as usize + 1];
        SendMessageW(
            item.hwndItem,
            read,
            item.itemID as usize,
            buffer.as_mut_ptr() as isize,
        );
        String::from_utf16_lossy(&buffer[..length as usize])
    };

    let color = if label.starts_with("CRÍTICO") {
        CRITICAL
    } else if label.starts_with("ATENÇÃO") {
        WARNING
    } else if item.itemState & ODS_DISABLED != 0 {
        MUTED
    } else {
        TEXT
    };
    let mut area = item.rcItem;
    area.left += 8;
    area.right -= 8;
    if item.CtlID as usize == FILES {
        file_row(item.hDC, theme, area, &label);
    } else {
        text(item.hDC, theme.body, area, &label, color, LINE);
    }
    if item.itemState & ODS_FOCUS != 0 {
        outline(item.hDC, item.rcItem, ACCENT);
    }
}

unsafe fn file_row(dc: HDC, theme: &Theme, area: RECT, label: &str) {
    let width = area.right - area.left;
    let mut fields = label.splitn(3, '\t');
    let name = fields.next().unwrap_or_default();
    let state = fields.next().unwrap_or_default();
    let size = fields.next();
    let name_width = width * 57 / 100;
    let kind_width = if size.is_some() {
        width / 7
    } else {
        width - name_width
    };
    let color = if state.contains("corrompido")
        || size.is_some_and(|value| value.starts_with("Indisponível"))
    {
        CRITICAL
    } else if state.contains("incompleto") || state.contains("execução") {
        WARNING
    } else if state == "Conclusão registrada" {
        theme::SUCCESS
    } else {
        MUTED
    };

    text(
        dc,
        theme.body,
        rect(area.left, area.top, name_width - 8, area.bottom - area.top),
        name,
        TEXT,
        LINE,
    );
    text(
        dc,
        theme.body,
        rect(
            area.left + name_width,
            area.top,
            kind_width - 5,
            area.bottom - area.top,
        ),
        state,
        color,
        LINE,
    );
    if let Some(size) = size {
        text(
            dc,
            theme.body,
            rect(
                area.left + name_width + kind_width,
                area.top,
                width - name_width - kind_width,
                area.bottom - area.top,
            ),
            size,
            color,
            LINE,
        );
    }
}

unsafe fn icon(dc: HDC, section: usize, x: i32, y: i32, color: COLORREF) {
    // Stroke coordinates use a 14px grid. The stock pen remains owned by GDI.
    let paths: &[&[(i32, i32)]] = match section {
        deployment::DEPLOY => &[
            &[(0, 0), (6, 0), (6, 6), (0, 6), (0, 0)],
            &[(9, 0), (14, 0), (14, 14), (9, 14), (9, 0)],
            &[(0, 9), (6, 9), (6, 14), (0, 14), (0, 9)],
        ],
        DIAGNOSIS => &[&[(0, 7), (4, 7), (6, 2), (8, 12), (10, 7), (14, 7)]],
        INTEGRITY_PAGE => &[
            &[(7, 0), (13, 3), (13, 9), (7, 14), (1, 9), (1, 3), (7, 0)],
            &[(4, 7), (6, 9), (10, 5)],
        ],
        DASHBOARD | DISKS => &[
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
        VERIFY_PAGE => &[
            &[
                (7, 0),
                (12, 2),
                (14, 7),
                (12, 12),
                (7, 14),
                (2, 12),
                (0, 7),
                (2, 2),
                (7, 0),
            ],
            &[(4, 7), (6, 9), (10, 5)],
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
        let width = paint.rcPaint.right - paint.rcPaint.left;
        let height = paint.rcPaint.bottom - paint.rcPaint.top;
        if width > 0 && height > 0 {
            buffered(dc, &area, paint.rcPaint, theme, app);
        }
    }
    EndPaint(hwnd, &paint);
}

unsafe fn buffered(dc: HDC, area: &Layout, dirty: RECT, theme: &Theme, app: Option<&App>) {
    // This DC and bitmap are local to WM_PAINT. The original bitmap is restored before deletion.
    let buffer = CreateCompatibleDC(dc);
    if buffer.is_null() {
        scene(dc, area, theme, app);
        return;
    }
    let width = dirty.right - dirty.left;
    let height = dirty.bottom - dirty.top;
    let bitmap = CreateCompatibleBitmap(dc, width, height);
    if bitmap.is_null() {
        DeleteDC(buffer);
        scene(dc, area, theme, app);
        return;
    }

    let previous = SelectObject(buffer, bitmap);
    SetViewportOrgEx(buffer, -dirty.left, -dirty.top, null_mut());
    scene(buffer, area, theme, app);
    BitBlt(
        dc, dirty.left, dirty.top, width, height, buffer, dirty.left, dirty.top, SRCCOPY,
    );

    SelectObject(buffer, previous);
    DeleteObject(bitmap);
    DeleteDC(buffer);
}

unsafe fn scene(dc: HDC, area: &Layout, theme: &Theme, app: Option<&App>) {
    fill(dc, rect(0, 0, area.width, area.height), BACKGROUND);
    fill(dc, rect(0, 0, area.rail, area.height), SIDEBAR);
    fill(dc, rect(area.rail - 1, 0, 1, area.height), BORDER);
    fill(dc, rect(14, 309, area.rail - 28, 1), BORDER);
    fill(dc, rect(14, area.height - 132, area.rail - 28, 1), BORDER);

    surface(dc, rect(15, 20, 30, 30), ACCENT, ACCENT, 9);
    icon(dc, INTEGRITY_PAGE, 23, 28, ON_ACCENT);
    text(
        dc,
        if area.rail < 200 {
            theme.body
        } else {
            theme.emphasis
        },
        rect(53, 16, area.rail - 57, 25),
        "ArchRescue",
        TEXT,
        LINE,
    );
    text(
        dc,
        theme.body,
        rect(54, 42, area.rail - 59, 18),
        "ArchGTi",
        MUTED,
        LINE,
    );

    if let Some(app) = app {
        let x = area.left();
        let width = area.body_width();
        let label = navigation::label(app.section, false);
        let (title, subtitle) = navigation::heading(app);
        icon(dc, app.section, x, 16, MUTED);
        text(
            dc,
            theme.body,
            rect(x + 23, 9, width - 170, 28),
            label,
            TEXT,
            LINE,
        );
        let source = app
            .roots
            .get(app.root_index)
            .map(|path| path.display().to_string());
        let context = if app.technician {
            "Modo técnico".into()
        } else {
            source.map_or_else(|| "Windows x64".into(), |path| format!("Origem  ·  {path}"))
        };
        surface(dc, rect(x + width - 145, 10, 145, 25), PANEL, BORDER, 7);
        text(
            dc,
            theme.body,
            rect(x + width - 136, 10, 128, 25),
            &context,
            MUTED,
            LINE | DT_CENTER,
        );
        fill(dc, rect(area.rail, 44, area.width - area.rail, 1), BORDER);
        text(dc, theme.heading, rect(x, 56, width, 38), title, TEXT, LINE);
        text(
            dc,
            theme.body,
            rect(x, 96, width, 24),
            subtitle,
            MUTED,
            LINE,
        );

        cards::render(dc, area, theme, app);
        footer(dc, area, app);
    }
}

unsafe fn footer(dc: HDC, area: &Layout, app: &App) {
    let x = area.left();
    let width = area.body_width();
    let top = area.footer();
    fill(dc, rect(area.rail, top, area.width - area.rail, 51), PANEL);
    fill(dc, rect(area.rail, top, area.width - area.rail, 1), BORDER);
    let color = if app.error_status { CRITICAL } else { ACCENT };
    icon(
        dc,
        if app.error_status {
            CANCEL
        } else {
            VERIFY_PAGE
        },
        x,
        top + 13,
        color,
    );

    if let Some(progress) = &app.progress {
        progress_bar(
            dc,
            x,
            top - 5,
            width,
            progress.acknowledged_bytes,
            progress.planned_bytes,
            ACCENT,
        );
        if progress.planned_bytes > 0 {
            let verified = ((progress.verified_bytes as u128 * width as u128)
                / progress.planned_bytes as u128)
                .min(width as u128) as i32;
            fill(dc, rect(x, top - 5, verified, 4), theme::SUCCESS);
        }
    } else if let Some((bytes, total)) = app.media_progress {
        progress_bar(dc, x, top - 5, width, bytes, total, ACCENT);
    } else if app.busy {
        fill(dc, rect(x, top - 5, 32, 4), ACCENT);
    }
}

unsafe fn progress_bar(
    dc: HDC,
    x: i32,
    y: i32,
    width: i32,
    value: u64,
    total: u64,
    color: COLORREF,
) {
    fill(dc, rect(x, y, width, 4), BORDER);
    if total > 0 {
        let filled = ((value as u128 * width as u128) / total as u128).min(width as u128) as i32;
        fill(dc, rect(x, y, filled, 4), color);
    }
}
