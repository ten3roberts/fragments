use fragments_core::{
    components::{color, text},
    effect::StreamEffect,
    layout::{min_width, position, size},
    signal::{Mutable, Signal},
    time::interval,
    Scope, Widget,
};
use fragments_wgpu::{app::AppBuilder, events::RedrawEvent, rectangle};
use futures::StreamExt;
use glam::{vec2, IVec2, Vec2};
use palette::{Hsla, IntoColor};
use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};
use tracing_subscriber::{prelude::*, EnvFilter};
use tracing_tree::HierarchicalLayer;

#[derive(Debug)]
struct DebugWorld;

impl Widget for DebugWorld {
    #[tracing::instrument(level = "info", skip(scope))]
    fn mount(self, scope: &mut Scope) {
        scope.set_default(size());
        scope.set_default(position());
        scope.create_effect(StreamEffect::new(
            interval(Duration::from_millis(2000)),
            |s: &mut Scope, _| {
                let frame = s.frame();
                tracing::info!("World: {:#?}", frame.world);
            },
        ));
    }
}

struct Rect {
    size: Vec2,
}

impl Widget for Rect {
    fn mount(self, scope: &mut Scope) {
        scope.set(size(), self.size);
        scope.set_default(rectangle());
        scope.set_default(position());
    }
}

struct GradientRect {
    size: Vec2,
    pos: Vec2,
}

impl Widget for GradientRect {
    fn mount(self, scope: &mut Scope) {
        scope.set(size(), self.size);
        scope.set_default(rectangle());
        scope.set(min_width(), self.size.x);
        scope.set_default(position());

        let now = Instant::now();
        scope.on_global_event(move |scope, &RedrawEvent| {
            scope.set(
                color(),
                Hsla::new(now.elapsed().as_secs_f32() * 10.0, 1.0, 0.5, 1.0).into_color(),
            )
        });
    }
}

struct Text(String);

impl Widget for Text {
    fn mount(self, scope: &mut fragments_core::Scope) {
        scope.set(text(), self.0);
    }
}

struct Animated<F>(F);

impl<F, W> Animated<F>
where
    F: 'static + FnMut(Duration) -> W,
    W: Widget,
{
    fn new(f: F) -> Self {
        Self(f)
    }
}

impl<F, W> Widget for Animated<F>
where
    F: 'static + FnMut(Duration) -> W,
    W: Widget,
{
    fn mount(self, scope: &mut Scope) {
        let mut f = self.0;

        let start = Instant::now();
        let mut child = scope.attach(f(Duration::from_secs(0)));

        scope.on_global_event(move |scope, &RedrawEvent| {
            scope.detach(child);

            child = scope.attach(f(start.elapsed()));
        });
    }
}

struct App {}

impl Widget for App {
    fn mount(self, scope: &mut fragments_core::Scope) {
        let count = Mutable::new(0);

        scope.set_default(size());
        scope.set_default(position());

        // scope.attach(count.signal().map(|v| Text(v.to_string())));

        // scope.attach(Animated::new(move |t| Rect {
        //     size: vec2(50.0, 50.0),
        //     // pos: vec2(
        //     //     400.0 + (t.as_secs_f32() * PI).sin() * 200.0,
        //     //     300.0 + (t.as_secs_f32() * PI).cos() * 200.0,
        //     // ),
        // }));

        // scope.attach(Animated::new(move |t| Rect {
        //     size: vec2(100.0, 100.0),
        //     // pos: vec2(
        //     //     400.0 + t.as_secs_f32().sin() * 400.0,
        //     //     200.0 + (t.as_secs_f32() * PI).cos() * 200.0,
        //     // ),
        // }));

        scope.create_effect(StreamEffect::new(
            interval(Duration::from_millis(200)).enumerate().take(10),
            move |s: &mut Scope, (i, _)| {
                let i = i as i32;

                let size = 20.0;
                let cols = (800.0 / size) as i32;
                let row = i / cols;

                // Snake downwards
                let col = if row % 2 == 0 {
                    i % cols
                } else {
                    cols - (i % cols) - 1
                };

                s.attach(GradientRect {
                    size: vec2(size, size),
                    pos: vec2(col as f32, row as f32) * (size + 2.0),
                });
            },
        ));

        let task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(200));
            loop {
                interval.tick().await;
                *count.write() += 1;
            }
        });

        scope.on_cleanup(move || task.abort())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(HierarchicalLayer::new(4).with_thread_ids(false))
        .init();

    AppBuilder::new().build().run(App {})
}
