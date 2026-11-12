//! CPU geometry. GDI draws queued text after geometry has been composited.

mod framebuffer;
mod text;

use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, Path, PathBuilder,
    Point, Rect, SpreadMode, Stroke, Transform,
};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*};

pub use framebuffer::Frame;

pub struct Scene<'a> {
    pixels: &'a mut tiny_skia::Pixmap,
    labels: &'a mut Vec<text::Label>,
}

impl Scene<'_> {
    pub fn clear(&mut self, color: COLORREF) {
        self.pixels.fill(rgb(color));
    }

    pub fn fill(&mut self, bounds: RECT, color: COLORREF) {
        let Some(bounds) = rectangle(bounds) else {
            return;
        };

        self.pixels
            .fill_rect(bounds, &solid(color), Transform::identity(), None);
    }

    pub fn surface(&mut self, bounds: RECT, color: COLORREF, border: COLORREF, radius: i32) {
        self.gradient(bounds, color, color, border, radius);
    }

    pub fn gradient(
        &mut self,
        bounds: RECT,
        start: COLORREF,
        end: COLORREF,
        border: COLORREF,
        radius: i32,
    ) {
        let Some(bounds) = rectangle(bounds) else {
            return;
        };

        let radius = radius as f32;
        let path = rounded(bounds, radius);
        self.pixels.fill_path(
            &path,
            &solid(border),
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        let Some(inner) = Rect::from_ltrb(
            bounds.left() + 1.0,
            bounds.top() + 1.0,
            bounds.right() - 1.0,
            bounds.bottom() - 1.0,
        )
        .filter(|inner| inner.width() > 0.0 && inner.height() > 0.0) else {
            return;
        };

        let mut paint = solid(start);
        if start != end {
            if let Some(shader) = LinearGradient::new(
                Point::from_xy(inner.left(), inner.top()),
                Point::from_xy(inner.right(), inner.bottom()),
                vec![
                    GradientStop::new(0.0, rgb(start)),
                    GradientStop::new(1.0, rgb(end)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            ) {
                paint.shader = shader;
            }
        }

        self.pixels.fill_path(
            &rounded(inner, (radius - 1.0).max(0.0)),
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    pub fn focus_outline(&mut self, bounds: RECT, color: COLORREF) {
        let Some(bounds) = rectangle(bounds) else {
            return;
        };

        let inset = Rect::from_ltrb(
            bounds.left() + 0.75,
            bounds.top() + 0.75,
            bounds.right() - 0.75,
            bounds.bottom() - 0.75,
        );
        if let Some(inset) = inset.filter(|bounds| bounds.width() > 0.0 && bounds.height() > 0.0) {
            self.stroke(&rounded(inset, 5.0), color, 1.5);
        }
    }

    pub fn stroke(&mut self, path: &Path, color: COLORREF, width: f32) {
        let stroke = Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        self.pixels
            .stroke_path(path, &solid(color), &stroke, Transform::identity(), None);
    }

    pub fn text(&mut self, font: HFONT, bounds: RECT, value: &str, color: COLORREF, flags: u32) {
        self.labels.push(text::Label {
            font,
            bounds,
            value: value.encode_utf16().collect(),
            color,
            flags,
        });
    }
}

fn rgb(color: COLORREF) -> Color {
    Color::from_rgba8(color as u8, (color >> 8) as u8, (color >> 16) as u8, 255)
}

fn solid(color: COLORREF) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(rgb(color));
    paint
}

fn rectangle(bounds: RECT) -> Option<Rect> {
    Rect::from_ltrb(
        bounds.left as f32,
        bounds.top as f32,
        bounds.right as f32,
        bounds.bottom as f32,
    )
    .filter(|bounds| bounds.width() > 0.0 && bounds.height() > 0.0)
}

fn rounded(bounds: Rect, radius: f32) -> Path {
    let radius = radius.min(bounds.width().min(bounds.height()) * 0.5);
    let left = bounds.left();
    let right = bounds.right();
    let top = bounds.top();
    let bottom = bounds.bottom();

    let mut path = PathBuilder::new();
    path.move_to(left + radius, top);
    path.line_to(right - radius, top);
    path.quad_to(right, top, right, top + radius);
    path.line_to(right, bottom - radius);
    path.quad_to(right, bottom, right - radius, bottom);
    path.line_to(left + radius, bottom);
    path.quad_to(left, bottom, left, bottom - radius);
    path.line_to(left, top + radius);
    path.quad_to(left, top, left + radius, top);
    path.close();

    path.finish().expect("a nonempty rounded rectangle")
}
