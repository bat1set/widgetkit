use crate::{Canvas, RenderSurface, SoftwareRenderer, Stroke, TextStyle, raw::Command};
use widgetkit_core::Result;
use widgetkit_core::{Color, Point, Rect, Size};

#[derive(Default)]
struct MemorySurface {
    size: (u32, u32),
    pixels: Vec<Color>,
}

impl MemorySurface {
    fn new(width: u32, height: u32) -> Self {
        Self {
            size: (width, height),
            pixels: Vec::new(),
        }
    }
}

impl RenderSurface for MemorySurface {
    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn present(&mut self, pixels: &[Color]) -> Result<()> {
        self.pixels.clear();
        self.pixels.extend_from_slice(pixels);
        Ok(())
    }
}

#[test]
fn canvas_builds_expected_command_sequence() {
    let mut canvas = Canvas::new(Size::new(64.0, 64.0));
    canvas.clear(Color::BLACK);
    canvas.rect(Rect::xywh(1.0, 2.0, 3.0, 4.0), Color::rgb(255, 0, 0));
    canvas.line(
        Point::new(0.0, 0.0),
        Point::new(5.0, 5.0),
        Stroke::new(2.0),
        Color::WHITE,
    );
    canvas.text(Point::new(2.0, 3.0), "ok", TextStyle::new(), Color::WHITE);

    let scene = canvas.into_scene();
    assert_eq!(scene.commands.len(), 4);
    assert!(matches!(scene.commands[0], Command::Clear { .. }));
    assert!(matches!(scene.commands[1], Command::Rect { .. }));
    assert!(matches!(scene.commands[2], Command::Line { .. }));
    assert!(matches!(scene.commands[3], Command::Text { .. }));
}

#[test]
fn canvas_emits_commands_and_renderer_draws_pixels() {
    let mut canvas = Canvas::new(Size::new(64.0, 64.0));
    canvas.clear(Color::BLACK);
    canvas.rect(Rect::xywh(4.0, 4.0, 20.0, 10.0), Color::rgb(255, 0, 0));
    canvas.round_rect(Rect::xywh(30.0, 4.0, 20.0, 20.0), 6.0, Color::rgb(0, 255, 0));
    canvas.line(
        Point::new(0.0, 63.0),
        Point::new(63.0, 0.0),
        Stroke::new(2.0),
        Color::rgb(0, 0, 255),
    );
    canvas.text(Point::new(4.0, 24.0), "Hi", TextStyle::new().size(16.0), Color::WHITE);

    let mut renderer = SoftwareRenderer::new();
    let mut surface = MemorySurface::new(64, 64);
    crate::Renderer::render_canvas(&mut renderer, canvas, &mut surface).unwrap();

    assert_eq!(surface.pixels.len(), 64 * 64);
    assert_eq!(surface.pixels[0], Color::BLACK);
    assert_eq!(surface.pixels[5 * 64 + 5], Color::rgb(255, 0, 0));
    assert_eq!(surface.pixels[10 * 64 + 40], Color::rgb(0, 255, 0));
    assert_eq!(surface.pixels[63], Color::rgb(0, 0, 255));
    assert!(surface.pixels.iter().any(|pixel| *pixel == Color::WHITE));
}

#[test]
fn image_placeholder_marks_target_area() {
    let mut canvas = Canvas::new(Size::new(32.0, 32.0));
    canvas.clear(Color::BLACK);
    canvas.image_placeholder(Rect::xywh(4.0, 4.0, 20.0, 20.0), Color::WHITE);

    let mut renderer = SoftwareRenderer::new();
    let mut surface = MemorySurface::new(32, 32);
    crate::Renderer::render_canvas(&mut renderer, canvas, &mut surface).unwrap();

    assert_eq!(surface.pixels[4 * 32 + 4], Color::WHITE);
    assert_eq!(surface.pixels[23 * 32 + 23], Color::WHITE);
}
