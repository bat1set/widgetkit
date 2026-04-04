use crate::{Stroke, TextStyle};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use widgetkit_core::{Color, Point, Rect, Size};

/// Internal unstable raw render foundation for v0.1.
/// This module backs `Canvas` but is intentionally not re-exported by the top-level crate.
#[derive(Debug)]
pub(crate) struct Scene {
    pub size: Size,
    pub commands: Vec<Command>,
}

#[derive(Clone, Debug)]
pub(crate) enum Command {
    Clear { color: Color },
    Rect { rect: Rect, color: Color },
    RoundRect { rect: Rect, radius: f32, color: Color },
    Line {
        start: Point,
        end: Point,
        stroke: Stroke,
        color: Color,
    },
    Circle { center: Point, radius: f32, color: Color },
    Text {
        position: Point,
        text: String,
        style: TextStyle,
        color: Color,
    },
    ImagePlaceholder { rect: Rect, color: Color },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClipState {
    rect: Option<Rect>,
}

impl ClipState {
    pub(crate) const fn none() -> Self {
        Self { rect: None }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TransformState {
    pub depth: usize,
}

pub(crate) struct Frame<'a> {
    width: u32,
    height: u32,
    pixels: &'a mut [Color],
    clip: ClipState,
    transforms: TransformState,
}

impl<'a> Frame<'a> {
    pub(crate) fn new(width: u32, height: u32, pixels: &'a mut [Color]) -> Self {
        Self {
            width,
            height,
            pixels,
            clip: ClipState::none(),
            transforms: TransformState::default(),
        }
    }

    pub(crate) fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    fn pixels_mut(&mut self) -> &mut [Color] {
        self.pixels
    }

    fn clip(&self) -> ClipState {
        self.clip
    }

    fn transforms(&self) -> TransformState {
        self.transforms
    }
}

pub(crate) struct Rasterizer<'a> {
    frame: Frame<'a>,
}

impl<'a> Rasterizer<'a> {
    pub(crate) fn new(frame: Frame<'a>) -> Self {
        Self { frame }
    }

    pub(crate) fn execute(&mut self, scene: Scene) {
        let _ = scene.size;
        let _ = self.frame.size();
        let _ = self.frame.clip();
        let _ = self.frame.transforms();
        // TODO(v0.2): wire Canvas clip commands into ClipState
        // TODO(v0.2): apply TransformState once Canvas exposes transforms
        // TODO(v0.8): add debug paint/invalidation instrumentation
        for command in scene.commands {
            match command {
                Command::Clear { color } => self.clear(color),
                Command::Rect { rect, color } => self.fill_rect(rect, color),
                Command::RoundRect { rect, radius, color } => self.fill_round_rect(rect, radius, color),
                Command::Line {
                    start,
                    end,
                    stroke,
                    color,
                } => self.draw_line(start, end, stroke.width.max(1.0), color),
                Command::Circle {
                    center,
                    radius,
                    color,
                } => self.fill_circle(center, radius, color),
                Command::Text {
                    position,
                    text,
                    style,
                    color,
                } => self.draw_text(position, &text, style.pixel_size(), color),
                Command::ImagePlaceholder { rect, color } => self.draw_image_placeholder(rect, color),
            }
        }
    }

    fn clear(&mut self, color: Color) {
        self.frame.pixels_mut().fill(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.for_each_pixel(rect, |_, _, pixel| *pixel = blend(*pixel, color));
    }

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let radius = radius.min(rect.width() * 0.5).min(rect.height() * 0.5).max(0.0);
        self.for_each_pixel(rect, |x, y, pixel| {
            let xf = x as f32 + 0.5;
            let yf = y as f32 + 0.5;
            if point_in_round_rect(xf, yf, rect, radius) {
                *pixel = blend(*pixel, color);
            }
        });
    }

    fn fill_circle(&mut self, center: Point, radius: f32, color: Color) {
        let bounds = Rect::xywh(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );
        let radius_sq = radius * radius;
        self.for_each_pixel(bounds, |x, y, pixel| {
            let dx = x as f32 + 0.5 - center.x;
            let dy = y as f32 + 0.5 - center.y;
            if dx * dx + dy * dy <= radius_sq {
                *pixel = blend(*pixel, color);
            }
        });
    }

    fn draw_line(&mut self, start: Point, end: Point, width: f32, color: Color) {
        let half_width = width * 0.5;
        let min_x = start.x.min(end.x) - half_width;
        let min_y = start.y.min(end.y) - half_width;
        let max_x = start.x.max(end.x) + half_width;
        let max_y = start.y.max(end.y) + half_width;
        let bounds = Rect::xywh(min_x, min_y, max_x - min_x, max_y - min_y);
        self.for_each_pixel(bounds, |x, y, pixel| {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            if distance_to_segment(px, py, start, end) <= half_width {
                *pixel = blend(*pixel, color);
            }
        });
    }

    fn draw_text(&mut self, position: Point, text: &str, size: f32, color: Color) {
        // TODO(v0.2): replace bitmap font rendering with a real text layout/rasterization path.
        let scale = (size / 8.0).round().max(1.0) as i32;
        let glyph_advance = 8 * scale;
        let mut cursor_x = position.x.round() as i32;
        let cursor_y = position.y.round() as i32;
        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = position.x.round() as i32;
                continue;
            }
            if let Some(glyph) = BASIC_FONTS.get(ch) {
                for (row, bits) in glyph.iter().copied().enumerate() {
                    for col in 0..8 {
                        if (bits >> col) & 1 == 1 {
                            let x = cursor_x + col * scale;
                            let y = cursor_y + row as i32 * scale;
                            let rect = Rect::xywh(x as f32, y as f32, scale as f32, scale as f32);
                            self.fill_rect(rect, color);
                        }
                    }
                }
            }
            cursor_x += glyph_advance;
        }
    }

    fn draw_image_placeholder(&mut self, rect: Rect, color: Color) {
        self.draw_line(Point::new(rect.x(), rect.y()), Point::new(rect.right(), rect.y()), 1.0, color);
        self.draw_line(Point::new(rect.right(), rect.y()), Point::new(rect.right(), rect.bottom()), 1.0, color);
        self.draw_line(Point::new(rect.right(), rect.bottom()), Point::new(rect.x(), rect.bottom()), 1.0, color);
        self.draw_line(Point::new(rect.x(), rect.bottom()), Point::new(rect.x(), rect.y()), 1.0, color);
        self.draw_line(Point::new(rect.x(), rect.y()), Point::new(rect.right(), rect.bottom()), 1.0, color);
        self.draw_line(Point::new(rect.right(), rect.y()), Point::new(rect.x(), rect.bottom()), 1.0, color);
    }

    fn for_each_pixel(&mut self, rect: Rect, mut draw: impl FnMut(i32, i32, &mut Color)) {
        let min_x = rect.x().floor().max(0.0) as i32;
        let min_y = rect.y().floor().max(0.0) as i32;
        let max_x = rect.right().ceil().min(self.frame.width as f32) as i32;
        let max_y = rect.bottom().ceil().min(self.frame.height as f32) as i32;
        for y in min_y..max_y {
            for x in min_x..max_x {
                let index = y as usize * self.frame.width as usize + x as usize;
                draw(x, y, &mut self.frame.pixels_mut()[index]);
            }
        }
    }
}

fn blend(dst: Color, src: Color) -> Color {
    if src.a == 255 {
        return src;
    }
    if src.a == 0 {
        return dst;
    }

    let sa = src.a as f32 / 255.0;
    let da = dst.a as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= f32::EPSILON {
        return Color::TRANSPARENT;
    }

    let blend_channel = |src_channel: u8, dst_channel: u8| -> u8 {
        let src_value = src_channel as f32 / 255.0;
        let dst_value = dst_channel as f32 / 255.0;
        let out = (src_value * sa + dst_value * da * (1.0 - sa)) / out_a;
        (out * 255.0).round().clamp(0.0, 255.0) as u8
    };

    Color::rgba(
        blend_channel(src.r, dst.r),
        blend_channel(src.g, dst.g),
        blend_channel(src.b, dst.b),
        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn point_in_round_rect(x: f32, y: f32, rect: Rect, radius: f32) -> bool {
    if x < rect.x() || x > rect.right() || y < rect.y() || y > rect.bottom() {
        return false;
    }
    if radius <= 0.0 {
        return true;
    }

    let left = rect.x() + radius;
    let right = rect.right() - radius;
    let top = rect.y() + radius;
    let bottom = rect.bottom() - radius;

    if (x >= left && x <= right) || (y >= top && y <= bottom) {
        return true;
    }

    let corner_x = if x < left { left } else { right };
    let corner_y = if y < top { top } else { bottom };
    let dx = x - corner_x;
    let dy = y - corner_y;
    dx * dx + dy * dy <= radius * radius
}

fn distance_to_segment(px: f32, py: f32, start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if dx.abs() <= f32::EPSILON && dy.abs() <= f32::EPSILON {
        return ((px - start.x).powi(2) + (py - start.y).powi(2)).sqrt();
    }
    let t = (((px - start.x) * dx + (py - start.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    let projection_x = start.x + t * dx;
    let projection_y = start.y + t * dy;
    ((px - projection_x).powi(2) + (py - projection_y).powi(2)).sqrt()
}
