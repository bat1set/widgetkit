//! Stable render surface: `Canvas`, text styles, and the software renderer.
//!
//! The command/frame pipeline is intentionally exposed only through [`unstable`]. It exists so
//! WidgetKit can evolve renderer backends and tests without treating raw render internals as a
//! stable application-facing contract.

mod canvas;
mod frame;
mod model;
mod raster;
mod style;
mod surface;
mod text;

pub use canvas::Canvas;
pub use style::{Stroke, TextAlign, TextBaseline, TextMetrics, TextStyle};
pub use surface::{RenderSurface, Renderer, SoftwareRenderer};

pub mod unstable {
    //! Unstable render internals.
    //!
    //! These types model WidgetKit's low-level frame and command pipeline. They are public so
    //! renderer experiments and tests can use them, but they are not part of the stable `Canvas`
    //! API and may change in any `0.x` release.

    pub use crate::canvas::RawCanvas;
    pub use crate::model::{
        ClearCommand, ClipCommand, ClipPrimitive, Fill, FillCommand, FillShape, ImageCommand,
        ImageSource, Paint, RenderCommand, RenderFrame, StateCommand, StrokeCommand, StrokeShape,
        TextCommand, Transform, TransformCommand,
    };
}

#[cfg(test)]
mod tests;
