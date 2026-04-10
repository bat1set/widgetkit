use softbuffer::{Context, Surface};
use std::{num::NonZeroU32, rc::Rc};
use widgetkit_core::{Color, Error, Result};
use widgetkit_render::RenderSurface;
use winit::window::Window;

pub(crate) struct SoftbufferSurface {
    window: Rc<Window>,
    context: Context<Rc<Window>>,
    surface: Surface<Rc<Window>, Rc<Window>>,
}

impl SoftbufferSurface {
    pub(crate) fn new(window: Rc<Window>) -> Result<Self> {
        let context =
            Context::new(window.clone()).map_err(|error| Error::platform(error.to_string()))?;
        let surface = Surface::new(&context, window.clone())
            .map_err(|error| Error::platform(error.to_string()))?;
        Ok(Self {
            window,
            context,
            surface,
        })
    }
}

impl RenderSurface for SoftbufferSurface {
    fn size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width.max(1), size.height.max(1))
    }

    fn present(&mut self, pixels: &[Color]) -> Result<()> {
        let _ = &self.context;
        let (width, height) = self.size();
        let Some(non_zero_width) = NonZeroU32::new(width.max(1)) else {
            return Ok(());
        };
        let Some(non_zero_height) = NonZeroU32::new(height.max(1)) else {
            return Ok(());
        };
        self.surface
            .resize(non_zero_width, non_zero_height)
            .map_err(|error| Error::platform(error.to_string()))?;
        let mut buffer = self
            .surface
            .buffer_mut()
            .map_err(|error| Error::platform(error.to_string()))?;
        for (dst, src) in buffer.iter_mut().zip(pixels.iter().copied()) {
            *dst = pack_color(src);
        }
        buffer
            .present()
            .map_err(|error| Error::platform(error.to_string()))
    }
}

fn pack_color(color: Color) -> u32 {
    let alpha = color.a as f32 / 255.0;
    let red = (color.r as f32 * alpha).round().clamp(0.0, 255.0) as u32;
    let green = (color.g as f32 * alpha).round().clamp(0.0, 255.0) as u32;
    let blue = (color.b as f32 * alpha).round().clamp(0.0, 255.0) as u32;
    (red << 16) | (green << 8) | blue
}
