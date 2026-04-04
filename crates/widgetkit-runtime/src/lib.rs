//! Lifecycle-driven runtime for WidgetKit v0.1.
//! The current runtime scope is intentionally a single widget instance per app/host pair.

mod app;
mod context;
mod event;
mod host;
mod internal;
mod scheduler;
mod tasks;
mod widget;

pub use app::{AppRunner, WidgetApp};
pub use context::{DisposeCtx, MountCtx, RenderCtx, StartCtx, StopCtx, UpdateCtx};
pub use event::Event;
pub use host::HostRunner;
pub use scheduler::Scheduler;
pub use tasks::Tasks;
pub use widget::Widget;

pub use widgetkit_core;
pub use widgetkit_render;

// TODO(v0.2): add widget instance generation guards for stale messages
// TODO(v0.7): allow lifecycle integration with hybrid/native-web host
// TODO(v0.8): support restart-safe instance isolation guarantees
// TODO(v0.2): named task handles
// TODO(v0.2): task cancellation tokens
// TODO(v0.8): structured concurrency/task groups debug inspection
// TODO(v0.8): expose task diagnostics/devtools hooks
// TODO(v0.3): debounce/throttle helpers
// TODO(v0.8): virtual time/testing scheduler

#[cfg(test)]
mod tests;
