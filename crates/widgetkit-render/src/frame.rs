use crate::model::Transform;
use widgetkit_core::{Color, Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClipState {
    rect: Option<Rect>,
}

impl ClipState {
    pub(crate) const fn none() -> Self {
        Self { rect: None }
    }

    pub(crate) fn rect(self) -> Option<Rect> {
        self.rect
    }

    pub(crate) fn intersect(self, rect: Rect) -> Self {
        Self {
            rect: match self.rect {
                Some(current) => intersect_rect(current, rect),
                None => Some(rect),
            },
        }
    }
}

impl Default for ClipState {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FrameState {
    pub(crate) clip: ClipState,
    pub(crate) transform: Transform,
}

pub(crate) struct Frame<'a> {
    width: u32,
    height: u32,
    pixels: &'a mut [Color],
    state: FrameState,
    state_stack: Vec<FrameState>,
}

impl<'a> Frame<'a> {
    pub(crate) fn new(width: u32, height: u32, pixels: &'a mut [Color]) -> Self {
        Self {
            width,
            height,
            pixels,
            state: FrameState::default(),
            state_stack: Vec::new(),
        }
    }

    pub(crate) fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    pub(crate) fn pixel_width(&self) -> u32 {
        self.width
    }

    pub(crate) fn pixel_height(&self) -> u32 {
        self.height
    }

    pub(crate) fn pixels_mut(&mut self) -> &mut [Color] {
        self.pixels
    }

    pub(crate) fn clip(&self) -> ClipState {
        self.state.clip
    }

    pub(crate) fn transform(&self) -> Transform {
        self.state.transform
    }

    pub(crate) fn save_state(&mut self) {
        self.state_stack.push(self.state);
    }

    pub(crate) fn restore_state(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.state = state;
        }
    }

    pub(crate) fn apply_clip_rect(&mut self, rect: Rect) {
        let mapped = self.state.transform.map_rect(rect);
        self.state.clip = self.state.clip.intersect(mapped);
    }

    pub(crate) fn apply_transform(&mut self, transform: Transform) {
        self.state.transform = self.state.transform.then(transform);
    }
}

pub(crate) fn intersect_rect(a: Rect, b: Rect) -> Option<Rect> {
    let left = a.x().max(b.x());
    let top = a.y().max(b.y());
    let right = a.right().min(b.right());
    let bottom = a.bottom().min(b.bottom());
    if right <= left || bottom <= top {
        return None;
    }

    Some(Rect::xywh(left, top, right - left, bottom - top))
}

// TODO(v0.3): monitor/work-area aware rendering constraints if needed
// TODO(v0.4): improve dirty-region strategy
