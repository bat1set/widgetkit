#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub width: f32,
}

impl Stroke {
    pub const fn new(width: f32) -> Self {
        Self { width }
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Self { width: 1.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBaseline {
    Top,
    Middle,
    Alphabetic,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub line_height: f32,
    pub baseline: f32,
    pub line_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    size: f32,
    line_height: Option<f32>,
    align: TextAlign,
    baseline: TextBaseline,
}

impl TextStyle {
    pub fn new() -> Self {
        Self {
            size: 14.0,
            line_height: None,
            align: TextAlign::Left,
            baseline: TextBaseline::Top,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(1.0);
        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = Some(line_height.max(1.0));
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn baseline(mut self, baseline: TextBaseline) -> Self {
        self.baseline = baseline;
        self
    }

    pub fn pixel_size(&self) -> f32 {
        self.size
    }

    pub(crate) fn line_height_override(&self) -> Option<f32> {
        self.line_height
    }

    pub(crate) fn align_mode(&self) -> TextAlign {
        self.align
    }

    pub(crate) fn baseline_mode(&self) -> TextBaseline {
        self.baseline
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}
