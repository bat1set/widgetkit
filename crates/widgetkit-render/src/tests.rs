use crate::frame::Frame;
use crate::model::RenderFrame;
use crate::raster::Rasterizer;
use crate::{
    Canvas, ClearCommand, ClipCommand, ClipPrimitive, Fill, FillCommand, FillShape, Paint,
    RawCanvas, RenderCommand, RenderSurface, SoftwareRenderer, StateCommand, TextAlign,
    TextBaseline, TextStyle, Transform, TransformCommand,
};
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
    canvas.save();
    canvas.clip_rect(Rect::xywh(2.0, 2.0, 10.0, 10.0));
    canvas.translate(4.0, 3.0);
    canvas.rect(Rect::xywh(0.0, 0.0, 8.0, 8.0), Color::rgb(255, 0, 0));
    canvas.restore();
    canvas.text(
        Point::new(2.0, 20.0),
        "ok",
        TextStyle::new().baseline(TextBaseline::Alphabetic),
        Color::WHITE,
    );

    let frame = canvas.into_frame();
    assert_eq!(frame.size(), Size::new(64.0, 64.0));
    assert_eq!(frame.commands().len(), 7);
    assert!(matches!(
        frame.commands()[0],
        RenderCommand::Clear(ClearCommand { .. })
    ));
    assert!(matches!(
        frame.commands()[1],
        RenderCommand::State(StateCommand::Save)
    ));
    assert!(matches!(
        frame.commands()[2],
        RenderCommand::Clip(ClipCommand {
            primitive: ClipPrimitive::Rect(_)
        })
    ));
    assert!(matches!(
        frame.commands()[3],
        RenderCommand::Transform(TransformCommand { .. })
    ));
    assert!(matches!(
        frame.commands()[4],
        RenderCommand::Fill(FillCommand {
            shape: FillShape::Rect(_),
            ..
        })
    ));
    assert!(matches!(
        frame.commands()[5],
        RenderCommand::State(StateCommand::Restore)
    ));
    assert!(matches!(frame.commands()[6], RenderCommand::Text(_)));
}

#[test]
fn renderer_consumes_render_frame_contract() {
    let mut canvas = Canvas::new(Size::new(64.0, 64.0));
    canvas.clear(Color::BLACK);
    canvas.rect(Rect::xywh(4.0, 4.0, 20.0, 10.0), Color::rgb(255, 0, 0));
    canvas.round_rect(
        Rect::xywh(30.0, 4.0, 20.0, 20.0),
        6.0,
        Color::rgb(0, 255, 0),
    );
    canvas.ellipse(Point::new(16.0, 40.0), 10.0, 6.0, Color::rgb(255, 200, 0));
    canvas.text(
        Point::new(32.0, 56.0),
        "Hi",
        TextStyle::new()
            .size(16.0)
            .align(TextAlign::Center)
            .baseline(TextBaseline::Bottom),
        Color::WHITE,
    );

    let mut renderer = SoftwareRenderer::new();
    let mut surface = MemorySurface::new(64, 64);
    crate::Renderer::render_frame(&mut renderer, canvas.into_frame(), &mut surface).unwrap();

    assert_eq!(surface.pixels.len(), 64 * 64);
    assert_eq!(surface.pixels[0], Color::BLACK);
    assert_eq!(surface.pixels[5 * 64 + 5], Color::rgb(255, 0, 0));
    assert_eq!(surface.pixels[10 * 64 + 40], Color::rgb(0, 255, 0));
    assert_eq!(surface.pixels[40 * 64 + 8], Color::rgb(255, 200, 0));
    assert!(surface.pixels.iter().any(|pixel| *pixel == Color::WHITE));
}

#[test]
fn image_placeholder_marks_target_area() {
    let mut canvas = Canvas::new(Size::new(32.0, 32.0));
    canvas.clear(Color::BLACK);
    canvas.image_placeholder(Rect::xywh(4.0, 4.0, 20.0, 20.0), Color::WHITE);

    let mut renderer = SoftwareRenderer::new();
    let mut surface = MemorySurface::new(32, 32);
    crate::Renderer::render_frame(&mut renderer, canvas.into_frame(), &mut surface).unwrap();

    assert_eq!(surface.pixels[4 * 32 + 4], Color::WHITE);
    assert_eq!(surface.pixels[23 * 32 + 23], Color::WHITE);
}

#[test]
fn canvas_save_restore_clip_and_translation_are_applied() {
    let mut canvas = Canvas::new(Size::new(24.0, 24.0));
    canvas.clear(Color::BLACK);
    canvas.save();
    canvas.clip_rect(Rect::xywh(6.0, 6.0, 6.0, 6.0));
    canvas.translate(4.0, 0.0);
    canvas.rect(Rect::xywh(4.0, 6.0, 8.0, 8.0), Color::WHITE);
    canvas.restore();
    canvas.rect(Rect::xywh(0.0, 0.0, 2.0, 2.0), Color::rgb(255, 0, 0));

    let mut renderer = SoftwareRenderer::new();
    let mut surface = MemorySurface::new(24, 24);
    crate::Renderer::render_frame(&mut renderer, canvas.into_frame(), &mut surface).unwrap();

    assert_eq!(surface.pixels[7 * 24 + 9], Color::WHITE);
    assert_eq!(surface.pixels[7 * 24 + 5], Color::BLACK);
    assert_eq!(surface.pixels[0], Color::rgb(255, 0, 0));
}

#[test]
fn measure_text_reports_layout_metrics() {
    let canvas = Canvas::new(Size::new(64.0, 64.0));
    let metrics = canvas.measure_text(
        "AB\nC",
        &TextStyle::new()
            .size(16.0)
            .line_height(20.0)
            .align(TextAlign::Center)
            .baseline(TextBaseline::Alphabetic),
    );

    assert_eq!(metrics.width, 32.0);
    assert_eq!(metrics.height, 40.0);
    assert_eq!(metrics.line_height, 20.0);
    assert_eq!(metrics.baseline, 14.0);
    assert_eq!(metrics.line_count, 2);
}

#[test]
fn clipped_text_draw_stays_inside_the_clip_region() {
    let mut canvas = Canvas::new(Size::new(24.0, 24.0));
    canvas.clear(Color::BLACK);
    canvas.save();
    canvas.clip_rect(Rect::xywh(8.0, 8.0, 4.0, 4.0));
    canvas.text(Point::new(8.0, 8.0), "A", TextStyle::new(), Color::WHITE);
    canvas.restore();

    let mut renderer = SoftwareRenderer::new();
    let mut surface = MemorySurface::new(24, 24);
    crate::Renderer::render_frame(&mut renderer, canvas.into_frame(), &mut surface).unwrap();

    let mut white_inside_clip = 0usize;
    let mut white_outside_clip = 0usize;

    for y in 0..24 {
        for x in 0..24 {
            if surface.pixels[y * 24 + x] != Color::WHITE {
                continue;
            }

            if (8..12).contains(&x) && (8..12).contains(&y) {
                white_inside_clip += 1;
            } else {
                white_outside_clip += 1;
            }
        }
    }

    assert!(white_inside_clip > 0);
    assert_eq!(white_outside_clip, 0);
}

#[test]
fn experimental_raw_sink_can_emit_low_level_commands() {
    let mut canvas = Canvas::new(Size::new(16.0, 16.0));
    canvas.experimental_raw(|raw: &mut RawCanvas<'_>| {
        raw.clear(Color::BLACK);
        raw.save();
        raw.clip_rect(Rect::xywh(4.0, 4.0, 4.0, 4.0));
        raw.translate(2.0, 0.0);
        raw.fill_rect(Rect::xywh(2.0, 4.0, 4.0, 4.0), Color::WHITE);
        raw.restore();
    });

    let frame = canvas.into_frame();
    assert_eq!(frame.commands().len(), 6);

    let mut pixels = vec![Color::TRANSPARENT; 16 * 16];
    let surface_frame = Frame::new(16, 16, &mut pixels);
    let mut raster = Rasterizer::new(surface_frame);
    raster.execute(frame);
    drop(raster);

    assert_eq!(pixels[5 * 16 + 5], Color::WHITE);
    assert_eq!(pixels[5 * 16 + 3], Color::BLACK);
    assert_eq!(pixels[5 * 16 + 8], Color::BLACK);
}

#[test]
fn manual_render_frame_executes_raw_commands() {
    let frame = RenderFrame::new(
        Size::new(16.0, 16.0),
        vec![
            RenderCommand::Clear(ClearCommand {
                paint: Paint::solid(Color::BLACK),
            }),
            RenderCommand::State(StateCommand::Save),
            RenderCommand::Clip(ClipCommand {
                primitive: ClipPrimitive::Rect(Rect::xywh(4.0, 4.0, 4.0, 4.0)),
            }),
            RenderCommand::Transform(TransformCommand {
                transform: Transform::translation(2.0, 0.0),
            }),
            RenderCommand::Fill(FillCommand {
                shape: FillShape::Rect(Rect::xywh(2.0, 4.0, 4.0, 4.0)),
                fill: Fill::solid(Color::WHITE),
            }),
            RenderCommand::State(StateCommand::Restore),
        ],
    );

    let mut pixels = vec![Color::TRANSPARENT; 16 * 16];
    let surface_frame = Frame::new(16, 16, &mut pixels);

    let mut raster = Rasterizer::new(surface_frame);
    raster.execute(frame);
    drop(raster);

    assert_eq!(pixels[5 * 16 + 5], Color::WHITE);
    assert_eq!(pixels[5 * 16 + 3], Color::BLACK);
    assert_eq!(pixels[5 * 16 + 8], Color::BLACK);
}
