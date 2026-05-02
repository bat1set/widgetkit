use crate::{
    context::{
        DisposeCtx, HitTestCtx, LayoutCtx, MountCtx, RenderCtx, StartCtx, StopCtx, UpdateCtx,
    },
    event::Event,
};
use widgetkit_core::{HitTest, Point, Size};
use widgetkit_render::Canvas;

pub trait Widget: Send + Sized + 'static {
    type State;
    type Message: Send + 'static;

    fn mount(&mut self, ctx: &mut MountCtx<Self>) -> Self::State;

    fn start(&mut self, _state: &mut Self::State, _ctx: &mut StartCtx<Self>) {}

    fn update(
        &mut self,
        _state: &mut Self::State,
        _event: Event<Self::Message>,
        _ctx: &mut UpdateCtx<Self>,
    ) {
    }

    fn preferred_size(&self, _state: &Self::State, ctx: &LayoutCtx<Self>) -> Size {
        ctx.constrain(ctx.available_size())
    }

    fn hit_test(&self, _state: &Self::State, _position: Point, _ctx: &HitTestCtx<Self>) -> HitTest {
        HitTest::Hit
    }

    fn render(&self, _state: &Self::State, _canvas: &mut Canvas, _ctx: &RenderCtx<Self>) {}

    fn stop(&mut self, _state: &mut Self::State, _ctx: &mut StopCtx<Self>) {}

    fn dispose(&mut self, _state: Self::State, _ctx: &mut DisposeCtx<Self>) {}
}
