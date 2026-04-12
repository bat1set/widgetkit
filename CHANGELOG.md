# Changelog

All notable changes to this project will be documented in this file.

## [0.2.1] - 2026-04-13

### Changed

- narrowed the top-level `widgetkit` facade and prelude back to the stable `Canvas`-first API
- moved raw render commands, frame types, and `RawCanvas` out of the facade route
- documented raw command/frame internals as `widgetkit_render::unstable`
- documented the redraw invalidation model in runtime crate docs and `docs/invalidation.md`
- updated workspace and crate dependency versions to `0.2.1`

### Notes

- `Canvas` remains the recommended drawing API
- `Canvas::experimental_raw(...)` remains available through `widgetkit_render`, but its command sink is explicitly
  unstable
- `RenderFrame` and `RenderCommand` are no longer promoted by `widgetkit::prelude`

## [0.2.0] - 2026-04-10

### Added

- explicit render pipeline built around `RenderFrame` and `RenderCommand`
- richer `Canvas` primitives: `circle`, `ellipse`, `clip_rect`, `save`, `restore`, and `translate`
- `Canvas::experimental_raw(...)` as an explicit low-level escape hatch
- text measurement primitives through `TextMetrics`
- text alignment, baseline, and line-height styling support
- public render-frame and render-command re-exports in the top-level crate
- frameless Windows host configuration through `with_standard_top_bar(false)`
- `pulse` example demonstrating repeated redraw and richer canvas primitives
- render tests covering clipped text drawing and the command pipeline

### Changed

- normalized the internal render model around a dedicated command list and frame contract
- clarified the renderer boundary so software rendering consumes `RenderFrame`
- moved text drawing onto a shared layout and rasterization path
- made redraw invalidation demand-driven and coalesced repeated requests before the next frame
- updated README files to match the `v0.2.0` public surface and examples
- bumped workspace and crate versions from `0.1.0` to `0.2.0`

### Notes

- `Canvas` remains the primary public API
- low-level drawing is available, but still considered evolving rather than fully stabilized
- software rendering is still the only backend shipped in this release

## [0.1.0] - 2026-04-04

### Added

- initial workspace layout
- Windows host based on `winit` and `softbuffer`
- software 2D renderer
- stable `Widget + Canvas + WindowsHost + WidgetApp` public path
- lifecycle-driven runtime for a single widget instance
- instance-scoped scheduler and task APIs
- demand-driven redraw model
- initial `clock` example
