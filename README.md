# WidgetKit

---

language: English, [Russian](README.ru.md)

---
[![Version](https://img.shields.io/badge/version-0.1.0-blue)](#)
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)](#)
[![Renderer](https://img.shields.io/badge/renderer-software%202D-lightgrey)](#)
[![Status](https://img.shields.io/badge/status-bootstrap-orange)](#)

WidgetKit is a modular Rust library for building desktop widgets.

The current implementation provides a working Windows host, a software 2D renderer, a stable `Widget + Canvas + WindowsHost + WidgetApp` public path, and a small runtime model centered on a single widget instance.

The project is intended to grow into a broader desktop widget framework with a stronger scheduler, a stabilized raw rendering API, layout primitives, declarative UI, desktop capabilities, and Tauri or hybrid integration. Some of that is already present in an early form, some is not implemented yet, and some remains internal or unstable in the current version.

## Project Status

### Implemented now

- [x] workspace
- [x] Windows host
- [x] software 2D renderer
- [x] stable `Canvas` drawing API
- [x] widget lifecycle with `mount`, `start`, `update`, `render`, `stop`, and `dispose`
- [x] instance-scoped scheduler API
- [x] instance-scoped task API
- [x] optional Tokio-backed task runtime
- [x] local `clock` example
- [x] demand-driven redraw model

### Present but not stable yet

- [x] internal raw rendering foundation
- [x] internal scene and command model
- [x] internal frame and surface abstractions

These parts exist in the codebase, but they are not part of the stable top-level public API in v0.1.

### Not implemented yet

- [ ] transparent host window mode
- [ ] frameless or overlay-style host window mode
- [ ] placement and desktop work-area integration
- [ ] multi-widget orchestration
- [ ] stable public raw rendering API
- [ ] declarative UI layer
- [ ] advanced pointer input
- [ ] keyboard input model
- [ ] GPU renderer backends
- [ ] Tauri integration
- [ ] hybrid native/web widget composition

## Intended End-State Direction

The long-term direction of the project includes the following capabilities.

### Planned public architecture

- [x] `Widget`
- [x] `Canvas`
- [x] `WindowsHost`
- [x] `WidgetApp`
- [ ] stable public raw rendering API
- [ ] layout primitives
- [ ] declarative UI layer
- [ ] desktop capability modules
- [ ] Tauri bridge
- [ ] hybrid host model

### Planned runtime direction

- [x] lifecycle-driven widget runtime
- [x] scheduler abstraction
- [x] task abstraction
- [x] optional Tokio adapter
- [ ] stronger scheduler internals
- [ ] stronger stale-instance protection
- [ ] broader input model
- [ ] multi-widget runtime model

### Planned rendering direction

- [x] stable `Canvas`
- [x] software renderer
- [x] internal raw rendering layer
- [ ] stable raw rendering API
- [ ] richer text pipeline
- [ ] image pipeline beyond placeholder commands
- [ ] GPU rendering backend

## Workspace Layout

```text
widgetkit/
  crates/
    widgetkit
    widgetkit-core
    widgetkit-runtime
    widgetkit-render
    widgetkit-host-windows
```

### `widgetkit`
Top-level facade crate. Re-exports the stable public API and feature-gated entry points.

### `widgetkit-core`
Shared primitives and contracts:
- error and result types
- geometry
- colors
- ids
- host events

### `widgetkit-runtime`
Runtime orchestration:
- widget lifecycle
- `WidgetApp`
- scheduler
- task abstraction
- event routing

### `widgetkit-render`
Rendering stack:
- stable `Canvas`
- style types
- software renderer
- internal raw rendering foundation

### `widgetkit-host-windows`
Windows-specific host implementation using `winit` and `softbuffer`.

## Public API Model

The current stable public path is:

```rust
Widget + Canvas + WindowsHost + WidgetApp
```

Typical startup path:

```rust
use widgetkit::prelude::*;

fn main() -> widgetkit::Result<()> {
    WidgetApp::new()
        .widget("clock", ClockWidget)
        .host(WindowsHost::new())
        .renderer(SoftwareRenderer::new())
        .run()
}
```

## Lifecycle

A widget implements the following lifecycle methods:

- `mount`
- `start`
- `update`
- `render`
- `stop`
- `dispose`

The current runtime model handles a single widget instance. Broader orchestration is not implemented yet.

## Event Model

The public event model is intentionally small:

```rust
Event::Message(M)
Event::Host(HostEvent)
```

Scheduler deliveries and task completions are routed internally and surface to widgets as regular `Message(M)` values.

## Rendering Model

Rendering is demand-driven. There is no permanent continuous render loop in the current version.

`Canvas` is the stable drawing surface for widgets. Internal raw rendering primitives already exist, but they remain crate-private and unstable in this version. A stable public raw rendering API is planned for a later stage.

## Features

Top-level features:

- `canvas`
- `windows`
- `runtime-tokio`

Example dependency setup:

```toml
[dependencies]
widgetkit = { version = "0.1.0", default-features = false, features = ["windows", "canvas"] }
```

To enable the optional Tokio-backed task runtime:

```toml
[dependencies]
widgetkit = { version = "0.1.0", default-features = false, features = ["windows", "canvas", "runtime-tokio"] }
```

## Example

The workspace includes a local `clock` example.

Run it with:

```bash
cargo run --example clock --features "windows canvas"
```

Or with the optional Tokio task backend enabled:

```bash
cargo run --example clock --features "windows canvas runtime-tokio"
```

## Writing a Widget

A widget defines a state type and a message type, then implements `Widget`.

```rust
use widgetkit::prelude::*;

struct MyWidget;

struct MyState {
    text: String,
}

#[derive(Clone, Debug)]
enum Msg {
    Tick,
}

impl Widget for MyWidget {
    type State = MyState;
    type Message = Msg;

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {
        MyState {
            text: "Initial".to_string(),
        }
    }

    fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
        ctx.scheduler().every(Duration::from_secs(1), Msg::Tick);
    }

    fn update(
        &mut self,
        state: &mut Self::State,
        event: Event<Self::Message>,
        ctx: &mut UpdateCtx<Self>,
    ) {
        match event {
            Event::Message(Msg::Tick) => {
                state.text = "Updated".to_string();
                ctx.request_render();
            }
            Event::Host(_) => {}
        }
    }

    fn render(&self, state: &Self::State, canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {
        canvas.clear(Color::BLACK);
        canvas.text(
            Point::new(16.0, 16.0),
            &state.text,
            TextStyle::new().size(16.0),
            Color::WHITE,
        );
    }

    fn stop(&mut self, _state: &mut Self::State, ctx: &mut StopCtx<Self>) {
        ctx.scheduler().clear();
        ctx.tasks().cancel_all();
    }
}
```

## Stability Notes

### Stable in v0.1
- `Widget`
- `WidgetApp`
- `Canvas`
- `WindowsHost`
- `SoftwareRenderer`
- lifecycle method order
- scheduler and task access through widget contexts

### Internal or unstable in v0.1
- raw render internals
- scene and command structures
- frame and surface internals
- task backend implementation details
- host internals

## Current Implementation Notes

Current implementation details:

- Windows host
- normal decorated debug window
- software 2D rendering
- single-widget runtime
- demand-driven redraw
- raw rendering internals kept unstable

## License

MIT
