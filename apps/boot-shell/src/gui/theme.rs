//! Reference palette and process-private fonts shared by every page.

use super::*;

pub const BACKGROUND: COLORREF = rgb(14, 14, 19);
pub const SIDEBAR: COLORREF = rgb(18, 18, 25);
pub const SHELL_LINE: COLORREF = rgb(43, 41, 54);
pub const FOOTER: COLORREF = rgb(21, 20, 29);
pub const CANDIDATE_START: COLORREF = rgb(42, 32, 60);
pub const CANDIDATE_BORDER: COLORREF = rgb(73, 58, 96);
pub const BADGE: COLORREF = rgb(48, 36, 66);
pub const PARTITION_DATA: COLORREF = rgb(113, 85, 160);
pub const PARTITION_EFI: COLORREF = rgb(192, 150, 247);
pub const PARTITION_RESERVED: COLORREF = rgb(78, 71, 91);
pub const PARTITION_RECOVERY: COLORREF = rgb(46, 92, 77);
pub const MODAL: COLORREF = rgb(25, 24, 35);
pub const MODAL_BORDER: COLORREF = rgb(100, 85, 120);
pub const METRIC: COLORREF = rgb(24, 24, 33);
pub const SELECTED_BORDER: COLORREF = rgb(73, 52, 95);
pub const NAV_TEXT: COLORREF = rgb(238, 228, 255);
pub const SCROLL_HOVER: COLORREF = rgb(135, 110, 171);
pub const SCROLL_THUMB: COLORREF = rgb(94, 79, 115);
pub const DISABLED: COLORREF = rgb(137, 133, 153);
pub const PANEL: COLORREF = rgb(22, 22, 30);
pub const INPUT: COLORREF = rgb(17, 17, 24);
pub const ELEVATED: COLORREF = rgb(34, 33, 44);
pub const SELECTED: COLORREF = rgb(42, 32, 59);
pub const TEXT: COLORREF = rgb(247, 246, 251);
pub const MUTED: COLORREF = rgb(180, 176, 194);
pub const HOVER: COLORREF = rgb(41, 34, 55);
pub const ACCENT_HOVER: COLORREF = rgb(220, 201, 255);
pub const ACCENT_PRESSED_SURFACE: COLORREF = rgb(56, 43, 79);
pub const ACCENT_PRESSED: COLORREF = rgb(161, 130, 232);
pub const ACCENT_END: COLORREF = rgb(175, 143, 249);
pub const ACCENT: COLORREF = rgb(196, 170, 255);
pub const ON_ACCENT: COLORREF = rgb(29, 21, 45);
pub const BORDER: COLORREF = rgb(45, 44, 57);
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
    faces: FontFaces,
    pub panel: HBRUSH,
    pub input: HBRUSH,
    pub footer: HBRUSH,
    pub context: &'static str,
}

#[derive(Clone, Copy)]
struct Fonts {
    dpi: u32,
    body: HFONT,
    heading: HFONT,
    emphasis: HFONT,
    mono: HFONT,
    brand: HFONT,
}

impl Fonts {
    unsafe fn new(faces: &FontFaces, dpi: u32) -> Result<Self> {
        let scale = |height| (height * dpi as i32 + 48) / 96;
        let fonts = Self {
            dpi,
            body: font(&faces.body, scale(15).max(12), 400),
            heading: font(&faces.body, scale(38), 700),
            emphasis: font(&faces.emphasis, scale(20), faces.emphasis_weight),
            mono: font(&faces.mono, scale(13).max(11), 400),
            brand: font(&faces.body, scale(24), 700),
        };

        if [
            fonts.body,
            fonts.heading,
            fonts.emphasis,
            fonts.mono,
            fonts.brand,
        ]
        .iter()
        .any(|handle| handle.is_null())
        {
            fonts.release();
            return Err(os_error("Create GDI fonts"));
        }

        Ok(fonts)
    }

    unsafe fn release(self) {
        for handle in [
            self.body,
            self.heading,
            self.emphasis,
            self.mono,
            self.brand,
        ] {
            if !handle.is_null() {
                DeleteObject(handle);
            }
        }
    }
}

impl Theme {
    pub unsafe fn new() -> Result<Self> {
        let faces = FontFaces::load();
        let fonts = Fonts::new(&faces, 96)?;
        let context = if PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_default())
            .starts_with(r"X:\")
        {
            "WinPE x64"
        } else {
            "Windows x64"
        };
        let theme = Self {
            fonts: Cell::new(fonts),
            faces,
            panel: CreateSolidBrush(PANEL),
            input: CreateSolidBrush(INPUT),
            footer: CreateSolidBrush(FOOTER),
            context,
        };

        if theme.panel.is_null() || theme.input.is_null() || theme.footer.is_null() {
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

    pub fn brand(&self) -> HFONT {
        self.fonts.get().brand
    }

    pub fn mono(&self) -> HFONT {
        self.fonts.get().mono
    }

    pub unsafe fn rescale(&self, hwnd: HWND, dpi: u32, layout: &layout::Layout) -> Result<()> {
        let dpi = layout.px(dpi.clamp(96, 120) as i32) as u32;
        if self.fonts.get().dpi == dpi {
            return Ok(());
        }

        let fonts = Fonts::new(&self.faces, dpi)?;
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
            for brush in [self.panel, self.input, self.footer] {
                if !brush.is_null() {
                    DeleteObject(brush);
                }
            }
        }
    }
}

struct FontFaces {
    body: Vec<u16>,
    emphasis: Vec<u16>,
    emphasis_weight: i32,
    mono: Vec<u16>,
    resources: Vec<HANDLE>,
}

impl FontFaces {
    unsafe fn load() -> Self {
        const GEIST: [&[u8]; 3] = [
            include_bytes!("../../assets/fonts/Geist-Regular.ttf"),
            include_bytes!("../../assets/fonts/Geist-SemiBold.ttf"),
            include_bytes!("../../assets/fonts/Geist-Bold.ttf"),
        ];
        const PLEX: &[u8] = include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf");

        let mut resources = Vec::with_capacity(4);
        let geist_available = GEIST.into_iter().all(|font| register(font, &mut resources));
        let mono_available = register(PLEX, &mut resources);
        let fallback = if std::env::var_os("SystemRoot")
            .is_some_and(|root| PathBuf::from(root).join("Fonts/segoeui.ttf").is_file())
        {
            "Segoe UI"
        } else {
            "Tahoma"
        };

        Self {
            body: wide(if geist_available { "Geist" } else { fallback }),
            emphasis: wide(if geist_available {
                "Geist SemiBold"
            } else {
                fallback
            }),
            emphasis_weight: if geist_available { 600 } else { 700 },
            mono: wide(if mono_available {
                "IBM Plex Mono"
            } else {
                "Consolas"
            }),
            resources,
        }
    }
}

impl Drop for FontFaces {
    fn drop(&mut self) {
        // SAFETY: Theme deletes the HFONTs before these private registrations are released.
        for resource in self.resources.drain(..) {
            unsafe { RemoveFontMemResourceEx(resource) };
        }
    }
}

unsafe fn register(bytes: &[u8], resources: &mut Vec<HANDLE>) -> bool {
    let mut count = 0;

    // GDI copies the bounded font data. The returned handle owns the registration.
    let handle = AddFontMemResourceEx(
        bytes.as_ptr().cast(),
        bytes.len() as u32,
        null(),
        std::ptr::addr_of_mut!(count),
    );
    if !handle.is_null() {
        resources.push(handle);
    }

    count != 0 && !handle.is_null()
}

unsafe fn font(face: &[u16], height: i32, weight: i32) -> HFONT {
    // Use grayscale coverage so the DIB is independent of display subpixel order.
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
