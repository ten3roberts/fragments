use std::time::Duration;

use fragments::{App, Scope};
use futures_signals::signal::Mutable;
use tokio::time::{interval, sleep};

fn main() {
    let app = App::builder().build().run(|mut s: Scope| {
        let counter = Mutable::new(0);

        s.create_effect(counter.signal(), |world, counter| {
            eprintln!("Counter: {counter}")
        });

        s.spawn_task(async move {
            let mut interval = interval(Duration::from_millis(1000));
            loop {
                interval.tick().await;
            }
        });
    });
}
