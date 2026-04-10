#![windows_subsystem = "windows"]

use widgetkit::prelude::*;

fn main() -> widgetkit::Result<()> {
    WidgetApp::new()
        .widget("pulse", PulseWidget)
        .host(WindowsHost::new().with_size(Size::new(320.0, 140.0)))
        .renderer(SoftwareRenderer::new())
        .run()
}

struct PulseWidget;

#[derive(Default)]
struct PulseState {
    phase: f32,
    status: String,
}

#[derive(Clone, Debug)]
enum Msg {
    Tick,
    Loaded,
}

impl Widget for PulseWidget {
    type State = PulseState;
    type Message = Msg;

    fn mount(&mut self, _ctx: &mut MountCtx<Self>) -> Self::State {
        PulseState {
            phase: 0.0,
            status: "booting module graph...".into(),
        }
    }

    fn start(&mut self, _state: &mut Self::State, ctx: &mut StartCtx<Self>) {
        ctx.scheduler()
            .every(std::time::Duration::from_millis(120), Msg::Tick);
        ctx.tasks()
            .spawn_named("load-status", async { Msg::Loaded });
    }

    fn update(
        &mut self,
        state: &mut Self::State,
        event: Event<Self::Message>,
        ctx: &mut UpdateCtx<Self>,
    ) {
        match event {
            Event::Message(Msg::Tick) => {
                state.phase += 0.1;
                if state.phase > 1.0 {
                    state.phase = 0.0;
                }
                ctx.request_render();
            }
            Event::Message(Msg::Loaded) => {
                state.status = "runtime stable".into();
                ctx.request_render();
            }
            Event::Host(_) => {}
        }
    }

    fn render(&self, state: &Self::State, canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {
        let bg = Color::rgb(12, 14, 18);
        let panel = Color::rgb(24, 28, 36);
        let accent = Color::rgb(90, 180, 255);
        let muted = Color::rgb(180, 188, 204);

        canvas.clear(bg);
        canvas.save();
        canvas.clip_rect(Rect::xywh(12.0, 12.0, 296.0, 116.0));
        canvas.round_rect(Rect::xywh(12.0, 12.0, 296.0, 116.0), 16.0, panel);

        canvas.text(
            Point::new(24.0, 28.0),
            "Activity",
            TextStyle::new().size(16.0),
            accent,
        );

        canvas.line(
            Point::new(24.0, 45.0),
            Point::new(284.0, 45.0),
            Stroke::new(1.0),
            Color::rgba(255, 255, 255, 24),
        );

        let base_x = 28.0;
        let y = 74.0;
        let step = 28.0;

        for i in 0..5 {
            let t = ((state.phase * 10.0) + i as f32) % 5.0;
            let radius = 6.0 + t;
            let alpha = 60 + (i as u8 * 30);

            canvas.circle(
                Point::new(base_x + i as f32 * step, y),
                radius,
                Color::rgba(90, 180, 255, alpha),
            );
        }

        canvas.text(
            Point::new(24.0, 108.0),
            &state.status,
            TextStyle::new().size(12.0).baseline(TextBaseline::Bottom),
            muted,
        );

        canvas.restore();
    }

    fn stop(&mut self, _state: &mut Self::State, ctx: &mut StopCtx<Self>) {
        ctx.tasks().cancel_all();
        ctx.scheduler().clear();
    }
}
