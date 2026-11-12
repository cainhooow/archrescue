//! Native owner-drawn buttons and page summaries. Input and focus stay with Win32.

use windows_sys::Win32::UI::Controls::{ODS_DISABLED, ODS_FOCUS, ODS_SELECTED};

use super::{layout::Layout, theme::*, *};

mod cards;

use super::renderer::{Frame, Scene};
use tiny_skia::PathBuilder;

fn fill(scene: &mut Scene<'_>, bounds: RECT, color: COLORREF) {
    scene.fill(bounds, color);
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    }
}

fn text(
    scene: &mut Scene<'_>,
    font: HFONT,
    bounds: RECT,
    label: &str,
    color: COLORREF,
    flags: u32,
) {
    scene.text(font, bounds, label, color, flags);
}

const LINE: u32 = DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX;

fn surface(
    scene: &mut Scene<'_>,
    bounds: RECT,
    background: COLORREF,
    border: COLORREF,
    radius: i32,
) {
    scene.surface(bounds, background, border, radius);
}

pub unsafe fn control_frame(
    frame: &mut Frame,
    item: &DRAWITEMSTRUCT,
    theme: &Theme,
    selected_page: usize,
    hovered: bool,
) {
    let area = item.rcItem;
    let width = area.right - area.left;
    let height = area.bottom - area.top;

    if !frame.reserve(item.hDC, width, height) {
        return;
    }

    let bounds = rect(0, 0, width, height);
    frame.draw(width, height, |scene| match item.CtlType {
        ODT_BUTTON => button(scene, item, bounds, theme, selected_page, hovered),
        _ => list_item(scene, item, bounds, theme),
    });
    frame.present(item.hDC, area, 0, 0);
}

unsafe fn button(
    scene: &mut Scene<'_>,
    item: &DRAWITEMSTRUCT,
    bounds: RECT,
    theme: &Theme,
    selected_page: usize,
    hovered: bool,
) {
    let id = item.CtlID as usize;
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
    let accent = (primary || id == NEXT && selected_page == DASHBOARD) && !disabled;
    let focused = item.itemState & ODS_FOCUS != 0;
    let backdrop = if nav {
        SIDEBAR
    } else if matches!(
        id,
        OPEN | UP | DISK_DETAILS | VERIFY | pages::SCAN | pages::PROTECT | pages::REPORT_ACTION
    ) {
        BACKGROUND
    } else {
        PANEL
    };
    let color = if disabled {
        backdrop
    } else if pressed && !accent {
        ACCENT_PRESSED_SURFACE
    } else if selected && hovered {
        HOVER
    } else if selected {
        SELECTED
    } else if accent && pressed {
        ACCENT_PRESSED
    } else if accent && hovered {
        ACCENT_HOVER
    } else if accent {
        ACCENT
    } else if hovered {
        HOVER
    } else if nav {
        SIDEBAR
    } else {
        ELEVATED
    };

    scene.clear(backdrop);
    if !nav || selected || pressed || hovered || focused {
        let border = if focused || accent { ACCENT } else { BORDER };
        let end = if accent && !pressed {
            ACCENT_END
        } else {
            color
        };
        scene.gradient(bounds, color, end, border, 8);
    }
    if focused && accent {
        scene.focus_outline(rect(3, 3, bounds.right - 6, bounds.bottom - 6), ON_ACCENT);
    }
    if selected {
        fill(
            scene,
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
    label_area.left += if nav { 32 } else { 7 };
    label_area.right -= 7;
    if nav {
        let symbol = page.unwrap_or(match id {
            TECHNICIAN => INTEGRITY_PAGE,
            REFRESH => deployment::RESTORE,
            CONSOLE => deployment::DEPLOY,
            _ => VERIFY_PAGE,
        });
        icon(
            scene,
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
        scene,
        theme.body(),
        label_area,
        &get(GetParent(item.hwndItem), id).replace('&', ""),
        foreground,
        LINE | if nav { DT_LEFT } else { DT_CENTER },
    );
}

unsafe fn list_item(scene: &mut Scene<'_>, item: &DRAWITEMSTRUCT, bounds: RECT, theme: &Theme) {
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
    scene.clear(if combo { INPUT } else { PANEL });
    if selected || !combo && item.itemID % 2 == 1 {
        scene.surface(bounds, background, background, 5);
    }

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
    let mut area = bounds;
    area.left += 8;
    area.right -= 8;
    if item.CtlID as usize == FILES {
        file_row(scene, theme, area, &label);
    } else {
        text(scene, theme.body(), area, &label, color, LINE);
    }
    if item.itemState & ODS_FOCUS != 0 {
        scene.focus_outline(bounds, ACCENT);
    }
}

fn file_row(dc: &mut Scene<'_>, theme: &Theme, area: RECT, label: &str) {
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
        theme.body(),
        rect(area.left, area.top, name_width - 8, area.bottom - area.top),
        name,
        TEXT,
        LINE,
    );
    text(
        dc,
        theme.body(),
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
            theme.body(),
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

fn icon(scene: &mut Scene<'_>, section: usize, x: i32, y: i32, color: COLORREF) {
    // Stroke coordinates use a 14-pixel grid.
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

    let mut builder = PathBuilder::new();
    for path in paths {
        let [(start_x, start_y), rest @ ..] = *path else {
            continue;
        };

        builder.move_to((x + start_x) as f32 + 0.5, (y + start_y) as f32 + 0.5);
        for &(px, py) in rest {
            builder.line_to((x + px) as f32 + 0.5, (y + py) as f32 + 0.5);
        }
    }
    if let Some(path) = builder.finish() {
        scene.stroke(&path, color, 1.4);
    }
}

pub unsafe fn paint(hwnd: HWND, state: &WindowState) {
    let mut paint = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    let dirty = paint.rcPaint;

    if !dc.is_null() && dirty.right > dirty.left && dirty.bottom > dirty.top {
        let area = Layout::read(hwnd);
        let mut frame = state.frame.borrow_mut();
        let redraw = state.scene_dirty.get();

        if frame.reserve(dc, area.width, area.height) {
            if redraw {
                if let Ok(app) = state.app.try_borrow() {
                    frame.draw(area.width, area.height, |canvas| {
                        scene(canvas, &area, &state.theme, &app);
                    });
                    state.scene_dirty.set(false);
                }
            }

            if !state.scene_dirty.get() && frame.present(dc, dirty, dirty.left, dirty.top) {
                EndPaint(hwnd, &paint);
                return;
            }
        }

        // Keep the window recoverable if the renderer runs out of resources.
        SetDCBrushColor(dc, BACKGROUND);
        FillRect(dc, &dirty, GetStockObject(DC_BRUSH));
        if state.app.try_borrow().is_ok() && !state.render_failed.replace(true) {
            PostMessageW(hwnd, RENDER_FAILURE, 0, 0);
        }
    }

    EndPaint(hwnd, &paint);
}

fn scene(dc: &mut Scene<'_>, area: &Layout, theme: &Theme, app: &App) {
    dc.clear(BACKGROUND);
    fill(dc, rect(0, 0, area.rail, area.height), SIDEBAR);
    fill(dc, rect(area.rail - 1, 0, 1, area.height), BORDER);
    fill(dc, rect(14, 312, area.rail - 28, 1), BORDER);
    fill(dc, rect(14, area.height - 132, area.rail - 28, 1), BORDER);

    dc.gradient(rect(15, 20, 30, 30), ACCENT, ACCENT_END, ACCENT, 9);
    icon(dc, INTEGRITY_PAGE, 23, 28, ON_ACCENT);
    text(
        dc,
        if area.rail < 200 {
            theme.body()
        } else {
            theme.emphasis()
        },
        rect(53, 16, area.rail - 57, 25),
        "ArchRescue",
        TEXT,
        LINE,
    );
    text(
        dc,
        theme.body(),
        rect(54, 42, area.rail - 59, 18),
        "ArchGTi",
        MUTED,
        LINE,
    );

    let x = area.left();
    let width = area.body_width();
    let label = navigation::label(app.section, false);
    let (title, subtitle) = navigation::heading(app);
    icon(dc, app.section, x, 16, MUTED);
    text(
        dc,
        theme.body(),
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
        theme.body(),
        rect(x + width - 136, 10, 128, 25),
        &context,
        MUTED,
        LINE | DT_CENTER,
    );
    fill(dc, rect(area.rail, 44, area.width - area.rail, 1), BORDER);
    text(
        dc,
        theme.heading(),
        rect(x, 56, width, 38),
        title,
        TEXT,
        LINE,
    );
    text(
        dc,
        theme.body(),
        rect(x, 96, width, 24),
        subtitle,
        MUTED,
        LINE,
    );

    cards::render(dc, area, theme, app);
    footer(dc, area, app);
}

fn footer(dc: &mut Scene<'_>, area: &Layout, app: &App) {
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

fn progress_bar(
    dc: &mut Scene<'_>,
    x: i32,
    y: i32,
    width: i32,
    value: u64,
    total: u64,
    color: COLORREF,
) {
    surface(dc, rect(x, y, width, 5), ELEVATED, BORDER, 2);
    if total > 0 {
        let filled = ((value as u128 * width as u128) / total as u128).min(width as u128) as i32;
        dc.gradient(rect(x, y, filled, 5), color, ACCENT_END, color, 2);
    }
}
