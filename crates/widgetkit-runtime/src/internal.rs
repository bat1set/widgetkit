use crate::{
    scheduler::SchedulerState,
    tasks::{TaskBackend, task_backend},
};
use crossbeam_channel::Sender;
use std::sync::{Arc, Mutex};
use widgetkit_core::{InstanceId, TaskId, TimerId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DispatchToken {
    pub(crate) instance_id: InstanceId,
    pub(crate) generation: u64,
}

impl DispatchToken {
    pub(crate) const fn new(instance_id: InstanceId, generation: u64) -> Self {
        Self {
            instance_id,
            generation,
        }
    }
}

#[derive(Debug)]
pub(crate) struct MessageEnvelope<M> {
    pub(crate) token: DispatchToken,
    pub(crate) message: M,
}

pub(crate) struct RuntimeServices<M> {
    pub(crate) dispatcher: Dispatcher<M>,
    pub(crate) scheduler: SchedulerState<M>,
    pub(crate) tasks: Box<dyn TaskBackend<M>>,
    invalidation: InvalidationState,
}

impl<M> RuntimeServices<M>
where
    M: Send + 'static,
{
    pub(crate) fn new(dispatcher: Dispatcher<M>) -> Self {
        Self {
            dispatcher: dispatcher.clone(),
            scheduler: SchedulerState::new(dispatcher.clone()),
            tasks: task_backend(dispatcher),
            invalidation: InvalidationState::default(),
        }
    }

    pub(crate) fn request_render(&mut self) -> bool {
        self.invalidation.request_render()
    }

    pub(crate) fn needs_redraw(&self) -> bool {
        self.invalidation.is_render_dirty()
    }

    pub(crate) fn take_redraw_request(&mut self) -> bool {
        self.invalidation.take_render_request()
    }

    pub(crate) fn begin_render(&mut self) -> bool {
        self.invalidation.begin_render()
    }

    pub(crate) fn finish_render(&mut self) {
        self.invalidation.finish_render();
    }

    pub(crate) fn needs_layout(&self) -> bool {
        self.invalidation.is_layout_dirty()
    }

    pub(crate) fn take_layout_request(&mut self) -> bool {
        self.invalidation.take_layout_request()
    }

    pub(crate) fn clear_redraw(&mut self) {
        self.invalidation.clear();
    }
}

pub(crate) enum RuntimeEvent<M> {
    Message(MessageEnvelope<M>),
    TaskFinished {
        token: DispatchToken,
        task_id: TaskId,
    },
    TimerFinished {
        token: DispatchToken,
        timer_id: TimerId,
    },
}

#[derive(Clone, Default)]
pub(crate) struct WakeHandle {
    callback: Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
}

impl WakeHandle {
    pub(crate) fn set<F>(&self, wake: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.callback.lock().expect("wake callback mutex poisoned") = Some(Arc::new(wake));
    }

    pub(crate) fn wake(&self) {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct InvalidationState {
    render_dirty: bool,
    render_scheduled: bool,
    layout_dirty: bool,
}

impl InvalidationState {
    fn request_render(&mut self) -> bool {
        let should_wake = !self.render_dirty;
        self.render_dirty = true;
        self.layout_dirty = true;
        should_wake
    }

    fn is_render_dirty(self) -> bool {
        self.render_dirty
    }

    fn is_layout_dirty(self) -> bool {
        self.layout_dirty
    }

    fn take_layout_request(&mut self) -> bool {
        if !self.layout_dirty {
            return false;
        }

        self.layout_dirty = false;
        true
    }

    fn take_render_request(&mut self) -> bool {
        if !self.render_dirty || self.render_scheduled {
            return false;
        }

        self.render_scheduled = true;
        true
    }

    fn begin_render(&mut self) -> bool {
        if !self.render_dirty {
            self.render_scheduled = false;
            return false;
        }

        self.render_scheduled = false;
        true
    }

    fn finish_render(&mut self) {
        self.render_dirty = false;
        self.render_scheduled = false;
    }

    fn clear(&mut self) {
        self.render_dirty = false;
        self.render_scheduled = false;
        self.layout_dirty = false;
    }
}

pub(crate) struct Dispatcher<M> {
    pub(crate) sender: Sender<RuntimeEvent<M>>,
    pub(crate) wake: WakeHandle,
    pub(crate) token: DispatchToken,
}

impl<M> Clone for Dispatcher<M> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            wake: self.wake.clone(),
            token: self.token,
        }
    }
}

impl<M> Dispatcher<M>
where
    M: Send + 'static,
{
    pub(crate) fn post_message(&self, message: M) -> bool {
        let envelope = MessageEnvelope {
            token: self.token,
            message,
        };
        if self.sender.send(RuntimeEvent::Message(envelope)).is_ok() {
            self.wake.wake();
            return true;
        }
        false
    }

    pub(crate) fn finish_task(&self, task_id: TaskId) {
        if self
            .sender
            .send(RuntimeEvent::TaskFinished {
                token: self.token,
                task_id,
            })
            .is_ok()
        {
            self.wake.wake();
        }
    }

    pub(crate) fn finish_timer(&self, timer_id: TimerId) {
        if self
            .sender
            .send(RuntimeEvent::TimerFinished {
                token: self.token,
                timer_id,
            })
            .is_ok()
        {
            self.wake.wake();
        }
    }
}
