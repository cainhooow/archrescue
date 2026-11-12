//! Fullscreen uses monitor bounds and preserves a separate windowed placement.

use std::mem::size_of;

use windows_sys::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

use super::*;

pub const FULLSCREEN_STYLE: u32 = WS_POPUP | WS_CLIPCHILDREN | WS_SYSMENU;
const WINDOWED_STYLE: u32 = WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN;

pub struct DpiContext(DPI_AWARENESS_CONTEXT);

impl DpiContext {
    pub unsafe fn enter() -> Self {
        Self(SetThreadDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ))
    }
}

impl Drop for DpiContext {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this guard lives on the GUI thread until its HWNDs are destroyed.
            unsafe { SetThreadDpiAwarenessContext(self.0) };
        }
    }
}

pub struct Mode {
    fullscreen: Cell<bool>,
    windowed: Cell<Option<WINDOWPLACEMENT>>,
}

impl Mode {
    pub fn fullscreen() -> Self {
        Self {
            fullscreen: Cell::new(true),
            windowed: Cell::new(None),
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen.get()
    }

    pub unsafe fn toggle(&self, hwnd: HWND) -> Result<()> {
        let fullscreen = self.is_fullscreen();
        let display = monitor(hwnd)?;
        if !fullscreen {
            let mut placement = WINDOWPLACEMENT {
                length: size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            if GetWindowPlacement(hwnd, &mut placement) == 0 {
                return Err(os_error("Save windowed placement"));
            }
            self.windowed.set(Some(placement));
        }

        let (style, previous_style) = if fullscreen {
            (WINDOWED_STYLE, FULLSCREEN_STYLE)
        } else {
            (FULLSCREEN_STYLE, WINDOWED_STYLE)
        };
        set_style(hwnd, style)?;
        self.fullscreen.set(!fullscreen);

        let result = if fullscreen {
            let restored = self.windowed.get().is_some_and(|placement| {
                // WINDOWPLACEMENT stores workspace coordinates. Let Win32 map
                // the saved placement and adjust it if its monitor was removed.
                SetWindowPlacement(hwnd, &placement) != 0
            });
            if restored {
                frame_change(hwnd)
            } else {
                position(hwnd, windowed_bounds(display.rcWork), SWP_FRAMECHANGED)
            }
        } else {
            position(hwnd, display.rcMonitor, SWP_FRAMECHANGED)
        };

        if result.is_err() {
            self.fullscreen.set(fullscreen);
            let _ = set_style(hwnd, previous_style);
            let _ = frame_change(hwnd);
        }

        result
    }

    pub unsafe fn reflow(&self, hwnd: HWND, suggested: Option<RECT>) -> Result<()> {
        if self.is_fullscreen() {
            position(hwnd, monitor(hwnd)?.rcMonitor, 0)
        } else if let Some(bounds) = suggested {
            position(hwnd, bounds, 0)
        } else {
            Ok(())
        }
    }
}

pub unsafe fn monitor(hwnd: HWND) -> Result<MONITORINFO> {
    let monitor = if hwnd.is_null() {
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor);
        MonitorFromPoint(cursor, MONITOR_DEFAULTTOPRIMARY)
    } else {
        MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)
    };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    if GetMonitorInfoW(monitor, &mut info) == 0 {
        return Err(os_error("Read monitor geometry"));
    }

    Ok(info)
}

fn windowed_bounds(work: RECT) -> RECT {
    let width = (work.right - work.left).min(1200);
    let height = (work.bottom - work.top).min(760);
    let left = work.left + (work.right - work.left - width) / 2;
    let top = work.top + (work.bottom - work.top - height) / 2;

    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

unsafe fn set_style(hwnd: HWND, style: u32) -> Result<()> {
    SetLastError(0);
    if SetWindowLongPtrW(hwnd, GWL_STYLE, (style | WS_VISIBLE) as isize) == 0 && GetLastError() != 0
    {
        return Err(os_error("Set window frame style"));
    }

    Ok(())
}

unsafe fn position(hwnd: HWND, bounds: RECT, flags: u32) -> Result<()> {
    if SetWindowPos(
        hwnd,
        null_mut(),
        bounds.left,
        bounds.top,
        bounds.right - bounds.left,
        bounds.bottom - bounds.top,
        flags | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
    ) == 0
    {
        return Err(os_error("Place recovery window"));
    }

    Ok(())
}

unsafe fn frame_change(hwnd: HWND) -> Result<()> {
    position(
        hwnd,
        RECT::default(),
        SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE,
    )
}
