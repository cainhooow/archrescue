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
    if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
        return;
    }

    let previous = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, color);
    DrawTextW(dc, value.as_ptr(), value.len() as i32, &mut bounds, flags);
    SelectObject(dc, previous);
}
