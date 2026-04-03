use crate::{app::AppRunner, widget::Widget};
use widgetkit_core::Result;
use widgetkit_render::Renderer;

pub trait HostRunner<W, R>
where
    W: Widget,
    R: Renderer,
{
    fn run(self, runner: AppRunner<W, R>) -> Result<()>;
}
