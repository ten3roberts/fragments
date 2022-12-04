use std::time::Duration;

use fragments::{signal, App, Scope};
use tokio::time::{interval, sleep};

fn main() {
    let app = App::new().run(|mut s: Scope| {
        let counter = signal::Mutable::new(0);

        s.create_effect(counter.signal(), |_, counter| {
            eprintln!("Counter: {counter}")
        });

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(1000));
            loop {
                interval.tick().await;
                eprintln!("Writing");
                *counter.write() += 1;
            }
        });
    });
}
