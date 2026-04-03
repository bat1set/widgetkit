use crate::raw;
use crate::{Stroke, TextStyle};
use widgetkit_core::{Color, Point, Rect, Size};

#[derive(Debug)]
pub struct Canvas {
    size: Size,
    commands: Vec<raw::Command>,
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            commands: Vec::new(),
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn clear(&mut self, color: Color) {
        self.commands.push(raw::Command::Clear { color });
    }

    pub fn rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(raw::Command::Rect { rect, color });
    }

    pub fn round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.commands.push(raw::Command::RoundRect {
            rect,
            radius: radius.max(0.0),
            color,
        });
    }

    pub fn line(&mut self, start: Point, end: Point, stroke: Stroke, color: Color) {
        self.commands.push(raw::Command::Line {
            start,
            end,
            stroke,
            color,
        });
    }

    pub fn circle(&mut self, center: Point, radius: f32, color: Color) {
        self.commands.push(raw::Command::Circle {
            center,
            radius: radius.max(0.0),
            color,
        });
    }

    pub fn text(&mut self, position: Point, text: impl Into<String>, style: TextStyle, color: Color) {
        self.commands.push(raw::Command::Text {
            position,
            text: text.into(),
            style,
            color,
        });
    }

    pub fn image_placeholder(&mut self, rect: Rect, color: Color) {
        self.commands.push(raw::Command::ImagePlaceholder { rect, color });
    }

    pub(crate) fn into_scene(self) -> raw::Scene {
        raw::Scene {
            size: self.size,
            commands: self.commands,
        }
    }
}
