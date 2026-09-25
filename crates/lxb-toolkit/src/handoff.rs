use std::fmt;

use crate::{palette::PALETTES, settings::WallpaperStyle, wallpaper::VISUAL};

pub const ENV: &str = "LXB_BACKGROUND_HANDOFF";

pub const VERSION: &str = "1";

pub const CLOCK: &str = "linux-monotonic";

pub const MAX_RECORD_BYTES: usize = 1024;

pub const MAX_AGE_NS: u64 = 30_000_000_000;

pub const NANOS_PER_SECOND: u64 = 1_000_000_000;

pub const BOOT_ID_PATH: &str = "/proc/sys/kernel/random/boot_id";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    pub boot_id: String,
    pub sample_ns: u64,
    pub scene_ns: u64,
    pub accent: String,
    pub theme: Option<String>,
    pub particles: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    TooLong,
    NonAscii,
    NonUtf8,
    MalformedField,
    UnknownField,
    DuplicateField,
    MissingField,
    UnsupportedVersion,
    UnsupportedVisual,
    UnsupportedClock,
    InvalidBootId,
    InvalidSample,
    InvalidScene,
    InvalidAccent,
    BootChanged,
    FutureSample,
    Expired,
    Overflow,
    BootIdUnavailable,
    MonotonicClockUnavailable,
}

impl fmt::Display for Rejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TooLong => "record is too long",
            Self::NonAscii => "record is not ASCII",
            Self::NonUtf8 => "record is not UTF-8",
            Self::MalformedField => "record has a malformed field",
            Self::UnknownField => "record has an unknown field",
            Self::DuplicateField => "record repeats a field",
            Self::MissingField => "record is missing a field",
            Self::UnsupportedVersion => "record version is unsupported",
            Self::UnsupportedVisual => "wallpaper visual version is unsupported",
            Self::UnsupportedClock => "record clock is unsupported",
            Self::InvalidBootId => "boot ID is malformed",
            Self::InvalidSample => "monotonic sample is malformed",
            Self::InvalidScene => "scene time is malformed",
            Self::InvalidAccent => "accent name is not canonical",
            Self::BootChanged => "record came from another boot",
            Self::FutureSample => "monotonic sample is in the future",
            Self::Expired => "record is stale",
            Self::Overflow => "scene time overflowed",
            Self::BootIdUnavailable => "current boot ID is unavailable",
            Self::MonotonicClockUnavailable => "monotonic clock is unavailable",
        })
    }
}

impl std::error::Error for Rejection {}

impl Handoff {
    pub fn of(
        boot_id: String,
        sample_ns: u64,
        scene_ns: u64,
        accent: &str,
        theme: Option<&str>,
        particles: Option<bool>,
    ) -> Option<Self> {
        if !valid_boot_id(&boot_id) || !canonical_accent(accent) {
            return None;
        }
        Some(Self {
            boot_id,
            sample_ns,
            scene_ns,
            accent: accent.to_string(),
            theme: theme.and_then(canonical_theme).map(str::to_string),
            particles,
        })
    }

    pub fn parse(record: &str) -> Result<Self, Rejection> {
        if record.len() > MAX_RECORD_BYTES {
            return Err(Rejection::TooLong);
        }
        if !record.is_ascii() {
            return Err(Rejection::NonAscii);
        }

        let mut version = None;
        let mut visual = None;
        let mut clock = None;
        let mut boot_id = None;
        let mut sample_ns = None;
        let mut scene_ns = None;
        let mut accent = None;
        let mut theme = None;
        let mut particles = None;

        for field in record.split(';') {
            let (key, value) = field.split_once('=').ok_or(Rejection::MalformedField)?;
            if key.is_empty() || value.is_empty() {
                return Err(Rejection::MalformedField);
            }

            let slot = match key {
                "v" => &mut version,
                "visual" => &mut visual,
                "clock" => &mut clock,
                "boot" => &mut boot_id,
                "sample-ns" => &mut sample_ns,
                "scene-ns" => &mut scene_ns,
                "accent" => &mut accent,
                "theme" => &mut theme,
                "particles" => &mut particles,
                _ => return Err(Rejection::UnknownField),
            };
            if slot.replace(value).is_some() {
                return Err(Rejection::DuplicateField);
            }
        }

        let version = version.ok_or(Rejection::MissingField)?;
        let visual = visual.ok_or(Rejection::MissingField)?;
        let clock = clock.ok_or(Rejection::MissingField)?;
        let boot_id = boot_id.ok_or(Rejection::MissingField)?;
        let sample_ns = sample_ns.ok_or(Rejection::MissingField)?;
        let scene_ns = scene_ns.ok_or(Rejection::MissingField)?;
        let accent = accent.ok_or(Rejection::MissingField)?;

        if version != VERSION {
            return Err(Rejection::UnsupportedVersion);
        }
        if visual != VISUAL {
            return Err(Rejection::UnsupportedVisual);
        }
        if clock != CLOCK {
            return Err(Rejection::UnsupportedClock);
        }
        if !valid_boot_id(boot_id) {
            return Err(Rejection::InvalidBootId);
        }
        if !canonical_accent(accent) {
            return Err(Rejection::InvalidAccent);
        }

        Ok(Self {
            boot_id: boot_id.to_string(),
            sample_ns: parse_decimal(sample_ns).ok_or(Rejection::InvalidSample)?,
            scene_ns: parse_decimal(scene_ns).ok_or(Rejection::InvalidScene)?,
            accent: accent.to_string(),
            theme: theme.and_then(canonical_theme).map(str::to_string),
            particles: particles.and_then(canonical_particles),
        })
    }

    pub fn encode(&self) -> String {
        let mut record = format!(
            "v={VERSION};visual={VISUAL};clock={CLOCK};boot={};sample-ns={};scene-ns={};accent={}",
            self.boot_id, self.sample_ns, self.scene_ns, self.accent
        );
        if let Some(theme) = &self.theme {
            record.push_str(";theme=");
            record.push_str(theme);
        }
        if let Some(particles) = self.particles {
            record.push_str(";particles=");
            record.push_str(if particles { "on" } else { "off" });
        }
        record
    }

    pub fn environment(&self) -> String {
        format!("{ENV}={}", self.encode())
    }

    pub fn scene_at(&self, boot_id: &str, now_ns: u64) -> Result<u64, Rejection> {
        if self.boot_id != boot_id {
            return Err(Rejection::BootChanged);
        }
        let age_ns = now_ns
            .checked_sub(self.sample_ns)
            .ok_or(Rejection::FutureSample)?;
        if age_ns > MAX_AGE_NS {
            return Err(Rejection::Expired);
        }
        self.scene_ns.checked_add(age_ns).ok_or(Rejection::Overflow)
    }
}

pub fn canonical_accent(name: &str) -> bool {
    PALETTES.iter().any(|palette| palette.name == name)
}

pub fn canonical_theme(value: &str) -> Option<&'static str> {
    match WallpaperStyle::configured(value)? {
        WallpaperStyle::Default | WallpaperStyle::Custom => Some(WallpaperStyle::Default.name()),
        WallpaperStyle::Simple => Some(WallpaperStyle::Simple.name()),
    }
}

pub fn canonical_particles(value: &str) -> Option<bool> {
    match value {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
}

pub fn valid_boot_id(id: &str) -> bool {
    if id.len() != 36 {
        return false;
    }
    id.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            byte == b'-'
        } else {
            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        }
    })
}

pub fn boot_id() -> Result<String, Rejection> {
    let id = std::fs::read_to_string(BOOT_ID_PATH).map_err(|_| Rejection::BootIdUnavailable)?;
    let id = id.trim();
    valid_boot_id(id)
        .then(|| id.to_string())
        .ok_or(Rejection::BootIdUnavailable)
}

pub fn scene_seconds(scene_ns: u64) -> f32 {
    (scene_ns as f64 / NANOS_PER_SECOND as f64) as f32
}

fn parse_decimal(value: &str) -> Option<u64> {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| value.parse().ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOT_ID: &str = "01234567-89ab-cdef-0123-456789abcdef";

    fn valid_record() -> String {
        format!(
            "v=1;visual={VISUAL};clock={CLOCK};boot={BOOT_ID};sample-ns=10000000000;scene-ns=42000000000;accent=Blue"
        )
    }

    #[test]
    fn canonical_fixture_matches_the_display_manager_encoder() {
        assert_eq!(
            valid_record(),
            "v=1;visual=lxb-wallpaper-v6;clock=linux-monotonic;boot=01234567-89ab-cdef-0123-456789abcdef;sample-ns=10000000000;scene-ns=42000000000;accent=Blue"
        );
    }

    #[test]
    fn parses_the_complete_versioned_record() {
        let handoff = Handoff::parse(&valid_record()).expect("valid handoff");
        assert_eq!(handoff.boot_id, BOOT_ID);
        assert_eq!(handoff.sample_ns, 10_000_000_000);
        assert_eq!(handoff.scene_ns, 42_000_000_000);
        assert_eq!(handoff.accent, "Blue");
        assert_eq!(handoff.theme, None);
        assert_eq!(handoff.particles, None);
    }

    #[test]
    fn what_it_reads_it_writes_back_byte_for_byte() {
        let handoff = Handoff::parse(&valid_record()).expect("valid handoff");
        assert_eq!(handoff.encode(), valid_record());
        assert_eq!(handoff.environment(), format!("{ENV}={}", valid_record()));

        let dressed = Handoff::of(BOOT_ID.to_string(), 2, 4, "Blue", Some("Simple"), None)
            .expect("a canonical accent");
        assert!(dressed.encode().ends_with(";theme=Simple"));
        assert_eq!(Handoff::parse(&dressed.encode()), Ok(dressed));

        let lit = Handoff::of(
            BOOT_ID.to_string(),
            2,
            4,
            "Blue",
            Some("Default"),
            Some(true),
        )
        .expect("a canonical accent");
        assert!(lit.encode().ends_with(";theme=Default;particles=on"));
        assert_eq!(Handoff::parse(&lit.encode()), Ok(lit));
        let dark = Handoff::of(BOOT_ID.to_string(), 2, 4, "Blue", None, Some(false))
            .expect("a canonical accent");
        assert!(dark.encode().ends_with(";accent=Blue;particles=off"));
        assert_eq!(Handoff::parse(&dark.encode()), Ok(dark));

        assert_eq!(
            Handoff::of(BOOT_ID.to_string(), 2, 4, "Chartreuse", None, None),
            None
        );
        assert_eq!(
            Handoff::of("not-a-boot-id".to_string(), 2, 4, "Blue", None, None),
            None
        );
    }

    #[test]
    fn field_order_does_not_change_the_record() {
        let handoff = Handoff::parse(&format!(
            "accent=Red;scene-ns=7;sample-ns=3;boot={BOOT_ID};clock={CLOCK};visual={VISUAL};v=1"
        ))
        .expect("reordered handoff");
        assert_eq!(handoff.scene_ns, 7);
        assert_eq!(handoff.accent, "Red");
    }

    #[test]
    fn rejects_partial_duplicate_and_extended_records() {
        assert_eq!(Handoff::parse("v=1").unwrap_err(), Rejection::MissingField);
        assert_eq!(
            Handoff::parse(&format!("{};v=1", valid_record())).unwrap_err(),
            Rejection::DuplicateField
        );
        assert_eq!(
            Handoff::parse(&format!("{};extra=1", valid_record())).unwrap_err(),
            Rejection::UnknownField
        );
    }

    #[test]
    fn rejects_incompatible_or_ambiguous_values() {
        assert_eq!(
            Handoff::parse(&valid_record().replace("v=1", "v=2")).unwrap_err(),
            Rejection::UnsupportedVersion
        );
        assert_eq!(
            Handoff::parse(&valid_record().replace(VISUAL, "lxb-wallpaper-v1")).unwrap_err(),
            Rejection::UnsupportedVisual
        );
        assert_eq!(
            Handoff::parse(&valid_record().replace(CLOCK, "realtime")).unwrap_err(),
            Rejection::UnsupportedClock
        );
        assert_eq!(
            Handoff::parse(&valid_record().replace("accent=Blue", "accent=blue")).unwrap_err(),
            Rejection::InvalidAccent
        );
        assert_eq!(
            Handoff::parse(&valid_record().replace(BOOT_ID, "not-a-boot-id")).unwrap_err(),
            Rejection::InvalidBootId
        );
        assert_eq!(
            Handoff::parse(
                &valid_record().replace("sample-ns=10000000000", "sample-ns=+10000000000")
            )
            .unwrap_err(),
            Rejection::InvalidSample
        );
    }

    #[test]
    fn a_material_it_does_not_know_is_no_material_rather_than_a_broken_record() {
        let plain = valid_record();
        assert_eq!(
            Handoff::parse(&format!("{plain};theme=Simple"))
                .expect("a record with a material")
                .theme
                .as_deref(),
            Some("Simple")
        );
        assert_eq!(
            Handoff::parse(&format!("{plain};theme=Custom wallpaper"))
                .expect("a picture is drawn as the scene by everything but the shell")
                .theme
                .as_deref(),
            Some("Default")
        );
        assert_eq!(
            Handoff::parse(&format!("{plain};theme=Glass"))
                .expect("still a usable phase")
                .theme,
            None
        );
        assert_eq!(
            Handoff::parse(&format!("{plain};theme=Simple;theme=Default")).unwrap_err(),
            Rejection::DuplicateField
        );
    }

    #[test]
    fn sparkles_it_cannot_read_are_no_answer_rather_than_a_broken_record() {
        let plain = valid_record();
        let particles = |field: &str| {
            Handoff::parse(&format!("{plain};{field}"))
                .expect("still a usable phase")
                .particles
        };
        assert_eq!(particles("particles=on"), Some(true));
        assert_eq!(particles("particles=off"), Some(false));
        assert_eq!(particles("particles=sometimes"), None);
        assert_eq!(
            Handoff::parse(&format!("{plain};particles=on;particles=off")).unwrap_err(),
            Rejection::DuplicateField
        );
    }

    #[test]
    fn rejects_non_ascii_and_oversized_records() {
        assert_eq!(
            Handoff::parse("accent=Bl\u{00fa}e").unwrap_err(),
            Rejection::NonAscii
        );
        assert_eq!(
            Handoff::parse(&"x".repeat(MAX_RECORD_BYTES + 1)).unwrap_err(),
            Rejection::TooLong
        );
    }

    #[test]
    fn advances_scene_time_by_monotonic_elapsed_time() {
        let handoff = Handoff::parse(&valid_record()).expect("valid handoff");
        assert_eq!(
            handoff.scene_at(BOOT_ID, 10_250_000_000).expect("fresh"),
            42_250_000_000
        );
        assert_eq!(scene_seconds(42_250_000_000), 42.25);
    }

    #[test]
    fn rejects_another_boot_future_samples_and_stale_samples() {
        let handoff = Handoff::parse(&valid_record()).expect("valid handoff");
        assert_eq!(
            handoff
                .scene_at("ffffffff-ffff-ffff-ffff-ffffffffffff", 10_000_000_000)
                .unwrap_err(),
            Rejection::BootChanged
        );
        assert_eq!(
            handoff.scene_at(BOOT_ID, 9_999_999_999).unwrap_err(),
            Rejection::FutureSample
        );
        assert_eq!(
            handoff
                .scene_at(BOOT_ID, 10_000_000_000 + MAX_AGE_NS + 1)
                .unwrap_err(),
            Rejection::Expired
        );
    }

    #[test]
    fn rejects_scene_time_overflow() {
        let record =
            valid_record().replace("scene-ns=42000000000", &format!("scene-ns={}", u64::MAX));
        let handoff = Handoff::parse(&record).expect("well-formed handoff");
        assert_eq!(
            handoff.scene_at(BOOT_ID, 10_000_000_001).unwrap_err(),
            Rejection::Overflow
        );
    }

    #[test]
    fn this_machines_boot_id_is_one_this_reader_would_accept() {
        match boot_id() {
            Ok(id) => assert!(valid_boot_id(&id), "{id}"),
            Err(reason) => assert_eq!(reason, Rejection::BootIdUnavailable),
        }
    }
}
