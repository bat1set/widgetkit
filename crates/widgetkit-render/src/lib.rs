//! Stable v0.1 render surface: `Canvas`, styles, and the software renderer.
//! Internal raw render primitives remain crate-private and intentionally unstable.

mod canvas;
mod raw;
mod style;
mod surface;

pub use canvas::Canvas;
pub use style::{Stroke, TextStyle};
pub use surface::{RenderSurface, Renderer, SoftwareRenderer};

#[cfg(test)]
mod tests;
