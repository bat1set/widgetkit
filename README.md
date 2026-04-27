# WidgetKit

---

language: English, [Russian](README.ru.md)

---
[![Version](https://img.shields.io/badge/version-0.3.0-blue)](#)
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)](#)
[![Renderer](https://img.shields.io/badge/renderer-software%202D-lightgrey)](#)
[![Status](https://img.shields.io/badge/status-window%20%2B%20layout%20foundations-green)](#)

Modular Rust library for building small desktop widgets.

Current scope: software 2D rendering on Windows with window configuration, content-driven sizing,
and layout measurement foundations behind the public `Canvas` API.

Core assembly: `Widget + Canvas + WindowsHost + WidgetApp`.

For the full release history, see [CHANGELOG.md](CHANGELOG.md).

## Overview

- `Canvas` — stable drawing API
- demand-driven redraw and layout invalidation
- `WindowConfig` and Windows host flags for frameless, transparent, resizable, visible, and always-on-top windows
- `SizePolicy` with fixed and content-driven sizing
- `LayoutCtx::measure_text(...)` for preferred-size calculations
- software 2D renderer backed by an internal render pipeline

## Quick Start

```rust
use widgetkit::prelude::*;

fn main() -> widgetkit::Result<()> {
    WidgetApp::new()
        .widget("clock", ClockWidget)
        .host(
            WindowsHost::new()
                .size_policy(SizePolicy::ContentWithLimits {
                    min: Some(Size::new(280.0, 112.0)),
                    max: Some(Size::new(520.0, 180.0)),
                })
                .frameless(true)
                .transparent(true)
                .always_on_top(true),
        )
        .renderer(SoftwareRenderer::new())
        .run()
}
```

## Example Widget

```rust
use widgetkit::prelude::*;

struct MyWidget;

struct MyState {
    label: String,
}

impl Widget for MyWidget {
    type State = MyState;
    type Message = ();

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {
        MyState {
            label: "Hello WidgetKit".to_string(),
        }
    }

    fn preferred_size(&self, state: &Self::State, ctx: &LayoutCtx<Self>) -> Size {
        let label = ctx.measure_text(&state.label, TextStyle::new().size(16.0));
        ctx.constrain(Size::new(label.width + 48.0, label.height + 48.0))
    }

    fn render(&self, state: &Self::State, canvas: &mut Canvas, ctx: &RenderCtx<Self>) {
        let size = ctx.surface_size();

        canvas.clear(Color::rgba(0, 0, 0, 0));
        canvas.round_rect(
            Rect::xywh(8.0, 8.0, size.width - 16.0, size.height - 16.0),
            16.0,
            Color::rgb(30, 36, 48),
        );
        canvas.text(
            Point::new(24.0, 28.0),
            &state.label,
            TextStyle::new().size(16.0),
            Color::rgb(127, 160, 255),
        );
    }
}
```

## Examples

```bash
cargo run --example clock --features "windows canvas"
cargo run --example pulse --features "windows canvas"
```

With the Tokio-backed task runtime:

```bash
cargo run --example clock --features "windows canvas runtime-tokio"
```

## Features

- `canvas`
- `windows`
- `runtime-tokio`

```toml
[dependencies]
widgetkit = { version = "0.3.0", default-features = false, features = ["windows", "canvas"] }
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

- `widgetkit` - top-level facade crate
- `widgetkit-core` - geometry, colors, ids, errors, host events
- `widgetkit-runtime` - lifecycle, scheduler, tasks, layout/redraw coordination
- `widgetkit-render` - `Canvas`, text styles, measurement, software renderer
- `widgetkit-host-windows` - Windows host built on `winit` and `softbuffer`

## Roadmap

- declarative UI and layout
- richer input model
- image pipeline stabilization
- GPU renderer backend
- hybrid and web-backed integration paths

## License

MIT