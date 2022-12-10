use std::time::Duration;

use fragments::{
    signal::{self, Signal},
    App, Scope,
};
use tokio::time::{interval, sleep};

fn main() {
    let app = App::new().run(|mut s: Scope| {
        let counter = signal::Mutable::new(0);

        let mapped = counter.signal_ref().map(|v| v.to_string());
        s.create_effect(mapped, |_: Scope<'_>, counter| {
            eprintln!("Counter: {:?}", counter);
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
