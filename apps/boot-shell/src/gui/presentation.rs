//! Native owner-drawn buttons and page summaries. Input and focus stay with Win32.

use windows_sys::Win32::UI::Controls::{ODS_DISABLED, ODS_FOCUS, ODS_SELECTED};

use super::{layout::Layout, theme::*, *};

mod cards;
mod icons;

use icons::Icon;

use super::renderer::{Frame, Scene};

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
    ) || deployment::primary(id)
        || id == IDYES as usize;
    let disabled = item.itemState & ODS_DISABLED != 0;
    let pressed = item.itemState & ODS_SELECTED != 0;
    let accent = primary && !disabled;
    let focused = item.itemState & ODS_FOCUS != 0;
    let modal = id == IDYES as usize || id == IDNO as usize;
    let backdrop = if nav {
        SIDEBAR
    } else if modal {
        MODAL
    } else if matches!(id, PREVIOUS | NEXT) {
        FOOTER
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
        let border = if focused || accent {
            ACCENT
        } else if selected {
            SELECTED_BORDER
        } else {
            BORDER
        };
        let end = if accent && !pressed {
            ACCENT_END
        } else {
            color
        };
        scene.gradient(bounds, color, end, border, 8);
    }
    if focused && modal {
        scene.focus_outline(bounds, ACCENT_HOVER);
    } else if focused && accent {
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

    if modal && focused && !accent {
        scene.focus_outline(
            rect(1, 1, bounds.right - 2, bounds.bottom - 2),
            ACCENT_HOVER,
        );
    }

    let mut label_area = bounds;
    label_area.left += if nav { 35 } else { 7 };
    label_area.right -= 7;
    let foreground = if accent {
        ON_ACCENT
    } else if disabled {
        DISABLED
    } else if selected {
        NAV_TEXT
    } else if nav {
        MUTED
    } else {
        TEXT
    };
    let symbol = page.map(Icon::page).or(match id {
        TECHNICIAN => Some(Icon::Sliders),
        REFRESH => Some(Icon::Refresh),
        CONSOLE => Some(Icon::Terminal),
        CANCEL => Some(Icon::Stop),
        PREVIOUS => Some(Icon::Left),
        NEXT => Some(Icon::Right),
        id if id == IDYES as usize => Some(Icon::Right),
        _ => None,
    });
    if let Some(symbol) = symbol {
        if !nav {
            label_area.left += 13;
        }
        symbol.draw(
            scene,
            bounds.left + 10,
            (bounds.top + bounds.bottom - 16) / 2,
            16,
            foreground,
        );
    }
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
    let selected = item.itemState & ODS_SELECTED != 0;
    let alternating = item.itemID % 2 == 1;
    scene.clear(PANEL);
    if selected || alternating {
        let background = if selected { SELECTED } else { METRIC };
        scene.surface(bounds, background, background, 5);
    }

    let length = SendMessageW(item.hwndItem, LB_GETTEXTLEN, item.itemID as usize, 0);
    if length < 0 {
        return;
    }
    let mut buffer = vec![0u16; length as usize + 1];
    SendMessageW(
        item.hwndItem,
        LB_GETTEXT,
        item.itemID as usize,
        buffer.as_mut_ptr() as isize,
    );
    let label = String::from_utf16_lossy(&buffer[..length as usize]);

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

pub fn scrollbar(scene: &mut Scene<'_>, height: i32, top: i32, length: i32, color: COLORREF) {
    surface(scene, rect(5, 3, 7, height - 6), INPUT, INPUT, 4);
    surface(scene, rect(5, top, 7, length), color, color, 4);
}

pub struct ConfirmationCard<'a> {
    pub bounds: RECT,
    pub title: &'a str,
    pub question: &'a str,
    pub labels: [&'a str; 2],
    pub rows: [i32; 2],
    pub notice: (&'a str, &'a str),
}

pub fn confirmation(
    scene: &mut Scene<'_>,
    area: &Layout,
    theme: &Theme,
    card: ConfirmationCard<'_>,
) {
    let px = |reference| area.px(reference);
    let bounds = card.bounds;
    surface(scene, bounds, MODAL, MODAL_BORDER, px(15));

    let x = bounds.left + px(27);
    let width = bounds.right - bounds.left - px(54);
    let y = bounds.top;
    surface(
        scene,
        rect(x, y + px(26), px(41), px(41)),
        SELECTED,
        SELECTED_BORDER,
        px(10),
    );
    Icon::Shield.draw(scene, x + px(11), y + px(37), px(19), ACCENT);
    text(
        scene,
        theme.brand(),
        rect(x + px(53), y + px(26), width - px(53), px(41)),
        card.title,
        TEXT,
        LINE,
    );
    text(
        scene,
        theme.body(),
        rect(x, y + px(77), width, px(26)),
        card.question,
        TEXT,
        LINE,
    );

    let mut top = y + px(112);
    for (label, height) in card.labels.into_iter().zip(card.rows) {
        text(
            scene,
            theme.body(),
            rect(x, top, px(132), px(20)),
            label,
            MUTED,
            LINE,
        );
        top += height;
    }

    let notice_top = top + px(9);
    surface(
        scene,
        rect(x, notice_top, width, px(67)),
        METRIC,
        BORDER,
        px(9),
    );
    Icon::Info.draw(scene, x + px(15), notice_top + px(15), px(16), MUTED);
    text(
        scene,
        theme.body(),
        rect(x + px(42), notice_top + px(9), width - px(55), px(27)),
        card.notice.0,
        TEXT,
        LINE,
    );
    text(
        scene,
        theme.body(),
        rect(x + px(42), notice_top + px(37), width - px(55), px(29)),
        card.notice.1,
        MUTED,
        DT_WORDBREAK | DT_NOPREFIX,
    );
}

pub fn select_field(
    scene: &mut Scene<'_>,
    theme: &Theme,
    bounds: RECT,
    label: &str,
    focused: bool,
    enabled: bool,
    expanded: bool,
) {
    scene.clear(PANEL);
    let border = if focused || expanded { ACCENT } else { BORDER };
    surface(scene, bounds, INPUT, border, 7);
    text(
        scene,
        theme.body(),
        rect(9, 0, bounds.right - 36, bounds.bottom),
        label,
        if enabled { TEXT } else { DISABLED },
        LINE,
    );
    Icon::Down.draw(
        scene,
        bounds.right - 24,
        (bounds.bottom - 16) / 2,
        16,
        if expanded { ACCENT } else { MUTED },
    );
}

pub struct SelectOptions<'a> {
    pub labels: &'a [String],
    pub selected: Option<usize>,
    pub highlighted: Option<usize>,
    pub scrollbar: Option<(i32, i32)>,
    pub inset: i32,
    pub row: i32,
    pub dragging: bool,
}

pub fn select_popup(
    scene: &mut Scene<'_>,
    theme: &Theme,
    bounds: RECT,
    options: SelectOptions<'_>,
) {
    scene.clear(BACKGROUND);
    surface(scene, bounds, PANEL, SELECTED_BORDER, 7);
    let scrollbar_width = if options.scrollbar.is_some() { 12 } else { 0 };
    let width = bounds.right - 2 * options.inset - scrollbar_width;
    for (index, label) in options.labels.iter().enumerate() {
        let top = options.inset + index as i32 * options.row;
        let selected = options.selected == Some(index);
        let highlighted = options.highlighted == Some(index);
        if selected || highlighted {
            let color = if highlighted { SELECTED } else { HOVER };
            let border = if highlighted { SELECTED_BORDER } else { color };
            surface(
                scene,
                rect(options.inset, top, width, options.row),
                color,
                border,
                5,
            );
        }
        text(
            scene,
            theme.body(),
            rect(options.inset + 7, top, width - 14, options.row),
            label,
            if selected { NAV_TEXT } else { TEXT },
            LINE,
        );
    }
    if let Some((length, top)) = options.scrollbar {
        let x = bounds.right - 9;
        surface(
            scene,
            rect(x, options.inset, 6, bounds.bottom - 2 * options.inset),
            INPUT,
            INPUT,
            3,
        );
        let color = if options.dragging {
            ACCENT
        } else {
            SCROLL_THUMB
        };
        surface(scene, rect(x, top, 6, length), color, color, 3);
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
    Icon::page(section).draw(scene, x, y, 16, color);
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
    let px = |reference| area.px(reference);
    fill(dc, rect(0, 0, area.rail, area.height), SIDEBAR);
    fill(dc, rect(area.rail - 1, 0, 1, area.height), SHELL_LINE);
    fill(
        dc,
        rect(px(23), area.navigation(6).0 + px(58), area.rail - px(46), 1),
        SHELL_LINE,
    );
    fill(
        dc,
        rect(px(17), area.utilities() - px(10), area.rail - px(34), 1),
        SHELL_LINE,
    );

    dc.gradient(
        rect(px(23), px(30), px(38), px(38)),
        ACCENT_HOVER,
        ACCENT_END,
        ACCENT,
        px(11),
    );
    Icon::Brand.draw(dc, px(32), px(39), px(20), ON_ACCENT);
    text(
        dc,
        theme.brand(),
        rect(px(70), px(19), area.rail - px(78), px(36)),
        "ArchRescue",
        TEXT,
        LINE,
    );
    text(
        dc,
        theme.mono(),
        rect(px(70), px(53), area.rail - px(78), px(20)),
        "ArchGTi",
        MUTED,
        LINE,
    );

    let x = area.left();
    let width = area.body_width();
    let label = navigation::label(app.section, false);
    let (title, subtitle) = navigation::heading(app);
    Icon::page(app.section).draw(dc, x, px(20), px(16), MUTED);
    text(
        dc,
        theme.body(),
        rect(x + px(26), px(13), width - px(170), px(28)),
        label,
        TEXT,
        LINE,
    );
    if app.technician {
        let badge_x = x + px(26) + px(navigation::context_offset(app.section));
        surface(
            dc,
            rect(badge_x, px(16), px(92), px(23)),
            SELECTED,
            SELECTED_BORDER,
            px(5),
        );
        text(
            dc,
            theme.body(),
            rect(badge_x, px(16), px(92), px(23)),
            "Modo técnico",
            NAV_TEXT,
            LINE | DT_CENTER,
        );
    }

    let context_width = px(108);
    let context_x = x + width - context_width;
    surface(
        dc,
        rect(context_x, px(14), context_width, px(27)),
        ELEVATED,
        BORDER,
        px(6),
    );
    Icon::Windows.draw(dc, context_x + px(10), px(21), px(12), ACCENT);
    text(
        dc,
        theme.mono(),
        rect(context_x + px(25), px(14), context_width - px(30), px(27)),
        theme.context,
        MUTED,
        LINE | DT_CENTER,
    );

    fill(
        dc,
        rect(area.rail, px(54), area.width - area.rail, 1),
        SHELL_LINE,
    );
    text(
        dc,
        theme.heading(),
        rect(x, px(73), width, px(56)),
        title,
        TEXT,
        LINE,
    );
    text(
        dc,
        theme.body(),
        rect(x, px(128), width, px(28)),
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
    fill(
        dc,
        rect(area.rail, top, area.width - area.rail, area.height - top),
        FOOTER,
    );
    fill(dc, rect(area.rail, top, area.width - area.rail, 1), BORDER);
    let (symbol, color) = if app.error_status {
        (Icon::Info, CRITICAL)
    } else if app.busy {
        (Icon::Refresh, ACCENT)
    } else {
        (Icon::Check, theme::SUCCESS)
    };
    symbol.draw(dc, x, top + area.px(25), area.px(14), color);

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
