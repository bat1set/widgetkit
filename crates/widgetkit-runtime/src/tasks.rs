use crate::internal::Dispatcher;
use futures::Future;
#[cfg(not(feature = "runtime-tokio"))]
use futures::future::{AbortHandle, Abortable};
#[cfg(not(feature = "runtime-tokio"))]
use std::thread;
use std::{collections::HashMap, pin::Pin};
use widgetkit_core::TaskId;

pub struct Tasks<'a, M> {
    backend: &'a mut dyn TaskBackend<M>,
}

impl<'a, M> Tasks<'a, M>
where
    M: Send + 'static,
{
    pub(crate) fn new(backend: &'a mut dyn TaskBackend<M>) -> Self {
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
        self.backend
            .spawn_boxed(Some(name.into()), Box::pin(future))
    }

    pub fn cancel(&mut self, task_id: TaskId) -> bool {
        self.backend.cancel(task_id)
    }

    pub fn cancel_all(&mut self) {
        self.backend.cancel_all();
    }
}

pub(crate) type BoxedFuture<M> = Pin<Box<dyn Future<Output = M> + Send + 'static>>;

pub(crate) trait TaskBackend<M>: Send {
    fn spawn_boxed(&mut self, name: Option<String>, future: BoxedFuture<M>) -> TaskId;
    fn cancel(&mut self, task_id: TaskId) -> bool;
    fn cancel_all(&mut self);
    fn reap(&mut self, task_id: TaskId);
    fn shutdown(&mut self);
    #[cfg(test)]
    fn active_count(&self) -> usize;
}

pub(crate) fn task_backend<M>(dispatcher: Dispatcher<M>) -> Box<dyn TaskBackend<M>>
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

#[cfg(not(feature = "runtime-tokio"))]
struct DefaultTaskBackend<M> {
    dispatcher: Dispatcher<M>,
    tasks: HashMap<TaskId, DefaultTaskControl>,
    shutting_down: bool,
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
            shutting_down: false,
        }
    }

    fn close(&mut self) {
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;
        self.cancel_all();
    }
}

#[cfg(not(feature = "runtime-tokio"))]
impl<M> TaskBackend<M> for DefaultTaskBackend<M>
where
    M: Send + 'static,
{
    fn spawn_boxed(&mut self, name: Option<String>, future: BoxedFuture<M>) -> TaskId {
        let task_id = TaskId::new();
        if self.shutting_down {
            drop(future);
            self.dispatcher.finish_task(task_id);
            return task_id;
        }

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let dispatcher = self.dispatcher.clone();
        thread::spawn(move || {
            let future = Abortable::new(future, abort_registration);
            if let Ok(message) = futures::executor::block_on(future) {
                let _ = dispatcher.post_message(message);
            }
            dispatcher.finish_task(task_id);
        });
        self.tasks
            .insert(task_id, DefaultTaskControl { name, abort_handle });
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

    fn shutdown(&mut self) {
        self.close();
    }

    #[cfg(test)]
    fn active_count(&self) -> usize {
        self.tasks.len()
    }
}

#[cfg(not(feature = "runtime-tokio"))]
impl<M> Drop for DefaultTaskBackend<M> {
    fn drop(&mut self) {
        self.shutting_down = true;
        for (_, control) in self.tasks.drain() {
            control.abort_handle.abort();
        }
    }
}

#[cfg(feature = "runtime-tokio")]
struct TokioTaskBackend<M> {
    dispatcher: Dispatcher<M>,
    runtime: tokio::runtime::Runtime,
    tasks: HashMap<TaskId, TokioTaskControl>,
    shutting_down: bool,
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
            shutting_down: false,
        }
    }

    fn close(&mut self) {
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;
        self.cancel_all();
    }
}

#[cfg(feature = "runtime-tokio")]
impl<M> TaskBackend<M> for TokioTaskBackend<M>
where
    M: Send + 'static,
{
    fn spawn_boxed(&mut self, name: Option<String>, future: BoxedFuture<M>) -> TaskId {
        let task_id = TaskId::new();
        if self.shutting_down {
            drop(future);
            self.dispatcher.finish_task(task_id);
            return task_id;
        }

        let dispatcher = self.dispatcher.clone();
        let join_handle = self.runtime.spawn(async move {
            let message = future.await;
            let _ = dispatcher.post_message(message);
            dispatcher.finish_task(task_id);
        });
        self.tasks
            .insert(task_id, TokioTaskControl { name, join_handle });
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

    fn shutdown(&mut self) {
        self.close();
    }

    #[cfg(test)]
    fn active_count(&self) -> usize {
        self.tasks.len()
    }
}

#[cfg(feature = "runtime-tokio")]
impl<M> Drop for TokioTaskBackend<M> {
    fn drop(&mut self) {
        self.shutting_down = true;
        for (_, control) in self.tasks.drain() {
            control.join_handle.abort();
        }
    }
}
