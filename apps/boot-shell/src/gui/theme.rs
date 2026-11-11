//! WinPE-safe GDI tokens and resources, shared by every page.

use super::*;

pub const BACKGROUND: COLORREF = rgb(11, 15, 23);
pub const SIDEBAR: COLORREF = rgb(16, 23, 34);
pub const PANEL: COLORREF = rgb(21, 30, 43);
pub const ELEVATED: COLORREF = rgb(27, 38, 54);
pub const TEXT: COLORREF = rgb(241, 245, 249);
pub const MUTED: COLORREF = rgb(160, 177, 198);
pub const ACCENT: COLORREF = rgb(128, 175, 255);
pub const PRIMARY: COLORREF = rgb(38, 83, 157);
pub const BORDER: COLORREF = rgb(45, 60, 80);
pub const SUCCESS: COLORREF = rgb(107, 222, 151);
pub const WARNING: COLORREF = rgb(255, 196, 107);
pub const CRITICAL: COLORREF = rgb(255, 145, 145);

const fn rgb(red: u32, green: u32, blue: u32) -> COLORREF {
    red | (green << 8) | (blue << 16)
}

pub struct Theme {
    pub body: HFONT,
    pub heading: HFONT,
    pub emphasis: HFONT,
    pub panel: HBRUSH,
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
        };

        if [theme.body, theme.heading, theme.emphasis, theme.panel]
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
        // SAFETY: all five resources are owned here and no child windows remain.
        unsafe {
            for handle in [self.body, self.heading, self.emphasis, self.panel] {
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
