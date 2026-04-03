use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
pub use std::time::Duration;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Message(String),
    Platform(String),
    Render(String),
}

impl Error {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn platform(message: impl Into<String>) -> Self {
        Self::Platform(message.into())
    }

    pub fn render(message: impl Into<String>) -> Self {
        Self::Render(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => write!(f, "{message}"),
            Self::Platform(message) => write!(f, "platform error: {message}"),
            Self::Render(message) => write!(f, "render error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::message(value)
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::message(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);
    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    pub fn with_alpha(self, alpha: u8) -> Self {
        Self { a: alpha, ..self }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    pub const fn xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::new(Point::new(x, y), Size::new(width, height))
    }

    pub fn x(self) -> f32 {
        self.origin.x
    }

    pub fn y(self) -> f32 {
        self.origin.y
    }

    pub fn width(self) -> f32 {
        self.size.width
    }

    pub fn height(self) -> f32 {
        self.size.height
    }

    pub fn right(self) -> f32 {
        self.origin.x + self.size.width
    }

    pub fn bottom(self) -> f32 {
        self.origin.y + self.size.height
    }

    pub fn inset(self, insets: Insets) -> Self {
        let x = self.x() + insets.left;
        let y = self.y() + insets.top;
        let width = (self.width() - insets.left - insets.right).max(0.0);
        let height = (self.height() - insets.top - insets.bottom).max(0.0);
        Self::xywh(x, y, width, height)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InstanceId(u64);

impl InstanceId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimerId(u64);

impl TimerId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum HostEvent {
    CloseRequested,
    Focused(bool),
    Resized(Size),
}

// TODO(v0.2): stabilize public raw render API
// TODO(v0.3): add layout constraints primitives
// TODO(v0.4): extend event model for pointer/keyboard input
// TODO(v0.5): prepare View/declarative layer abstractions

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_inset_clamps_to_zero() {
        let rect = Rect::xywh(0.0, 0.0, 10.0, 5.0);
        let inset = rect.inset(Insets::all(10.0));
        assert_eq!(inset, Rect::xywh(10.0, 10.0, 0.0, 0.0));
    }

    #[test]
    fn color_alpha_override_keeps_rgb() {
        let color = Color::rgb(10, 20, 30).with_alpha(40);
        assert_eq!(color, Color::rgba(10, 20, 30, 40));
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(WidgetId::new(), WidgetId::new());
        assert_ne!(InstanceId::new(), InstanceId::new());
        assert_ne!(TimerId::new(), TimerId::new());
        assert_ne!(TaskId::new(), TaskId::new());
    }
}
