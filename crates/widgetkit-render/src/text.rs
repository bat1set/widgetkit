use crate::{TextAlign, TextBaseline, TextMetrics, TextStyle};
use widgetkit_core::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedTextStyle {
    pub(crate) scale: i32,
    pub(crate) glyph_width: i32,
    pub(crate) glyph_height: i32,
    pub(crate) line_height: i32,
    pub(crate) baseline: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextLayout {
    origin: Point,
    metrics: TextMetrics,
    line_widths: Vec<i32>,
    resolved: ResolvedTextStyle,
}

impl TextLayout {
    pub(crate) fn new(position: Point, text: &str, style: &TextStyle) -> Self {
        let resolved = resolve_text_style(style);
        let line_widths = line_widths(text, resolved.glyph_width);
        let line_count = line_widths.len().max(1);
        let width = line_widths.iter().copied().max().unwrap_or_default() as f32;
        let height = (line_count as i32 * resolved.line_height) as f32;
        let metrics = TextMetrics {
            width,
            height,
            line_height: resolved.line_height as f32,
            baseline: resolved.baseline as f32,
            line_count,
        };
        let origin = layout_origin(position, metrics, style.baseline_mode());

        Self {
            origin,
            metrics,
            line_widths,
            resolved,
        }
    }

    pub(crate) fn metrics(&self) -> TextMetrics {
        self.metrics
    }

    pub(crate) fn origin(&self) -> Point {
        self.origin
    }

    pub(crate) fn resolved(&self) -> ResolvedTextStyle {
        self.resolved
    }

    pub(crate) fn line_start_x(&self, line_index: usize, align: TextAlign) -> i32 {
        let base_x = self.origin.x.round() as i32;
        let line_width = self
            .line_widths
            .get(line_index)
            .copied()
            .unwrap_or_default();
        match align {
            TextAlign::Left => base_x,
            TextAlign::Center => base_x - line_width / 2,
            TextAlign::Right => base_x - line_width,
        }
    }
}

pub(crate) fn measure_text(text: &str, style: &TextStyle) -> TextMetrics {
    TextLayout::new(Point::new(0.0, 0.0), text, style).metrics()
}

fn resolve_text_style(style: &TextStyle) -> ResolvedTextStyle {
    let scale = (style.pixel_size() / 8.0).round().max(1.0) as i32;
    let glyph_width = 8 * scale;
    let glyph_height = 8 * scale;
    let line_height = style
        .line_height_override()
        .map(|value| value.round().max(glyph_height as f32) as i32)
        .unwrap_or(glyph_height);
    let baseline = (glyph_height - scale).max(1);

    ResolvedTextStyle {
        scale,
        glyph_width,
        glyph_height,
        line_height,
        baseline,
    }
}

fn line_widths(text: &str, glyph_width: i32) -> Vec<i32> {
    let mut widths: Vec<i32> = text
        .split('\n')
        .map(|line| line.chars().count() as i32 * glyph_width)
        .collect();
    if widths.is_empty() {
        widths.push(0);
    }
    widths
}

fn layout_origin(position: Point, metrics: TextMetrics, baseline: TextBaseline) -> Point {
    let y = match baseline {
        TextBaseline::Top => position.y,
        TextBaseline::Middle => position.y - metrics.height * 0.5,
        TextBaseline::Alphabetic => position.y - metrics.baseline,
        TextBaseline::Bottom => position.y - metrics.height,
    };

    Point::new(position.x, y)
}
