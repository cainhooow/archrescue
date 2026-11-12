//! An opaque software frame and its reusable top-down GDI presentation surface.

use std::{mem::size_of, ptr::null_mut, slice};

use tiny_skia::{IntSize, Pixmap, PremultipliedColorU8};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*};

use super::{text, Scene};

#[derive(Default)]
pub struct Frame {
    buffer: Option<Buffer>,
    labels: Vec<text::Label>,
}

impl Frame {
    pub unsafe fn reserve(&mut self, target: HDC, width: i32, height: i32) -> bool {
        if width <= 0 || height <= 0 {
            return false;
        }

        let (width, height) = match &self.buffer {
            Some(buffer) if buffer.width >= width && buffer.height >= height => return true,
            Some(buffer) => (width.max(buffer.width), height.max(buffer.height)),
            None => (width, height),
        };

        // Round capacity to reduce reallocations during interactive sizing.
        let Some(width) = width.checked_add(63).map(|value| value & !63) else {
            return false;
        };
        let Some(height) = height.checked_add(31).map(|value| value & !31) else {
            return false;
        };
        let Some(buffer) = Buffer::new(target, width, height) else {
            return false;
        };

        self.buffer = Some(buffer);
        true
    }

    pub unsafe fn draw(&mut self, width: i32, height: i32, paint: impl FnOnce(&mut Scene<'_>)) {
        let Some(buffer) = &mut self.buffer else {
            return;
        };

        self.labels.clear();
        paint(&mut Scene {
            pixels: &mut buffer.pixels,
            labels: &mut self.labels,
        });

        // GDI can batch writes to the selected bitmap. Finish them before
        // the CPU accesses its storage again.
        GdiFlush();
        let stride = buffer.width as usize;

        // SAFETY: CreateDIBSection allocated height rows of four bytes per
        // pixel. The bitmap stays selected and owned by this GUI thread.
        let dib = slice::from_raw_parts_mut(
            buffer.bits.cast::<[u8; 4]>(),
            stride * buffer.height as usize,
        );
        let rows = buffer.pixels.pixels().chunks_exact(stride);
        for (source, destination) in rows.zip(dib.chunks_exact_mut(stride)).take(height as usize) {
            opaque_rgba_to_bgrx(
                &source[..width as usize],
                &mut destination[..width as usize],
            );
        }

        for label in self.labels.drain(..) {
            text::draw(
                buffer.dc,
                label.font,
                label.bounds,
                &label.value,
                label.color,
                label.flags,
            );
        }
    }

    pub unsafe fn present(&self, target: HDC, bounds: RECT, source_x: i32, source_y: i32) -> bool {
        let Some(buffer) = &self.buffer else {
            return false;
        };

        let width = bounds.right - bounds.left;
        let height = bounds.bottom - bounds.top;

        if BitBlt(
            target,
            bounds.left,
            bounds.top,
            width,
            height,
            buffer.dc,
            source_x,
            source_y,
            SRCCOPY,
        ) != 0
        {
            return true;
        }

        // BitBlt requires DCs on the same device. Present the whole top-down
        // DIB through the damaged rectangle if the target device changed.
        let saved = SaveDC(target);
        if saved == 0 {
            return false;
        }

        IntersectClipRect(target, bounds.left, bounds.top, bounds.right, bounds.bottom);
        GdiFlush();
        let copied = StretchDIBits(
            target,
            bounds.left - source_x,
            bounds.top - source_y,
            buffer.width,
            buffer.height,
            0,
            0,
            buffer.width,
            buffer.height,
            buffer.bits.cast(),
            &buffer.info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        RestoreDC(target, saved);
        copied > 0
    }
}

struct Buffer {
    width: i32,
    height: i32,
    pixels: Pixmap,
    info: BITMAPINFO,
    bits: *mut u8,
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}

impl Buffer {
    unsafe fn new(target: HDC, width: i32, height: i32) -> Option<Self> {
        let size = IntSize::from_wh(width as u32, height as u32)?;
        let bytes = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(bytes).ok()?;
        rgba.resize(bytes, 0);
        let pixels = Pixmap::from_vec(rgba, size)?;

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let dc = CreateCompatibleDC(target);
        if dc.is_null() {
            return None;
        }

        let mut bits = null_mut();
        let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if bitmap.is_null() || bits.is_null() {
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            DeleteDC(dc);
            return None;
        }

        let previous = SelectObject(dc, bitmap);
        if previous.is_null() || previous as isize == -1 {
            DeleteObject(bitmap);
            DeleteDC(dc);
            return None;
        }

        Some(Self {
            width,
            height,
            pixels,
            info,
            bits: bits.cast(),
            dc,
            bitmap,
            previous,
        })
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: the DC and DIB belong to this thread. Deselect the DIB
        // before deleting it. The original bitmap is owned by the DC.
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

fn opaque_rgba_to_bgrx(rgba: &[PremultipliedColorU8], bgrx: &mut [[u8; 4]]) {
    for (rgba, bgrx) in rgba.iter().zip(bgrx) {
        // tiny-skia has already composited coverage and alpha against the
        // opaque background. BI_RGB expects B, G, R and an unused high byte.
        debug_assert_eq!(rgba.alpha(), 255);
        *bgrx = [rgba.blue(), rgba.green(), rgba.red(), 0];
    }
}

#[cfg(test)]
mod tests {
    use tiny_skia::{Color, Paint, Rect, Transform};

    use super::*;

    #[test]
    fn dib_rows_preserve_rgb_and_composited_alpha() {
        let mut pixels = Pixmap::new(4, 1).unwrap();
        pixels.fill(Color::from_rgba8(0, 0, 255, 255));
        for (x, color) in [
            (0.0, Color::from_rgba8(255, 0, 0, 255)),
            (1.0, Color::from_rgba8(0, 255, 0, 255)),
            (3.0, Color::from_rgba8(255, 0, 0, 128)),
        ] {
            let mut paint = Paint::default();
            paint.set_color(color);
            pixels.fill_rect(
                Rect::from_xywh(x, 0.0, 1.0, 1.0).unwrap(),
                &paint,
                Transform::identity(),
                None,
            );
        }

        let mut dib = [[0; 4]; 4];
        opaque_rgba_to_bgrx(pixels.pixels(), &mut dib);

        assert_eq!(
            dib,
            [
                [0, 0, 255, 0],
                [0, 255, 0, 0],
                [255, 0, 0, 0],
                [127, 0, 128, 0]
            ]
        );
    }

    #[test]
    fn software_frames_reuse_gdi_storage_at_smaller_sizes() {
        // SAFETY: this test creates and inspects its own DIBs on one thread.
        unsafe {
            for (width, height) in [(800, 600), (1024, 768), (1280, 720), (1920, 1080)] {
                let mut frame = Frame::default();
                assert!(frame.reserve(null_mut(), width, height));
                let buffer = frame.buffer.as_ref().unwrap();
                let mut bitmap = BITMAP::default();
                assert_eq!(
                    GetObjectW(
                        buffer.bitmap,
                        size_of::<BITMAP>() as i32,
                        (&mut bitmap as *mut BITMAP).cast(),
                    ),
                    size_of::<BITMAP>() as i32,
                );
                assert!(bitmap.bmWidth >= width && bitmap.bmHeight >= height);
                assert_eq!(bitmap.bmBitsPixel, 32);
                let dib_bytes = bitmap.bmWidthBytes as usize * bitmap.bmHeight as usize;
                let pixmap_bytes = buffer.pixels.data().len();
                let original_bitmap = buffer.bitmap;
                println!("{width}x{height}: DIB {dib_bytes} bytes, pixmap {pixmap_bytes} bytes");

                assert!(frame.reserve(null_mut(), width - 16, height - 16));
                assert_eq!(frame.buffer.as_ref().unwrap().bitmap, original_bitmap);
            }
        }
    }
}
