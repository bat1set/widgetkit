use chrono::Local;
use widgetkit::prelude::*;

fn main() -> widgetkit::Result<()> {
    WidgetApp::new()
        .widget("clock", ClockWidget)
        .host(WindowsHost::new().with_size(Size::new(360.0, 135.0)))
        .renderer(SoftwareRenderer::new())
        .run()
}

struct ClockWidget;

#[derive(Default)]
struct ClockState {
    time_text: String,
    status: String,
}

#[derive(Clone, Debug)]
enum Msg {
    Tick,
    StatusLoaded(String),
}

impl Widget for ClockWidget {
    type State = ClockState;
    type Message = Msg;

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {
        ClockState {
            time_text: current_time_string(),
            status: "starting runtime...".into(),
        }
    }

    fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
        ctx.scheduler().every(std::time::Duration::from_secs(1), Msg::Tick);
        ctx.tasks()
            .spawn_named("load-status", async { Msg::StatusLoaded("task backend ready".to_string()) });
        ctx.post(Msg::Tick);
    }

    fn update(&mut self, state: &mut Self::State, event: Event<Self::Message>, ctx: &mut UpdateCtx<Self>) {
        match event {
            Event::Message(Msg::Tick) => {
                state.time_text = current_time_string();
                ctx.request_render();
            }
            Event::Message(Msg::StatusLoaded(status)) => {
                state.status = status;
                ctx.request_render();
            }
            Event::Host(_) => {}
        }
    }

    fn render(&self, state: &Self::State, canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {
        canvas.clear(Color::rgb(14, 17, 22));
        canvas.round_rect(
            Rect::xywh(12.0, 12.0, 336.0, 112.0),
            18.0,
            Color::rgba(30, 36, 48, 255),
        );
        canvas.text(
            Point::new(28.0, 28.0),
            "WidgetKit v0.1",
            TextStyle::new().size(16.0),
            Color::rgb(127, 160, 255),
        );
        canvas.text(
            Point::new(28.0, 50.0),
            &state.time_text,
            TextStyle::new().size(24.0),
            Color::WHITE,
        );
        canvas.text(
            Point::new(28.0, 88.0),
            &state.status,
            TextStyle::new().size(10.0),
            Color::rgb(190, 198, 212),
        );
        canvas.line(
            Point::new(24.0, 45.0),
            Point::new(324.0, 45.0),
            Stroke::new(1.0),
            Color::rgba(255, 255, 255, 32),
        );
    }

    fn stop(&mut self, _state: &mut Self::State, ctx: &mut StopCtx<Self>) {
        ctx.tasks().cancel_all();
        ctx.scheduler().clear();
    }
}

fn current_time_string() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
