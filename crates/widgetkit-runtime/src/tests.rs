use crate::{
    AppRunner, DisposeCtx, Event, LayoutCtx, MountCtx, RenderCtx, StartCtx, StopCtx, UpdateCtx,
    Widget, internal::DispatchToken,
};
use futures::future;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Arc, Mutex},
    thread,
    time::{Duration as StdDuration, Instant},
};
use widgetkit_core::{Color, Constraints, Point, Rect, Result, Size};
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

    fn update(
        &mut self,
        _state: &mut Self::State,
        event: Event<Self::Message>,
        ctx: &mut UpdateCtx<Self>,
    ) {
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
    let widget = LifecycleWidget {
        log: Arc::clone(&log),
    };
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
        ctx.scheduler()
            .after(StdDuration::from_millis(5), SchedulerMsg::Once);
        ctx.scheduler()
            .every(StdDuration::from_millis(5), SchedulerMsg::Tick);
    }

    fn update(
        &mut self,
        _state: &mut Self::State,
        event: Event<Self::Message>,
        _ctx: &mut UpdateCtx<Self>,
    ) {
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
    let widget = SchedulerWidget {
        counts: Arc::clone(&counts),
    };
    let mut runner = AppRunner::new("scheduler", widget, SoftwareRenderer::new());
    runner.initialize(Size::new(32.0, 32.0)).unwrap();
    assert_eq!(runner.scheduler_active_count(), 2);

    let deadline = Instant::now() + StdDuration::from_millis(100);
    while {
        let snapshot = *counts.lock().unwrap();
        (snapshot.0 < 1 || snapshot.1 < 1) && Instant::now() < deadline
    } {
        thread::sleep(StdDuration::from_millis(5));
        runner.process_pending().unwrap();
    }
    let snapshot = *counts.lock().unwrap();
    assert_eq!(snapshot.0, 1);
    assert!(snapshot.1 >= 1);
    assert_eq!(runner.scheduler_active_count(), 1);

    runner.shutdown().unwrap();
    thread::sleep(StdDuration::from_millis(12));
    runner.process_pending().unwrap();
    assert_eq!(*counts.lock().unwrap(), snapshot);
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

    fn update(
        &mut self,
        _state: &mut Self::State,
        event: Event<Self::Message>,
        _ctx: &mut UpdateCtx<Self>,
    ) {
        if let Event::Message(TaskMsg::Loaded) = event {
            *self.hits.lock().unwrap() += 1;
        }
    }
}

#[test]
fn task_backend_routes_completions_to_widget_messages() {
    let hits = Arc::new(Mutex::new(0));
    let widget = TaskWidget {
        hits: Arc::clone(&hits),
    };
    let mut runner = AppRunner::new("tasks", widget, SoftwareRenderer::new());
    runner.initialize(Size::new(32.0, 32.0)).unwrap();
    let deadline = Instant::now() + StdDuration::from_millis(100);
    while *hits.lock().unwrap() == 0 && Instant::now() < deadline {
        thread::sleep(StdDuration::from_millis(5));
        runner.process_pending().unwrap();
    }
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
        ctx.tasks()
            .spawn(async { future::pending::<PendingTaskMsg>().await });
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

    fn update(
        &mut self,
        _state: &mut Self::State,
        event: Event<Self::Message>,
        _ctx: &mut UpdateCtx<Self>,
    ) {
        if let Event::Message(GuardMsg::Hit) = event {
            *self.hits.lock().unwrap() += 1;
        }
    }
}

#[test]
fn stale_instance_messages_are_ignored_after_shutdown() {
    let hits = Arc::new(Mutex::new(0));
    let widget = GuardWidget {
        hits: Arc::clone(&hits),
    };
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

struct CoalescedRedrawWidget;

enum CoalescedRedrawMsg {}

impl Widget for CoalescedRedrawWidget {
    type State = ();
    type Message = CoalescedRedrawMsg;

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {}

    fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
        ctx.request_render();
        ctx.request_render();
        ctx.request_render();
    }
}

#[test]
fn repeated_render_requests_collapse_until_the_pending_frame_is_rendered() {
    let wakes = Arc::new(AtomicUsize::new(0));
    let mut runner = AppRunner::new(
        "coalesced-redraw",
        CoalescedRedrawWidget,
        SoftwareRenderer::new(),
    );
    let wake_count = Arc::clone(&wakes);
    runner.attach_waker(move || {
        wake_count.fetch_add(1, Ordering::SeqCst);
    });

    runner.initialize(Size::new(32.0, 32.0)).unwrap();

    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    assert!(runner.needs_redraw());
    assert!(runner.take_redraw_request());
    assert!(!runner.take_redraw_request());

    let mut surface = MemorySurface::new(32, 32);
    runner.render(&mut surface).unwrap();

    assert!(!runner.needs_redraw());
    assert!(!runner.take_redraw_request());
}

struct LateRenderWidget {
    renders: Arc<Mutex<u32>>,
}

enum LateRenderMsg {}

impl Widget for LateRenderWidget {
    type State = ();
    type Message = LateRenderMsg;

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {}

    fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
        ctx.request_render();
    }

    fn render(&self, _state: &Self::State, canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {
        *self.renders.lock().unwrap() += 1;
        canvas.clear(Color::BLACK);
    }
}

#[test]
fn shutdown_clears_pending_redraw_and_skips_late_render_calls() {
    let renders = Arc::new(Mutex::new(0));
    let widget = LateRenderWidget {
        renders: Arc::clone(&renders),
    };
    let mut runner = AppRunner::new("late-render", widget, SoftwareRenderer::new());
    runner.initialize(Size::new(32.0, 32.0)).unwrap();

    assert!(runner.needs_redraw());
    assert!(runner.take_redraw_request());

    runner.shutdown().unwrap();

    assert!(!runner.needs_redraw());
    assert!(!runner.take_redraw_request());

    let mut surface = MemorySurface::new(32, 32);
    runner.render(&mut surface).unwrap();

    assert_eq!(*renders.lock().unwrap(), 0);
}

struct PreferredSizeWidget;

enum PreferredSizeMsg {}

impl Widget for PreferredSizeWidget {
    type State = Size;
    type Message = PreferredSizeMsg;

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {
        Size::new(240.0, 90.0)
    }

    fn preferred_size(&self, state: &Self::State, ctx: &LayoutCtx<Self>) -> Size {
        let label = ctx.measure_text("WidgetKit", TextStyle::new().size(16.0));
        ctx.constrain(Size::new(
            state.width.max(label.width),
            state.height.max(label.height),
        ))
    }
}

#[test]
fn preferred_size_uses_layout_constraints() {
    let mut runner = AppRunner::new(
        "preferred-size",
        PreferredSizeWidget,
        SoftwareRenderer::new(),
    );
    runner.initialize(Size::new(320.0, 120.0)).unwrap();

    let constraints =
        Constraints::new(Some(Size::new(100.0, 100.0)), Some(Size::new(200.0, 180.0)));

    assert_eq!(
        runner.preferred_size(constraints),
        Some(Size::new(200.0, 100.0))
    );
}
