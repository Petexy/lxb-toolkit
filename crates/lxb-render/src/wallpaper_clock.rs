use std::time::Instant;

use lxb_toolkit::handoff::{self, Handoff, Rejection};

pub struct WallpaperClock {
    started: Instant,
    scene_ns_at_start: u64,
}

impl WallpaperClock {
    pub fn from_environment(configured_accent: &str) -> Option<Self> {
        let raw = std::env::var_os(handoff::ENV)?;
        std::env::remove_var(handoff::ENV);

        let accepted = raw
            .to_str()
            .ok_or(Rejection::NonUtf8)
            .and_then(|record| Self::from_record(record, configured_accent));
        match accepted {
            Ok(clock) => Some(clock),
            Err(reason) => {
                eprintln!("ignoring the wallpaper handoff: {reason}");
                None
            }
        }
    }

    pub fn from_record(record: &str, configured_accent: &str) -> Result<Self, Rejection> {
        let handoff = Handoff::parse(record)?;
        let boot_id = handoff::boot_id()?;
        let now_ns = monotonic_now_ns()?;
        let scene_ns = handoff.scene_at(&boot_id, now_ns)?;
        if handoff.accent != configured_accent {
            eprintln!(
                "the wallpaper handoff was drawn in {} and this application is set to {configured_accent}: keeping the setting",
                handoff.accent
            );
        }
        Ok(Self {
            started: Instant::now(),
            scene_ns_at_start: scene_ns,
        })
    }

    pub fn local() -> Self {
        Self {
            started: Instant::now(),
            scene_ns_at_start: 0,
        }
    }

    pub fn restart(&mut self) {
        if self.scene_ns_at_start == 0 {
            self.started = Instant::now();
        }
    }

    pub fn elapsed_ns(&self) -> u64 {
        let elapsed_ns = u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.scene_ns_at_start.saturating_add(elapsed_ns)
    }

    pub fn elapsed_secs(&self) -> f32 {
        handoff::scene_seconds(self.elapsed_ns())
    }

    pub fn capture(&self, accent: &str, theme: Option<&str>) -> Option<Handoff> {
        Handoff::of(
            handoff::boot_id().ok()?,
            monotonic_now_ns().ok()?,
            self.elapsed_ns(),
            accent,
            theme,
        )
    }
}

pub fn monotonic_now_ns() -> Result<u64, Rejection> {
    let mut timestamp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut timestamp) } != 0
        || timestamp.tv_sec < 0
        || !(0..handoff::NANOS_PER_SECOND as _).contains(&timestamp.tv_nsec)
    {
        return Err(Rejection::MonotonicClockUnavailable);
    }

    let seconds =
        u64::try_from(timestamp.tv_sec).map_err(|_| Rejection::MonotonicClockUnavailable)?;
    let nanos =
        u64::try_from(timestamp.tv_nsec).map_err(|_| Rejection::MonotonicClockUnavailable)?;
    seconds
        .checked_mul(handoff::NANOS_PER_SECOND)
        .and_then(|value| value.checked_add(nanos))
        .ok_or(Rejection::MonotonicClockUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn record(scene_ns: u64) -> String {
        WallpaperClock {
            started: Instant::now(),
            scene_ns_at_start: scene_ns,
        }
        .capture("Purple", None)
        .expect("a machine with a monotonic clock and a boot id")
        .encode()
    }

    #[test]
    fn a_window_with_nothing_to_continue_starts_the_wallpaper_at_the_beginning() {
        let clock = WallpaperClock::local();
        assert!(clock.elapsed_secs() < 1.0);
        assert!(WallpaperClock::from_record("nonsense", "Purple").is_err());
    }

    #[test]
    fn a_window_carries_on_from_the_wallpaper_already_on_screen() {
        let clock = WallpaperClock::from_record(&record(42_000_000_000), "Purple")
            .expect("a record this machine wrote a moment ago");
        let carried = clock.elapsed_secs();
        assert!(carried >= 42.0, "{carried}");
        assert!(carried < 43.0, "{carried}");
    }

    #[test]
    fn a_local_clock_starts_again_with_the_window_and_a_continued_one_never_does() {
        let mut local = WallpaperClock::local();
        let mut carried = WallpaperClock::from_record(&record(42_000_000_000), "Purple")
            .expect("a record this machine wrote a moment ago");
        std::thread::sleep(Duration::from_millis(20));

        local.restart();
        carried.restart();
        assert!(local.elapsed_secs() < 0.02);
        assert!(
            carried.elapsed_secs() >= 42.02,
            "{}",
            carried.elapsed_secs()
        );
    }

    #[test]
    fn what_it_hands_on_is_what_the_next_window_reads() {
        let carried = WallpaperClock::from_record(&record(9_000_000_000), "Purple")
            .expect("a record this machine wrote a moment ago");
        let onwards = carried
            .capture("Purple", Some("Simple"))
            .expect("a machine with a monotonic clock and a boot id");
        assert_eq!(onwards.theme.as_deref(), Some("Simple"));

        let next = WallpaperClock::from_record(&onwards.encode(), "Purple")
            .expect("the record it just wrote");
        assert!(next.elapsed_secs() >= carried.elapsed_secs() - 0.001);
        assert!(next.elapsed_secs() < 10.0);
    }

    #[test]
    fn the_monotonic_clock_runs_forwards() {
        let first = monotonic_now_ns().expect("a monotonic clock");
        std::thread::sleep(Duration::from_millis(5));
        let second = monotonic_now_ns().expect("a monotonic clock");
        assert!(second > first);
        assert!(second - first >= 5_000_000);
    }
}
