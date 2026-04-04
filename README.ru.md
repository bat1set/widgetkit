# WidgetKit

---

language: [English](README.md), Russian

---
[![Version](https://img.shields.io/badge/version-0.1.0-blue)](#)
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)](#)
[![Renderer](https://img.shields.io/badge/renderer-software%202D-lightgrey)](#)
[![Status](https://img.shields.io/badge/status-bootstrap-orange)](#)

WidgetKit - модульная библиотека для создания desktop-виджетов на rust.

Текущая реализация уже включает рабочий Windows host, software 2D renderer, стабильный публичный путь `Widget + Canvas + WindowsHost + WidgetApp` и небольшую runtime-модель, рассчитанную на один экземпляр виджета.

Проект будет развиваться в сторону более широкого desktop widget framework: с улучшенным scheduler, стабилизированным raw rendering API, layout-примитивами, declarative UI, desktop capabilities и Tauri или гибридной интеграции. Часть этого уже есть в раннем виде, часть пока не реализована, а часть остаётся внутренней или нестабильной в текущей версии.

## Текущий статус проекта

### Уже реализовано

- [x] workspace 
- [x] Windows host
- [x] software 2D renderer
- [x] стабильный `Canvas`
- [x] lifecycle виджета: `mount`, `start`, `update`, `render`, `stop`, `dispose`
- [x] instance-scoped scheduler API
- [x] instance-scoped task API
- [x] optional Tokio-backed task runtime
- [x] локальный пример `clock`
- [x] demand-driven redraw

### Уже существует, но пока не стабильно

- [x] raw rendering foundation
- [x] scene/command модель
- [x] frame и surface abstractions

Эти части уже есть в кодовой базе, но пока не входят в стабильный верхнеуровневый публичный API v0.1.

### Пока не реализовано

- [ ] transparent host window mode
- [ ] frameless или overlay-style host window mode
- [ ] placement и интеграция с рабочей областью desktop
- [ ] orchestration для нескольких виджетов
- [ ] стабильный публичный raw rendering API
- [ ] declarative UI layer
- [ ] advanced pointer input
- [ ] keyboard input model
- [ ] GPU renderer backends
- [ ] Tauri integration
- [ ] hybrid native/web widget composition

## Направление итоговой реализации

Долгосрочное направление проекта включает следующие возможности.

### Планируемая публичная архитектура

- [x] `Widget`
- [x] `Canvas`
- [x] `WindowsHost`
- [x] `WidgetApp`
- [ ] стабильный публичный raw rendering API
- [ ] layout primitives
- [ ] declarative UI layer
- [ ] desktop capability modules
- [ ] Tauri bridge
- [ ] hybrid host model

### Планируемое развитие runtime

- [x] lifecycle-driven widget runtime
- [x] scheduler abstraction
- [x] task abstraction
- [x] optional Tokio adapter
- [ ] внутренняя реализация scheduler
- [ ] stale-instance protection
- [ ] input model
- [ ] multi-widget runtime model

### Планируемое развитие рендера

- [x] стабильный `Canvas`
- [x] software renderer
- [x] внутренний raw rendering layer
- [ ] стабильный raw rendering API
- [ ] text pipeline
- [ ] image pipeline шире placeholder-команд
- [ ] GPU rendering backend

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

### `widgetkit`
Верхний facade crate. Реэкспортирует стабильный публичный API и feature-gated entry points.

### `widgetkit-core`
Общие примитивы и контракты:
- ошибки и result types
- геометрия
- цвета
- идентификаторы
- host events

### `widgetkit-runtime`
Runtime orchestration:
- lifecycle виджета
- `WidgetApp`
- scheduler
- task abstraction
- event routing

### `widgetkit-render`
Стек рендера:
- стабильный `Canvas`
- style types
- software renderer
- внутренний raw rendering foundation

### `widgetkit-host-windows`
Windows-specific host implementation на базе `winit` и `softbuffer`.

## Модель публичного API

Текущий стабильный публичный путь:

```
Widget + Canvas + WindowsHost + WidgetApp
```

Типовой запуск:

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

Виджет реализует следующие методы lifecycle:

- `mount`
- `start`
- `update`
- `render`
- `stop`
- `dispose`

Текущая runtime-модель работает с одним экземпляром виджета. Более широкая orchestration-модель пока не реализована.

## Event Model

Публичная модель событий намеренно небольшая:

```rust
Event::Message(M)
Event::Host(HostEvent)
```

Срабатывания scheduler и завершения задач маршрутизируются внутри runtime и попадают в виджет как обычные `Message(M)`.

## Rendering Model

Рендер demand-driven. Постоянного непрерывного render loop в текущей версии нет.

`Canvas` — стабильная поверхность рисования для виджетов. Внутренние raw rendering primitives уже существуют, но в этой версии остаются crate-private и нестабильными. Стабильный публичный raw rendering API планируется позже.

## Features

Верхнеуровневые features:

- `canvas`
- `windows`
- `runtime-tokio`

Пример подключения:

```toml
[dependencies]
widgetkit = { version = "0.1.0", default-features = false, features = ["windows", "canvas"] }
```

Чтобы включить optional Tokio-backed task runtime:

```toml
[dependencies]
widgetkit = { version = "0.1.0", default-features = false, features = ["windows", "canvas", "runtime-tokio"] }
```

## Пример

В workspace есть локальный пример `clock`.

Запуск:

```bash
cargo run --example clock --features "windows canvas"
```

Или с optional Tokio backend:

```bash
cargo run --example clock --features "windows canvas runtime-tokio"
```

## Как написать свой виджет

Виджет задаёт тип состояния и тип сообщений, затем реализует `Widget`.

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

## Примечания по стабильности

### Стабильно в v0.1
- `Widget`
- `WidgetApp`
- `Canvas`
- `WindowsHost`
- `SoftwareRenderer`
- порядок lifecycle методов
- доступ к scheduler и tasks через widget contexts

### Internal или unstable в v0.1
- raw render internals
- scene и command structures
- frame и surface internals
- детали реализации task backend
- host internals

## Текущие примечания по реализации

Сейчас в реализации есть:

- Windows host
- обычное decorated debug window
- software 2D rendering
- single-widget runtime
- demand-driven redraw
- raw rendering internals остаются нестабильными

## License

MIT
