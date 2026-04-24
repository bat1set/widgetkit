//! WidgetKit stable surface centered on `Widget + Canvas + WindowsHost + WidgetApp`.
//! Rendering is demand-driven, routed through an internal render pipeline, and currently
//! targets a single widget instance per app/host pair.

#[cfg(feature = "canvas")]
pub use widgetkit_core as core;
#[cfg(feature = "windows")]
pub use widgetkit_host_windows as windows;
#[cfg(feature = "canvas")]
pub use widgetkit_runtime as runtime;

#[cfg(feature = "canvas")]
pub mod render {
    //! Stable render exports.
    //!
    //! Raw command/frame internals live in `widgetkit_render::unstable` and are not part of
    //! WidgetKit's stable facade API.

    pub use widgetkit_render::{
        Canvas, SoftwareRenderer, Stroke, TextAlign, TextBaseline, TextMetrics, TextStyle,
    };
}

#[cfg(feature = "canvas")]
pub use widgetkit_core::{
    Color, Constraints, Duration, HostEvent, Insets, InstanceId, Point, Rect, Result, Size,
    SizePolicy, TaskId, TimerId, WidgetId,
};
#[cfg(feature = "windows")]
pub use widgetkit_host_windows::{WindowConfig, WindowsHost};
#[cfg(feature = "canvas")]
pub use widgetkit_render::{
    Canvas, SoftwareRenderer, Stroke, TextAlign, TextBaseline, TextMetrics, TextStyle,
};
#[cfg(feature = "canvas")]
pub use widgetkit_runtime::{
    AppRunner, DisposeCtx, Event, HostRunner, LayoutCtx, MountCtx, RenderCtx, Scheduler, StartCtx,
    StopCtx, Tasks, UpdateCtx, Widget, WidgetApp,
};

#[cfg(feature = "canvas")]
pub mod prelude {
    pub use crate::{
        Canvas, Color, Constraints, DisposeCtx, Duration, Event, HostEvent, Insets, InstanceId,
        LayoutCtx, MountCtx, Point, Rect, RenderCtx, Result, Scheduler, Size, SizePolicy,
        SoftwareRenderer, StartCtx, StopCtx, Stroke, TaskId, Tasks, TextAlign, TextBaseline,
        TextMetrics, TextStyle, TimerId, UpdateCtx, Widget, WidgetApp, WidgetId,
    };
    #[cfg(feature = "windows")]
    pub use crate::{WindowConfig, WindowsHost};
}

// TODO(v0.3): export stable window sizing/config APIs
// TODO(v0.5): add declarative feature exports
// TODO(v0.6): desktop capability modules
// TODO(v0.7): tauri bridge exports
// TODO(v0.9): review feature flag naming before 1.0
