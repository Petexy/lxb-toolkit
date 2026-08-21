use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::cpal::{self, traits::HostTrait, StreamError};
use rodio::{
    Decoder, DeviceSinkBuilder, DeviceSinkError, DeviceTrait, MixerDeviceSink, Player, Sample,
    Source,
};

use lxb_toolkit::sound::{self, Sound};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    pub value: f32,

    pub muted: bool,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            value: 1.0,
            muted: false,
        }
    }
}

impl Level {
    fn audible(self) -> bool {
        !self.muted && self.value > 0.0
    }
}

const EFFECTS: usize = Sound::ALL.len();

struct Music {
    player: Player,
    started: Instant,

    going: Option<(Instant, f32)>,
}

pub struct Sounds {
    device: Option<MixerDeviceSink>,

    lost: Arc<AtomicBool>,

    clips: [Option<SamplesBuffer>; EFFECTS],

    played: [Option<Instant>; EFFECTS],
    music: Option<Music>,

    music_wanted: bool,

    music_broken: bool,
    level: Level,

    retry: Instant,

    trouble: Option<String>,
}

impl Default for Sounds {
    fn default() -> Self {
        Self::new()
    }
}

impl Sounds {
    pub fn new() -> Self {
        let mut sounds = Self {
            device: None,
            lost: Arc::new(AtomicBool::new(false)),
            clips: Sound::ALL.map(|sound| (!sound.loops()).then(|| decode(sound)).flatten()),
            played: [None; EFFECTS],
            music: None,
            music_wanted: false,
            music_broken: false,
            level: Level::default(),
            retry: Instant::now(),
            trouble: None,
        };
        sounds.open();
        sounds
    }

    pub fn silent() -> Self {
        Self {
            device: None,
            lost: Arc::new(AtomicBool::new(false)),
            clips: Sound::ALL.map(|_| None),
            played: [None; EFFECTS],
            music: None,
            music_wanted: false,

            music_broken: true,
            level: Level {
                value: 0.0,
                muted: true,
            },
            retry: Instant::now(),
            trouble: None,
        }
    }

    pub fn set_level(&mut self, level: Level) {
        self.level = Level {
            value: level.value.clamp(0.0, 1.0),
            muted: level.muted,
        };
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn trouble(&self) -> Option<&str> {
        self.trouble.as_deref()
    }

    pub fn play(&mut self, sound: Sound) {
        if sound.loops() || !self.level.audible() {
            return;
        }
        let index = sound as usize;
        let now = Instant::now();
        if !rested(self.played[index], now) {
            return;
        }

        let Some(clip) = self.clips[index].clone() else {
            return;
        };
        self.reopen_if_lost(now);
        if self.device.is_none() {
            self.open();
        }
        if let Some(device) = &self.device {
            device
                .mixer()
                .add(clip.amplify_normalized(self.level.value));

            self.played[index] = Some(now);
        }
    }

    pub fn music(&mut self, wanted: bool) {
        let now = Instant::now();
        self.reopen_if_lost(now);

        let rising = wanted && !self.music_wanted;
        let falling = !wanted && self.music_wanted;
        self.music_wanted = wanted;
        if rising {
            self.stop_music();
        }
        if falling {
            if let Some(music) = self.music.as_mut() {
                music.going = Some((now, music.player.volume()));
            }
        }

        if !self.level.audible() {
            self.stop_music();
            return;
        }

        if wanted {
            if self.music.is_none() {
                self.start_music(now);
            }
            let level = self.level.value;
            if let Some(music) = self.music.as_mut() {
                let through = sound::fade(
                    now.saturating_duration_since(music.started).as_secs_f32(),
                    sound::MUSIC_FADE_IN,
                );
                music.player.set_volume(sound::amplitude(level) * through);
            }
            return;
        }

        let mut finished = false;
        if let Some(music) = self.music.as_mut() {
            let (started, from) = *music.going.get_or_insert((now, music.player.volume()));
            let through = sound::fade(
                now.saturating_duration_since(started).as_secs_f32(),
                sound::MUSIC_FADE_OUT,
            );
            let volume = from * (1.0 - through);
            music.player.set_volume(volume);
            finished = volume <= 0.0;
        }
        if finished {
            self.stop_music();
        }
    }

    fn start_music(&mut self, now: Instant) {
        if self.music_broken {
            return;
        }
        if self.device.is_none() {
            self.open();
        }
        let Some(device) = self.device.as_ref() else {
            return;
        };
        let Some(bytes) = Sound::Music.bytes() else {
            self.music_broken = true;
            return;
        };
        let Ok(source) = Decoder::new_looped(Cursor::new(bytes)) else {
            self.music_broken = true;
            return;
        };
        let player = Player::connect_new(device.mixer());
        player.set_volume(0.0);
        player.append(source);
        self.music = Some(Music {
            player,
            started: now,
            going: None,
        });
    }

    fn stop_music(&mut self) {
        if let Some(music) = self.music.take() {
            music.player.stop();
        }
    }

    fn reopen_if_lost(&mut self, now: Instant) {
        if self.lost.swap(false, Ordering::AcqRel) {
            self.stop_music();
            self.device = None;
            self.retry = now;
        }
    }

    fn open(&mut self) {
        if Instant::now() < self.retry {
            return;
        }
        let lost = Arc::new(AtomicBool::new(false));
        match open_output(Arc::clone(&lost)) {
            Ok(mut device) => {
                device.log_on_drop(false);
                self.lost = lost;
                self.device = Some(device);
                self.trouble = None;
            }
            Err(err) => {
                self.trouble = Some(err.to_string());
                self.retry = Instant::now() + Duration::from_secs_f32(sound::RETRY_AFTER);
            }
        }
    }
}

fn rested(played: Option<Instant>, now: Instant) -> bool {
    played.is_none_or(|last| {
        now.saturating_duration_since(last) >= Duration::from_secs_f32(sound::REST)
    })
}

fn decode(sound: Sound) -> Option<SamplesBuffer> {
    let decoder = Decoder::new(Cursor::new(sound.bytes()?)).ok()?;
    let channels = decoder.channels();
    let rate = decoder.sample_rate();
    let samples: Vec<Sample> = decoder.collect();
    if samples.is_empty() {
        return None;
    }
    Some(SamplesBuffer::new(channels, rate, samples))
}

fn open_output(lost: Arc<AtomicBool>) -> Result<MixerDeviceSink, DeviceSinkError> {
    let callback = error_callback(lost);
    DeviceSinkBuilder::from_default_device()
        .and_then(|builder| builder.with_error_callback(callback.clone()).open_stream())
        .or_else(|first| {
            let Ok(devices) = cpal::default_host().output_devices() else {
                return Err(first);
            };
            devices
                .filter(|device| {
                    device.description().is_ok_and(|description| {
                        description.driver().is_some_and(|driver| driver != "null")
                    })
                })
                .find_map(|device| {
                    DeviceSinkBuilder::from_device(device)
                        .and_then(|builder| {
                            builder
                                .with_error_callback(callback.clone())
                                .open_sink_or_fallback()
                        })
                        .ok()
                })
                .ok_or(first)
        })
}

fn error_callback(lost: Arc<AtomicBool>) -> impl FnMut(StreamError) + Clone + Send + 'static {
    move |err| {
        if matches!(
            err,
            StreamError::DeviceNotAvailable | StreamError::StreamInvalidated
        ) {
            lost.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_recording_decodes() {
        for sound in Sound::ALL {
            let clip = decode(sound).unwrap_or_else(|| panic!("{} decodes", sound.name()));
            assert!(
                clip.total_duration().is_some_and(|held| !held.is_zero()),
                "{} has no samples in it, so it is a silent one",
                sound.name()
            );
        }
    }

    #[test]
    fn the_short_clips_are_shorter_than_the_rest_between_them() {
        let rest = Duration::from_secs_f32(sound::REST);
        for sound in [Sound::Move, Sound::GuideMove, Sound::Key] {
            let held = decode(sound)
                .and_then(|clip| clip.total_duration())
                .unwrap();
            assert!(
                held <= rest * 4,
                "{} is {held:?}, which is not a click",
                sound.name()
            );
        }
    }

    #[test]
    fn a_clip_is_never_laid_over_a_copy_of_itself() {
        let now = Instant::now();
        let rest = Duration::from_secs_f32(sound::REST);
        assert!(rested(None, now));
        assert!(!rested(Some(now), now));
        assert!(!rested(Some(now), now + rest - Duration::from_millis(1)));
        assert!(rested(Some(now), now + rest));
    }

    #[test]
    fn a_silenced_level_is_inaudible_either_way() {
        assert!(Level::default().audible());
        assert!(!Level {
            value: 1.0,
            muted: true
        }
        .audible());
        assert!(!Level {
            value: 0.0,
            muted: false
        }
        .audible());
    }
}
