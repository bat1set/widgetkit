# WidgetKit

---

language: English, [Russian](README.ru.md)

---
[![Version](https://img.shields.io/badge/version-0.2.0-blue)](#)
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)](#)
[![Renderer](https://img.shields.io/badge/renderer-software%202D-lightgrey)](#)
[![Status](https://img.shields.io/badge/status-render%20v0.2-green)](#)

WidgetKit is a modular Rust library for building desktop widgets.

It currently focuses on a simple but structured native path:

- `Widget + Canvas + WindowsHost + WidgetApp`
- software 2D rendering on Windows
- demand-driven redraw
- a clearer render pipeline under the public `Canvas` API

If you want the detailed list of what changed in `v0.2`, see [CHANGELOG.md](CHANGELOG.md).

## What It Feels Like

WidgetKit is still intentionally small, but it already has a usable shape:

- `Canvas` is the main public drawing API
- rendering goes through `RenderFrame` and `RenderCommand`
- the runtime owns widget lifecycle, scheduler, tasks, and redraw invalidation
- the Windows host can run decorated or frameless windows

## Quick Start

```rust
use widgetkit::prelude::*;

fn main() -> widgetkit::Result<()> {
    WidgetApp::new()
        .widget("clock", ClockWidget)
        .host(
            WindowsHost::new()
                .with_size(Size::new(360.0, 135.0))
                .with_standard_top_bar(true),
        )
        .renderer(SoftwareRenderer::new())
        .run()
}
```

## Example Widget

```rust
use widgetkit::prelude::*;

struct MyWidget;

impl Widget for MyWidget {
    type State = ();
    type Message = ();

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {}

    fn render(&self, _state: &Self::State, canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {
        canvas.clear(Color::rgb(14, 17, 22));
        canvas.save();
        canvas.clip_rect(Rect::xywh(12.0, 12.0, 220.0, 72.0));
        canvas.round_rect(
            Rect::xywh(12.0, 12.0, 220.0, 72.0),
            16.0,
            Color::rgb(30, 36, 48),
        );
        canvas.text(
            Point::new(24.0, 28.0),
            "Hello",
            TextStyle::new().size(16.0),
            Color::rgb(127, 160, 255),
        );
        canvas.circle(Point::new(206.0, 28.0), 6.0, Color::rgb(127, 160, 255));
        canvas.restore();
    }
}
```

## Examples

The workspace includes:

- `clock`
- `pulse`

Run them with:

```bash
cargo run --example clock --features "windows canvas"
```

```bash
cargo run --example pulse --features "windows canvas"
```

With the optional Tokio-backed task runtime:

```bash
cargo run --example clock --features "windows canvas runtime-tokio"
```

## Features

Top-level features:

- `canvas`
- `windows`
- `runtime-tokio`

Example dependency setup:

```toml
[dependencies]
widgetkit = { version = "0.2.0", default-features = false, features = ["windows", "canvas"] }
```

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

- `widgetkit`: top-level facade crate
- `widgetkit-core`: geometry, colors, ids, errors, host events
- `widgetkit-runtime`: lifecycle, scheduler, tasks, redraw coordination
- `widgetkit-render`: `Canvas`, render commands, text styles, software renderer
- `widgetkit-host-windows`: Windows host based on `winit` and `softbuffer`

## Direction

Planned later:

- stable low-level raw rendering API
- richer image and text pipelines
- declarative UI and layout
- broader input model
- GPU renderer backend
- hybrid or web-backed integration paths

## License

MIT
