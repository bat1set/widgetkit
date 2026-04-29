pub mod color;
pub mod error;
pub mod event;
pub mod geometry;
pub mod ids;
pub mod layout;

pub use color::Color;
pub use error::{Error, Result};
pub use event::{HostEvent, Key, KeyboardEvent, MouseButton, MouseEvent, MouseWheelDelta};
pub use geometry::{Insets, Point, Rect, Size};
pub use ids::{InstanceId, TaskId, TimerId, WidgetId};
pub use layout::{Constraints, SizePolicy};
pub use std::time::Duration;

// TODO(v0.4): stabilize MouseEvent and KeyboardEvent basics
// TODO(v0.5): prepare View/declarative layer abstractions

#[cfg(test)]
mod tests;
