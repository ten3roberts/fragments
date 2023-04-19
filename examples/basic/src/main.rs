use std::time::Duration;

use fragments_core::{
    components::text,
    effect::{Effect, FutureEffect},
    signal::{Mutable, Signal},
    Widget,
};
use fragments_wgpu::app::AppBuilder;
use tokio::time::{self, interval};
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

struct Text(String);

impl Widget for Text {
    fn render(self, scope: &mut fragments_core::Scope) {
        scope.set(text(), self.0);
    }
}

struct App {}

impl Widget for App {
    fn render(self, scope: &mut fragments_core::Scope) {
        let count = Mutable::new(0);

        scope.attach(count.signal().map(|v| Text(v.to_string())));
        scope.attach(DebugWorld);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(200));
            loop {
                interval.tick().await;
                tracing::info!("Updating count");
                *count.write() += 1;
            }
        });
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(HierarchicalLayer::new(4))
        .init();

    AppBuilder::new().build().run(App {})
}
