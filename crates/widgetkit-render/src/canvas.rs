use crate::model::{
    ClearCommand, CommandList, DrawCommand, Fill, FillCommand, FillShape, ImageCommand,
    ImageSource, Paint, RenderScene, StrokeCommand, StrokeShape, TextCommand,
};
use crate::{Stroke, TextStyle};
use widgetkit_core::{Color, Point, Rect, Size};

#[derive(Debug)]
pub struct Canvas {
    size: Size,
    commands: CommandList,
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

    pub fn clear(&mut self, color: Color) {
        self.commands.push(DrawCommand::Clear(ClearCommand {
            paint: Paint::solid(color),
        }));
    }

    pub fn rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(DrawCommand::Fill(FillCommand {
            shape: FillShape::Rect(rect),
            fill: Fill::solid(color),
        }));
    }

    pub fn round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.commands.push(DrawCommand::Fill(FillCommand {
            shape: FillShape::RoundRect {
                rect,
                radius: radius.max(0.0),
            },
            fill: Fill::solid(color),
        }));
    }

    pub fn line(&mut self, start: Point, end: Point, stroke: Stroke, color: Color) {
        self.commands.push(DrawCommand::Stroke(StrokeCommand {
            shape: StrokeShape::Line { start, end },
            stroke,
            paint: Paint::solid(color),
        }));
    }

    pub fn circle(&mut self, center: Point, radius: f32, color: Color) {
        self.commands.push(DrawCommand::Fill(FillCommand {
            shape: FillShape::Circle {
                center,
                radius: radius.max(0.0),
            },
            fill: Fill::solid(color),
        }));
    }

    pub fn text(
        &mut self,
        position: Point,
        text: impl Into<String>,
        style: TextStyle,
        color: Color,
    ) {
        self.commands.push(DrawCommand::Text(TextCommand {
            position,
            text: text.into(),
            style,
            paint: Paint::solid(color),
        }));
    }

    pub fn image_placeholder(&mut self, rect: Rect, color: Color) {
        self.commands.push(DrawCommand::Image(ImageCommand {
            rect,
            source: ImageSource::Placeholder,
            paint: Paint::solid(color),
        }));
    }

    pub(crate) fn into_scene(self) -> RenderScene {
        RenderScene::new(self.size, self.commands)
    }
}

// TODO(v0.3): image draw API stabilization
// TODO(v0.3): transform stack stabilization
// TODO(v0.3): richer clipping model
// TODO(v0.5): map declarative nodes to Canvas without API gaps
