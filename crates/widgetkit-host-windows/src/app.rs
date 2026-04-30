use crate::surface::SoftbufferSurface;
use std::rc::Rc;
use widgetkit_core::{
    Constraints, Error, HostEvent, Key, KeyboardEvent, MouseButton, MouseEvent, MouseWheelDelta,
    Point, Result, Size, SizePolicy,
};
use widgetkit_runtime::{AppRunner, HostRunner, Widget, WindowCommand};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{
    ButtonSource, ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::run_on_demand::EventLoopExtRunOnDemand;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::Key as WinitKey;
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

const DEFAULT_WINDOW_SIZE: Size = Size::new(320.0, 120.0);
const DEFAULT_OFFSET: Point = Point::new(0.0, 0.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionConfig {
    pub point: Option<Point>,
    pub anchor: Option<Anchor>,
    pub offset: Point,
}

impl PositionConfig {
    pub const fn new() -> Self {
        Self {
            point: None,
            anchor: None,
            offset: DEFAULT_OFFSET,
        }
    }

    pub const fn at(point: Point) -> Self {
        Self {
            point: Some(point),
            anchor: None,
            offset: DEFAULT_OFFSET,
        }
    }

    pub const fn anchored(anchor: Anchor) -> Self {
        Self {
            point: None,
            anchor: Some(anchor),
            offset: DEFAULT_OFFSET,
        }
    }

    pub const fn with_offset(mut self, offset: Point) -> Self {
        self.offset = offset;
        self
    }
}

impl Default for PositionConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowConfig {
    pub size: Option<Size>,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub size_policy: SizePolicy,
    pub resizable: bool,
    pub frameless: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub visible: bool,
    pub position: PositionConfig,
}

impl WindowConfig {
    fn normalized(mut self) -> Self {
        self.size = valid_size(self.size);
        self.min_size = valid_size(self.min_size);
        self.max_size = valid_size(self.max_size);
        self.size_policy = normalize_size_policy(self.size_policy, self.size);
        if let SizePolicy::Fixed(size) = self.size_policy {
            self.size = Some(size);
        }
        if let SizePolicy::ContentWithLimits { min, max } = self.size_policy {
            self.min_size = self.min_size.or(min);
            self.max_size = self.max_size.or(max);
        }
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
            size_policy: SizePolicy::Fixed(DEFAULT_WINDOW_SIZE),
            resizable: true,
            frameless: false,
            transparent: false,
            always_on_top: false,
            visible: true,
            position: PositionConfig::default(),
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
            self.config.size_policy = SizePolicy::Fixed(size);
        }
        self
    }

    pub fn with_min_size(mut self, size: Size) -> Self {
        self.config.min_size = valid_size(Some(size));
        self
    }

    pub fn with_max_size(mut self, size: Size) -> Self {
        self.config.max_size = valid_size(Some(size));
        self
    }

    pub fn size_policy(mut self, size_policy: SizePolicy) -> Self {
        self.config.size_policy = normalize_size_policy(size_policy, self.config.size);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.config.resizable = resizable;
        self
    }

    pub fn frameless(mut self, frameless: bool) -> Self {
        self.config.frameless = frameless;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.config.transparent = transparent;
        self
    }

    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.config.always_on_top = always_on_top;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.config.visible = visible;
        self
    }

    pub fn position(mut self, position: Point) -> Self {
        self.config.position.point = Some(position);
        self.config.position.anchor = None;
        self
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.config.position.point = None;
        self.config.position.anchor = Some(anchor);
        self
    }

    pub fn position_config(mut self, position: PositionConfig) -> Self {
        self.config.position = position;
        self
    }

    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.config.position.offset = Point::new(x, y);
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
        let mut event_loop =
            EventLoop::new().map_err(|error| Error::platform(error.to_string()))?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let proxy = event_loop.create_proxy();
        let wake_proxy = proxy.clone();
        runner.attach_waker(move || {
            wake_proxy.wake_up();
        });

        let mut app = WindowsApp::new(self, runner);
        event_loop
            .run_app_on_demand(&mut app)
            .map_err(|error| Error::platform(error.to_string()))?;
        if let Some(error) = app.exit_error {
            return Err(error);
        }
        Ok(())
    }
}

struct WindowsApp<W, R>
where
    W: Widget,
    R: widgetkit_render::Renderer,
{
    host: WindowsHost,
    runner: AppRunner<W, R>,
    window: Option<Rc<dyn Window>>,
    surface: Option<SoftbufferSurface>,
    exit_error: Option<Error>,
    last_content_size: Option<Size>,
    last_pointer_position: Point,
    window_visible: bool,
}

impl<W, R> WindowsApp<W, R>
where
    W: Widget,
    R: widgetkit_render::Renderer,
{
    fn new(host: WindowsHost, mut runner: AppRunner<W, R>) -> Self {
        let window_visible = host.config.normalized().visible;
        runner.set_window_visible(window_visible);
        Self {
            host,
            runner,
            window: None,
            surface: None,
            exit_error: None,
            last_content_size: None,
            last_pointer_position: Point::new(0.0, 0.0),
            window_visible,
        }
    }

    fn fail(&mut self, event_loop: &dyn ActiveEventLoop, error: Error) {
        self.exit_error = Some(error);
        let _ = self.runner.shutdown();
        event_loop.exit();
    }

    fn handle_host_event(&mut self, event_loop: &dyn ActiveEventLoop, event: HostEvent) -> bool {
        if let Err(error) = self.runner.handle_host_event(event) {
            self.fail(event_loop, error);
            return false;
        }
        true
    }

    fn dispatch_host_event(&mut self, event_loop: &dyn ActiveEventLoop, event: HostEvent) -> bool {
        if !self.handle_host_event(event_loop, event) {
            return false;
        }
        self.apply_window_commands(event_loop);
        true
    }

    fn request_redraw_if_needed(&mut self) {
        if self.runner.take_redraw_request() {
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    fn process_runtime(&mut self, event_loop: &dyn ActiveEventLoop) {
        if let Err(error) = self.runner.process_pending() {
            self.fail(event_loop, error);
            return;
        }
        self.apply_window_commands(event_loop);
        self.apply_content_size_if_needed();
        self.request_redraw_if_needed();
    }

    fn apply_window_commands(&mut self, event_loop: &dyn ActiveEventLoop) {
        let Some(window) = self.window.as_ref().cloned() else {
            return;
        };
        let commands = self.runner.take_window_commands();
        if commands.is_empty() {
            return;
        }

        for command in commands {
            match command {
                WindowCommand::StartDrag => {
                    let _ = window.drag_window();
                }
                WindowCommand::SetPosition(position) => {
                    window.set_outer_position(logical_position(position).into());
                }
                WindowCommand::SetSize(size) => {
                    if let Some(actual_size) =
                        window.request_surface_size(logical_size(size).into())
                    {
                        self.runner.set_surface_size(Size::new(
                            actual_size.width.max(1) as f32,
                            actual_size.height.max(1) as f32,
                        ));
                    }
                }
                WindowCommand::SetVisible(visible) => {
                    window.set_visible(visible);
                    self.runner.set_window_visible(visible);
                    if self.window_visible != visible {
                        self.window_visible = visible;
                        if !self.handle_host_event(event_loop, HostEvent::WindowVisible(visible)) {
                            return;
                        }
                    }
                }
                WindowCommand::SetAlwaysOnTop(always_on_top) => {
                    window.set_window_level(if always_on_top {
                        WindowLevel::AlwaysOnTop
                    } else {
                        WindowLevel::Normal
                    });
                }
            }
        }
    }

    fn create_window(&mut self, event_loop: &dyn ActiveEventLoop) -> Result<Rc<dyn Window>> {
        let config = self.host.config.normalized();
        let position = initial_position(config, event_loop);
        let attributes = window_attributes(self.runner.widget_name(), config, position);
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| Error::platform(error.to_string()))?;
        Ok(Rc::from(window))
    }

    fn apply_content_size_if_needed(&mut self) {
        if !self.runner.take_layout_request() {
            return;
        }

        let config = self.host.config.normalized();
        let Some(constraints) = content_constraints(config) else {
            return;
        };
        let Some(preferred_size) = self.runner.preferred_size(constraints) else {
            return;
        };
        let Some(window) = self.window.as_ref().cloned() else {
            return;
        };
        if preferred_size.is_empty() {
            return;
        }

        let current = window.surface_size();
        let current_size = Size::new(current.width as f32, current.height as f32);
        let Some(target_size) =
            content_resize_target(current_size, preferred_size, self.last_content_size)
        else {
            self.last_content_size = Some(preferred_size);
            return;
        };

        if let Some(actual_size) = window.request_surface_size(logical_size(target_size).into()) {
            self.runner.set_surface_size(Size::new(
                actual_size.width.max(1) as f32,
                actual_size.height.max(1) as f32,
            ));
        }
        self.last_content_size = Some(target_size);
    }
}

impl<W, R> ApplicationHandler for WindowsApp<W, R>
where
    W: Widget,
    R: widgetkit_render::Renderer,
{
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

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

        let size = window.surface_size();
        if let Err(error) = self.runner.initialize(Size::new(
            size.width.max(1) as f32,
            size.height.max(1) as f32,
        )) {
            self.fail(event_loop, error);
            return;
        }

        self.window = Some(window);
        self.surface = Some(surface);
        self.apply_content_size_if_needed();
        self.request_redraw_if_needed();
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.process_runtime(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
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
                if !self.dispatch_host_event(event_loop, HostEvent::CloseRequested) {
                    return;
                }
                if let Err(error) = self.runner.shutdown() {
                    self.fail(event_loop, error);
                    return;
                }
                event_loop.exit();
            }
            WindowEvent::Focused(focused) => {
                if !self.dispatch_host_event(event_loop, HostEvent::WindowFocused(focused)) {
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::SurfaceResized(size) => {
                if !self.dispatch_host_event(
                    event_loop,
                    HostEvent::Resized(Size::new(
                        size.width.max(1) as f32,
                        size.height.max(1) as f32,
                    )),
                ) {
                    return;
                }
                self.apply_content_size_if_needed();
                self.request_redraw_if_needed();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if !self.dispatch_host_event(
                    event_loop,
                    HostEvent::ScaleFactorChanged(scale_factor as f32),
                ) {
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::PointerMoved { position, .. } => {
                let position = point_from_physical(position);
                self.last_pointer_position = position;
                if !self.dispatch_host_event(
                    event_loop,
                    HostEvent::Mouse(MouseEvent::Moved { position }),
                ) {
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::PointerEntered { position, .. } => {
                self.last_pointer_position = point_from_physical(position);
                if !self.dispatch_host_event(event_loop, HostEvent::Mouse(MouseEvent::Entered)) {
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::PointerLeft { position, .. } => {
                if let Some(position) = position {
                    self.last_pointer_position = point_from_physical(position);
                }
                if !self.dispatch_host_event(event_loop, HostEvent::Mouse(MouseEvent::Left)) {
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::PointerButton {
                state,
                position,
                button,
                ..
            } => {
                let position = point_from_physical(position);
                self.last_pointer_position = position;
                let button = mouse_button_from_source(button);
                let event = match state {
                    ElementState::Pressed => MouseEvent::Pressed { button, position },
                    ElementState::Released => MouseEvent::Released { button, position },
                };
                if !self.dispatch_host_event(event_loop, HostEvent::Mouse(event)) {
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = mouse_wheel_delta(delta);
                if !self.dispatch_host_event(
                    event_loop,
                    HostEvent::Mouse(MouseEvent::Wheel {
                        delta,
                        position: self.last_pointer_position,
                    }),
                ) {
                    return;
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let winit::event::KeyEvent {
                    logical_key,
                    state,
                    text,
                    ..
                } = event;
                let key = key_from_winit(logical_key);
                let event = match state {
                    ElementState::Pressed => KeyboardEvent::Pressed { key },
                    ElementState::Released => KeyboardEvent::Released { key },
                };
                if !self.dispatch_host_event(event_loop, HostEvent::Keyboard(event)) {
                    return;
                }
                if state == ElementState::Pressed {
                    if let Some(text) = text {
                        let text = text.to_string();
                        if !text.is_empty()
                            && !self.dispatch_host_event(
                                event_loop,
                                HostEvent::Keyboard(KeyboardEvent::TextInput(text)),
                            )
                        {
                            return;
                        }
                    }
                }
                self.request_redraw_if_needed();
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if !text.is_empty()
                    && !self.dispatch_host_event(
                        event_loop,
                        HostEvent::Keyboard(KeyboardEvent::TextInput(text)),
                    )
                {
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

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.process_runtime(event_loop);
    }
}

// TODO(v0.3): guard against resize-relayout-resize feedback loops
// TODO(v0.6): integrate reserved/work-area awareness
// TODO(v0.4): add close behavior policy: Exit / HideWindow / AskRuntime
// TODO(v0.7): hybrid host compatibility

fn valid_size(size: Option<Size>) -> Option<Size> {
    size.filter(|size| !size.is_empty())
}

fn content_constraints(config: WindowConfig) -> Option<Constraints> {
    match config.size_policy {
        SizePolicy::Fixed(_) => None,
        SizePolicy::Content => Some(Constraints::new(config.min_size, config.max_size)),
        SizePolicy::ContentWithLimits { min, max } => Some(Constraints::new(
            min.or(config.min_size),
            max.or(config.max_size),
        )),
    }
}

fn normalize_size_policy(size_policy: SizePolicy, fallback_size: Option<Size>) -> SizePolicy {
    match size_policy {
        SizePolicy::Fixed(size) => {
            let size = valid_size(Some(size))
                .or(fallback_size)
                .unwrap_or(DEFAULT_WINDOW_SIZE);
            SizePolicy::Fixed(size)
        }
        SizePolicy::Content => SizePolicy::Content,
        SizePolicy::ContentWithLimits { min, max } => SizePolicy::ContentWithLimits {
            min: valid_size(min),
            max: valid_size(max),
        },
    }
}

fn window_attributes(
    title: &str,
    config: WindowConfig,
    position: Option<Point>,
) -> WindowAttributes {
    let config = config.normalized();
    let size = config.resolved_size();
    let mut attributes = WindowAttributes::default()
        .with_title(title)
        .with_surface_size(logical_size(size))
        .with_decorations(!config.frameless)
        .with_resizable(config.resizable)
        .with_transparent(config.transparent)
        .with_visible(config.visible)
        .with_window_level(if config.always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        });

    if let Some(position) = position {
        attributes = attributes.with_position(logical_position(position));
    }

    if let Some(min_size) = config.min_size {
        attributes = attributes.with_min_surface_size(logical_size(min_size));
    }

    if let Some(max_size) = config.max_size {
        attributes = attributes.with_max_surface_size(logical_size(max_size));
    }

    attributes
}

fn initial_position(config: WindowConfig, event_loop: &dyn ActiveEventLoop) -> Option<Point> {
    let config = config.normalized();
    let position = config.position;
    if let Some(point) = position.point {
        return Some(apply_position_offset(point, position.offset));
    }

    let anchor = position.anchor?;
    let monitor = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next())?;
    let monitor_position = monitor.position()?;
    let monitor_size = monitor.current_video_mode()?.size();

    Some(anchor_position(
        anchor,
        RectLike {
            x: monitor_position.x as f32,
            y: monitor_position.y as f32,
            width: monitor_size.width as f32,
            height: monitor_size.height as f32,
        },
        config.resolved_size(),
        position.offset,
    ))
}

#[derive(Clone, Copy)]
struct RectLike {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn anchor_position(anchor: Anchor, bounds: RectLike, window_size: Size, offset: Point) -> Point {
    let x = match anchor {
        Anchor::TopLeft | Anchor::Left | Anchor::BottomLeft => bounds.x + offset.x,
        Anchor::Top | Anchor::Center | Anchor::Bottom => {
            bounds.x + (bounds.width - window_size.width) * 0.5 + offset.x
        }
        Anchor::TopRight | Anchor::Right | Anchor::BottomRight => {
            bounds.x + bounds.width - window_size.width - offset.x
        }
    };
    let y = match anchor {
        Anchor::TopLeft | Anchor::Top | Anchor::TopRight => bounds.y + offset.y,
        Anchor::Left | Anchor::Center | Anchor::Right => {
            bounds.y + (bounds.height - window_size.height) * 0.5 + offset.y
        }
        Anchor::BottomLeft | Anchor::Bottom | Anchor::BottomRight => {
            bounds.y + bounds.height - window_size.height - offset.y
        }
    };

    Point::new(x, y)
}

fn apply_position_offset(position: Point, offset: Point) -> Point {
    Point::new(position.x + offset.x, position.y + offset.y)
}

fn logical_size(size: Size) -> LogicalSize<f64> {
    LogicalSize::new(size.width as f64, size.height as f64)
}

fn logical_position(position: Point) -> LogicalPosition<f64> {
    LogicalPosition::new(position.x as f64, position.y as f64)
}

fn point_from_physical(position: winit::dpi::PhysicalPosition<f64>) -> Point {
    Point::new(position.x as f32, position.y as f32)
}

fn mouse_button_from_source(button: ButtonSource) -> MouseButton {
    match button {
        ButtonSource::Mouse(button) => mouse_button_from_winit(button),
        ButtonSource::Touch { .. } => MouseButton::Left,
        ButtonSource::TabletTool { .. } => MouseButton::Other(0),
        ButtonSource::Unknown(button) => MouseButton::Other(button),
    }
}

fn mouse_button_from_winit(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        button => MouseButton::Other(button as u16),
    }
}

fn mouse_wheel_delta(delta: MouseScrollDelta) -> MouseWheelDelta {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => MouseWheelDelta::LineDelta { x, y },
        MouseScrollDelta::PixelDelta(position) => MouseWheelDelta::PixelDelta {
            x: position.x as f32,
            y: position.y as f32,
        },
    }
}

fn key_from_winit(key: WinitKey) -> Key {
    match key {
        WinitKey::Character(value) => Key::Character(value.to_string()),
        WinitKey::Named(value) => Key::Named(format!("{value:?}")),
        WinitKey::Dead(value) => Key::Dead(value),
        WinitKey::Unidentified(value) => Key::Unidentified(format!("{value:?}")),
    }
}

fn same_size(a: Size, b: Size) -> bool {
    (a.width - b.width).abs() < 0.5 && (a.height - b.height).abs() < 0.5
}

fn content_resize_target(
    current_size: Size,
    target_size: Size,
    last_content_size: Option<Size>,
) -> Option<Size> {
    if target_size.is_empty() || same_size(current_size, target_size) {
        return None;
    }

    if last_content_size == Some(target_size) {
        return None;
    }

    Some(target_size)
}

#[cfg(test)]
mod tests {
    use super::{
        Anchor, PositionConfig, RectLike, WindowConfig, WindowsHost, anchor_position,
        window_attributes,
    };
    use widgetkit_core::SizePolicy;
    use widgetkit_core::{Point, Size};
    use winit::dpi::Position as WinitPosition;
    use winit::dpi::Size as WinitSize;
    use winit::window::WindowLevel;

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
            size_policy: SizePolicy::Fixed(Size::new(400.0, 240.0)),
            resizable: false,
            frameless: true,
            transparent: true,
            always_on_top: true,
            visible: false,
            position: PositionConfig::anchored(Anchor::TopRight).with_offset(Point::new(4.0, 8.0)),
        };

        let host = WindowsHost::new().with_config(config);

        assert_eq!(host.config(), &config);
    }

    #[test]
    fn windows_host_size_builder_updates_window_config() {
        let host = WindowsHost::new().with_size(Size::new(400.0, 240.0));

        assert_eq!(host.config().size, Some(Size::new(400.0, 240.0)));
        assert_eq!(
            host.config().size_policy,
            SizePolicy::Fixed(Size::new(400.0, 240.0))
        );
    }

    #[test]
    fn windows_host_can_disable_standard_top_bar_through_config() {
        let host = WindowsHost::new()
            .with_size(Size::new(400.0, 240.0))
            .with_standard_top_bar(false);

        assert_eq!(host.config().size, Some(Size::new(400.0, 240.0)));
        assert!(host.config().frameless);
    }

    #[test]
    fn windows_host_builders_update_window_config_flags() {
        let host = WindowsHost::new()
            .with_min_size(Size::new(200.0, 100.0))
            .with_max_size(Size::new(800.0, 600.0))
            .size_policy(SizePolicy::Content)
            .resizable(false)
            .frameless(true)
            .transparent(true)
            .always_on_top(true)
            .visible(false)
            .position(Point::new(20.0, 30.0))
            .anchor(Anchor::BottomRight)
            .offset(12.0, 16.0);

        assert_eq!(host.config().min_size, Some(Size::new(200.0, 100.0)));
        assert_eq!(host.config().max_size, Some(Size::new(800.0, 600.0)));
        assert_eq!(host.config().size_policy, SizePolicy::Content);
        assert!(!host.config().resizable);
        assert!(host.config().frameless);
        assert!(host.config().transparent);
        assert!(host.config().always_on_top);
        assert!(!host.config().visible);
        assert_eq!(host.config().position.point, None);
        assert_eq!(host.config().position.anchor, Some(Anchor::BottomRight));
        assert_eq!(host.config().position.offset, Point::new(12.0, 16.0));
    }

    #[test]
    fn windows_host_accepts_explicit_position_config() {
        let host = WindowsHost::new().position_config(
            PositionConfig::at(Point::new(20.0, 30.0)).with_offset(Point::new(4.0, 8.0)),
        );

        assert_eq!(host.config().position.point, Some(Point::new(20.0, 30.0)));
        assert_eq!(host.config().position.anchor, None);
        assert_eq!(host.config().position.offset, Point::new(4.0, 8.0));
    }

    #[test]
    fn window_attributes_apply_config_flags() {
        let attributes = window_attributes(
            "widget",
            WindowConfig {
                size: Some(Size::new(400.0, 240.0)),
                min_size: Some(Size::new(280.0, 120.0)),
                max_size: Some(Size::new(640.0, 360.0)),
                size_policy: SizePolicy::Fixed(Size::new(400.0, 240.0)),
                resizable: false,
                frameless: true,
                transparent: true,
                always_on_top: true,
                visible: false,
                position: PositionConfig::at(Point::new(20.0, 30.0))
                    .with_offset(Point::new(12.0, 16.0)),
            },
            Some(Point::new(32.0, 46.0)),
        );

        assert_eq!(attributes.title, "widget");
        assert_eq!(
            attributes.surface_size,
            Some(WinitSize::Logical(logical_size(400.0, 240.0)))
        );
        assert_eq!(
            attributes.min_surface_size,
            Some(WinitSize::Logical(logical_size(280.0, 120.0)))
        );
        assert_eq!(
            attributes.max_surface_size,
            Some(WinitSize::Logical(logical_size(640.0, 360.0)))
        );
        assert!(!attributes.resizable);
        assert!(!attributes.decorations);
        assert!(attributes.transparent);
        assert_eq!(attributes.window_level, WindowLevel::AlwaysOnTop);
        assert!(!attributes.visible);
        assert_eq!(
            attributes.position,
            Some(WinitPosition::Logical(logical_position(32.0, 46.0)))
        );
    }

    #[test]
    fn anchor_position_offsets_from_the_nearest_edges() {
        let position = anchor_position(
            Anchor::TopRight,
            RectLike {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            Size::new(320.0, 120.0),
            Point::new(12.0, 24.0),
        );

        assert_eq!(position, Point::new(1588.0, 24.0));
    }

    #[test]
    fn content_constraints_follow_size_policy_limits() {
        let constraints = super::content_constraints(WindowConfig {
            size: None,
            min_size: None,
            max_size: None,
            size_policy: SizePolicy::ContentWithLimits {
                min: Some(Size::new(120.0, 80.0)),
                max: Some(Size::new(420.0, 260.0)),
            },
            resizable: true,
            frameless: false,
            transparent: false,
            always_on_top: false,
            visible: true,
            position: PositionConfig::default(),
        })
        .unwrap();

        assert_eq!(constraints.min, Some(Size::new(120.0, 80.0)));
        assert_eq!(constraints.max, Some(Size::new(420.0, 260.0)));
    }

    #[test]
    fn content_resize_target_coalesces_repeated_target_size() {
        let target = super::content_resize_target(
            Size::new(320.0, 120.0),
            Size::new(420.0, 180.0),
            Some(Size::new(420.0, 180.0)),
        );

        assert_eq!(target, None);
    }

    #[test]
    fn content_resize_target_skips_current_window_size() {
        let target =
            super::content_resize_target(Size::new(420.0, 180.0), Size::new(420.0, 180.0), None);

        assert_eq!(target, None);
    }

    #[test]
    fn content_resize_target_requests_changed_size() {
        let target =
            super::content_resize_target(Size::new(320.0, 120.0), Size::new(420.0, 180.0), None);

        assert_eq!(target, Some(Size::new(420.0, 180.0)));
    }

    fn logical_size(width: f64, height: f64) -> winit::dpi::LogicalSize<f64> {
        winit::dpi::LogicalSize::new(width, height)
    }

    fn logical_position(x: f64, y: f64) -> winit::dpi::LogicalPosition<f64> {
        winit::dpi::LogicalPosition::new(x, y)
    }
}
