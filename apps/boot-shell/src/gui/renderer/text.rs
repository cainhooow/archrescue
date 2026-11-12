//! UTF-16 GDI text over the already composited software surfaces.

use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*};

pub(super) struct Label {
    pub font: HFONT,
    pub bounds: RECT,
    pub value: Vec<u16>,
    pub color: COLORREF,
    pub flags: u32,
}

pub(super) unsafe fn draw(
    dc: HDC,
    font: HFONT,
    mut bounds: RECT,
    value: &[u16],
    color: COLORREF,
    flags: u32,
) {
    // DrawTextW can inspect the first code unit even with a count of zero.
    // An empty Rust slice has no readable code unit at its pointer.
    if value.is_empty() || bounds.right <= bounds.left || bounds.bottom <= bounds.top {
        return;
    }

    let previous = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, color);
    DrawTextW(dc, value.as_ptr(), value.len() as i32, &mut bounds, flags);
    SelectObject(dc, previous);
}

#[cfg(test)]
mod tests {
    use std::ptr::null_mut;

    use super::*;

    #[test]
    fn empty_label_is_safe_to_draw() {
        // SAFETY: the DC is owned by this test and the stock font is borrowed.
        unsafe {
            let dc = CreateCompatibleDC(null_mut());
            assert!(!dc.is_null());
            let font = GetStockObject(DEFAULT_GUI_FONT);
            let bounds = RECT {
                left: 0,
                top: 0,
                right: 100,
                bottom: 30,
            };

            draw(dc, font, bounds, &[], 0, DT_SINGLELINE);

            assert_ne!(DeleteDC(dc), 0);
        }
    }
}
