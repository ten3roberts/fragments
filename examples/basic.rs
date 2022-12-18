use std::time::{Duration, Instant};
use tracing_subscriber::{prelude::*, util::SubscriberInitExt};
use tracing_tree::HierarchicalLayer;

use eyre::Context;
use flax::name;
use fragments::{
    components::text,
    effect::{from_stream, Effect},
    App, Scope, Widget,
};

use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

struct CustomWidget {
    text: String,
}

impl Widget for CustomWidget {
    fn render(self, mut scope: Scope) {
        scope.set(text(), self.text);
    }
}

struct Clock {}

impl Widget for Clock {
    fn render(self, mut scope: Scope) {
        let now = Instant::now();

        scope.use_stream(
            IntervalStream::new(tokio::time::interval(Duration::from_secs(1))),
            move |scope, _| {
                scope.set(text(), format!("Elapsed: {:.2?}", now.elapsed()));
            },
        );
    }
}

fn main() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(HierarchicalLayer::new(4))
        .init();

    color_eyre::install()?;

    App::new()
        .run(|mut s: Scope| {
            s.set(name(), "Root".into());

            s.attach_child(Clock {});
            s.attach_child(CustomWidget {
                text: "Hello, World!".into(),
            });

            s.use_stream(
                IntervalStream::new(interval(Duration::from_millis(500))),
                |scope, _| {
                    let world = scope.entity().world();

                    tracing::info!("World: {world:#?}");
                },
            );
        })
        .wrap_err("Failed to run app")
}
