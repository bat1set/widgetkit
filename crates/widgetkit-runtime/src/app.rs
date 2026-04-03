use crate::{context::{DisposeCtx, MountCtx, RenderCtx, StartCtx, StopCtx, UpdateCtx}, event::Event, host::HostRunner, internal::{Dispatcher, RuntimeEvent, RuntimeServices, WakeHandle}, widget::Widget};
use crossbeam_channel::{Receiver, TryRecvError, unbounded};
use widgetkit_core::{HostEvent, InstanceId, Result, Size, WidgetId};
use widgetkit_render::{Canvas, RenderSurface, Renderer};

pub struct WidgetApp<W = (), H = (), R = ()> {
    widget_name: Option<String>,
    widget: Option<W>,
    host: Option<H>,
    renderer: Option<R>,
}

impl WidgetApp<(), (), ()> {
    pub fn new() -> Self {
        Self {
            widget_name: None,
            widget: None,
            host: None,
            renderer: None,
        }
    }
}

impl<H, R> WidgetApp<(), H, R> {
    pub fn widget<W>(self, name: impl Into<String>, widget: W) -> WidgetApp<W, H, R>
    where
        W: Widget,
    {
        WidgetApp {
            widget_name: Some(name.into()),
            widget: Some(widget),
            host: self.host,
            renderer: self.renderer,
        }
    }
}

impl<W, R> WidgetApp<W, (), R>
where
    W: Widget,
{
    pub fn host<H>(self, host: H) -> WidgetApp<W, H, R> {
        WidgetApp {
            widget_name: self.widget_name,
            widget: self.widget,
            host: Some(host),
            renderer: self.renderer,
        }
    }
}

impl<W, H> WidgetApp<W, H, ()>
where
    W: Widget,
{
    pub fn renderer<R>(self, renderer: R) -> WidgetApp<W, H, R> {
        WidgetApp {
            widget_name: self.widget_name,
            widget: self.widget,
            host: self.host,
            renderer: Some(renderer),
        }
    }
}

impl<W, H, R> WidgetApp<W, H, R>
where
    W: Widget,
    H: HostRunner<W, R>,
    R: Renderer,
{
    pub fn run(self) -> Result<()> {
        let runner = AppRunner::new(
            self.widget_name.expect("widget name must be configured"),
            self.widget.expect("widget must be configured"),
            self.renderer.expect("renderer must be configured"),
        );
        self.host.expect("host must be configured").run(runner)
    }
}

impl Default for WidgetApp<(), (), ()> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AppRunner<W, R>
where
    W: Widget,
    R: Renderer,
{
    widget_name: String,
    widget_id: WidgetId,
    instance_id: InstanceId,
    widget: W,
    state: Option<W::State>,
    renderer: R,
    receiver: Receiver<RuntimeEvent<W::Message>>,
    services: RuntimeServices<W::Message>,
    surface_size: Size,
    initialized: bool,
    shut_down: bool,
}

impl<W, R> AppRunner<W, R>
where
    W: Widget,
    R: Renderer,
{
    pub fn new(widget_name: impl Into<String>, widget: W, renderer: R) -> Self {
        let (sender, receiver) = unbounded();
        let wake = WakeHandle::default();
        let dispatcher = Dispatcher { sender, wake };
        let services = RuntimeServices::new(dispatcher);
        Self {
            widget_name: widget_name.into(),
            widget_id: WidgetId::new(),
            instance_id: InstanceId::new(),
            widget,
            state: None,
            renderer,
            receiver,
            services,
            surface_size: Size::new(320.0, 120.0),
            initialized: false,
            shut_down: false,
        }
    }

    pub fn widget_name(&self) -> &str {
        &self.widget_name
    }

    pub fn surface_size(&self) -> Size {
        self.surface_size
    }

    pub fn set_surface_size(&mut self, size: Size) {
        if !size.is_empty() {
            self.surface_size = size;
            self.request_render();
        }
    }

    pub fn attach_waker<F>(&mut self, wake: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.services.dispatcher.wake.set(wake);
    }

    pub fn initialize(&mut self, surface_size: Size) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        if !surface_size.is_empty() {
            self.surface_size = surface_size;
        }

        let mut mount_ctx = MountCtx::new(self.widget_id, self.instance_id);
        let state = self.widget.mount(&mut mount_ctx);
        self.state = Some(state);
        self.with_state_mut(|widget, state, services, widget_id, instance_id| {
            let mut ctx = StartCtx::new(widget_id, instance_id, std::ptr::NonNull::from(services));
            widget.start(state, &mut ctx);
        });
        self.initialized = true;
        self.request_render();
        self.process_pending()
    }

    pub fn process_pending(&mut self) -> Result<()> {
        loop {
            match self.receiver.try_recv() {
                Ok(RuntimeEvent::Message(message)) => self.dispatch_event(Event::Message(message)),
                Ok(RuntimeEvent::TaskFinished(task_id)) => self.services.tasks.reap(task_id),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        Ok(())
    }

    pub fn handle_host_event(&mut self, event: HostEvent) -> Result<()> {
        if let HostEvent::Resized(size) = event.clone() {
            self.set_surface_size(size);
        }
        self.dispatch_event(Event::Host(event));
        self.process_pending()
    }

    pub fn needs_redraw(&self) -> bool {
        self.services.render_requested
    }

    pub fn render(&mut self, surface: &mut dyn RenderSurface) -> Result<()> {
        let (width, height) = surface.size();
        self.surface_size = Size::new(width as f32, height as f32);
        if let Some(state) = self.state.as_ref() {
            let mut canvas = Canvas::new(self.surface_size);
            let ctx = RenderCtx::new(self.widget_id, self.instance_id, self.surface_size);
            self.widget.render(state, &mut canvas, &ctx);
            self.renderer.render_canvas(canvas, surface)?;
            self.services.render_requested = false;
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        if self.shut_down {
            return Ok(());
        }
        self.with_state_mut(|widget, state, services, widget_id, instance_id| {
            let mut ctx = StopCtx::new(widget_id, instance_id, std::ptr::NonNull::from(services));
            widget.stop(state, &mut ctx);
        });
        self.services.scheduler.clear();
        self.services.tasks.cancel_all();
        if let Some(state) = self.state.take() {
            let mut ctx = DisposeCtx::new(self.widget_id, self.instance_id);
            self.widget.dispose(state, &mut ctx);
        }
        self.shut_down = true;
        Ok(())
    }

    fn request_render(&mut self) {
        self.services.render_requested = true;
        self.services.dispatcher.wake.wake();
    }

    fn dispatch_event(&mut self, event: Event<W::Message>) {
        self.with_state_mut(|widget, state, services, widget_id, instance_id| {
            let mut ctx = UpdateCtx::new(widget_id, instance_id, std::ptr::NonNull::from(services));
            widget.update(state, event, &mut ctx);
        });
    }

    fn with_state_mut(
        &mut self,
        f: impl FnOnce(&mut W, &mut W::State, &mut RuntimeServices<W::Message>, WidgetId, InstanceId),
    ) {
        if let Some(state) = self.state.as_mut() {
            f(&mut self.widget, state, &mut self.services, self.widget_id, self.instance_id);
        }
    }
}

impl<W, R> Drop for AppRunner<W, R>
where
    W: Widget,
    R: Renderer,
{
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
