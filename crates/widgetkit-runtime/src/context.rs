use crate::{internal::RuntimeServices, scheduler::Scheduler, tasks::Tasks, widget::Widget};
use std::{marker::PhantomData, ptr::NonNull};
use widgetkit_core::{Constraints, InstanceId, Size, WidgetId};

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
    pub(crate) fn new(widget_id: WidgetId, instance_id: InstanceId) -> Self {
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
    pub(crate) fn new(
        widget_id: WidgetId,
        instance_id: InstanceId,
        services: NonNull<RuntimeServices<W::Message>>,
    ) -> Self {
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

    /// Marks the current frame dirty and wakes the host if this is the first pending redraw.
    ///
    /// Repeated calls before the host consumes the pending frame are coalesced into one redraw.
    pub fn request_render(&mut self) {
        let services = self.services_mut();
        if services.request_render() {
            services.dispatcher.wake.wake();
        }
    }

    /// Returns the scheduler owned by the current widget instance.
    /// All timers created here are cleared when the instance stops or is disposed.
    pub fn scheduler(&mut self) -> Scheduler<'_, W::Message> {
        let services = self.services_mut();
        Scheduler::new(&mut services.scheduler, services.dispatcher.clone())
    }

    /// Returns the task backend owned by the current widget instance.
    /// All tasks created here are canceled when the instance stops or is disposed.
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
    pub(crate) fn new(
        widget_id: WidgetId,
        instance_id: InstanceId,
        services: NonNull<RuntimeServices<W::Message>>,
    ) -> Self {
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

    /// Marks the current frame dirty and wakes the host if this is the first pending redraw.
    ///
    /// Repeated calls before the host consumes the pending frame are coalesced into one redraw.
    pub fn request_render(&mut self) {
        let services = self.services_mut();
        if services.request_render() {
            services.dispatcher.wake.wake();
        }
    }

    /// Returns the scheduler owned by the current widget instance.
    /// All timers created here are cleared when the instance stops or is disposed.
    pub fn scheduler(&mut self) -> Scheduler<'_, W::Message> {
        let services = self.services_mut();
        Scheduler::new(&mut services.scheduler, services.dispatcher.clone())
    }

    /// Returns the task backend owned by the current widget instance.
    /// All tasks created here are canceled when the instance stops or is disposed.
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
    pub(crate) fn new(widget_id: WidgetId, instance_id: InstanceId, surface_size: Size) -> Self {
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

pub struct LayoutCtx<W>
where
    W: Widget,
{
    widget_id: WidgetId,
    instance_id: InstanceId,
    available_size: Size,
    constraints: Constraints,
    _marker: PhantomData<fn() -> W>,
}

impl<W> LayoutCtx<W>
where
    W: Widget,
{
    pub(crate) fn new(
        widget_id: WidgetId,
        instance_id: InstanceId,
        available_size: Size,
        constraints: Constraints,
    ) -> Self {
        Self {
            widget_id,
            instance_id,
            available_size,
            constraints,
            _marker: PhantomData,
        }
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    pub fn available_size(&self) -> Size {
        self.available_size
    }

    pub fn constraints(&self) -> Constraints {
        self.constraints
    }

    pub fn constrain(&self, size: Size) -> Size {
        self.constraints.clamp(size)
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
    pub(crate) fn new(
        widget_id: WidgetId,
        instance_id: InstanceId,
        services: NonNull<RuntimeServices<W::Message>>,
    ) -> Self {
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

    /// Returns the scheduler owned by the current widget instance.
    /// All timers created here are cleared when the instance stops or is disposed.
    pub fn scheduler(&mut self) -> Scheduler<'_, W::Message> {
        let services = self.services_mut();
        Scheduler::new(&mut services.scheduler, services.dispatcher.clone())
    }

    /// Returns the task backend owned by the current widget instance.
    /// All tasks created here are canceled when the instance stops or is disposed.
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
    pub(crate) fn new(widget_id: WidgetId, instance_id: InstanceId) -> Self {
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
