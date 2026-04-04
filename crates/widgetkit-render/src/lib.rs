//! Stable render surface: `Canvas`, styles, and the software renderer.
//! Internally, WidgetKit now routes drawing through an explicit command model,
//! while keeping the low-level layer crate-private and intentionally unstable.

mod canvas;
mod frame;
mod model;
mod raster;
mod style;
mod surface;

pub use canvas::Canvas;
pub use style::{Stroke, TextStyle};
pub use surface::{RenderSurface, Renderer, SoftwareRenderer};

#[cfg(test)]
mod tests;
