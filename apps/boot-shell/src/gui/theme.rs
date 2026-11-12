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
pub const HOVER: COLORREF = rgb(41, 34, 55);
pub const ACCENT_HOVER: COLORREF = rgb(220, 201, 255);
pub const ACCENT_PRESSED_SURFACE: COLORREF = rgb(56, 43, 79);
pub const ACCENT_PRESSED: COLORREF = rgb(161, 130, 232);
pub const ACCENT_END: COLORREF = rgb(175, 143, 249);
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
    fonts: Cell<Fonts>,
    face: Vec<u16>,
    pub panel: HBRUSH,
    pub input: HBRUSH,
}

#[derive(Clone, Copy)]
struct Fonts {
    dpi: u32,
    body: HFONT,
    heading: HFONT,
    emphasis: HFONT,
}

impl Fonts {
    unsafe fn new(face: &[u16], dpi: u32) -> Result<Self> {
        // Compact cards reserve space for a 125% text scale at the minimum viewport.
        let scale = |height| (height * dpi as i32 + 48) / 96;
        let fonts = Self {
            dpi,
            body: font(face, scale(14), 400),
            heading: font(face, scale(30), 700),
            emphasis: font(face, scale(16), 700),
        };

        if [fonts.body, fonts.heading, fonts.emphasis]
            .iter()
            .any(|handle| handle.is_null())
        {
            fonts.release();
            return Err(os_error("Create GDI fonts"));
        }

        Ok(fonts)
    }

    unsafe fn release(self) {
        for handle in [self.body, self.heading, self.emphasis] {
            if !handle.is_null() {
                DeleteObject(handle);
            }
        }
    }
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
        let fonts = Fonts::new(&face, 96)?;
        let theme = Self {
            fonts: Cell::new(fonts),
            face,
            panel: CreateSolidBrush(PANEL),
            input: CreateSolidBrush(INPUT),
        };

        if theme.panel.is_null() || theme.input.is_null() {
            return Err(os_error("Create GDI brushes"));
        }

        Ok(theme)
    }

    pub fn body(&self) -> HFONT {
        self.fonts.get().body
    }

    pub fn heading(&self) -> HFONT {
        self.fonts.get().heading
    }

    pub fn emphasis(&self) -> HFONT {
        self.fonts.get().emphasis
    }

    pub unsafe fn rescale(&self, hwnd: HWND, dpi: u32) -> Result<()> {
        let dpi = dpi.clamp(96, 120);
        if self.fonts.get().dpi == dpi {
            return Ok(());
        }

        let fonts = Fonts::new(&self.face, dpi)?;
        let previous = self.fonts.replace(fonts);

        // Children receive the new borrowed font handle synchronously before the
        // previous handle is released. The parent always owns both sets here.
        EnumChildWindows(hwnd, Some(update_font), fonts.body as LPARAM);
        previous.release();

        Ok(())
    }
}

unsafe extern "system" fn update_font(hwnd: HWND, font: LPARAM) -> windows_sys::core::BOOL {
    SendMessageW(hwnd, WM_SETFONT, font as WPARAM, 1);
    1
}

impl Drop for Theme {
    fn drop(&mut self) {
        // SAFETY: all resources are owned here and no child windows remain.
        unsafe {
            self.fonts.get().release();
            for brush in [self.panel, self.input] {
                if !brush.is_null() {
                    DeleteObject(brush);
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
        ANTIALIASED_QUALITY as u32,
        DEFAULT_PITCH as u32,
        face.as_ptr(),
    )
}
