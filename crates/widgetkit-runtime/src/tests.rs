use crate::{
    AppRunner, DisposeCtx, Event, MountCtx, RenderCtx, StartCtx, StopCtx, UpdateCtx, Widget,
    internal::DispatchToken,
};
use futures::future;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration as StdDuration,
};
use widgetkit_core::{Color, Point, Rect, Result, Size};
use widgetkit_render::{Canvas, RenderSurface, SoftwareRenderer, TextStyle};

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
fn scheduler_routes_messages_and_reaps_completed_timers() {
    let counts = Arc::new(Mutex::new((0, 0)));
    let widget = SchedulerWidget { counts: Arc::clone(&counts) };
    let mut runner = AppRunner::new("scheduler", widget, SoftwareRenderer::new());
    runner.initialize(Size::new(32.0, 32.0)).unwrap();
    assert_eq!(runner.scheduler_active_count(), 2);

    thread::sleep(StdDuration::from_millis(18));
    runner.process_pending().unwrap();
    let snapshot = *counts.lock().unwrap();
    assert_eq!(snapshot.0, 1);
    assert!(snapshot.1 >= 1);
    assert_eq!(runner.scheduler_active_count(), 1);

    runner.shutdown().unwrap();
    assert_eq!(runner.scheduler_active_count(), 0);
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
    assert_eq!(runner.task_active_count(), 0);
}

struct PendingTaskWidget;

enum PendingTaskMsg {}

impl Widget for PendingTaskWidget {
    type State = ();
    type Message = PendingTaskMsg;

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {}

    fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
        ctx.tasks().spawn(async { future::pending::<PendingTaskMsg>().await });
    }
}

#[test]
fn shutdown_cleans_up_pending_tasks() {
    let mut runner = AppRunner::new("pending-tasks", PendingTaskWidget, SoftwareRenderer::new());
    runner.initialize(Size::new(32.0, 32.0)).unwrap();
    assert_eq!(runner.task_active_count(), 1);

    runner.shutdown().unwrap();
    assert_eq!(runner.task_active_count(), 0);
}

struct GuardWidget {
    hits: Arc<Mutex<u32>>,
}

enum GuardMsg {
    Hit,
}

impl Widget for GuardWidget {
    type State = ();
    type Message = GuardMsg;

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {}

    fn update(&mut self, _state: &mut Self::State, event: Event<Self::Message>, _ctx: &mut UpdateCtx<Self>) {
        if let Event::Message(GuardMsg::Hit) = event {
            *self.hits.lock().unwrap() += 1;
        }
    }
}

#[test]
fn stale_instance_messages_are_ignored_after_shutdown() {
    let hits = Arc::new(Mutex::new(0));
    let widget = GuardWidget { hits: Arc::clone(&hits) };
    let mut runner = AppRunner::new("guard", widget, SoftwareRenderer::new());
    runner.initialize(Size::new(32.0, 32.0)).unwrap();
    let stale_token = runner.test_token();
    runner.shutdown().unwrap();
    runner.dispatch_test_message(stale_token, GuardMsg::Hit);
    runner.process_pending().unwrap();

    assert_eq!(*hits.lock().unwrap(), 0);
}

#[test]
fn stale_generation_token_does_not_match_current_runtime() {
    let token = DispatchToken::new(widgetkit_core::InstanceId::new(), 1);
    let next_generation = DispatchToken::new(token.instance_id, 2);
    assert_ne!(token, next_generation);
}
