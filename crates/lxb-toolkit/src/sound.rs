pub const REST: f32 = 0.060;

pub const MUSIC_FADE_IN: f32 = 0.400;

pub const MUSIC_FADE_OUT: f32 = 0.600;

pub const RETRY_AFTER: f32 = 5.0;

pub fn amplitude(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    let mut amplitude = (6.907_755_4 * value).exp() / 1000.0;
    if value < 0.1 {
        amplitude *= value * 10.0;
    }
    amplitude
}

pub fn fade(elapsed: f32, over: f32) -> f32 {
    let progress = (elapsed / over).clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voice {
    Start,

    Guide,

    Anywhere,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sound {
    Move,

    Press,

    GuideMove,

    GuidePress,

    Back,

    Key,

    Launch,

    GuideOpen,

    Shutter,

    Authenticate,

    Notify,

    Trash,

    Error,

    Music,
}

impl Sound {
    pub const ALL: [Sound; 14] = [
        Sound::Move,
        Sound::Press,
        Sound::GuideMove,
        Sound::GuidePress,
        Sound::Back,
        Sound::Key,
        Sound::Launch,
        Sound::GuideOpen,
        Sound::Shutter,
        Sound::Authenticate,
        Sound::Notify,
        Sound::Trash,
        Sound::Error,
        Sound::Music,
    ];

    pub const SHELL_USED: [Sound; 12] = [
        Sound::Move,
        Sound::Press,
        Sound::GuideMove,
        Sound::GuidePress,
        Sound::Back,
        Sound::Key,
        Sound::Launch,
        Sound::GuideOpen,
        Sound::Shutter,
        Sound::Authenticate,
        Sound::Notify,
        Sound::Music,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Sound::Move => "press",
            Sound::Press => "press-selected",
            Sound::GuideMove => "press-guide",
            Sound::GuidePress => "press-guide-selected",
            Sound::Back => "press-back",
            Sound::Key => "keyboard-click",
            Sound::Launch => "app-launch",
            Sound::GuideOpen => "guide-open",
            Sound::Shutter => "screenshot",
            Sound::Authenticate => "polkit",
            Sound::Notify => "notification",
            Sound::Trash => "trash",
            Sound::Error => "error",
            Sound::Music => "start-bg-music",
        }
    }

    pub const fn voice(self) -> Voice {
        match self {
            Sound::Move | Sound::Press | Sound::Launch | Sound::Music => Voice::Start,
            Sound::GuideMove | Sound::GuidePress => Voice::Guide,
            _ => Voice::Anywhere,
        }
    }

    pub const fn loops(self) -> bool {
        matches!(self, Sound::Music)
    }

    pub const fn used_by_shell(self) -> bool {
        !matches!(self, Sound::Trash | Sound::Error)
    }

    pub fn bytes(self) -> Option<&'static [u8]> {
        crate::assets::sound(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_has_its_recording() {
        for sound in Sound::ALL {
            assert!(sound.bytes().is_some(), "{:?} has no clip", sound);
        }
    }

    #[test]
    fn each_screen_answers_a_move_and_a_press_in_its_own_voice() {
        let start = [Sound::Move, Sound::Press];
        let guide = [Sound::GuideMove, Sound::GuidePress];
        for pair in [start, guide] {
            assert_ne!(pair[0].name(), pair[1].name());
        }
        for one in start {
            for other in guide {
                assert_ne!(
                    one.name(),
                    other.name(),
                    "the two screens would sound the same"
                );
            }
        }
        assert_eq!(Sound::Move.voice(), Voice::Start);
        assert_eq!(Sound::GuidePress.voice(), Voice::Guide);
    }

    #[test]
    fn a_launch_belongs_to_the_screen_that_launches() {
        assert_eq!(Sound::Launch.voice(), Voice::Start);
        assert_eq!(Sound::GuideOpen.voice(), Voice::Anywhere);
        assert_eq!(Sound::Shutter.voice(), Voice::Anywhere);
    }

    #[test]
    fn exactly_one_recording_loops() {
        let looping: Vec<Sound> = Sound::ALL.into_iter().filter(|s| s.loops()).collect();
        assert_eq!(looping, vec![Sound::Music]);
    }

    #[test]
    fn compatibility_recordings_are_not_presented_as_shell_actions() {
        assert_eq!(Sound::SHELL_USED.len(), 12);
        assert!(Sound::SHELL_USED.iter().all(|sound| sound.used_by_shell()));
        assert!(!Sound::Trash.used_by_shell());
        assert!(!Sound::Error.used_by_shell());
        assert!(Sound::Music.used_by_shell());
    }

    #[test]
    fn a_level_is_loudness_rather_than_amplitude() {
        assert_eq!(amplitude(0.0), 0.0);
        assert!((amplitude(1.0) - 1.0).abs() < 1e-5);
        assert!(
            amplitude(0.5) < 0.25,
            "half a control is not half the scale"
        );

        let mut last = -1.0;
        for step in 0..=100 {
            let value = amplitude(step as f32 / 100.0);
            assert!(value >= last, "{step}");
            last = value;
        }
        assert_eq!(amplitude(-1.0), 0.0);
        assert_eq!(amplitude(2.0), amplitude(1.0));
    }

    #[test]
    fn a_fade_starts_and_ends_flat() {
        assert_eq!(fade(0.0, MUSIC_FADE_IN), 0.0);
        assert_eq!(fade(MUSIC_FADE_IN, MUSIC_FADE_IN), 1.0);
        assert_eq!(fade(99.0, MUSIC_FADE_IN), 1.0);
        assert!((fade(MUSIC_FADE_IN / 2.0, MUSIC_FADE_IN) - 0.5).abs() < 1e-6);

        let near_start = fade(0.1 * MUSIC_FADE_IN, MUSIC_FADE_IN);
        let across_middle =
            fade(0.55 * MUSIC_FADE_IN, MUSIC_FADE_IN) - fade(0.45 * MUSIC_FADE_IN, MUSIC_FADE_IN);
        assert!(near_start < across_middle);
    }

    const _: () = assert!(MUSIC_FADE_OUT > MUSIC_FADE_IN);

    const _: () = assert!(REST > 0.0 && REST < 0.090, "a held direction would stutter");
}
