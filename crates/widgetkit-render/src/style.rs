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

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    size: f32,
}

impl TextStyle {
    pub fn new() -> Self {
        Self { size: 14.0 }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(1.0);
        self
    }

    pub fn pixel_size(&self) -> f32 {
        self.size
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}
