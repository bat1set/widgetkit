#![cfg(target_os = "windows")]

use std::num::NonZeroU32;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use widgetkit_core::{Color, Error, HostEvent, Result, Size};
use widgetkit_render::RenderSurface;
use widgetkit_runtime::{AppRunner, HostRunner, Widget};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

pub struct WindowsHost {
    size: Size,
}

impl WindowsHost {
    pub fn new() -> Self {
        Self {
            size: Size::new(320.0, 120.0),
        }
    }

    pub fn with_size(mut self, size: Size) -> Self {
        if !size.is_empty() {
            self.size = size;
        }
        self
    }
}

impl Default for WindowsHost {
    fn default() -> Self {
        Self::new()
    }
}

impl<W, R> HostRunner<W, R> for WindowsHost
where
    W: Widget,
    R: widgetkit_render::Renderer,
{
    fn run(self, mut runner: AppRunner<W, R>) -> Result<()> {
        let event_loop = EventLoop::<HostUserEvent>::with_user_event()
            .build()
            .map_err(|error| Error::platform(error.to_string()))?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let proxy = event_loop.create_proxy();
        let wake_proxy = proxy.clone();
        runner.attach_waker(move || {
            let _ = wake_proxy.send_event(HostUserEvent::Wake);
        });

        let mut app = WindowsApp::new(self, runner, proxy);
        event_loop
            .run_app(&mut app)
            .map_err(|error| Error::platform(error.to_string()))?;
        if let Some(error) = app.exit_error {
            return Err(error);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum HostUserEvent {
    Wake,
}

struct WindowsApp<W, R>
where
    W: Widget,
    R: widgetkit_render::Renderer,
{
    host: WindowsHost,
    runner: AppRunner<W, R>,
    window: Option<Rc<Window>>,
    surface: Option<SoftbufferSurface>,
    exit_error: Option<Error>,
}

impl<W, R> WindowsApp<W, R>
where
    W: Widget,
    R: widgetkit_render::Renderer,
{
    fn new(host: WindowsHost, runner: AppRunner<W, R>, _proxy: EventLoopProxy<HostUserEvent>) -> Self {
        Self {
            host,
            runner,
            window: None,
            surface: None,
            exit_error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Error) {
        self.exit_error = Some(error);
        let _ = self.runner.shutdown();
        event_loop.exit();
    }

    fn request_redraw_if_needed(&self) {
        if self.runner.needs_redraw() {
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    fn process_runtime(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.runner.process_pending() {
            self.fail(event_loop, error);
            return;
        }
        self.request_redraw_if_needed();
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<Rc<Window>> {
        let attributes: WindowAttributes = Window::default_attributes()
            .with_title(self.runner.widget_name())
            .with_inner_size(LogicalSize::new(self.host.size.width as f64, self.host.size.height as f64));
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| Error::platform(error.to_string()))?;
        Ok(Rc::new(window))
    }
}

impl<W, R> ApplicationHandler<HostUserEvent> for WindowsApp<W, R>
where
    W: Widget,
    R: widgetkit_render::Renderer,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match self.create_window(event_loop) {
            Ok(window) => window,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };

        let surface = match SoftbufferSurface::new(window.clone()) {
            Ok(surface) => surface,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };

        let size = window.inner_size();
        if let Err(error) = self
            .runner
            .initialize(Size::new(size.width.max(1) as f32, size.height.max(1) as f32))
        {
            self.fail(event_loop, error);
            return;
        }

        self.window = Some(window.clone());
        self.surface = Some(surface);
        window.request_redraw();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: HostUserEvent) {
        self.process_runtime(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                if let Err(error) = self.runner.handle_host_event(HostEvent::CloseRequested) {
                    self.fail(event_loop, error);
                    return;
                }
                if let Err(error) = self.runner.shutdown() {
                    self.fail(event_loop, error);
                    return;
                }
                event_loop.exit();
            }
            WindowEvent::Focused(focused) => {
                if let Err(error) = self.runner.handle_host_event(HostEvent::Focused(focused)) {
                    self.fail(event_loop, error);
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::Resized(size) => {
                if let Err(error) = self.runner.handle_host_event(HostEvent::Resized(Size::new(
                    size.width.max(1) as f32,
                    size.height.max(1) as f32,
                ))) {
                    self.fail(event_loop, error);
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::RedrawRequested => {
                let Some(surface) = self.surface.as_mut() else {
                    return;
                };
                if let Err(error) = self.runner.render(surface) {
                    self.fail(event_loop, error);
                    return;
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_runtime(event_loop);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.runner.shutdown() {
            self.fail(event_loop, error);
        }
    }
}

struct SoftbufferSurface {
    window: Rc<Window>,
    context: Context<Rc<Window>>,
    surface: Surface<Rc<Window>, Rc<Window>>,
}

impl SoftbufferSurface {
    fn new(window: Rc<Window>) -> Result<Self> {
        let context = Context::new(window.clone()).map_err(|error| Error::platform(error.to_string()))?;
        let surface = Surface::new(&context, window.clone()).map_err(|error| Error::platform(error.to_string()))?;
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

// TODO(v0.3): placement integration
// TODO(v0.3): monitor/work-area abstraction
// TODO(v0.4): input events routing
// TODO(v0.7): hybrid host compatibility
