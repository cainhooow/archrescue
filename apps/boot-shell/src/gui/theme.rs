//! WinPE-safe GDI tokens and resources, shared by every page.

use super::*;

pub const BACKGROUND: COLORREF = rgb(14, 14, 19);
pub const SIDEBAR: COLORREF = rgb(19, 19, 27);
pub const PANEL: COLORREF = rgb(22, 22, 30);
pub const INPUT: COLORREF = rgb(17, 17, 24);
pub const ELEVATED: COLORREF = rgb(32, 31, 43);
pub const SELECTED: COLORREF = rgb(43, 33, 59);
pub const TEXT: COLORREF = rgb(247, 246, 251);
pub const MUTED: COLORREF = rgb(180, 174, 194);
pub const ACCENT: COLORREF = rgb(195, 168, 255);
pub const ON_ACCENT: COLORREF = rgb(29, 21, 45);
pub const BORDER: COLORREF = rgb(51, 47, 65);
pub const SUCCESS: COLORREF = rgb(131, 232, 170);
pub const WARNING: COLORREF = rgb(255, 204, 128);
pub const CRITICAL: COLORREF = rgb(255, 160, 166);
pub const SUCCESS_SURFACE: COLORREF = rgb(23, 45, 34);
pub const WARNING_SURFACE: COLORREF = rgb(48, 36, 24);
pub const ERROR_SURFACE: COLORREF = rgb(48, 27, 36);

const fn rgb(red: u32, green: u32, blue: u32) -> COLORREF {
    red | (green << 8) | (blue << 16)
}

pub struct Theme {
    pub body: HFONT,
    pub heading: HFONT,
    pub emphasis: HFONT,
    pub panel: HBRUSH,
    pub input: HBRUSH,
}

impl Theme {
    pub unsafe fn new() -> Result<Self> {
        // WinPE can ship italic Segoe variants without the regular face.
        let face = if std::env::var_os("SystemRoot")
            .is_some_and(|root| PathBuf::from(root).join("Fonts/segoeui.ttf").is_file())
        {
            wide("Segoe UI")
        } else {
            wide("Tahoma")
        };
        let theme = Self {
            body: font(&face, 15, 400),
            heading: font(&face, 29, 600),
            emphasis: font(&face, 17, 600),
            panel: CreateSolidBrush(PANEL),
            input: CreateSolidBrush(INPUT),
        };

        if [
            theme.body,
            theme.heading,
            theme.emphasis,
            theme.panel,
            theme.input,
        ]
        .iter()
        .any(|handle| handle.is_null())
        {
            return Err(os_error("Create GDI theme"));
        }

        Ok(theme)
    }
}

impl Drop for Theme {
    fn drop(&mut self) {
        // SAFETY: all resources are owned here and no child windows remain.
        unsafe {
            for handle in [
                self.body,
                self.heading,
                self.emphasis,
                self.panel,
                self.input,
            ] {
                if !handle.is_null() {
                    DeleteObject(handle);
                }
            }
        }
    }
}

unsafe fn font(face: &[u16], height: i32, weight: i32) -> HFONT {
    // The GDI font mapper selects a system fallback if Segoe UI is unavailable.
    CreateFontW(
        -height,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        DEFAULT_QUALITY as u32,
        DEFAULT_PITCH as u32,
        face.as_ptr(),
    )
}
