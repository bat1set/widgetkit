use crate::surface::SoftbufferSurface;
use std::rc::Rc;
use widgetkit_core::{Error, HostEvent, Result, Size};
use widgetkit_runtime::{AppRunner, HostRunner, Widget};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const DEFAULT_WINDOW_SIZE: Size = Size::new(320.0, 120.0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowConfig {
    pub size: Option<Size>,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub resizable: bool,
    pub frameless: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub visible: bool,
}

impl WindowConfig {
    fn normalized(mut self) -> Self {
        self.size = valid_size(self.size);
        self.min_size = valid_size(self.min_size);
        self.max_size = valid_size(self.max_size);
        self
    }

    fn resolved_size(self) -> Size {
        self.size.unwrap_or(DEFAULT_WINDOW_SIZE)
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            size: Some(DEFAULT_WINDOW_SIZE),
            min_size: None,
            max_size: None,
            resizable: true,
            frameless: false,
            transparent: false,
            always_on_top: false,
            visible: true,
        }
    }
}

pub struct WindowsHost {
    config: WindowConfig,
}

impl WindowsHost {
    pub fn new() -> Self {
        Self {
            config: WindowConfig::default(),
        }
    }

    pub fn with_config(mut self, config: WindowConfig) -> Self {
        self.config = config.normalized();
        self
    }

    pub fn config(&self) -> &WindowConfig {
        &self.config
    }

    pub fn with_size(mut self, size: Size) -> Self {
        if !size.is_empty() {
            self.config.size = Some(size);
        }
        self
    }

    pub fn with_standard_top_bar(mut self, visible: bool) -> Self {
        self.config.frameless = !visible;
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

        let mut app = WindowsApp::new(self, runner);
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
    fn new(host: WindowsHost, runner: AppRunner<W, R>) -> Self {
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

    fn request_redraw_if_needed(&mut self) {
        if self.runner.take_redraw_request() {
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
        let config = self.host.config.normalized();
        let size = config.resolved_size();
        let attributes: WindowAttributes = Window::default_attributes()
            .with_title(self.runner.widget_name())
            .with_inner_size(LogicalSize::new(size.width as f64, size.height as f64))
            .with_decorations(!config.frameless);
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
        if let Err(error) = self.runner.initialize(Size::new(
            size.width.max(1) as f32,
            size.height.max(1) as f32,
        )) {
            self.fail(event_loop, error);
            return;
        }

        self.window = Some(window);
        self.surface = Some(surface);
        self.request_redraw_if_needed();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: HostUserEvent) {
        self.process_runtime(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
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

// TODO(v0.3): apply min/max/resizable/transparent/always-on-top/visible WindowConfig flags
// TODO(v0.3): add SizePolicy-driven window sizing
// TODO(v0.3): add initial anchor/offset positioning groundwork
// TODO(v0.6): integrate reserved/work-area awareness
// TODO(v0.4): input events routing
// TODO(v0.7): hybrid host compatibility

fn valid_size(size: Option<Size>) -> Option<Size> {
    size.filter(|size| !size.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{WindowConfig, WindowsHost};
    use widgetkit_core::Size;

    #[test]
    fn windows_host_defaults_to_window_config() {
        let host = WindowsHost::new();

        assert_eq!(host.config(), &WindowConfig::default());
    }

    #[test]
    fn windows_host_uses_window_config() {
        let config = WindowConfig {
            size: Some(Size::new(400.0, 240.0)),
            min_size: Some(Size::new(280.0, 120.0)),
            max_size: Some(Size::new(640.0, 360.0)),
            resizable: false,
            frameless: true,
            transparent: true,
            always_on_top: true,
            visible: false,
        };

        let host = WindowsHost::new().with_config(config);

        assert_eq!(host.config(), &config);
    }

    #[test]
    fn windows_host_size_builder_updates_window_config() {
        let host = WindowsHost::new().with_size(Size::new(400.0, 240.0));

        assert_eq!(host.config().size, Some(Size::new(400.0, 240.0)));
    }

    #[test]
    fn windows_host_can_disable_standard_top_bar_through_config() {
        let host = WindowsHost::new()
            .with_size(Size::new(400.0, 240.0))
            .with_standard_top_bar(false);

        assert_eq!(host.config().size, Some(Size::new(400.0, 240.0)));
        assert!(host.config().frameless);
    }
}
