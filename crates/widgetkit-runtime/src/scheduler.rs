use crate::internal::Dispatcher;
use std::{collections::HashMap, marker::PhantomData, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, thread};
use widgetkit_core::{Duration, TimerId};

pub struct Scheduler<'a, M> {
    state: &'a mut SchedulerState<M>,
    dispatcher: Dispatcher<M>,
}

impl<'a, M> Scheduler<'a, M>
where
    M: Send + 'static,
{
    pub(crate) fn new(state: &'a mut SchedulerState<M>, dispatcher: Dispatcher<M>) -> Self {
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

pub(crate) struct SchedulerState<M> {
    timers: Arc<Mutex<HashMap<TimerId, Arc<AtomicBool>>>>,
    _marker: PhantomData<fn() -> M>,
}

impl<M> SchedulerState<M>
where
    M: Send + 'static,
{
    pub(crate) fn new() -> Self {
        Self {
            timers: Arc::new(Mutex::new(HashMap::new())),
            _marker: PhantomData,
        }
    }

    fn after(&mut self, duration: Duration, message: M, dispatcher: Dispatcher<M>) -> TimerId {
        let timer_id = TimerId::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.timers.lock().expect("scheduler timers mutex poisoned").insert(timer_id, cancelled.clone());
        let timers = Arc::clone(&self.timers);
        thread::spawn(move || {
            thread::sleep(duration);
            if !cancelled.load(Ordering::Acquire) {
                let _ = dispatcher.post_message(message);
            }
            let _ = timers.lock().expect("scheduler timers mutex poisoned").remove(&timer_id);
        });
        timer_id
    }

    fn every(&mut self, duration: Duration, message: M, dispatcher: Dispatcher<M>) -> TimerId
    where
        M: Clone,
    {
        let timer_id = TimerId::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.timers.lock().expect("scheduler timers mutex poisoned").insert(timer_id, cancelled.clone());
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
            let _ = timers.lock().expect("scheduler timers mutex poisoned").remove(&timer_id);
        });
        timer_id
    }

    fn cancel(&mut self, timer_id: TimerId) -> bool {
        if let Some(cancelled) = self.timers.lock().expect("scheduler timers mutex poisoned").remove(&timer_id) {
            cancelled.store(true, Ordering::Release);
            return true;
        }
        false
    }

    pub(crate) fn clear(&mut self) {
        let mut timers = self.timers.lock().expect("scheduler timers mutex poisoned");
        for cancelled in timers.values() {
            cancelled.store(true, Ordering::Release);
        }
        timers.clear();
    }
}
