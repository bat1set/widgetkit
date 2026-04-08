//! Stable render surface: `Canvas`, text styles, and the software renderer.
//! WidgetKit routes drawing through an explicit `RenderFrame` command model,
//! while keeping direct low-level access behind the `Canvas::experimental_raw` escape hatch.

mod canvas;
mod frame;
mod model;
mod raster;
mod style;
mod surface;
mod text;

pub use canvas::{Canvas, RawCanvas};
pub use model::{
    ClearCommand, ClipCommand, ClipPrimitive, Fill, FillCommand, FillShape, ImageCommand,
    ImageSource, Paint, RenderCommand, RenderFrame, StateCommand, StrokeCommand, StrokeShape,
    TextCommand, Transform, TransformCommand,
};
pub use style::{Stroke, TextAlign, TextBaseline, TextMetrics, TextStyle};
pub use surface::{RenderSurface, Renderer, SoftwareRenderer};

#[cfg(test)]
mod tests;
