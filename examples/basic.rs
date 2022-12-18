use std::time::Duration;

use eyre::Context;
use flax::name;
use fragments::{
    components::text,
    signal::{self, Signal},
    App, Scope,
};

use tokio::time::interval;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    App::new()
        .run(|mut s: Scope| {
            s.set(name(), "Root".into());
            let counter = signal::Mutable::new(0);

            let mapped = counter.signal_ref().map(|v| v.to_string());
            s.use_signal(mapped, |mut s, counter| {
                eprintln!("Counter: {:?}", counter);
                s.set(text(), counter);
            });

            s.use_signal(counter.signal(), |_, _| {
                eprintln!("counter changed");
            });

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(1000));
                loop {
                    interval.tick().await;
                    eprintln!("Writing");
                    *counter.write() += 1;
                }
            });
        })
        .wrap_err("Failed to run app")
}
