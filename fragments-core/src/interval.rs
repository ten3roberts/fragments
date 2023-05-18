use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{ready, Future, Stream};
use pin_project::pin_project;

use crate::time::{Sleep, TimersHandle};

/// Ticks at a fixed interval.
#[pin_project]
#[derive(Debug)]
pub struct Interval {
    #[pin]
    sleep: Sleep,
    period: Duration,
}

impl Interval {
    /// Creates a new interval which will fire at `start` and then every `period` duration.
    pub fn new(handle: &TimersHandle, start: Instant, period: Duration) -> Self {
        Self {
            sleep: Sleep::new(handle, start),
            period,
        }
    }

    pub fn poll_tick(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Instant> {
        let mut p = self.project();

        let deadline = p.sleep.deadline();

        // Wait until the next tick
        ready!(p.sleep.as_mut().poll(cx));

        // Calculate the next deadline
        let new_deadline = deadline + *p.period;

        // Reset the timer
        // Note: will not be registered until the interval is polled again
        p.sleep.reset(new_deadline);

        Poll::Ready(deadline)
    }
}

impl Stream for Interval {
    type Item = Instant;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_tick(cx).map(Some)
    }
}

#[cfg(test)]
mod test {
    use std::thread;

    use futures::StreamExt;
    use itertools::Itertools;

    use crate::time::{assert_dur, Timers};

    use super::*;

    fn assert_interval(
        start: Instant,
        stream: impl Stream<Item = Instant> + Unpin,
        expected: impl IntoIterator<Item = (Duration, Duration)>,
    ) {
        let mut expected_deadline = start;
        let mut last = start;

        for (i, (deadline, (expected_fixed, expected_wall))) in
            futures::executor::block_on_stream(stream)
                .zip(expected)
                .enumerate()
        {
            let elapsed = last.elapsed();
            last = Instant::now();

            eprintln!("[{i}] Took: {elapsed:?}");

            expected_deadline += expected_fixed;

            // What the deadline should have been
            // Compare the returned deadline to the expected one
            assert_dur(
                deadline.duration_since(start),
                expected_deadline.duration_since(start),
                "next returned deadline",
            );

            assert_dur(elapsed, expected_wall, "elapsed wall time");
        }
    }

    #[test]
    fn interval() {
        let timers = Timers::new();
        let handle = timers.handle();
        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();

        let expected = [
            // First tick is immediate
            (Duration::ZERO, Duration::ZERO),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
        ];

        let interval = Interval::new(&handle, now, Duration::from_millis(100));

        assert_interval(now, interval, expected);
    }

    #[test]
    fn interval_burst() {
        let timers = Timers::new();
        let handle = timers.handle();
        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();

        let delays = futures::stream::iter([
            Duration::ZERO,
            Duration::ZERO,
            Duration::from_millis(150),
            // Duration::from_millis(50),
            Duration::ZERO,
            Duration::from_millis(50),
            Duration::ZERO,
            Duration::from_millis(350),
            Duration::ZERO,
            Duration::ZERO,
            Duration::ZERO,
            Duration::ZERO,
            Duration::ZERO,
            Duration::ZERO,
            Duration::ZERO,
        ])
        .then(|d| Sleep::new(&handle, Instant::now() + d));

        let expected = [
            (Duration::ZERO, Duration::ZERO),
            // Normal tick
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(150)),
            // 50 ms behind
            (Duration::from_millis(100), Duration::from_millis(50)),
            // In phase
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(350)),
            // 250 ms behind
            (Duration::from_millis(100), Duration::ZERO),
            // 150 ms behind
            (Duration::from_millis(100), Duration::ZERO),
            // 50 ms behind
            (Duration::from_millis(100), Duration::from_millis(50)),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
            (Duration::from_millis(100), Duration::from_millis(100)),
        ];

        let interval = Interval::new(&handle, now, Duration::from_millis(100))
            .zip(delays)
            .map(|v| v.0);

        assert_interval(now, interval, expected);
    }
}
