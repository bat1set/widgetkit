use crate::{scheduler::SchedulerState, tasks::{TaskBackend, task_backend}};
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
    pub(crate) render_requested: bool,
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
            render_requested: true,
        }
    }
}

pub(crate) enum RuntimeEvent<M> {
    Message(MessageEnvelope<M>),
    TaskFinished { token: DispatchToken, task_id: TaskId },
    TimerFinished { token: DispatchToken, timer_id: TimerId },
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
        let callback = self.callback.lock().expect("wake callback mutex poisoned").clone();
        if let Some(callback) = callback {
            callback();
        }
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
