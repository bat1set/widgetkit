//! Lifecycle-driven runtime for WidgetKit.
//! The current runtime scope is intentionally a single widget instance per app/host pair.
//! All timers and background tasks belong to that widget instance and are shut down with it.
//! Rendering is demand-driven and redraw requests are coalesced until the pending frame is consumed.
//!
//! Redraw invalidation model:
//!
//! - `request_render()` marks layout and render dirty.
//! - layout is consumed before host content sizing and can trigger a window resize.
//! - host resize updates the available surface size, marks layout dirty, and schedules render.
//! - repeated render requests before the host consumes the pending frame are coalesced.
//! - hosts should request redraws on demand instead of running a permanent render loop.
//! - `shutdown` clears pending redraw state before `dispose` completes.
//! - late messages, timer completions, task completions, and render requests are ignored once
//!   their widget instance token no longer matches the live instance.

mod app;
mod context;
mod event;
mod host;
mod internal;
mod scheduler;
mod tasks;
mod widget;

pub use app::{AppRunner, WidgetApp};
pub use context::{DisposeCtx, LayoutCtx, MountCtx, RenderCtx, StartCtx, StopCtx, UpdateCtx};
pub use event::Event;
pub use host::HostRunner;
pub use scheduler::Scheduler;
pub use tasks::Tasks;
pub use widget::Widget;

pub use widgetkit_core;

// TODO(v0.3): guard against resize-relayout-resize feedback loops
// TODO(v0.4): route richer host/input events
// TODO(v0.5): connect sizing contracts to declarative layout
// TODO(v0.7): allow lifecycle integration with hybrid/native-web host
// TODO(v0.8): support restart-safe instance isolation guarantees
// TODO(v0.8): structured concurrency/task groups debug inspection
// TODO(v0.8): expose task diagnostics/devtools hooks
// TODO(v0.3): debounce/throttle helpers
// TODO(v0.8): virtual time/testing scheduler

#[cfg(test)]
mod tests;
