# WidgetKit

---

language: [English](README.md), Russian

---
[![Version](https://img.shields.io/badge/version-0.2.1-blue)](#)
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)](#)
[![Renderer](https://img.shields.io/badge/renderer-software%202D-lightgrey)](#)
[![Status](https://img.shields.io/badge/status-render%20v0.2-green)](#)

Модульная Rust-библиотека для создания desktop-виджетов.

Текущий охват: software 2D рендеринг на Windows с demand-driven моделью перерисовки и внутренним render pipeline
под публичным `Canvas` API.

Базовая сборка: `Widget + Canvas + WindowsHost + WidgetApp`.

Подробная история релизов в [CHANGELOG.md](CHANGELOG.md).

## Обзор

- `Canvas` - публичный drawing API
- рендеринг проходит через внутренний command pipeline
- runtime управляет lifecycle виджета, scheduler, tasks и redraw invalidation
- Windows host поддерживает decorated и frameless окна

## Быстрый старт

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

## Пример виджета

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

## Примеры

```bash
cargo run --example clock --features "windows canvas"
cargo run --example pulse --features "windows canvas"
```

С Tokio-backed task runtime:

```bash
cargo run --example clock --features "windows canvas runtime-tokio"
```

## Features

- `canvas`
- `windows`
- `runtime-tokio`

```toml
[dependencies]
widgetkit = { version = "0.2.1", default-features = false, features = ["windows", "canvas"] }
```

## Структура workspace

```text
widgetkit/
  crates/
    widgetkit
    widgetkit-core
    widgetkit-runtime
    widgetkit-render
    widgetkit-host-windows
```

- `widgetkit` - верхний facade crate
- `widgetkit-core` - геометрия, цвета, ids, ошибки, host events
- `widgetkit-runtime` - lifecycle, scheduler, tasks, redraw coordination
- `widgetkit-render` - `Canvas`, text styles, software renderer
- `widgetkit-host-windows` - Windows host на базе `winit` и `softbuffer`

## Roadmap

- стабильные API для размеров и конфигурации окна
- расширенные image и text pipeline
- declarative UI и layout
- расширенная input model
- GPU renderer backend
- hybrid и web-backed integration paths

## License

MIT