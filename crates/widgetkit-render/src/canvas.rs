use crate::model::{
    ClearCommand, ClipCommand, ClipPrimitive, CommandList, Fill, FillCommand, FillShape,
    ImageCommand, ImageSource, Paint, RenderCommand, RenderFrame, StateCommand, StrokeCommand,
    StrokeShape, TextCommand, Transform, TransformCommand,
};
use crate::text::measure_text;
use crate::{Stroke, TextMetrics, TextStyle};
use widgetkit_core::{Color, Point, Rect, Size};

#[derive(Debug)]
pub struct Canvas {
    size: Size,
    commands: CommandList,
}

/// Experimental command sink used by `Canvas::experimental_raw`.
///
/// This type is exposed through `widgetkit_render::unstable` only and may change in any `0.x`
/// release.
pub struct RawCanvas<'a> {
    commands: &'a mut CommandList,
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            commands: CommandList::new(),
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn measure_text(&self, text: impl AsRef<str>, style: &TextStyle) -> TextMetrics {
        measure_text(text.as_ref(), style)
    }

    pub fn clear(&mut self, color: Color) {
        self.raw().clear(color);
    }

    pub fn rect(&mut self, rect: Rect, color: Color) {
        self.raw().fill_rect(rect, color);
    }

    pub fn round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.raw().fill_round_rect(rect, radius, color);
    }

    pub fn line(&mut self, start: Point, end: Point, stroke: Stroke, color: Color) {
        self.raw().stroke_line(start, end, stroke, color);
    }

    pub fn circle(&mut self, center: Point, radius: f32, color: Color) {
        self.raw().fill_circle(center, radius, color);
    }

    pub fn ellipse(&mut self, center: Point, radius_x: f32, radius_y: f32, color: Color) {
        self.raw().fill_ellipse(center, radius_x, radius_y, color);
    }

    pub fn text(
        &mut self,
        position: Point,
        text: impl Into<String>,
        style: TextStyle,
        color: Color,
    ) {
        self.raw().draw_text(position, text, style, color);
    }

    pub fn image_placeholder(&mut self, rect: Rect, color: Color) {
        self.raw().image_placeholder(rect, color);
    }

    pub fn clip_rect(&mut self, rect: Rect) {
        self.raw().clip_rect(rect);
    }

    pub fn save(&mut self) {
        self.raw().save();
    }

    pub fn restore(&mut self) {
        self.raw().restore();
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.raw().translate(dx, dy);
    }

    /// Runs an experimental low-level command sink against this canvas.
    ///
    /// `RawCanvas` and the command types behind it are available from
    /// `widgetkit_render::unstable`. They are intentionally not re-exported by the top-level
    /// `widgetkit` facade and may change in any `0.x` release.
    pub fn experimental_raw(&mut self, f: impl FnOnce(&mut RawCanvas<'_>)) {
        let mut raw = self.raw();
        f(&mut raw);
    }

    fn raw(&mut self) -> RawCanvas<'_> {
        RawCanvas {
            commands: &mut self.commands,
        }
    }

    #[doc(hidden)]
    pub fn into_frame(self) -> RenderFrame {
        RenderFrame::from_list(self.size, self.commands)
    }
}

impl RawCanvas<'_> {
    pub fn push(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    pub fn clear(&mut self, color: Color) {
        self.push(RenderCommand::Clear(ClearCommand {
            paint: Paint::solid(color),
        }));
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.push(RenderCommand::Fill(FillCommand {
            shape: FillShape::Rect(rect),
            fill: Fill::solid(color),
        }));
    }

    pub fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.push(RenderCommand::Fill(FillCommand {
            shape: FillShape::RoundRect {
                rect,
                radius: radius.max(0.0),
            },
            fill: Fill::solid(color),
        }));
    }

    pub fn fill_circle(&mut self, center: Point, radius: f32, color: Color) {
        self.push(RenderCommand::Fill(FillCommand {
            shape: FillShape::Circle {
                center,
                radius: radius.max(0.0),
            },
            fill: Fill::solid(color),
        }));
    }

    pub fn fill_ellipse(&mut self, center: Point, radius_x: f32, radius_y: f32, color: Color) {
        self.push(RenderCommand::Fill(FillCommand {
            shape: FillShape::Ellipse {
                center,
                radius_x: radius_x.max(0.0),
                radius_y: radius_y.max(0.0),
            },
            fill: Fill::solid(color),
        }));
    }

    pub fn stroke_line(&mut self, start: Point, end: Point, stroke: Stroke, color: Color) {
        self.push(RenderCommand::Stroke(StrokeCommand {
            shape: StrokeShape::Line { start, end },
            stroke,
            paint: Paint::solid(color),
        }));
    }

    pub fn draw_text(
        &mut self,
        position: Point,
        text: impl Into<String>,
        style: TextStyle,
        color: Color,
    ) {
        self.push(RenderCommand::Text(TextCommand {
            position,
            text: text.into(),
            style,
            paint: Paint::solid(color),
        }));
    }

    pub fn image_placeholder(&mut self, rect: Rect, color: Color) {
        self.push(RenderCommand::Image(ImageCommand {
            rect,
            source: ImageSource::Placeholder,
            paint: Paint::solid(color),
        }));
    }

    pub fn clip_rect(&mut self, rect: Rect) {
        self.push(RenderCommand::Clip(ClipCommand {
            primitive: ClipPrimitive::Rect(rect),
        }));
    }

    pub fn save(&mut self) {
        self.push(RenderCommand::State(StateCommand::Save));
    }

    pub fn restore(&mut self) {
        self.push(RenderCommand::State(StateCommand::Restore));
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.push(RenderCommand::Transform(TransformCommand {
            transform: Transform::translation(dx, dy),
        }));
    }
}

// TODO(v0.3): image draw API stabilization
// TODO(v0.3): transform stack stabilization
// TODO(v0.3): richer clipping model
// TODO(v0.5): map declarative nodes to Canvas without API gaps
