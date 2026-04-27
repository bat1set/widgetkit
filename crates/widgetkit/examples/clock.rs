#![windows_subsystem = "windows"]

use chrono::Local;
use widgetkit::prelude::*;

fn main() -> widgetkit::Result<()> {
    WidgetApp::new()
        .widget("clock", ClockWidget)
        .host(
            WindowsHost::new()
                .size_policy(SizePolicy::ContentWithLimits {
                    min: Some(Size::new(280.0, 112.0)),
                    max: Some(Size::new(520.0, 180.0)),
                })
                .frameless(true)
                .transparent(true)
                .always_on_top(true),
        )
        .renderer(SoftwareRenderer::new())
        .run()
}

struct ClockWidget;

#[derive(Default)]
struct ClockState {
    time_text: String,
    status: String,
    accent: Color,
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
            accent: Color::rgb(127, 160, 255),
        }
    }

    fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
        ctx.scheduler()
            .every(Duration::from_secs(1), Msg::Tick);
        ctx.tasks().spawn_named("load-status", async {
            Msg::StatusLoaded("software renderer + render frame ready".to_string())
        });
        ctx.post(Msg::Tick);
    }

    fn update(
        &mut self,
        state: &mut Self::State,
        event: Event<Self::Message>,
        ctx: &mut UpdateCtx<Self>,
    ) {
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

    fn preferred_size(&self, state: &Self::State, ctx: &LayoutCtx<Self>) -> Size {
        let title = ctx.measure_text("WidgetKit", TextStyle::new().size(16.0));
        let time = ctx.measure_text(&state.time_text, TextStyle::new().size(24.0));
        let status = ctx.measure_text(
            &state.status,
            TextStyle::new()
                .size(10.0)
                .line_height(12.0)
                .baseline(TextBaseline::Bottom),
        );

        let width = title.width.max(time.width).max(status.width) + 64.0;
        let height = title.height + time.height + status.height + 64.0;

        ctx.constrain(Size::new(width, height))
    }

    fn render(&self, state: &Self::State, canvas: &mut Canvas, ctx: &RenderCtx<Self>) {
        let bg = Color::rgb(14, 17, 22);
        let card = Color::rgb(30, 36, 48);
        let muted = Color::rgb(190, 198, 212);
        let divider = Color::rgba(255, 255, 255, 32);
        let size = ctx.surface_size();
        let card_rect = Rect::xywh(12.0, 12.0, size.width - 24.0, size.height - 24.0);
        let divider_y = 45.0;

        canvas.clear(bg);

        canvas.save();
        canvas.clip_rect(card_rect);
        canvas.round_rect(card_rect, 18.0, card);

        canvas.text(
            Point::new(28.0, 28.0),
            "WidgetKit",
            TextStyle::new().size(16.0),
            state.accent,
        );

        canvas.line(
            Point::new(24.0, divider_y),
            Point::new(size.width - 36.0, divider_y),
            Stroke::new(1.0),
            divider,
        );

        canvas.text(
            Point::new(28.0, 50.0),
            &state.time_text,
            TextStyle::new().size(24.0),
            Color::WHITE,
        );

        canvas.circle(Point::new(size.width - 48.0, 30.0), 6.0, state.accent);

        canvas.text(
            Point::new(28.0, size.height - 25.0),
            &state.status,
            TextStyle::new()
                .size(10.0)
                .line_height(12.0)
                .baseline(TextBaseline::Bottom),
            muted,
        );

        canvas.restore();
    }

    fn stop(&mut self, _state: &mut Self::State, ctx: &mut StopCtx<Self>) {
        ctx.tasks().cancel_all();
        ctx.scheduler().clear();
    }
}

fn current_time_string() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
