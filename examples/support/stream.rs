//! Deterministic synthetic input and a deadline clock, both owned by the hosts.

use std::time::{Duration, Instant};

pub const TICK_INTERVAL: Duration = Duration::from_millis(250);
pub const HISTORY_SAMPLES: usize = 256;
const TX: &[f64] = &[
    8.0, 14.0, 28.0, 52.0, 78.0, 96.0, 88.0, 68.0, 42.0, 20.0, 10.0, 4.0,
];
const RX: &[f64] = &[
    65.0, 78.0, 90.0, 98.0, 86.0, 64.0, 36.0, 15.0, 5.0, 9.0, 24.0, 46.0, 72.0, 82.0, 74.0, 56.0,
    34.0, 18.0, 9.0, 30.0,
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DemoStream {
    tick: usize,
    paused: bool,
}

impl DemoStream {
    pub fn tick(&self) -> usize {
        self.tick
    }
    pub fn paused(&self) -> bool {
        self.paused
    }
    pub fn toggle(&mut self) {
        self.paused = !self.paused;
    }
    pub fn reset(&mut self) {
        self.tick = 0;
    }
    pub fn step(&mut self) {
        self.paused = true;
        self.tick = self.tick.wrapping_add(1);
    }
    pub fn advance(&mut self, steps: usize) -> bool {
        if self.paused || steps == 0 {
            return false;
        }
        self.tick = self.tick.wrapping_add(steps);
        true
    }
    pub fn rates(&self) -> (f64, f64) {
        (TX[self.tick % TX.len()], RX[self.tick % RX.len()])
    }
    pub fn histories(&self) -> [Vec<f64>; 2] {
        [TX, RX].map(|samples| {
            let latest = self.tick % samples.len();
            (0..HISTORY_SAMPLES)
                .map(|row| {
                    let before = (HISTORY_SAMPLES - row - 1) % samples.len();
                    samples[(latest + samples.len() - before) % samples.len()]
                })
                .collect()
        })
    }
    pub fn core_histories(&self) -> Vec<Vec<f64>> {
        (0..12)
            .map(|core| {
                let period = 18 + core * 3;
                let burst = 4 + core % 5;
                (0..HISTORY_SAMPLES)
                    .map(|row| {
                        let before = (HISTORY_SAMPLES - row - 1) % period;
                        let phase = (self.tick % period + core * 7 + period - before) % period;
                        if phase < burst {
                            80.0 + f64::from(u32::try_from((phase + core) % 4).unwrap()) * 6.0
                        } else if phase < burst + 6 {
                            f64::from(u32::try_from(burst + 6 - phase).unwrap()) * 12.0 + 10.0
                        } else {
                            5.0 + f64::from(u32::try_from((phase * 7 + core * 3) % 22).unwrap())
                        }
                    })
                    .collect()
            })
            .collect()
    }
    pub fn caption(&self, animated: bool) -> String {
        let (tx, rx) = self.rates();
        format!(
            "SYNTHETIC {} / tick {} / TX {tx:.0} RX {rx:.0} MiB/s",
            if !animated {
                "STILL"
            } else if self.paused {
                "PAUSED"
            } else {
                "LIVE"
            },
            self.tick
        )
    }
}

/// Key events do not advance this clock, and a delayed event catches up exactly.
pub struct TickClock {
    next: Instant,
}

impl TickClock {
    pub fn new(now: Instant) -> Self {
        Self {
            next: now + TICK_INTERVAL,
        }
    }
    pub fn timeout(&self, now: Instant) -> Duration {
        self.next.saturating_duration_since(now)
    }
    pub fn take_due(&mut self, now: Instant) -> usize {
        if now < self.next {
            return 0;
        }
        let elapsed = now.duration_since(self.next);
        let remainder = Duration::from_nanos(
            u64::try_from(elapsed.as_nanos() % TICK_INTERVAL.as_nanos())
                .expect("remainder is shorter than one tick"),
        );
        self.next = now + (TICK_INTERVAL - remainder);
        usize::try_from(elapsed.as_nanos() / TICK_INTERVAL.as_nanos() + 1).unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histories_shift_and_rates_follow_independent_reproducible_waves() {
        let mut stream = DemoStream::default();
        let original = stream.histories();
        assert_eq!(stream.rates(), (8.0, 65.0));
        assert!(stream.advance(1));
        let next = stream.histories();
        for side in 0..2 {
            assert_eq!(next[side][..HISTORY_SAMPLES - 1], original[side][1..]);
        }
        assert_eq!(
            (next[0][HISTORY_SAMPLES - 1], next[1][HISTORY_SAMPLES - 1]),
            stream.rates()
        );
        assert_eq!(stream.rates(), (14.0, 78.0));
        assert!(stream.advance(5));
        assert_eq!(stream.rates(), (88.0, 36.0));
        stream.toggle();
        let paused = stream.clone();
        assert!(!stream.advance(20));
        assert_eq!(stream, paused);
        stream.step();
        assert_eq!(stream.tick(), 7);
        assert!(stream.paused());
        stream.reset();
        assert_eq!(stream.histories(), original);
        assert!(stream.caption(true).contains("PAUSED"));
        assert!(stream.caption(false).contains("STILL"));
    }

    #[test]
    fn deadline_is_independent_of_key_frequency_and_preserves_catchup_phase() {
        let start = Instant::now();
        let mut clock = TickClock::new(start);
        for milliseconds in 0..250 {
            assert_eq!(
                clock.take_due(start + Duration::from_millis(milliseconds)),
                0
            );
        }
        assert_eq!(clock.take_due(start + TICK_INTERVAL), 1);
        assert_eq!(clock.take_due(start + TICK_INTERVAL), 0);
        assert_eq!(clock.take_due(start + Duration::from_millis(1125)), 3);
        assert_eq!(
            clock.timeout(start + Duration::from_millis(1125)),
            Duration::from_millis(125)
        );
        assert_eq!(clock.take_due(start + Duration::from_millis(1250)), 1);
    }
}
