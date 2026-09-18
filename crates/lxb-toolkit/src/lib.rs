pub mod accent;
pub mod assets;
pub mod color;
pub mod control;
pub mod css;
pub mod glyph_material;
pub mod handoff;
pub mod i18n;
pub mod input;
pub mod material;
pub mod menu;
pub mod metrics;
pub mod motion;
pub mod paint;
pub mod palette;
pub mod picker;
pub mod settings;
pub mod sound;
pub mod typography;
pub mod wallpaper;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const TRANSCRIBED_FROM: &str = "d3adc00";

pub fn motion_durations() -> [(&'static str, f32); 34] {
    use motion::duration::*;
    [
        ("flight", FLIGHT),
        ("panel", PANEL),
        ("launch-open", LAUNCH_OPEN),
        ("launch-settle", LAUNCH_SETTLE),
        ("launch-handover", LAUNCH_HANDOVER),
        ("arrival", ARRIVAL),
        ("black-handover", BLACK_HANDOVER),
        ("sidebar-slide", SIDEBAR_SLIDE),
        ("entry-lead", ENTRY_LEAD),
        ("entry-stagger", ENTRY_STAGGER),
        ("entry-slide", ENTRY_SLIDE),
        ("accent-change", ACCENT_CHANGE),
        ("scenery-fade", SCENERY_FADE),
        ("colour-fade", COLOUR_FADE),
        ("pulse", PULSE),
        ("spin", SPIN),
        ("black-in", BLACK_IN),
        ("black-hold", BLACK_HOLD),
        ("black-out", BLACK_OUT),
        ("menu-flight", MENU_FLIGHT),
        ("menu-unfold", MENU_UNFOLD),
        ("menu-press", MENU_PRESS),
        ("guide-press", GUIDE_PRESS),
        ("guide-power-flight", GUIDE_POWER_FLIGHT),
        ("guide-media-flight", GUIDE_MEDIA_FLIGHT),
        ("guide-transport-flight", GUIDE_TRANSPORT_FLIGHT),
        ("notification-hold", NOTIFICATION_HOLD),
        ("notification-enter", NOTIFICATION_ENTER),
        ("notification-leave", NOTIFICATION_LEAVE),
        ("notification-badge", NOTIFICATION_BADGE),
        ("volume-fill", VOLUME_FILL),
        ("volume-hold", VOLUME_HOLD),
        ("volume-leave", VOLUME_LEAVE),
        ("keyboard-slide", KEYBOARD_SLIDE),
    ]
}

#[cfg(test)]
mod tests {

    #[test]
    fn the_toolkit_is_whole() {
        assert!(!super::VERSION.is_empty());
        assert_eq!(super::TRANSCRIBED_FROM.len(), 7);
        assert_eq!(crate::palette::PALETTES.len(), 12);
        assert_eq!(crate::assets::GLYPHS.len(), 109);
        assert_eq!(crate::sound::Sound::ALL.len(), 14);
    }

    #[test]
    fn duration_names_travel_to_other_languages() {
        let mut names: Vec<&str> = super::motion_durations()
            .iter()
            .map(|(name, _)| *name)
            .collect();
        for name in &names {
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{name}"
            );
        }
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
    }
}
