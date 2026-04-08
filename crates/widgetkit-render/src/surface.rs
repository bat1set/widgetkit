use crate::{Canvas, RenderFrame, frame::Frame, raster::Rasterizer};
use widgetkit_core::{Color, Error, Result};

pub trait RenderSurface {
    fn size(&self) -> (u32, u32);
    fn present(&mut self, pixels: &[Color]) -> Result<()>;
}

pub trait Renderer: Send {
    fn render_frame(&mut self, frame: RenderFrame, surface: &mut dyn RenderSurface) -> Result<()>;

    fn render_canvas(&mut self, canvas: Canvas, surface: &mut dyn RenderSurface) -> Result<()> {
        self.render_frame(canvas.into_frame(), surface)
    }
}

pub struct SoftwareRenderer {
    pixels: Vec<Color>,
}

impl SoftwareRenderer {
    pub fn new() -> Self {
        Self { pixels: Vec::new() }
    }

    fn ensure_buffer(&mut self, width: u32, height: u32) -> Result<()> {
        let len = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().map(|h| w.saturating_mul(h)))
            .ok_or_else(|| Error::render("surface dimensions exceed addressable memory"))?;
        if self.pixels.len() != len {
            self.pixels.resize(len, Color::TRANSPARENT);
        }
        Ok(())
    }
}

impl Default for SoftwareRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for SoftwareRenderer {
    fn render_frame(
        &mut self,
        render_frame: RenderFrame,
        surface: &mut dyn RenderSurface,
    ) -> Result<()> {
        let (width, height) = surface.size();
        self.ensure_buffer(width, height)?;
        let surface_frame = Frame::new(width, height, &mut self.pixels);
        let mut raster = Rasterizer::new(surface_frame);
        raster.execute(render_frame);
        drop(raster);
        surface.present(&self.pixels)
    }
}
