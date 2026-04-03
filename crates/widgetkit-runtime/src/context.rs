use crate::{internal::RuntimeServices, scheduler::Scheduler, tasks::Tasks, widget::Widget};
use std::{marker::PhantomData, ptr::NonNull};
use widgetkit_core::{InstanceId, Size, WidgetId};

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
    pub(crate) fn new(widget_id: WidgetId, instance_id: InstanceId, services: NonNull<RuntimeServices<W::Message>>) -> Self {
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
    pub(crate) fn new(widget_id: WidgetId, instance_id: InstanceId, services: NonNull<RuntimeServices<W::Message>>) -> Self {
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
    pub(crate) fn new(widget_id: WidgetId, instance_id: InstanceId, services: NonNull<RuntimeServices<W::Message>>) -> Self {
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
