//! Custom vertical tracks over the native list and read-only edit scroll areas.

use windows_sys::Win32::UI::Controls::{EM_GETFIRSTVISIBLELINE, EM_LINESCROLL};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};

use super::*;

const CLASS: &str = "ArchRescueScroll";
const WIDTH: i32 = 17;
const INSET: i32 = 3;

struct Scroll {
    target: HWND,
    dragging: Cell<Option<i32>>,
    hovering: Cell<bool>,
}

pub unsafe fn register(instance: HINSTANCE) -> Result<()> {
    let name = wide(CLASS);
    let class = WNDCLASSW {
        lpfnWndProc: Some(procedure),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: name.as_ptr(),
        ..Default::default()
    };
    if RegisterClassW(&class) == 0 {
        return Err(os_error("Register scrollbar"));
    }

    Ok(())
}

fn overlay_id(id: usize) -> usize {
    id + 1000
}

pub unsafe fn attach(parent: HWND, id: usize, target: HWND) -> Result<()> {
    let scrollbar = CreateWindowExW(
        0,
        wide(CLASS).as_ptr(),
        null(),
        WS_CHILD,
        0,
        0,
        WIDTH,
        1,
        parent,
        overlay_id(id) as HMENU,
        GetModuleHandleW(null()),
        target.cast(),
    );
    if scrollbar.is_null() {
        return Err(os_error("Create scrollbar"));
    }

    Ok(())
}

pub unsafe fn place(parent: HWND, id: usize, x: i32, y: i32, width: i32, height: i32) {
    let scrollbar = control(parent, overlay_id(id));
    if !scrollbar.is_null() {
        SetWindowPos(
            scrollbar,
            HWND_TOP,
            x + width - WIDTH,
            y,
            WIDTH,
            height,
            SWP_NOACTIVATE,
        );
        InvalidateRect(scrollbar, null(), 0);
    }
}

pub unsafe fn show(parent: HWND, id: usize, visible: bool) {
    let scrollbar = control(parent, overlay_id(id));
    if !scrollbar.is_null() {
        ShowWindow(scrollbar, if visible { SW_SHOWNA } else { SW_HIDE });
    }
}

pub unsafe fn invalidate(parent: HWND) {
    for id in [FILES, CONTENT, BACKUP_SELECTION, pages::ROWS] {
        let scrollbar = control(parent, overlay_id(id));
        if !scrollbar.is_null() && IsWindowVisible(scrollbar) != 0 {
            InvalidateRect(scrollbar, null(), 0);
        }
    }
}

struct Thumb {
    min: i32,
    max: i32,
    position: i32,
    page: i32,
    top: i32,
    length: i32,
    travel: i32,
}

unsafe fn thumb(hwnd: HWND, target: HWND) -> Option<Thumb> {
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    let mut info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        ..Default::default()
    };
    if GetScrollInfo(target, SB_VERT, &mut info) == 0 {
        return None;
    }

    let track = client.bottom - 2 * INSET;
    let page = (info.nPage as i32).max(1);
    let total = info.nMax - info.nMin + 1;
    let range = total - page;
    if track <= 18 || range <= 0 {
        return None;
    }

    let length = (track * page / total).clamp(18, track - 1);
    let travel = track - length;
    let offset = (info.nPos - info.nMin).clamp(0, range);
    Some(Thumb {
        min: info.nMin,
        max: info.nMin + range,
        position: info.nPos,
        page,
        top: INSET + offset * travel / range,
        length,
        travel,
    })
}

unsafe fn move_to(target: HWND, position: i32) {
    match GetDlgCtrlID(target) as usize {
        FILES | pages::ROWS => {
            SendMessageW(target, LB_SETTOPINDEX, position.max(0) as usize, 0);
        }
        _ => {
            let top = SendMessageW(target, EM_GETFIRSTVISIBLELINE, 0, 0);
            SendMessageW(target, EM_LINESCROLL, 0, position as isize - top);
        }
    }
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let creation = &*(lp as *const CREATESTRUCTW);
        let scrollbar = Box::new(Scroll {
            target: creation.lpCreateParams as HWND,
            dragging: Cell::new(None),
            hovering: Cell::new(false),
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(scrollbar) as isize);
        return 1;
    }

    let data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Scroll;
    let Some(scrollbar) = data.as_ref() else {
        return DefWindowProcW(hwnd, message, wp, lp);
    };
    let target = scrollbar.target;

    match message {
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            // SAFETY: WM_NCCREATE transferred this allocation to the HWND.
            drop(Box::from_raw(data));
        }
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            paint(hwnd, scrollbar);
            return 0;
        }
        WM_MOUSEMOVE => {
            if !scrollbar.hovering.replace(true) {
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut tracking);
            }
            if let (Some(offset), Some(thumb)) = (scrollbar.dragging.get(), thumb(hwnd, target)) {
                let y = (lp >> 16) as i16 as i32;
                let position = (y - INSET - offset).clamp(0, thumb.travel);
                move_to(
                    target,
                    thumb.min + position * (thumb.max - thumb.min) / thumb.travel,
                );
            }
            InvalidateRect(hwnd, null(), 0);
        }
        WM_MOUSELEAVE | WM_CAPTURECHANGED | WM_LBUTTONUP => {
            if message == WM_MOUSELEAVE {
                scrollbar.hovering.set(false);
            } else {
                scrollbar.dragging.set(None);
                if GetCapture() == hwnd {
                    ReleaseCapture();
                }
            }
            InvalidateRect(hwnd, null(), 0);
        }
        WM_LBUTTONDOWN => {
            if let Some(thumb) = thumb(hwnd, target) {
                SetFocus(target);
                let y = (lp >> 16) as i16 as i32;
                if (thumb.top..thumb.top + thumb.length).contains(&y) {
                    scrollbar.dragging.set(Some(y - thumb.top));
                    SetCapture(hwnd);
                } else {
                    let delta = if y < thumb.top {
                        -thumb.page
                    } else {
                        thumb.page
                    };
                    move_to(target, (thumb.position + delta).clamp(thumb.min, thumb.max));
                }
                InvalidateRect(hwnd, null(), 0);
            }
            return 0;
        }
        WM_MOUSEWHEEL => {
            SendMessageW(target, message, wp, lp);
            InvalidateRect(hwnd, null(), 0);
            return 0;
        }
        _ => {}
    }

    DefWindowProcW(hwnd, message, wp, lp)
}

unsafe fn paint(hwnd: HWND, scrollbar: &Scroll) {
    let mut paint = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    let state = GetWindowLongPtrW(GetParent(hwnd), GWLP_USERDATA) as *const WindowState;
    if let Some(state) = state.as_ref() {
        if let Ok(mut frame) = state.control_frame.try_borrow_mut() {
            if frame.reserve(dc, client.right, client.bottom) {
                let thumb = thumb(hwnd, scrollbar.target);
                frame.draw(client.right, client.bottom, |scene| {
                    scene.clear(theme::PANEL);
                    if let Some(thumb) = thumb {
                        let color = if scrollbar.dragging.get().is_some() {
                            theme::ACCENT
                        } else if scrollbar.hovering.get() {
                            theme::SCROLL_HOVER
                        } else {
                            theme::SCROLL_THUMB
                        };
                        presentation::scrollbar(
                            scene,
                            client.bottom,
                            thumb.top,
                            thumb.length,
                            color,
                        );
                    }
                });
                frame.present(dc, paint.rcPaint, paint.rcPaint.left, paint.rcPaint.top);
            }
        }
    }
    EndPaint(hwnd, &paint);
}
