//! WidgetKit v0.1 stable surface.
//! The public default path is `Widget + Canvas + WindowsHost + WidgetApp`.
//! Runtime orchestration is intentionally single-widget in v0.1.

#[cfg(feature = "canvas")]
pub use widgetkit_core as core;
#[cfg(feature = "canvas")]
pub use widgetkit_render as render;
#[cfg(feature = "canvas")]
pub use widgetkit_runtime as runtime;
#[cfg(feature = "windows")]
pub use widgetkit_host_windows as windows;

#[cfg(feature = "canvas")]
pub use widgetkit_core::{Color, Duration, HostEvent, Insets, InstanceId, Point, Rect, Result, Size, TaskId, TimerId, WidgetId};
#[cfg(feature = "canvas")]
pub use widgetkit_render::{Canvas, SoftwareRenderer, Stroke, TextStyle};
#[cfg(feature = "canvas")]
pub use widgetkit_runtime::{
    AppRunner, DisposeCtx, Event, HostRunner, MountCtx, RenderCtx, Scheduler, StartCtx, StopCtx,
    Tasks, UpdateCtx, Widget, WidgetApp,
};
#[cfg(feature = "windows")]
pub use widgetkit_host_windows::WindowsHost;

#[cfg(feature = "canvas")]
pub mod prelude {
    pub use crate::{
        Canvas, Color, DisposeCtx, Duration, Event, HostEvent, Insets, InstanceId, MountCtx,
        Point, Rect, RenderCtx, Result, Scheduler, Size, SoftwareRenderer, StartCtx, StopCtx,
        Stroke, TaskId, Tasks, TextStyle, TimerId, UpdateCtx, Widget, WidgetApp, WidgetId,
    };
    #[cfg(feature = "windows")]
    pub use crate::WindowsHost;
}

// TODO(v0.5): add declarative feature exports
// TODO(v0.6): desktop capability modules
// TODO(v0.7): tauri bridge exports
// TODO(v0.9): review feature flag naming before 1.0
