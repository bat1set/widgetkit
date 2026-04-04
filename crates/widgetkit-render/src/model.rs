use crate::{Stroke, TextStyle};
use widgetkit_core::{Color, Point, Rect, Size};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderScene {
    size: Size,
    commands: CommandList,
}

impl RenderScene {
    pub(crate) fn new(size: Size, commands: CommandList) -> Self {
        Self { size, commands }
    }

    pub(crate) fn size(&self) -> Size {
        self.size
    }

    #[cfg(test)]
    pub(crate) fn commands(&self) -> &[DrawCommand] {
        self.commands.commands()
    }

    pub(crate) fn into_parts(self) -> (Size, CommandList) {
        (self.size, self.commands)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CommandList {
    commands: Vec<DrawCommand>,
}

impl CommandList {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn from_commands(commands: Vec<DrawCommand>) -> Self {
        Self { commands }
    }

    pub(crate) fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }

    #[cfg(test)]
    pub(crate) fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub(crate) fn into_commands(self) -> Vec<DrawCommand> {
        self.commands
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DrawCommand {
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
pub(crate) struct Paint {
    pub(crate) color: Color,
}

impl Paint {
    pub(crate) const fn solid(color: Color) -> Self {
        Self { color }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClearCommand {
    pub(crate) paint: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fill {
    pub(crate) paint: Paint,
}

impl Fill {
    pub(crate) const fn solid(color: Color) -> Self {
        Self {
            paint: Paint::solid(color),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillCommand {
    pub(crate) shape: FillShape,
    pub(crate) fill: Fill,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FillShape {
    Rect(Rect),
    RoundRect { rect: Rect, radius: f32 },
    Circle { center: Point, radius: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StrokeCommand {
    pub(crate) shape: StrokeShape,
    pub(crate) stroke: Stroke,
    pub(crate) paint: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum StrokeShape {
    Line { start: Point, end: Point },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextCommand {
    pub(crate) position: Point,
    pub(crate) text: String,
    pub(crate) style: TextStyle,
    pub(crate) paint: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ImageCommand {
    pub(crate) rect: Rect,
    pub(crate) source: ImageSource,
    pub(crate) paint: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ImageSource {
    Placeholder,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClipCommand {
    pub(crate) primitive: ClipPrimitive,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClipPrimitive {
    Rect(Rect),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Transform {
    pub(crate) translate_x: f32,
    pub(crate) translate_y: f32,
}

impl Transform {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const fn translation(x: f32, y: f32) -> Self {
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

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TransformCommand {
    pub(crate) transform: Transform,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StateCommand {
    Save,
    Restore,
}

// TODO(v0.3): layout-aware text measurement hooks
// TODO(v0.5): ensure command model is sufficient for declarative layer
