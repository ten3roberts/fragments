use std::time::Duration;

use fragments_core::{
    effect::{Effect, FutureEffect},
    Widget,
};
use fragments_wgpu::app::{App, AppBuilder};
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tracing_subscriber::{prelude::*, EnvFilter};
use tracing_tree::HierarchicalLayer;

#[derive(Debug)]
struct DebugWorld;

impl Widget for DebugWorld {
    #[tracing::instrument(level = "info", skip(scope))]
    fn render(self, scope: &mut fragments_core::Scope) {
        scope.use_stream(
            IntervalStream::new(time::interval(Duration::from_millis(1000))),
            |s, at| {
                tracing::info!("Interval {at:?}");
                let frame = s.frame();
                tracing::info!("World: {:#?}", frame.world);
            },
        );
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(HierarchicalLayer::new(4))
        .init();

    AppBuilder::new().build().run(DebugWorld)
}
