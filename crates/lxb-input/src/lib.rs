use std::time::Duration;

use gilrs::{Axis, Button as PadButton, EventType, Gilrs, GilrsBuilder, MappingSource};

use lxb_toolkit::input::{Action, Button, Direction, Key, Repeat};

#[cfg(feature = "winit")]
mod from_winit;
#[cfg(feature = "winit")]
pub use from_winit::key_of;

const HAT: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    Mapped,

    Guessed,
}

#[derive(Debug, Clone, Copy)]
struct HeldKey {
    action: Action,
    next: Duration,
}

pub struct Controls {
    pads: Option<Gilrs>,
    repeat: Repeat,
    held: Option<HeldKey>,

    guide_held: bool,

    trouble: Option<String>,
}

impl Default for Controls {
    fn default() -> Self {
        Self::new()
    }
}

impl Controls {
    pub fn new() -> Self {
        let (pads, trouble) = match GilrsBuilder::new().with_force_feedback(false).build() {
            Ok(pads) => (Some(pads), None),
            Err(err) => (None, Some(err.to_string())),
        };
        Self {
            pads,
            repeat: Repeat::default(),
            held: None,
            guide_held: false,
            trouble,
        }
    }

    pub fn pads(&self) -> usize {
        self.pads
            .as_ref()
            .map(|pads| pads.gamepads().count())
            .unwrap_or(0)
    }

    pub fn trouble(&self) -> Option<&str> {
        self.trouble.as_deref()
    }

    pub fn key(&mut self, key: Key, down: bool, now: Duration) -> Option<Action> {
        let action = Action::of_key(key);
        if !down {
            if self.held.map(|held| held.action) == action {
                self.held = None;
            }
            return None;
        }
        let action = action?;
        self.held = action.repeats().then_some(HeldKey {
            action,
            next: now + lxb_toolkit::input::INITIAL_REPEAT_DELAY,
        });
        Some(action)
    }

    pub fn poll(&mut self, now: Duration) -> Vec<Action> {
        let mut actions = Vec::new();

        if let Some(held) = self.held.as_mut() {
            if now >= held.next {
                held.next = now + lxb_toolkit::input::REPEAT_INTERVAL;
                actions.push(held.action);
            }
        }

        let mut pressed = [false; Direction::COUNT];
        let mut stick = (0.0_f32, 0.0_f32);

        if let Some(pads) = self.pads.as_mut() {
            while let Some(event) = pads.next_event() {
                let (button, code, down) = match event.event {
                    EventType::ButtonPressed(button, code) => (button, code.into_u32(), true),
                    EventType::ButtonReleased(button, code) => (button, code.into_u32(), false),
                    _ => continue,
                };
                let layout = match pads.gamepad(event.id).mapping_source() {
                    MappingSource::SdlMappings => Layout::Mapped,

                    MappingSource::Driver | MappingSource::None => Layout::Guessed,
                };
                let Some(control) = control_of(button, code, layout) else {
                    continue;
                };
                if control == Button::Guide {
                    self.guide_held = down;
                    continue;
                }

                if !down || control.direction().is_some() {
                    continue;
                }
                if control == Button::RightBumper && self.guide_held {
                    continue;
                }
                actions.extend(Action::of_button(control));
            }

            for (_, pad) in pads.gamepads() {
                pressed[Direction::Left.index()] |= pad.is_pressed(PadButton::DPadLeft);
                pressed[Direction::Right.index()] |= pad.is_pressed(PadButton::DPadRight);
                pressed[Direction::Up.index()] |= pad.is_pressed(PadButton::DPadUp);
                pressed[Direction::Down.index()] |= pad.is_pressed(PadButton::DPadDown);

                let (hx, hy) = (pad.value(Axis::DPadX), pad.value(Axis::DPadY));
                pressed[Direction::Left.index()] |= hx <= -HAT;
                pressed[Direction::Right.index()] |= hx >= HAT;
                pressed[Direction::Up.index()] |= hy >= HAT;
                pressed[Direction::Down.index()] |= hy <= -HAT;

                stick.0 = further(stick.0, pad.value(Axis::LeftStickX));
                stick.1 = further(stick.1, pad.value(Axis::LeftStickY));
            }
        }

        self.repeat.update(now, pressed, stick, &mut actions);
        actions
    }

    pub fn release(&mut self) {
        self.repeat.reset();
        self.held = None;
        self.guide_held = false;
    }
}

fn further(current: f32, candidate: f32) -> f32 {
    if candidate.abs() > current.abs() {
        candidate
    } else {
        current
    }
}

fn control_of(button: PadButton, code: u32, layout: Layout) -> Option<Button> {
    if is_top_face(button, code, layout) {
        return Some(Button::North);
    }
    let named = match button {
        PadButton::South => Some(Button::South),
        PadButton::East => Some(Button::East),
        PadButton::West => Some(Button::West),
        PadButton::Start => Some(Button::Start),
        PadButton::Select => Some(Button::Select),

        PadButton::LeftTrigger => Some(Button::LeftBumper),
        PadButton::RightTrigger => Some(Button::RightBumper),
        PadButton::Mode => Some(Button::Guide),
        PadButton::DPadLeft => Some(Button::DPadLeft),
        PadButton::DPadRight => Some(Button::DPadRight),
        PadButton::DPadUp => Some(Button::DPadUp),
        PadButton::DPadDown => Some(Button::DPadDown),
        PadButton::Unknown => None,
        _ => return None,
    };
    if named.is_some() {
        return named;
    }
    match code {
        evdev::BTN_SOUTH | evdev::BTN_TRIGGER => Some(Button::South),
        evdev::BTN_EAST | evdev::BTN_THUMB => Some(Button::East),
        evdev::BTN_START => Some(Button::Start),
        evdev::BTN_SELECT => Some(Button::Select),
        evdev::BTN_TL => Some(Button::LeftBumper),
        evdev::BTN_TR => Some(Button::RightBumper),
        evdev::BTN_MODE => Some(Button::Guide),
        _ => None,
    }
}

fn is_top_face(button: PadButton, code: u32, layout: Layout) -> bool {
    match layout {
        Layout::Mapped => matches!(button, PadButton::North),
        Layout::Guessed => match button {
            PadButton::North | PadButton::West => matches!(code, evdev::BTN_X | evdev::BTN_WEST),

            PadButton::Unknown => matches!(code, evdev::BTN_TOP | evdev::BTN_TOP2),
            _ => false,
        },
    }
}

mod evdev {
    const EV_KEY: u32 = 0x01;
    const fn key(code: u32) -> u32 {
        (EV_KEY << 16) | code
    }

    pub const BTN_TRIGGER: u32 = key(0x120);
    pub const BTN_THUMB: u32 = key(0x121);
    pub const BTN_TOP: u32 = key(0x122);
    pub const BTN_TOP2: u32 = key(0x123);

    pub const BTN_SOUTH: u32 = key(0x130);

    pub const BTN_EAST: u32 = key(0x131);

    pub const BTN_X: u32 = key(0x133);

    pub const BTN_WEST: u32 = key(0x134);
    pub const BTN_TL: u32 = key(0x136);
    pub const BTN_TR: u32 = key(0x137);
    pub const BTN_SELECT: u32 = key(0x13a);
    pub const BTN_START: u32 = key(0x13b);
    pub const BTN_MODE: u32 = key(0x13c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_button_and_its_chord_are_held_back() {
        assert_eq!(Action::of_button(Button::Guide), None);
        assert_eq!(
            control_of(PadButton::Mode, evdev::BTN_MODE, Layout::Mapped),
            Some(Button::Guide)
        );
        assert_eq!(
            control_of(PadButton::Unknown, evdev::BTN_MODE, Layout::Guessed),
            Some(Button::Guide)
        );
    }

    #[test]
    fn a_pad_with_no_mapping_still_has_a_context_menu() {
        assert!(is_top_face(PadButton::North, 0, Layout::Mapped));
        assert!(!is_top_face(PadButton::West, 0, Layout::Mapped));

        assert!(is_top_face(PadButton::North, evdev::BTN_X, Layout::Guessed));
        assert!(is_top_face(
            PadButton::West,
            evdev::BTN_WEST,
            Layout::Guessed
        ));
        assert!(is_top_face(
            PadButton::Unknown,
            evdev::BTN_TOP,
            Layout::Guessed
        ));
        assert!(!is_top_face(
            PadButton::South,
            evdev::BTN_SOUTH,
            Layout::Guessed
        ));
    }

    #[test]
    fn a_nameless_joystick_still_answers() {
        for (code, expected) in [
            (evdev::BTN_TRIGGER, Button::South),
            (evdev::BTN_THUMB, Button::East),
            (evdev::BTN_SOUTH, Button::South),
            (evdev::BTN_START, Button::Start),
            (evdev::BTN_TR, Button::RightBumper),
        ] {
            assert_eq!(
                control_of(PadButton::Unknown, code, Layout::Guessed),
                Some(expected),
                "{code:#x}"
            );
        }
        assert_eq!(control_of(PadButton::Unknown, 0, Layout::Guessed), None);
    }

    #[test]
    fn the_stick_pushed_furthest_is_the_one_read() {
        assert_eq!(further(0.0, -0.9), -0.9);
        assert_eq!(further(-0.9, 0.4), -0.9);
        assert_eq!(further(0.2, -0.5), -0.5);
    }

    #[test]
    fn a_held_key_repeats_at_the_pads_cadence() {
        let mut controls = Controls::new();
        let at = |ms| Duration::from_millis(ms);

        assert_eq!(controls.key(Key::Down, true, at(0)), Some(Action::Down));
        assert!(controls.poll(at(349)).is_empty());
        assert_eq!(controls.poll(at(350)), [Action::Down]);
        assert!(controls.poll(at(439)).is_empty());
        assert_eq!(controls.poll(at(440)), [Action::Down]);

        assert_eq!(controls.key(Key::Down, false, at(450)), None);
        assert!(controls.poll(at(900)).is_empty());
    }

    #[test]
    fn a_held_accept_is_one_press() {
        let mut controls = Controls::new();
        let at = |ms| Duration::from_millis(ms);
        assert_eq!(controls.key(Key::Enter, true, at(0)), Some(Action::Accept));
        assert!(controls.poll(at(1000)).is_empty());
    }

    #[test]
    fn releasing_forgets_a_held_direction() {
        let mut controls = Controls::new();
        let at = |ms| Duration::from_millis(ms);
        assert_eq!(controls.key(Key::Up, true, at(0)), Some(Action::Up));
        controls.release();
        assert!(controls.poll(at(1000)).is_empty());
    }

    #[test]
    fn another_key_coming_up_does_not_stop_the_held_one() {
        let mut controls = Controls::new();
        let at = |ms| Duration::from_millis(ms);
        assert_eq!(controls.key(Key::Down, true, at(0)), Some(Action::Down));
        assert_eq!(controls.key(Key::Enter, false, at(10)), None);
        assert_eq!(controls.poll(at(350)), [Action::Down]);
    }
}
