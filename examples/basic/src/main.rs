use std::{
    string::FromUtf8Error,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::Duration,
};

use fragments_core::{
    components::text,
    effect::{Effect, FutureEffect, StreamEffect},
    signal::{Mutable, Signal},
    Scope, Widget,
};
use fragments_wgpu::app::AppBuilder;
use futures::{
    future::BoxFuture,
    ready,
    stream::{unfold, BoxStream},
    task::{waker, ArcWake},
    Future, FutureExt, StreamExt,
};
use parking_lot::Mutex;
use tokio::time::{self, interval};
use tokio_stream::wrappers::IntervalStream;
use tracing::{info_span, Instrument};
use tracing_subscriber::{prelude::*, EnvFilter};
use tracing_tree::HierarchicalLayer;

#[derive(Debug)]
struct DebugWorld;

impl Widget for DebugWorld {
    #[tracing::instrument(level = "info", skip(scope))]
    fn mount(self, scope: &mut Scope) {
        scope.create_effect(StreamEffect::new(
            IntervalStream::new(interval(Duration::from_millis(50))),
            |s: &mut Scope, _| {
                let frame = s.frame();
                tracing::info!("World: {:#?}", frame.world);
            },
        ));
    }
}

struct Text(String);

impl Widget for Text {
    fn mount(self, scope: &mut fragments_core::Scope) {
        scope.set(text(), self.0);
    }
}

struct App {}

impl Widget for App {
    fn mount(self, scope: &mut fragments_core::Scope) {
        let count = Mutable::new(0);

        scope.attach(count.signal().map(|v| Text(v.to_string())));
        scope.attach(DebugWorld);

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
        .with(
            HierarchicalLayer::new(4)
                .with_thread_ids(false)
                .with_indent_lines(false)
                .with_verbose_entry(true)
                .with_verbose_exit(true),
        )
        .init();

    tokio::task::block_in_place(|| AppBuilder::new().build().run(App {}))
}

#[tracing::instrument(level = "info")]
fn test_manual_driver() {
    let mut interval = time::interval(Duration::from_millis(100));
    // let mut tick = Tick {
    //     interval: Box::pin(unfold(interval, |mut interval| async {
    //         let at = interval.tick().await;

    //         Some((at, interval))
    //     })),
    // };

    let mut tick = Box::pin(
        async {
            loop {
                let v = interval.tick().await;
                tracing::info!("Tick: {:?}", v);
            }
        }
        .instrument(info_span!("tick_future")),
    );

    let woken = Arc::new(Mutex::new(true));
    let woken2 = woken.clone();
    let waker = waker(Arc::new(FuncWaker {
        func: Box::new(move || *woken2.lock() = true),
    }));

    let cx = &mut Context::from_waker(&waker);

    loop {
        std::thread::sleep(Duration::from_millis(23));
        if !std::mem::take(&mut *woken.lock()) {
            continue;
        }

        if tick.poll_unpin(cx).is_ready() {
            break;
        };
    }
}

pub struct FuncWaker {
    func: Box<dyn Fn() + Send + Sync>,
}

impl ArcWake for FuncWaker {
    #[tracing::instrument(level = "info", skip(arc_self))]
    fn wake_by_ref(arc_self: &Arc<Self>) {
        tracing::info!("Called waker");
        (arc_self.func)();
    }
}
