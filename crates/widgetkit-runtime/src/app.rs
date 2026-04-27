use crate::{
    context::{DisposeCtx, LayoutCtx, MountCtx, RenderCtx, StartCtx, StopCtx, UpdateCtx},
    event::Event,
    host::HostRunner,
    internal::{DispatchToken, Dispatcher, RuntimeEvent, RuntimeServices, WakeHandle},
    widget::Widget,
};
use crossbeam_channel::{Receiver, TryRecvError, unbounded};
use widgetkit_core::{Constraints, Error, HostEvent, InstanceId, Result, Size, WidgetId};
use widgetkit_render::{Canvas, RenderSurface, Renderer};

/// Application bootstrap for a single widget instance bound to one host and one renderer.
/// The runtime is demand-driven and only redraws when widget code requests it.
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

/// Runtime runner for one widget instance lifetime.
/// Timers and tasks are instance-owned and are shut down with `stop`/`dispose`.
pub struct AppRunner<W, R>
where
    W: Widget,
    R: Renderer,
{
    widget_name: String,
    widget_id: WidgetId,
    instance_id: InstanceId,
    instance_generation: u64,
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
        let instance_id = InstanceId::new();
        let instance_generation = 1;
        let dispatcher = Dispatcher {
            sender,
            wake,
            token: DispatchToken::new(instance_id, instance_generation),
        };
        let services = RuntimeServices::new(dispatcher);
        Self {
            widget_name: widget_name.into(),
            widget_id: WidgetId::new(),
            instance_id,
            instance_generation,
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

    pub fn preferred_size(&self, constraints: Constraints) -> Option<Size> {
        if !self.initialized || self.shut_down {
            return None;
        }

        self.state.as_ref().map(|state| {
            let ctx = LayoutCtx::new(
                self.widget_id,
                self.instance_id,
                self.surface_size,
                constraints,
            );
            ctx.constrain(self.widget.preferred_size(state, &ctx))
        })
    }

    pub fn attach_waker<F>(&mut self, wake: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.services.dispatcher.wake.set(wake);
    }

    pub fn initialize(&mut self, surface_size: Size) -> Result<()> {
        if self.shut_down {
            return Err(Error::message(
                "widgetkit v0.1 AppRunner supports a single widget instance lifetime",
            ));
        }
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
                Ok(RuntimeEvent::Message(envelope)) => {
                    if self.accepts_token(envelope.token) {
                        self.dispatch_event(Event::Message(envelope.message));
                    }
                }
                Ok(RuntimeEvent::TaskFinished { token, task_id }) => {
                    if self.matches_token(token) {
                        self.services.tasks.reap(task_id);
                    }
                }
                Ok(RuntimeEvent::TimerFinished { token, timer_id }) => {
                    if self.matches_token(token) {
                        self.services.scheduler.reap(timer_id);
                    }
                }
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
        self.services.needs_redraw()
    }

    pub fn needs_layout(&self) -> bool {
        self.services.needs_layout()
    }

    pub fn take_layout_request(&mut self) -> bool {
        self.initialized && !self.shut_down && self.services.take_layout_request()
    }

    pub fn take_redraw_request(&mut self) -> bool {
        self.initialized && !self.shut_down && self.services.take_redraw_request()
    }

    pub fn render(&mut self, surface: &mut dyn RenderSurface) -> Result<()> {
        if !self.initialized || self.shut_down || !self.services.begin_render() {
            return Ok(());
        }

        let (width, height) = surface.size();
        self.surface_size = Size::new(width as f32, height as f32);
        if let Some(state) = self.state.as_ref() {
            let mut canvas = Canvas::new(self.surface_size);
            let ctx = RenderCtx::new(self.widget_id, self.instance_id, self.surface_size);
            self.widget.render(state, &mut canvas, &ctx);
            self.renderer.render_frame(canvas.into_frame(), surface)?;
            self.services.finish_render();
        } else {
            self.services.clear_redraw();
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
        self.services.scheduler.shutdown();
        self.services.tasks.shutdown();
        if let Some(state) = self.state.take() {
            let mut ctx = DisposeCtx::new(self.widget_id, self.instance_id);
            self.widget.dispose(state, &mut ctx);
        }
        self.instance_generation += 1;
        self.initialized = false;
        self.shut_down = true;
        self.services.clear_redraw();
        Ok(())
    }

    fn request_render(&mut self) {
        if self.services.request_render() {
            self.services.dispatcher.wake.wake();
        }
    }

    fn dispatch_event(&mut self, event: Event<W::Message>) {
        self.with_state_mut(|widget, state, services, widget_id, instance_id| {
            let mut ctx = UpdateCtx::new(widget_id, instance_id, std::ptr::NonNull::from(services));
            widget.update(state, event, &mut ctx);
        });
    }

    fn accepts_token(&self, token: DispatchToken) -> bool {
        self.initialized && !self.shut_down && self.matches_token(token)
    }

    fn matches_token(&self, token: DispatchToken) -> bool {
        token == self.current_dispatch_token()
    }

    fn current_dispatch_token(&self) -> DispatchToken {
        DispatchToken::new(self.instance_id, self.instance_generation)
    }

    #[cfg(test)]
    pub(crate) fn scheduler_active_count(&self) -> usize {
        self.services.scheduler.active_count()
    }

    #[cfg(test)]
    pub(crate) fn task_active_count(&self) -> usize {
        self.services.tasks.active_count()
    }

    #[cfg(test)]
    pub(crate) fn test_token(&self) -> DispatchToken {
        self.services.dispatcher.token
    }

    #[cfg(test)]
    pub(crate) fn dispatch_test_message(&self, token: DispatchToken, message: W::Message) {
        let _ = self.services.dispatcher.sender.send(RuntimeEvent::Message(
            crate::internal::MessageEnvelope { token, message },
        ));
    }

    fn with_state_mut(
        &mut self,
        f: impl FnOnce(&mut W, &mut W::State, &mut RuntimeServices<W::Message>, WidgetId, InstanceId),
    ) {
        if let Some(state) = self.state.as_mut() {
            f(
                &mut self.widget,
                state,
                &mut self.services,
                self.widget_id,
                self.instance_id,
            );
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
