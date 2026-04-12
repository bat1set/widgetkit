use crate::{Stroke, TextStyle};
use widgetkit_core::{Color, Point, Rect, Size};

// Unstable render command model used behind `Canvas`.
//
// These types are implementation details for the current software renderer and future backend
// experiments. They are exposed through `widgetkit_render::unstable` only and should not be
// treated as a stable WidgetKit application API.

/// Unstable render frame consumed by renderer backends.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderFrame {
    size: Size,
    commands: Vec<RenderCommand>,
}

impl RenderFrame {
    pub fn new(size: Size, commands: Vec<RenderCommand>) -> Self {
        Self { size, commands }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    pub fn into_commands(self) -> Vec<RenderCommand> {
        self.commands
    }

    pub(crate) fn from_list(size: Size, commands: CommandList) -> Self {
        Self::new(size, commands.into_commands())
    }

    pub(crate) fn into_parts(self) -> (Size, Vec<RenderCommand>) {
        (self.size, self.commands)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CommandList {
    commands: Vec<RenderCommand>,
}

impl CommandList {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    pub(crate) fn into_commands(self) -> Vec<RenderCommand> {
        self.commands
    }
}

/// Unstable low-level drawing command.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderCommand {
    Clear(ClearCommand),
    Fill(FillCommand),
    Stroke(StrokeCommand),
    Text(TextCommand),
    Image(ImageCommand),
    Clip(ClipCommand),
    Transform(TransformCommand),
    State(StateCommand),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub color: Color,
}

impl Paint {
    pub const fn solid(color: Color) -> Self {
        Self { color }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClearCommand {
    pub paint: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fill {
    pub paint: Paint,
}

impl Fill {
    pub const fn solid(color: Color) -> Self {
        Self {
            paint: Paint::solid(color),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillCommand {
    pub shape: FillShape,
    pub fill: Fill,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FillShape {
    Rect(Rect),
    RoundRect {
        rect: Rect,
        radius: f32,
    },
    Circle {
        center: Point,
        radius: f32,
    },
    Ellipse {
        center: Point,
        radius_x: f32,
        radius_y: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeCommand {
    pub shape: StrokeShape,
    pub stroke: Stroke,
    pub paint: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeShape {
    Line { start: Point, end: Point },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextCommand {
    pub position: Point,
    pub text: String,
    pub style: TextStyle,
    pub paint: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageCommand {
    pub rect: Rect,
    pub source: ImageSource,
    pub paint: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageSource {
    Placeholder,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipCommand {
    pub primitive: ClipPrimitive,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClipPrimitive {
    Rect(Rect),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Transform {
    pub translate_x: f32,
    pub translate_y: f32,
}

impl Transform {
    pub const fn translation(x: f32, y: f32) -> Self {
        Self {
            translate_x: x,
            translate_y: y,
        }
    }

    pub(crate) fn then(self, next: Self) -> Self {
        Self {
            translate_x: self.translate_x + next.translate_x,
            translate_y: self.translate_y + next.translate_y,
        }
    }

    pub(crate) fn map_point(self, point: Point) -> Point {
        Point::new(point.x + self.translate_x, point.y + self.translate_y)
    }

    pub(crate) fn map_rect(self, rect: Rect) -> Rect {
        Rect::xywh(
            rect.x() + self.translate_x,
            rect.y() + self.translate_y,
            rect.width(),
            rect.height(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformCommand {
    pub transform: Transform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateCommand {
    Save,
    Restore,
}

// TODO(v0.3): layout-aware text measurement hooks
// TODO(v0.3): keep raw render internals unstable until post-v0.3 review
// TODO(v0.4): input-related hit-test helpers for shapes
// TODO(v0.5): ensure command model is sufficient for declarative layer
