//! Owner-painted selection control with the CB_* operations used by the shell.

use std::ffi::c_void;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, ReleaseCapture, SetCapture, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_NEXT, VK_PRIOR,
    VK_SPACE, VK_UP,
};

use super::*;

const CLASS: &str = "ArchRescueSelect";
const POPUP_CLASS: &str = "ArchRescueOptions";
const ROW: i32 = 30;
const INSET: i32 = 4;
const SCROLL_WIDTH: i32 = 12;

#[derive(Default)]
struct Choice {
    options: RefCell<Vec<String>>,
    selected: Cell<Option<usize>>,
    highlight: Cell<Option<usize>>,
    popup: Cell<HWND>,
    top: Cell<usize>,
    rows: Cell<usize>,
    thumb_drag: Cell<Option<i32>>,
}

pub fn class() -> &'static str {
    CLASS
}

pub unsafe fn register(instance: HINSTANCE) -> Result<()> {
    let classes: [(&str, WNDPROC); 2] = [
        (CLASS, Some(procedure)),
        (POPUP_CLASS, Some(popup_procedure)),
    ];
    for (name, handler) in classes {
        let name = wide(name);
        let class = WNDCLASSW {
            lpfnWndProc: handler,
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: name.as_ptr(),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(os_error("Register selection control"));
        }
    }

    Ok(())
}

unsafe fn state(hwnd: HWND) -> Option<&'static Choice> {
    // The control owns this allocation until WM_NCDESTROY on the GUI thread.
    (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Choice).as_ref()
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let choice = Box::into_raw(Box::<Choice>::default());
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, choice as isize);
        return 1;
    }

    let Some(choice) = state(hwnd) else {
        return DefWindowProcW(hwnd, message, wp, lp);
    };

    match message {
        WM_NCDESTROY => {
            close(hwnd, choice);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            // SAFETY: WM_NCCREATE transferred this allocation to this control.
            drop(Box::from_raw(choice as *const Choice as *mut Choice));
        }
        CB_ADDSTRING if lp != 0 => {
            // These messages are sent synchronously with a live NUL-terminated UTF-16 string.
            let value = lp as *const u16;
            let mut end = value;
            while *end != 0 {
                end = end.add(1);
            }
            let length = end.offset_from(value) as usize;
            let label = String::from_utf16_lossy(std::slice::from_raw_parts(value, length));
            let mut options = choice.options.borrow_mut();
            let index = options.len();
            options.push(label);
            return index as isize;
        }
        CB_RESETCONTENT => {
            close(hwnd, choice);
            choice.options.borrow_mut().clear();
            choice.selected.set(None);
            InvalidateRect(hwnd, null(), 0);
            return 0;
        }
        CB_SETCURSEL => {
            let selected = (wp < choice.options.borrow().len()).then_some(wp);
            choice.selected.set(selected);
            InvalidateRect(hwnd, null(), 0);
            return selected.map_or(CB_ERR as isize, |index| index as isize);
        }
        CB_GETCURSEL => {
            return choice
                .selected
                .get()
                .map_or(CB_ERR as isize, |index| index as isize)
        }
        WM_PAINT => {
            paint(hwnd, hwnd, choice, false);
            return 0;
        }
        WM_ERASEBKGND => return 1,
        WM_GETDLGCODE => {
            let key = (lp as *const MSG).as_ref().map(|msg| msg.wParam as u16);
            let wants_key =
                key == Some(VK_RETURN) || key == Some(VK_ESCAPE) && !choice.popup.get().is_null();
            return (DLGC_WANTARROWS | if wants_key { DLGC_WANTMESSAGE } else { 0 }) as isize;
        }
        WM_SETFOCUS | WM_ENABLE => {
            if message == WM_ENABLE && wp == 0 {
                close(hwnd, choice);
            }
            InvalidateRect(hwnd, null(), 0);
        }
        WM_KILLFOCUS | WM_CAPTURECHANGED | WM_WINDOWPOSCHANGED => {
            close(hwnd, choice);
            InvalidateRect(hwnd, null(), 0);
        }
        WM_SHOWWINDOW if wp == 0 => close(hwnd, choice),
        WM_KEYDOWN => {
            if keyboard(hwnd, choice, wp as u16, lp) {
                return 0;
            }
        }
        WM_LBUTTONDOWN => {
            if choice.popup.get().is_null() {
                SetFocus(hwnd);
                open(hwnd, choice);
            } else {
                popup_pointer(hwnd, choice, client_point(lp), true);
            }
            return 0;
        }
        WM_LBUTTONUP => {
            choice.thumb_drag.set(None);
            return 0;
        }
        WM_MOUSEMOVE if !choice.popup.get().is_null() => {
            popup_pointer(hwnd, choice, client_point(lp), false);
            return 0;
        }
        WM_MOUSEWHEEL if !choice.popup.get().is_null() => {
            let delta = ((wp >> 16) as i16) as i32;
            scroll(choice, -delta.signum() * 3);
            return 0;
        }
        _ => {}
    }

    DefWindowProcW(hwnd, message, wp, lp)
}

unsafe fn keyboard(hwnd: HWND, choice: &Choice, key: u16, lp: LPARAM) -> bool {
    let count = choice.options.borrow().len();
    let expanded = !choice.popup.get().is_null();

    match key {
        VK_ESCAPE if expanded => close(hwnd, choice),
        VK_RETURN | VK_SPACE if expanded => {
            if lp & (1 << 30) == 0 {
                choose(hwnd, choice, choice.highlight.get());
            }
        }
        VK_RETURN | VK_SPACE if lp & (1 << 30) == 0 => open(hwnd, choice),
        VK_DOWN | VK_UP | VK_HOME | VK_END | VK_PRIOR | VK_NEXT if count > 0 => {
            let index = if expanded {
                choice.highlight.get()
            } else {
                choice.selected.get()
            };
            let page = choice.rows.get().max(1);
            let next = match key {
                VK_DOWN => index.map_or(0, |index| index + 1),
                VK_UP => index.unwrap_or(1).saturating_sub(1),
                VK_HOME => 0,
                VK_END => count - 1,
                VK_PRIOR => index.unwrap_or(0).saturating_sub(page),
                _ => index.unwrap_or(0) + page,
            }
            .min(count - 1);

            if expanded {
                choice.highlight.set(Some(next));
                let top = choice.top.get().clamp(next.saturating_sub(page - 1), next);
                choice.top.set(top);
                InvalidateRect(choice.popup.get(), null(), 0);
            } else {
                choose(hwnd, choice, Some(next));
            }
        }
        _ => return false,
    }

    true
}

unsafe fn open(hwnd: HWND, choice: &Choice) {
    let count = choice.options.borrow().len();
    if count == 0 || IsWindowEnabled(hwnd) == 0 {
        return;
    }

    let Ok(monitor) = window::monitor(hwnd) else {
        return;
    };
    let mut bounds = RECT::default();
    GetWindowRect(hwnd, &mut bounds);
    let below = monitor.rcWork.bottom - bounds.bottom;
    let above = bounds.top - monitor.rcWork.top;
    let requested = count.min(7);
    let rows_below = ((below - 2 * INSET) / ROW).max(0) as usize;
    let rows_above = ((above - 2 * INSET) / ROW).max(0) as usize;
    let upward = rows_below < requested && rows_above > rows_below;
    let rows = requested.min(if upward { rows_above } else { rows_below });
    if rows == 0 {
        return;
    }

    let height = rows as i32 * ROW + 2 * INSET;
    let popup = CreateWindowExW(
        WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
        wide(POPUP_CLASS).as_ptr(),
        null(),
        WS_POPUP,
        bounds.left,
        if upward {
            bounds.top - height - 1
        } else {
            bounds.bottom + 1
        },
        bounds.right - bounds.left,
        height,
        GetParent(hwnd),
        null_mut(),
        GetModuleHandleW(null()),
        hwnd.cast::<c_void>(),
    );
    if popup.is_null() {
        return;
    }

    let selected = choice.selected.get().unwrap_or(0);
    choice.rows.set(rows);
    choice.top.set(selected.min(count - rows));
    choice.highlight.set(Some(selected));
    choice.popup.set(popup);
    SetCapture(hwnd);
    ShowWindow(popup, SW_SHOWNOACTIVATE);
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn close(hwnd: HWND, choice: &Choice) {
    let popup = choice.popup.replace(null_mut());
    if popup.is_null() {
        return;
    }

    choice.thumb_drag.set(None);
    if GetCapture() == hwnd {
        ReleaseCapture();
    }
    DestroyWindow(popup);
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn choose(hwnd: HWND, choice: &Choice, selected: Option<usize>) {
    close(hwnd, choice);
    if choice.selected.replace(selected) != selected {
        InvalidateRect(hwnd, null(), 0);
        let command = GetDlgCtrlID(hwnd) as usize | ((CBN_SELCHANGE as usize) << 16);
        SendMessageW(GetParent(hwnd), WM_COMMAND, command, hwnd as isize);
    }
}

fn client_point(lp: LPARAM) -> POINT {
    POINT {
        x: lp as i16 as i32,
        y: (lp >> 16) as i16 as i32,
    }
}

unsafe fn scroll(choice: &Choice, delta: i32) {
    let max_top = choice
        .options
        .borrow()
        .len()
        .saturating_sub(choice.rows.get());
    let top = choice
        .top
        .get()
        .saturating_add_signed(delta as isize)
        .min(max_top);
    choice.top.set(top);
    InvalidateRect(choice.popup.get(), null(), 0);
}

unsafe fn popup_pointer(hwnd: HWND, choice: &Choice, mut point: POINT, pressed: bool) {
    let popup = choice.popup.get();
    ClientToScreen(hwnd, &mut point);
    ScreenToClient(popup, &mut point);

    let mut area = RECT::default();
    GetClientRect(popup, &mut area);
    let count = choice.options.borrow().len();
    let rows = choice.rows.get();
    let has_scroll = count > rows;
    let over_scroll = has_scroll && point.x >= area.right - SCROLL_WIDTH;
    if let Some(grab) = choice.thumb_drag.get() {
        let (length, _) = thumb(choice, area.bottom);
        let travel = (area.bottom - 2 * INSET - length).max(1);
        let position = (point.y - grab - INSET).clamp(0, travel);
        choice
            .top
            .set(position as usize * (count - rows) / travel as usize);
        InvalidateRect(popup, null(), 0);
    } else if PtInRect(&area, point) != 0 {
        if over_scroll {
            if pressed {
                let (length, top) = thumb(choice, area.bottom);
                if (top..top + length).contains(&point.y) {
                    choice.thumb_drag.set(Some(point.y - top));
                } else {
                    scroll(
                        choice,
                        if point.y < top {
                            -(rows as i32)
                        } else {
                            rows as i32
                        },
                    );
                }
            }
        } else if point.y >= INSET {
            let row = ((point.y - INSET) / ROW) as usize;
            if row < rows {
                let index = choice.top.get() + row;
                if pressed {
                    choose(hwnd, choice, Some(index));
                } else if choice.highlight.replace(Some(index)) != Some(index) {
                    InvalidateRect(popup, null(), 0);
                }
            }
        }
    } else if pressed {
        close(hwnd, choice);
    }
}

fn thumb(choice: &Choice, height: i32) -> (i32, i32) {
    let track = height - 2 * INSET;
    let count = choice.options.borrow().len();
    let rows = choice.rows.get();
    let length = (track * rows as i32 / count as i32).max(20);
    let top = (track - length) * choice.top.get() as i32 / (count - rows) as i32;

    (length, INSET + top)
}

unsafe extern "system" fn popup_procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    match message {
        WM_NCCREATE => {
            let creation = &*(lp as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, creation.lpCreateParams as isize);
            1
        }
        WM_PAINT => {
            let owner = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
            if let Some(choice) = state(owner) {
                paint(hwnd, owner, choice, true);
            }
            0
        }
        WM_ERASEBKGND => 1,
        _ => DefWindowProcW(hwnd, message, wp, lp),
    }
}

unsafe fn paint(hwnd: HWND, owner: HWND, choice: &Choice, popup: bool) {
    let mut painting = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut painting);
    let mut bounds = RECT::default();
    GetClientRect(hwnd, &mut bounds);

    let parent = GetWindowLongPtrW(GetParent(owner), GWLP_USERDATA) as *const WindowState;
    if let Some(parent) = parent.as_ref() {
        let options = choice.options.borrow();
        if let Ok(mut frame) = parent.control_frame.try_borrow_mut() {
            if frame.reserve(dc, bounds.right, bounds.bottom) {
                frame.draw(bounds.right, bounds.bottom, |scene| {
                    if popup {
                        let top = choice.top.get();
                        let rows = choice.rows.get();
                        let selected = choice
                            .selected
                            .get()
                            .and_then(|index| index.checked_sub(top));
                        let highlighted = choice
                            .highlight
                            .get()
                            .and_then(|index| index.checked_sub(top));
                        let scrollbar =
                            (options.len() > rows).then(|| thumb(choice, bounds.bottom));
                        presentation::select_popup(
                            scene,
                            &parent.theme,
                            bounds,
                            presentation::SelectOptions {
                                labels: &options[top..top + rows],
                                selected,
                                highlighted,
                                scrollbar,
                                inset: INSET,
                                row: ROW,
                                dragging: choice.thumb_drag.get().is_some(),
                            },
                        );
                    } else {
                        let id = GetDlgCtrlID(owner) as usize;
                        let label = choice
                            .selected
                            .get()
                            .and_then(|index| options.get(index))
                            .map_or_else(
                                || {
                                    if id == pages::ROOT {
                                        "Selecione a origem"
                                    } else {
                                        deployment::empty_choice(id)
                                    }
                                },
                                String::as_str,
                            );
                        presentation::select_field(
                            scene,
                            &parent.theme,
                            bounds,
                            label,
                            GetFocus() == owner,
                            IsWindowEnabled(owner) != 0,
                            !choice.popup.get().is_null(),
                        );
                    }
                });
                frame.present(
                    dc,
                    painting.rcPaint,
                    painting.rcPaint.left,
                    painting.rcPaint.top,
                );
            }
        }
    }

    EndPaint(hwnd, &painting);
}
