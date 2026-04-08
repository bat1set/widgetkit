use crate::frame::{Frame, intersect_rect};
use crate::model::{
    ClearCommand, ClipCommand, ClipPrimitive, FillCommand, FillShape, ImageCommand, ImageSource,
    RenderCommand, RenderFrame, StateCommand, StrokeCommand, StrokeShape, TextCommand,
    TransformCommand,
};
use crate::text::TextLayout;
use font8x8::{BASIC_FONTS, UnicodeFonts};
use widgetkit_core::{Color, Point, Rect};

pub(crate) struct Rasterizer<'a> {
    frame: Frame<'a>,
}

impl<'a> Rasterizer<'a> {
    pub(crate) fn new(frame: Frame<'a>) -> Self {
        Self { frame }
    }

    pub(crate) fn execute(&mut self, frame: RenderFrame) {
        let _ = frame.size();
        let _ = self.frame.size();
        let (_, commands) = frame.into_parts();
        // TODO(v0.8): add debug paint/invalidation instrumentation
        for command in commands {
            match command {
                RenderCommand::Clear(command) => self.clear(command),
                RenderCommand::Fill(command) => self.fill(command),
                RenderCommand::Stroke(command) => self.stroke(command),
                RenderCommand::Text(command) => self.text(command),
                RenderCommand::Image(command) => self.image(command),
                RenderCommand::Clip(command) => self.clip(command),
                RenderCommand::Transform(command) => self.transform(command),
                RenderCommand::State(command) => self.state(command),
            }
        }
    }

    fn clear(&mut self, command: ClearCommand) {
        self.frame.pixels_mut().fill(command.paint.color);
    }

    fn fill(&mut self, command: FillCommand) {
        match command.shape {
            FillShape::Rect(rect) => self.fill_rect(rect, command.fill.paint.color),
            FillShape::RoundRect { rect, radius } => {
                self.fill_round_rect(rect, radius, command.fill.paint.color)
            }
            FillShape::Circle { center, radius } => {
                self.fill_circle(center, radius, command.fill.paint.color)
            }
            FillShape::Ellipse {
                center,
                radius_x,
                radius_y,
            } => self.fill_ellipse(center, radius_x, radius_y, command.fill.paint.color),
        }
    }

    fn stroke(&mut self, command: StrokeCommand) {
        match command.shape {
            StrokeShape::Line { start, end } => self.draw_line(
                start,
                end,
                command.stroke.width.max(1.0),
                command.paint.color,
            ),
        }
    }

    fn text(&mut self, command: TextCommand) {
        self.draw_text(
            command.position,
            &command.text,
            &command.style,
            command.paint.color,
        );
    }

    fn image(&mut self, command: ImageCommand) {
        match command.source {
            ImageSource::Placeholder => {
                self.draw_image_placeholder(command.rect, command.paint.color)
            }
        }
    }

    fn clip(&mut self, command: ClipCommand) {
        match command.primitive {
            ClipPrimitive::Rect(rect) => self.frame.apply_clip_rect(rect),
        }
    }

    fn transform(&mut self, command: TransformCommand) {
        self.frame.apply_transform(command.transform);
    }

    fn state(&mut self, command: StateCommand) {
        match command {
            StateCommand::Save => self.frame.save_state(),
            StateCommand::Restore => self.frame.restore_state(),
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let rect = self.frame.transform().map_rect(rect);
        self.fill_rect_mapped(rect, color);
    }

    fn fill_rect_mapped(&mut self, rect: Rect, color: Color) {
        self.for_each_pixel(rect, |_, _, pixel| *pixel = blend(*pixel, color));
    }

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let rect = self.frame.transform().map_rect(rect);
        let radius = radius
            .min(rect.width() * 0.5)
            .min(rect.height() * 0.5)
            .max(0.0);
        self.for_each_pixel(rect, |x, y, pixel| {
            let xf = x as f32 + 0.5;
            let yf = y as f32 + 0.5;
            if point_in_round_rect(xf, yf, rect, radius) {
                *pixel = blend(*pixel, color);
            }
        });
    }

    fn fill_circle(&mut self, center: Point, radius: f32, color: Color) {
        let center = self.frame.transform().map_point(center);
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

    fn fill_ellipse(&mut self, center: Point, radius_x: f32, radius_y: f32, color: Color) {
        let radius_x = radius_x.max(0.0);
        let radius_y = radius_y.max(0.0);
        if radius_x <= f32::EPSILON || radius_y <= f32::EPSILON {
            return;
        }

        let center = self.frame.transform().map_point(center);
        let bounds = Rect::xywh(
            center.x - radius_x,
            center.y - radius_y,
            radius_x * 2.0,
            radius_y * 2.0,
        );
        self.for_each_pixel(bounds, |x, y, pixel| {
            let dx = (x as f32 + 0.5 - center.x) / radius_x;
            let dy = (y as f32 + 0.5 - center.y) / radius_y;
            if dx * dx + dy * dy <= 1.0 {
                *pixel = blend(*pixel, color);
            }
        });
    }

    fn draw_line(&mut self, start: Point, end: Point, width: f32, color: Color) {
        let transform = self.frame.transform();
        let start = transform.map_point(start);
        let end = transform.map_point(end);
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

    fn draw_text(&mut self, position: Point, text: &str, style: &crate::TextStyle, color: Color) {
        let layout = TextLayout::new(position, text, style);
        let resolved = layout.resolved();
        let origin = layout.origin();
        let mut cursor_y = origin.y.round() as i32;

        for (line_index, line) in text.split('\n').enumerate() {
            let cursor_x = layout.line_start_x(line_index, style.align_mode());
            self.draw_text_line(cursor_x, cursor_y, line, resolved.scale, color);
            cursor_y += resolved.line_height;
        }
    }

    fn draw_text_line(
        &mut self,
        cursor_x: i32,
        cursor_y: i32,
        text: &str,
        scale: i32,
        color: Color,
    ) {
        let mut cursor_x = cursor_x;
        let glyph_advance = 8 * scale;

        for ch in text.chars() {
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
        self.draw_line(
            Point::new(rect.x(), rect.y()),
            Point::new(rect.right(), rect.y()),
            1.0,
            color,
        );
        self.draw_line(
            Point::new(rect.right(), rect.y()),
            Point::new(rect.right(), rect.bottom()),
            1.0,
            color,
        );
        self.draw_line(
            Point::new(rect.right(), rect.bottom()),
            Point::new(rect.x(), rect.bottom()),
            1.0,
            color,
        );
        self.draw_line(
            Point::new(rect.x(), rect.bottom()),
            Point::new(rect.x(), rect.y()),
            1.0,
            color,
        );
        self.draw_line(
            Point::new(rect.x(), rect.y()),
            Point::new(rect.right(), rect.bottom()),
            1.0,
            color,
        );
        self.draw_line(
            Point::new(rect.right(), rect.y()),
            Point::new(rect.x(), rect.bottom()),
            1.0,
            color,
        );
    }

    fn for_each_pixel(&mut self, rect: Rect, mut draw: impl FnMut(i32, i32, &mut Color)) {
        let rect = match self.frame.clip().rect() {
            Some(clip) => match intersect_rect(rect, clip) {
                Some(rect) => rect,
                None => return,
            },
            None => rect,
        };

        let min_x = rect.x().floor().max(0.0) as i32;
        let min_y = rect.y().floor().max(0.0) as i32;
        let max_x = rect.right().ceil().min(self.frame.pixel_width() as f32) as i32;
        let max_y = rect.bottom().ceil().min(self.frame.pixel_height() as f32) as i32;
        let width = self.frame.pixel_width() as usize;

        for y in min_y..max_y {
            for x in min_x..max_x {
                let index = y as usize * width + x as usize;
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

// TODO(v0.3): define renderer capability flags
// TODO(post-1.0): evaluate GPU backend based on stabilized command model
