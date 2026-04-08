//! WidgetKit v0.1 stable surface.
//! The public default path is `Widget + Canvas + WindowsHost + WidgetApp`.
//! Runtime orchestration is intentionally single-widget in v0.1.
//! Rendering is demand-driven rather than tied to a permanent render loop.

#[cfg(feature = "canvas")]
pub use widgetkit_core as core;
#[cfg(feature = "windows")]
pub use widgetkit_host_windows as windows;
#[cfg(feature = "canvas")]
pub use widgetkit_render as render;
#[cfg(feature = "canvas")]
pub use widgetkit_runtime as runtime;

#[cfg(feature = "canvas")]
pub use widgetkit_core::{
    Color, Duration, HostEvent, Insets, InstanceId, Point, Rect, Result, Size, TaskId, TimerId,
    WidgetId,
};
#[cfg(feature = "windows")]
pub use widgetkit_host_windows::WindowsHost;
#[cfg(feature = "canvas")]
pub use widgetkit_render::{
    Canvas, RawCanvas, RenderCommand, RenderFrame, SoftwareRenderer, Stroke, TextAlign,
    TextBaseline, TextMetrics, TextStyle,
};
#[cfg(feature = "canvas")]
pub use widgetkit_runtime::{
    AppRunner, DisposeCtx, Event, HostRunner, MountCtx, RenderCtx, Scheduler, StartCtx, StopCtx,
    Tasks, UpdateCtx, Widget, WidgetApp,
};

#[cfg(feature = "canvas")]
pub mod prelude {
    #[cfg(feature = "windows")]
    pub use crate::WindowsHost;
    pub use crate::{
        Canvas, Color, DisposeCtx, Duration, Event, HostEvent, Insets, InstanceId, MountCtx, Point,
        RawCanvas, Rect, RenderCommand, RenderCtx, RenderFrame, Result, Scheduler, Size,
        SoftwareRenderer, StartCtx, StopCtx, Stroke, TaskId, Tasks, TextAlign, TextBaseline,
        TextMetrics, TextStyle, TimerId, UpdateCtx, Widget, WidgetApp, WidgetId,
    };
}

// TODO(v0.5): add declarative feature exports
// TODO(v0.6): desktop capability modules
// TODO(v0.7): tauri bridge exports
// TODO(v0.9): review feature flag naming before 1.0
