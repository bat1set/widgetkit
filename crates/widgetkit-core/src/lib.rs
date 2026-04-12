pub mod color;
pub mod error;
pub mod event;
pub mod geometry;
pub mod ids;

pub use color::Color;
pub use error::{Error, Result};
pub use event::HostEvent;
pub use geometry::{Insets, Point, Rect, Size};
pub use ids::{InstanceId, TaskId, TimerId, WidgetId};
pub use std::time::Duration;

// TODO(v0.3): add layout constraints primitives
// TODO(v0.4): extend event model for pointer/keyboard input
// TODO(v0.5): prepare View/declarative layer abstractions

#[cfg(test)]
mod tests;
