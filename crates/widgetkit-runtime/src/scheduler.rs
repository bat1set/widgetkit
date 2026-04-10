use crate::internal::Dispatcher;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
    thread::{self, JoinHandle},
    time::Instant,
};
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
    command_tx: Option<Sender<SchedulerCommand<M>>>,
    active_timers: HashSet<TimerId>,
    worker: Option<JoinHandle<()>>,
}

impl<M> SchedulerState<M>
where
    M: Send + 'static,
{
    pub(crate) fn new(dispatcher: Dispatcher<M>) -> Self {
        let (command_tx, command_rx) = unbounded();
        let worker = thread::spawn(move || scheduler_worker(dispatcher, command_rx));
        Self {
            command_tx: Some(command_tx),
            active_timers: HashSet::new(),
            worker: Some(worker),
        }
    }

    fn after(&mut self, duration: Duration, message: M, _dispatcher: Dispatcher<M>) -> TimerId {
        let timer_id = TimerId::new();
        self.active_timers.insert(timer_id);
        self.send_command(SchedulerCommand::Schedule {
            timer_id,
            deadline: Instant::now() + duration,
            interval: None,
            delivery: TimerDelivery::Once(Some(message)),
        });
        timer_id
    }

    fn every(&mut self, duration: Duration, message: M, _dispatcher: Dispatcher<M>) -> TimerId
    where
        M: Clone,
    {
        let timer_id = TimerId::new();
        self.active_timers.insert(timer_id);
        let factory: Box<dyn Fn() -> M + Send> = Box::new(move || message.clone());
        self.send_command(SchedulerCommand::Schedule {
            timer_id,
            deadline: Instant::now() + duration,
            interval: Some(duration),
            delivery: TimerDelivery::Repeat(factory),
        });
        timer_id
    }

    fn cancel(&mut self, timer_id: TimerId) -> bool {
        let existed = self.active_timers.remove(&timer_id);
        if existed {
            self.send_command(SchedulerCommand::Cancel { timer_id });
        }
        existed
    }

    pub(crate) fn reap(&mut self, timer_id: TimerId) {
        self.active_timers.remove(&timer_id);
    }

    pub(crate) fn clear(&mut self) {
        if self.active_timers.is_empty() {
            return;
        }
        self.active_timers.clear();
        self.send_command(SchedulerCommand::Clear);
    }

    pub(crate) fn shutdown(&mut self) {
        self.active_timers.clear();
        if let Some(command_tx) = self.command_tx.take() {
            let _ = command_tx.send(SchedulerCommand::Shutdown);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    #[cfg(test)]
    pub(crate) fn active_count(&self) -> usize {
        self.active_timers.len()
    }

    fn send_command(&self, command: SchedulerCommand<M>) {
        if let Some(command_tx) = self.command_tx.as_ref() {
            let _ = command_tx.send(command);
        }
    }
}

impl<M> Drop for SchedulerState<M> {
    fn drop(&mut self) {
        self.active_timers.clear();
        if let Some(command_tx) = self.command_tx.take() {
            let _ = command_tx.send(SchedulerCommand::Shutdown);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

enum SchedulerCommand<M> {
    Schedule {
        timer_id: TimerId,
        deadline: Instant,
        interval: Option<Duration>,
        delivery: TimerDelivery<M>,
    },
    Cancel {
        timer_id: TimerId,
    },
    Clear,
    Shutdown,
}

enum TimerDelivery<M> {
    Once(Option<M>),
    Repeat(Box<dyn Fn() -> M + Send>),
}

struct TimerEntry<M> {
    deadline: Instant,
    interval: Option<Duration>,
    delivery: TimerDelivery<M>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeadlineKey {
    deadline: Instant,
    timer_id: TimerId,
}

impl Ord for DeadlineKey {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .deadline
            .cmp(&self.deadline)
            .then_with(|| other.timer_id.into_raw().cmp(&self.timer_id.into_raw()))
    }
}

impl PartialOrd for DeadlineKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn scheduler_worker<M>(dispatcher: Dispatcher<M>, command_rx: Receiver<SchedulerCommand<M>>)
where
    M: Send + 'static,
{
    let mut entries: HashMap<TimerId, TimerEntry<M>> = HashMap::new();
    let mut deadlines = BinaryHeap::new();

    loop {
        dispatch_due(&dispatcher, &mut entries, &mut deadlines);

        let Some(timeout) = next_timeout(&entries, &mut deadlines) else {
            match command_rx.recv() {
                Ok(command) => {
                    if !apply_command(command, &mut entries, &mut deadlines) {
                        break;
                    }
                }
                Err(_) => break,
            }
            continue;
        };

        match command_rx.recv_timeout(timeout) {
            Ok(command) => {
                if !apply_command(command, &mut entries, &mut deadlines) {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    entries.clear();
    deadlines.clear();
}

fn apply_command<M>(
    command: SchedulerCommand<M>,
    entries: &mut HashMap<TimerId, TimerEntry<M>>,
    deadlines: &mut BinaryHeap<DeadlineKey>,
) -> bool {
    match command {
        SchedulerCommand::Schedule {
            timer_id,
            deadline,
            interval,
            delivery,
        } => {
            entries.insert(
                timer_id,
                TimerEntry {
                    deadline,
                    interval,
                    delivery,
                },
            );
            deadlines.push(DeadlineKey { deadline, timer_id });
            true
        }
        SchedulerCommand::Cancel { timer_id } => {
            entries.remove(&timer_id);
            true
        }
        SchedulerCommand::Clear => {
            entries.clear();
            deadlines.clear();
            true
        }
        SchedulerCommand::Shutdown => false,
    }
}

fn dispatch_due<M>(
    dispatcher: &Dispatcher<M>,
    entries: &mut HashMap<TimerId, TimerEntry<M>>,
    deadlines: &mut BinaryHeap<DeadlineKey>,
) where
    M: Send + 'static,
{
    let now = Instant::now();
    loop {
        prune_stale(entries, deadlines);
        let Some(next) = deadlines.peek().copied() else {
            break;
        };
        if next.deadline > now {
            break;
        }
        let _ = deadlines.pop();

        let Some(entry) = entries.get_mut(&next.timer_id) else {
            continue;
        };
        if entry.deadline != next.deadline {
            continue;
        }

        match &mut entry.delivery {
            TimerDelivery::Once(message) => {
                if let Some(message) = message.take() {
                    let _ = dispatcher.post_message(message);
                }
                entries.remove(&next.timer_id);
                dispatcher.finish_timer(next.timer_id);
            }
            TimerDelivery::Repeat(factory) => {
                let _ = dispatcher.post_message(factory());
                let interval = entry
                    .interval
                    .expect("repeat timers must carry an interval");
                entry.deadline = advance_deadline(entry.deadline, interval, now);
                deadlines.push(DeadlineKey {
                    deadline: entry.deadline,
                    timer_id: next.timer_id,
                });
            }
        }
    }
}

fn next_timeout<M>(
    entries: &HashMap<TimerId, TimerEntry<M>>,
    deadlines: &mut BinaryHeap<DeadlineKey>,
) -> Option<Duration> {
    prune_stale(entries, deadlines);
    deadlines
        .peek()
        .map(|next| next.deadline.saturating_duration_since(Instant::now()))
}

fn prune_stale<M>(
    entries: &HashMap<TimerId, TimerEntry<M>>,
    deadlines: &mut BinaryHeap<DeadlineKey>,
) {
    while let Some(next) = deadlines.peek() {
        let Some(entry) = entries.get(&next.timer_id) else {
            let _ = deadlines.pop();
            continue;
        };
        if entry.deadline != next.deadline {
            let _ = deadlines.pop();
            continue;
        }
        break;
    }
}

fn advance_deadline(previous_deadline: Instant, interval: Duration, now: Instant) -> Instant {
    let mut next_deadline = previous_deadline + interval;
    while next_deadline <= now {
        next_deadline += interval;
    }
    next_deadline
}
