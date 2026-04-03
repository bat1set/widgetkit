use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use futures::Future;
#[cfg(not(feature = "runtime-tokio"))]
use futures::future::{AbortHandle, Abortable};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use widgetkit_core::{Duration, HostEvent, InstanceId, Result, Size, TaskId, TimerId, WidgetId};
use widgetkit_render::{Canvas, RenderSurface, Renderer};

pub use widgetkit_core;
pub use widgetkit_render;

pub enum Event<M> {
    Message(M),
    Host(HostEvent),
}

pub trait Widget: Send + Sized + 'static {
    type State;
    type Message: Send + 'static;

    fn mount(&mut self, ctx: &mut MountCtx<Self>) -> Self::State;

    fn start(&mut self, _state: &mut Self::State, _ctx: &mut StartCtx<Self>) {}

    fn update(&mut self, _state: &mut Self::State, _event: Event<Self::Message>, _ctx: &mut UpdateCtx<Self>) {}

    fn render(&self, _state: &Self::State, _canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {}

    fn stop(&mut self, _state: &mut Self::State, _ctx: &mut StopCtx<Self>) {}

    fn dispose(&mut self, _state: Self::State, _ctx: &mut DisposeCtx<Self>) {}
}

pub trait HostRunner<W, R>
where
    W: Widget,
    R: Renderer,
{
    fn run(self, runner: AppRunner<W, R>) -> Result<()>;
}

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
        let dispatcher = Dispatcher {
            sender,
            wake: wake.clone(),
        };
        let services = RuntimeServices {
            dispatcher: dispatcher.clone(),
            scheduler: SchedulerState::new(),
            tasks: task_backend(dispatcher),
            render_requested: true,
        };
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
            let mut ctx = StartCtx::new(widget_id, instance_id, NonNull::from(services));
            widget.start(state, &mut ctx);
        });
        self.initialized = true;
        self.request_render();
        self.process_pending()
    }

    pub fn process_pending(&mut self) -> Result<()> {
        loop {
            match self.receiver.try_recv() {
                Ok(RuntimeEvent::Message(message)) => {
                    self.dispatch_event(Event::Message(message));
                }
                Ok(RuntimeEvent::TaskFinished(task_id)) => {
                    self.services.tasks.reap(task_id);
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
            let mut ctx = StopCtx::new(widget_id, instance_id, NonNull::from(services));
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
            let mut ctx = UpdateCtx::new(widget_id, instance_id, NonNull::from(services));
            widget.update(state, event, &mut ctx);
        });
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

pub struct MountCtx<W>
where
    W: Widget,
{
    widget_id: WidgetId,
    instance_id: InstanceId,
    _marker: PhantomData<fn() -> W>,
}

impl<W> MountCtx<W>
where
    W: Widget,
{
    fn new(widget_id: WidgetId, instance_id: InstanceId) -> Self {
        Self {
            widget_id,
            instance_id,
            _marker: PhantomData,
        }
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }
}

pub struct StartCtx<W>
where
    W: Widget,
{
    widget_id: WidgetId,
    instance_id: InstanceId,
    services: NonNull<RuntimeServices<W::Message>>,
    _marker: PhantomData<fn() -> W>,
}

impl<W> StartCtx<W>
where
    W: Widget,
{
    fn new(widget_id: WidgetId, instance_id: InstanceId, services: NonNull<RuntimeServices<W::Message>>) -> Self {
        Self {
            widget_id,
            instance_id,
            services,
            _marker: PhantomData,
        }
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    pub fn post(&mut self, message: W::Message) {
        let services = self.services_mut();
        let _ = services.dispatcher.post_message(message);
    }

    pub fn request_render(&mut self) {
        let services = self.services_mut();
        services.render_requested = true;
        services.dispatcher.wake.wake();
    }

    pub fn scheduler(&mut self) -> Scheduler<'_, W::Message> {
        let services = self.services_mut();
        Scheduler::new(&mut services.scheduler, services.dispatcher.clone())
    }

    pub fn tasks(&mut self) -> Tasks<'_, W::Message> {
        let services = self.services_mut();
        Tasks::new(services.tasks.as_mut())
    }

    fn services_mut(&mut self) -> &mut RuntimeServices<W::Message> {
        unsafe { self.services.as_mut() }
    }
}

pub struct UpdateCtx<W>
where
    W: Widget,
{
    widget_id: WidgetId,
    instance_id: InstanceId,
    services: NonNull<RuntimeServices<W::Message>>,
    _marker: PhantomData<fn() -> W>,
}

impl<W> UpdateCtx<W>
where
    W: Widget,
{
    fn new(widget_id: WidgetId, instance_id: InstanceId, services: NonNull<RuntimeServices<W::Message>>) -> Self {
        Self {
            widget_id,
            instance_id,
            services,
            _marker: PhantomData,
        }
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    pub fn post(&mut self, message: W::Message) {
        let services = self.services_mut();
        let _ = services.dispatcher.post_message(message);
    }

    pub fn request_render(&mut self) {
        let services = self.services_mut();
        services.render_requested = true;
        services.dispatcher.wake.wake();
    }

    pub fn scheduler(&mut self) -> Scheduler<'_, W::Message> {
        let services = self.services_mut();
        Scheduler::new(&mut services.scheduler, services.dispatcher.clone())
    }

    pub fn tasks(&mut self) -> Tasks<'_, W::Message> {
        let services = self.services_mut();
        Tasks::new(services.tasks.as_mut())
    }

    fn services_mut(&mut self) -> &mut RuntimeServices<W::Message> {
        unsafe { self.services.as_mut() }
    }
}

pub struct RenderCtx<W>
where
    W: Widget,
{
    widget_id: WidgetId,
    instance_id: InstanceId,
    surface_size: Size,
    _marker: PhantomData<fn() -> W>,
}

impl<W> RenderCtx<W>
where
    W: Widget,
{
    fn new(widget_id: WidgetId, instance_id: InstanceId, surface_size: Size) -> Self {
        Self {
            widget_id,
            instance_id,
            surface_size,
            _marker: PhantomData,
        }
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    pub fn surface_size(&self) -> Size {
        self.surface_size
    }
}

pub struct StopCtx<W>
where
    W: Widget,
{
    widget_id: WidgetId,
    instance_id: InstanceId,
    services: NonNull<RuntimeServices<W::Message>>,
    _marker: PhantomData<fn() -> W>,
}

impl<W> StopCtx<W>
where
    W: Widget,
{
    fn new(widget_id: WidgetId, instance_id: InstanceId, services: NonNull<RuntimeServices<W::Message>>) -> Self {
        Self {
            widget_id,
            instance_id,
            services,
            _marker: PhantomData,
        }
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    pub fn scheduler(&mut self) -> Scheduler<'_, W::Message> {
        let services = self.services_mut();
        Scheduler::new(&mut services.scheduler, services.dispatcher.clone())
    }

    pub fn tasks(&mut self) -> Tasks<'_, W::Message> {
        let services = self.services_mut();
        Tasks::new(services.tasks.as_mut())
    }

    fn services_mut(&mut self) -> &mut RuntimeServices<W::Message> {
        unsafe { self.services.as_mut() }
    }
}

pub struct DisposeCtx<W>
where
    W: Widget,
{
    widget_id: WidgetId,
    instance_id: InstanceId,
    _marker: PhantomData<fn() -> W>,
}

impl<W> DisposeCtx<W>
where
    W: Widget,
{
    fn new(widget_id: WidgetId, instance_id: InstanceId) -> Self {
        Self {
            widget_id,
            instance_id,
            _marker: PhantomData,
        }
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }
}

pub struct Scheduler<'a, M> {
    state: &'a mut SchedulerState<M>,
    dispatcher: Dispatcher<M>,
}

impl<'a, M> Scheduler<'a, M>
where
    M: Send + 'static,
{
    fn new(state: &'a mut SchedulerState<M>, dispatcher: Dispatcher<M>) -> Self {
        Self { state, dispatcher }
    }

    pub fn after(&mut self, duration: Duration, message: M) -> TimerId {
        self.state.after(duration, message, self.dispatcher.clone())
    }

    pub fn every(&mut self, duration: Duration, message: M) -> TimerId
    where
        M: Clone,
    {
        self.state.every(duration, message, self.dispatcher.clone())
    }

    pub fn cancel(&mut self, timer_id: TimerId) -> bool {
        self.state.cancel(timer_id)
    }

    pub fn clear(&mut self) {
        self.state.clear();
    }
}

pub struct Tasks<'a, M> {
    backend: &'a mut dyn TaskBackend<M>,
}

impl<'a, M> Tasks<'a, M>
where
    M: Send + 'static,
{
    fn new(backend: &'a mut dyn TaskBackend<M>) -> Self {
        Self { backend }
    }

    pub fn spawn<F>(&mut self, future: F) -> TaskId
    where
        F: Future<Output = M> + Send + 'static,
    {
        self.backend.spawn_boxed(None, Box::pin(future))
    }

    pub fn spawn_named<F>(&mut self, name: impl Into<String>, future: F) -> TaskId
    where
        F: Future<Output = M> + Send + 'static,
    {
        self.backend.spawn_boxed(Some(name.into()), Box::pin(future))
    }

    pub fn cancel(&mut self, task_id: TaskId) -> bool {
        self.backend.cancel(task_id)
    }

    pub fn cancel_all(&mut self) {
        self.backend.cancel_all();
    }
}

struct RuntimeServices<M> {
    dispatcher: Dispatcher<M>,
    scheduler: SchedulerState<M>,
    tasks: Box<dyn TaskBackend<M>>,
    render_requested: bool,
}

struct SchedulerState<M> {
    timers: Arc<Mutex<HashMap<TimerId, Arc<AtomicBool>>>>,
    _marker: PhantomData<fn() -> M>,
}

impl<M> SchedulerState<M>
where
    M: Send + 'static,
{
    fn new() -> Self {
        Self {
            timers: Arc::new(Mutex::new(HashMap::new())),
            _marker: PhantomData,
        }
    }

    fn after(&mut self, duration: Duration, message: M, dispatcher: Dispatcher<M>) -> TimerId {
        let timer_id = TimerId::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.timers
            .lock()
            .expect("scheduler timers mutex poisoned")
            .insert(timer_id, cancelled.clone());
        let timers = Arc::clone(&self.timers);
        thread::spawn(move || {
            thread::sleep(duration);
            if !cancelled.load(Ordering::Acquire) {
                let _ = dispatcher.post_message(message);
            }
            let _ = timers
                .lock()
                .expect("scheduler timers mutex poisoned")
                .remove(&timer_id);
        });
        timer_id
    }

    fn every(&mut self, duration: Duration, message: M, dispatcher: Dispatcher<M>) -> TimerId
    where
        M: Clone,
    {
        let timer_id = TimerId::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.timers
            .lock()
            .expect("scheduler timers mutex poisoned")
            .insert(timer_id, cancelled.clone());
        let timers = Arc::clone(&self.timers);
        thread::spawn(move || {
            loop {
                thread::sleep(duration);
                if cancelled.load(Ordering::Acquire) {
                    break;
                }
                if !dispatcher.post_message(message.clone()) {
                    break;
                }
            }
            let _ = timers
                .lock()
                .expect("scheduler timers mutex poisoned")
                .remove(&timer_id);
        });
        timer_id
    }

    fn cancel(&mut self, timer_id: TimerId) -> bool {
        if let Some(cancelled) = self
            .timers
            .lock()
            .expect("scheduler timers mutex poisoned")
            .remove(&timer_id)
        {
            cancelled.store(true, Ordering::Release);
            return true;
        }
        false
    }

    fn clear(&mut self) {
        let mut timers = self.timers.lock().expect("scheduler timers mutex poisoned");
        for cancelled in timers.values() {
            cancelled.store(true, Ordering::Release);
        }
        timers.clear();
    }
}

enum RuntimeEvent<M> {
    Message(M),
    TaskFinished(TaskId),
}

#[derive(Clone, Default)]
struct WakeHandle {
    callback: Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
}

impl WakeHandle {
    fn set<F>(&self, wake: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.callback.lock().expect("wake callback mutex poisoned") = Some(Arc::new(wake));
    }

    fn wake(&self) {
        let callback = self
            .callback
            .lock()
            .expect("wake callback mutex poisoned")
            .clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}

struct Dispatcher<M> {
    sender: Sender<RuntimeEvent<M>>,
    wake: WakeHandle,
}

impl<M> Clone for Dispatcher<M> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            wake: self.wake.clone(),
        }
    }
}

impl<M> Dispatcher<M>
where
    M: Send + 'static,
{
    fn post_message(&self, message: M) -> bool {
        if self.sender.send(RuntimeEvent::Message(message)).is_ok() {
            self.wake.wake();
            return true;
        }
        false
    }

    fn finish_task(&self, task_id: TaskId) {
        if self.sender.send(RuntimeEvent::TaskFinished(task_id)).is_ok() {
            self.wake.wake();
        }
    }
}

type BoxedFuture<M> = Pin<Box<dyn Future<Output = M> + Send + 'static>>;

trait TaskBackend<M>: Send {
    fn spawn_boxed(&mut self, name: Option<String>, future: BoxedFuture<M>) -> TaskId;
    fn cancel(&mut self, task_id: TaskId) -> bool;
    fn cancel_all(&mut self);
    fn reap(&mut self, task_id: TaskId);
}

#[cfg(not(feature = "runtime-tokio"))]
struct DefaultTaskBackend<M> {
    dispatcher: Dispatcher<M>,
    tasks: HashMap<TaskId, DefaultTaskControl>,
}

#[cfg(not(feature = "runtime-tokio"))]
struct DefaultTaskControl {
    #[allow(dead_code)]
    name: Option<String>,
    abort_handle: AbortHandle,
}

#[cfg(not(feature = "runtime-tokio"))]
impl<M> DefaultTaskBackend<M>
where
    M: Send + 'static,
{
    fn new(dispatcher: Dispatcher<M>) -> Self {
        Self {
            dispatcher,
            tasks: HashMap::new(),
        }
    }
}

#[cfg(not(feature = "runtime-tokio"))]
impl<M> TaskBackend<M> for DefaultTaskBackend<M>
where
    M: Send + 'static,
{
    fn spawn_boxed(&mut self, name: Option<String>, future: BoxedFuture<M>) -> TaskId {
        let task_id = TaskId::new();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let dispatcher = self.dispatcher.clone();
        thread::spawn(move || {
            let future = Abortable::new(future, abort_registration);
            if let Ok(message) = futures::executor::block_on(future) {
                let _ = dispatcher.post_message(message);
            }
            dispatcher.finish_task(task_id);
        });
        self.tasks.insert(
            task_id,
            DefaultTaskControl {
                name,
                abort_handle,
            },
        );
        task_id
    }

    fn cancel(&mut self, task_id: TaskId) -> bool {
        if let Some(control) = self.tasks.remove(&task_id) {
            control.abort_handle.abort();
            return true;
        }
        false
    }

    fn cancel_all(&mut self) {
        for (_, control) in self.tasks.drain() {
            control.abort_handle.abort();
        }
    }

    fn reap(&mut self, task_id: TaskId) {
        let _ = self.tasks.remove(&task_id);
    }
}

#[cfg(feature = "runtime-tokio")]
struct TokioTaskBackend<M> {
    dispatcher: Dispatcher<M>,
    runtime: tokio::runtime::Runtime,
    tasks: HashMap<TaskId, TokioTaskControl>,
}

#[cfg(feature = "runtime-tokio")]
struct TokioTaskControl {
    #[allow(dead_code)]
    name: Option<String>,
    join_handle: tokio::task::JoinHandle<()>,
}

#[cfg(feature = "runtime-tokio")]
impl<M> TokioTaskBackend<M>
where
    M: Send + 'static,
{
    fn new(dispatcher: Dispatcher<M>) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime backend must initialize");
        Self {
            dispatcher,
            runtime,
            tasks: HashMap::new(),
        }
    }
}

#[cfg(feature = "runtime-tokio")]
impl<M> TaskBackend<M> for TokioTaskBackend<M>
where
    M: Send + 'static,
{
    fn spawn_boxed(&mut self, name: Option<String>, future: BoxedFuture<M>) -> TaskId {
        let task_id = TaskId::new();
        let dispatcher = self.dispatcher.clone();
        let join_handle = self.runtime.spawn(async move {
            let message = future.await;
            let _ = dispatcher.post_message(message);
            dispatcher.finish_task(task_id);
        });
        self.tasks.insert(
            task_id,
            TokioTaskControl {
                name,
                join_handle,
            },
        );
        task_id
    }

    fn cancel(&mut self, task_id: TaskId) -> bool {
        if let Some(control) = self.tasks.remove(&task_id) {
            control.join_handle.abort();
            return true;
        }
        false
    }

    fn cancel_all(&mut self) {
        for (_, control) in self.tasks.drain() {
            control.join_handle.abort();
        }
    }

    fn reap(&mut self, task_id: TaskId) {
        let _ = self.tasks.remove(&task_id);
    }
}

fn task_backend<M>(dispatcher: Dispatcher<M>) -> Box<dyn TaskBackend<M>>
where
    M: Send + 'static,
{
    #[cfg(feature = "runtime-tokio")]
    {
        return Box::new(TokioTaskBackend::new(dispatcher));
    }

    #[cfg(not(feature = "runtime-tokio"))]
    {
        Box::new(DefaultTaskBackend::new(dispatcher))
    }
}

// TODO(v0.2): add widget instance generation guards for stale messages
// TODO(v0.7): allow lifecycle integration with hybrid/native-web host
// TODO(v0.8): support restart-safe instance isolation guarantees
// TODO(v0.2): named task handles
// TODO(v0.2): task cancellation tokens
// TODO(v0.8): structured concurrency/task groups debug inspection
// TODO(v0.8): expose task diagnostics/devtools hooks
// TODO(v0.3): debounce/throttle helpers
// TODO(v0.8): virtual time/testing scheduler

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration as StdDuration;
    use widgetkit_core::{Color, Point, Rect};
    use widgetkit_render::{SoftwareRenderer, TextStyle};

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
            self.pixels = pixels.to_vec();
            Ok(())
        }
    }

    struct LifecycleWidget {
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    enum LifecycleMsg {
        Boot,
    }

    impl Widget for LifecycleWidget {
        type State = ();
        type Message = LifecycleMsg;

        fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {
            self.log.lock().unwrap().push("mount");
        }

        fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
            self.log.lock().unwrap().push("start");
            ctx.post(LifecycleMsg::Boot);
        }

        fn update(&mut self, _state: &mut Self::State, event: Event<Self::Message>, ctx: &mut UpdateCtx<Self>) {
            if let Event::Message(LifecycleMsg::Boot) = event {
                self.log.lock().unwrap().push("update");
                ctx.request_render();
            }
        }

        fn render(&self, _state: &Self::State, canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {
            self.log.lock().unwrap().push("render");
            canvas.clear(Color::BLACK);
            canvas.text(Point::new(2.0, 2.0), "OK", TextStyle::new(), Color::WHITE);
        }

        fn stop(&mut self, _state: &mut Self::State, ctx: &mut StopCtx<Self>) {
            self.log.lock().unwrap().push("stop");
            ctx.scheduler().clear();
            ctx.tasks().cancel_all();
        }

        fn dispose(&mut self, _state: Self::State, _ctx: &mut DisposeCtx<Self>) {
            self.log.lock().unwrap().push("dispose");
        }
    }

    #[test]
    fn lifecycle_runs_in_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let widget = LifecycleWidget { log: Arc::clone(&log) };
        let mut runner = AppRunner::new("lifecycle", widget, SoftwareRenderer::new());
        runner.initialize(Size::new(64.0, 32.0)).unwrap();
        let mut surface = MemorySurface::new(64, 32);
        runner.render(&mut surface).unwrap();
        runner.shutdown().unwrap();

        assert_eq!(
            *log.lock().unwrap(),
            vec!["mount", "start", "update", "render", "stop", "dispose"]
        );
    }

    struct SchedulerWidget {
        counts: Arc<Mutex<(u32, u32)>>,
    }

    #[derive(Clone)]
    enum SchedulerMsg {
        Once,
        Tick,
    }

    impl Widget for SchedulerWidget {
        type State = ();
        type Message = SchedulerMsg;

        fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {}

        fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
            ctx.scheduler().after(StdDuration::from_millis(5), SchedulerMsg::Once);
            ctx.scheduler().every(StdDuration::from_millis(5), SchedulerMsg::Tick);
        }

        fn update(&mut self, _state: &mut Self::State, event: Event<Self::Message>, _ctx: &mut UpdateCtx<Self>) {
            match event {
                Event::Message(SchedulerMsg::Once) => self.counts.lock().unwrap().0 += 1,
                Event::Message(SchedulerMsg::Tick) => self.counts.lock().unwrap().1 += 1,
                Event::Host(_) => {}
            }
        }

        fn render(&self, _state: &Self::State, canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {
            canvas.clear(Color::BLACK);
            canvas.rect(Rect::xywh(0.0, 0.0, 1.0, 1.0), Color::WHITE);
        }

        fn stop(&mut self, _state: &mut Self::State, ctx: &mut StopCtx<Self>) {
            ctx.scheduler().clear();
        }
    }

    #[test]
    fn scheduler_routes_messages_and_stop_cancels_future_ticks() {
        let counts = Arc::new(Mutex::new((0, 0)));
        let widget = SchedulerWidget { counts: Arc::clone(&counts) };
        let mut runner = AppRunner::new("scheduler", widget, SoftwareRenderer::new());
        runner.initialize(Size::new(32.0, 32.0)).unwrap();
        thread::sleep(StdDuration::from_millis(18));
        runner.process_pending().unwrap();
        let before_shutdown = *counts.lock().unwrap();
        assert_eq!(before_shutdown.0, 1);
        assert!(before_shutdown.1 >= 2);

        runner.shutdown().unwrap();
        thread::sleep(StdDuration::from_millis(15));
        runner.process_pending().unwrap();
        assert_eq!(*counts.lock().unwrap(), before_shutdown);
    }

    struct TaskWidget {
        hits: Arc<Mutex<u32>>,
    }

    enum TaskMsg {
        Loaded,
    }

    impl Widget for TaskWidget {
        type State = ();
        type Message = TaskMsg;

        fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {}

        fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
            ctx.tasks().spawn(async { TaskMsg::Loaded });
        }

        fn update(&mut self, _state: &mut Self::State, event: Event<Self::Message>, _ctx: &mut UpdateCtx<Self>) {
            if let Event::Message(TaskMsg::Loaded) = event {
                *self.hits.lock().unwrap() += 1;
            }
        }
    }

    #[test]
    fn task_backend_routes_completions_to_widget_messages() {
        let hits = Arc::new(Mutex::new(0));
        let widget = TaskWidget { hits: Arc::clone(&hits) };
        let mut runner = AppRunner::new("tasks", widget, SoftwareRenderer::new());
        runner.initialize(Size::new(32.0, 32.0)).unwrap();
        thread::sleep(StdDuration::from_millis(10));
        runner.process_pending().unwrap();
        runner.shutdown().unwrap();

        assert_eq!(*hits.lock().unwrap(), 1);
    }
}
