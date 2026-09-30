use std::time::{Duration, Instant};

const SIXTY_HERTZ: Duration = Duration::from_nanos(16_666_667);
const LOOKED_AT: u32 = 30;
const MISSES_ALLOWED: u32 = 5;
const TRUST: u32 = 60;
const TRUST_AT_MOST: u32 = 960;
const ANSWER_AT_MOST: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    Now,
    At(Instant),
    Answer,
}

#[derive(Debug, Clone)]
pub struct Cadence {
    period: Duration,
    halved: bool,
    drawn: Option<Instant>,
    moved: bool,
    timed: bool,
    answered: Option<Instant>,
    shown: bool,
    misses: u64,
    kept: u32,
    trust: u32,
}

impl Default for Cadence {
    fn default() -> Self {
        Self {
            period: SIXTY_HERTZ,
            halved: false,
            drawn: None,
            moved: false,
            timed: false,
            answered: None,
            shown: false,
            misses: 0,
            kept: 0,
            trust: TRUST,
        }
    }
}

impl Cadence {
    pub fn set_refresh(&mut self, millihertz: u32) {
        if millihertz >= 1000 {
            self.period = Duration::from_nanos(1_000_000_000_000 / u64::from(millihertz));
        }
    }

    pub fn period(&self) -> Duration {
        self.period
    }

    pub fn halved(&self) -> bool {
        self.halved
    }

    pub fn moved(&self) -> bool {
        self.moved
    }

    pub fn awaiting(&self) -> bool {
        self.moved && !self.shown
    }

    pub fn next(&self, now: Instant) -> Next {
        let Some(drawn) = self.drawn.filter(|_| self.moved) else {
            return Next::Now;
        };
        let Some(answered) = self.answered.filter(|_| self.shown) else {
            return if now.saturating_duration_since(drawn) >= ANSWER_AT_MOST {
                Next::Now
            } else {
                Next::Answer
            };
        };
        if !self.halved {
            return Next::Now;
        }
        let wanted = drawn + self.period * 3 / 2;
        let beat = if answered >= wanted {
            answered
        } else {
            let beats = (wanted - answered)
                .as_nanos()
                .div_ceil(self.period.as_nanos().max(1));
            answered + self.period * u32::try_from(beats).unwrap_or(u32::MAX)
        };
        if now >= beat {
            Next::Now
        } else {
            Next::At(beat)
        }
    }

    pub fn drew(&mut self, now: Instant, moving: bool) {
        self.timed = moving && self.moved && self.shown;
        self.drawn = Some(now);
        self.moved = moving;
        self.shown = false;
    }

    pub fn answered(&mut self, now: Instant) {
        let first = self.drawn.is_some() && !self.shown;
        self.answered = Some(now);
        self.shown = true;
        if !(first && self.timed) {
            return;
        }
        self.timed = false;
        let took = self
            .drawn
            .map_or(Duration::ZERO, |drawn| now.saturating_duration_since(drawn));
        self.judge(took > self.period * 3 / 2);
    }

    fn judge(&mut self, missed: bool) {
        if self.halved {
            self.kept = if missed { 0 } else { self.kept + 1 };
            if self.kept >= self.trust {
                self.halved = false;
                self.misses = 0;
                self.trust = (self.trust * 2).min(TRUST_AT_MOST);
            }
            return;
        }
        self.misses = (self.misses << 1 | u64::from(missed)) & ((1 << LOOKED_AT) - 1);
        if self.misses.count_ones() > MISSES_ALLOWED {
            self.halved = true;
            self.kept = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REFRESH: Duration = SIXTY_HERTZ;

    fn moving(cadence: &mut Cadence, from: Instant, frames: u32, took: Duration) -> Instant {
        let mut now = from;
        for _ in 0..frames {
            if let Next::At(beat) = cadence.next(now) {
                now = beat;
            }
            assert_eq!(cadence.next(now), Next::Now);
            cadence.drew(now, true);
            now += took;
            cadence.answered(now);
        }
        now
    }

    #[test]
    fn a_device_that_keeps_up_draws_at_every_refresh() {
        let mut cadence = Cadence::default();
        let now = moving(&mut cadence, Instant::now(), 120, REFRESH);
        assert!(!cadence.halved());
        assert_eq!(cadence.next(now), Next::Now);
    }

    #[test]
    fn a_frame_waits_for_the_last_one_to_be_shown() {
        let mut cadence = Cadence::default();
        let start = Instant::now();
        cadence.drew(start, true);
        assert!(cadence.awaiting());
        assert_eq!(cadence.next(start + REFRESH / 2), Next::Answer);
        assert_eq!(cadence.next(start + ANSWER_AT_MOST), Next::Now);
        cadence.answered(start + REFRESH);
        assert!(!cadence.awaiting());
        assert_eq!(cadence.next(start + REFRESH), Next::Now);
    }

    #[test]
    fn nothing_waits_on_a_still_frame() {
        let mut cadence = Cadence::default();
        let start = Instant::now();
        cadence.drew(start, false);
        assert!(!cadence.awaiting());
        assert_eq!(cadence.next(start + Duration::from_millis(1)), Next::Now);
    }

    #[test]
    fn a_few_late_frames_do_not_halve_it() {
        let mut cadence = Cadence::default();
        let mut now = moving(&mut cadence, Instant::now(), 10, REFRESH);
        now = moving(&mut cadence, now, MISSES_ALLOWED, REFRESH * 2);
        moving(&mut cadence, now, 40, REFRESH);
        assert!(!cadence.halved());
    }

    #[test]
    fn a_device_that_misses_goes_to_every_other_refresh_on_the_beat() {
        let mut cadence = Cadence::default();
        let mut now = Instant::now();
        for frame in 0..LOOKED_AT {
            let took = if frame % 2 == 0 { REFRESH } else { REFRESH * 2 };
            now = moving(&mut cadence, now, 1, took);
        }
        assert!(cadence.halved());

        let drawn = match cadence.next(now) {
            Next::At(beat) => beat,
            _ => now,
        };
        cadence.drew(drawn, true);
        let shown = drawn + REFRESH + Duration::from_micros(300);
        cadence.answered(shown);
        assert_eq!(cadence.next(shown), Next::At(shown + REFRESH));
        assert_eq!(cadence.next(shown + REFRESH), Next::Now);

        let drawn = shown + REFRESH;
        cadence.drew(drawn, true);
        let shown = drawn + REFRESH * 2;
        cadence.answered(shown);
        assert_eq!(cadence.next(shown), Next::Now);
    }

    #[test]
    fn a_halved_device_is_trusted_again_slower_each_time() {
        let mut cadence = Cadence::default();
        let now = moving(
            &mut cadence,
            Instant::now(),
            MISSES_ALLOWED + 2,
            REFRESH * 2,
        );
        assert!(cadence.halved());
        let now = moving(&mut cadence, now, TRUST - 1, REFRESH);
        assert!(cadence.halved());
        let now = moving(&mut cadence, now, 1, REFRESH);
        assert!(!cadence.halved());

        let now = moving(&mut cadence, now, MISSES_ALLOWED + 1, REFRESH * 2);
        assert!(cadence.halved());
        let now = moving(&mut cadence, now, TRUST, REFRESH);
        assert!(cadence.halved());
        moving(&mut cadence, now, TRUST, REFRESH);
        assert!(!cadence.halved());
    }

    #[test]
    fn only_frames_drawn_in_the_middle_of_a_movement_are_judged() {
        let mut cadence = Cadence::default();
        let mut now = Instant::now();
        for _ in 0..20 {
            cadence.drew(now, false);
            now += REFRESH * 2;
            cadence.answered(now);
            cadence.drew(now, true);
            now += REFRESH * 2;
            cadence.answered(now);
            now += Duration::from_millis(500);
        }
        assert!(!cadence.halved());
    }

    #[test]
    fn a_display_says_its_own_refresh() {
        let mut cadence = Cadence::default();
        cadence.set_refresh(90_000);
        assert_eq!(cadence.period(), Duration::from_nanos(11_111_111));
        cadence.set_refresh(0);
        assert_eq!(cadence.period(), Duration::from_nanos(11_111_111));
    }
}
