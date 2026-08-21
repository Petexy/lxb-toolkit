use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Left,
    Right,
    Up,
    Down,

    Accept,

    Back,

    Menu,

    Submit,

    Previous,
    Next,
}

impl Action {
    pub const ALL: [Action; 10] = [
        Action::Left,
        Action::Right,
        Action::Up,
        Action::Down,
        Action::Accept,
        Action::Back,
        Action::Menu,
        Action::Submit,
        Action::Previous,
        Action::Next,
    ];

    pub const fn direction(self) -> Option<Direction> {
        match self {
            Action::Left => Some(Direction::Left),
            Action::Right => Some(Direction::Right),
            Action::Up => Some(Direction::Up),
            Action::Down => Some(Direction::Down),
            _ => None,
        }
    }

    pub const fn repeats(self) -> bool {
        self.direction().is_some()
    }

    pub const fn name(self) -> &'static str {
        match self {
            Action::Left => "left",
            Action::Right => "right",
            Action::Up => "up",
            Action::Down => "down",
            Action::Accept => "accept",
            Action::Back => "back",
            Action::Menu => "menu",
            Action::Submit => "submit",
            Action::Previous => "previous",
            Action::Next => "next",
        }
    }

    pub fn named(name: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|action| action.name() == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ];
    pub const COUNT: usize = Direction::ALL.len();

    pub const fn index(self) -> usize {
        match self {
            Direction::Left => 0,
            Direction::Right => 1,
            Direction::Up => 2,
            Direction::Down => 3,
        }
    }

    pub const fn action(self) -> Action {
        match self {
            Direction::Left => Action::Left,
            Direction::Right => Action::Right,
            Direction::Up => Action::Up,
            Direction::Down => Action::Down,
        }
    }

    pub const fn step(self) -> isize {
        match self {
            Direction::Left | Direction::Up => -1,
            Direction::Right | Direction::Down => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Enter,
    Space,
    Escape,
    Backspace,

    Tab,
    BackTab,

    Menu,
    F10,

    Letter(char),
}

impl Action {
    pub fn of_key(key: Key) -> Option<Action> {
        match key {
            Key::Left => Some(Action::Left),
            Key::Right => Some(Action::Right),
            Key::Up => Some(Action::Up),
            Key::Down => Some(Action::Down),
            Key::Enter | Key::Space => Some(Action::Accept),
            Key::Escape | Key::Backspace => Some(Action::Back),
            Key::Menu | Key::F10 => Some(Action::Menu),
            Key::Tab => Some(Action::Next),
            Key::BackTab => Some(Action::Previous),
            Key::Letter(_) => None,
        }
    }

    pub fn of_letter(letter: char) -> Option<Action> {
        match letter.to_ascii_lowercase() {
            'a' | 'h' => Some(Action::Left),
            'd' | 'l' => Some(Action::Right),
            'w' | 'k' => Some(Action::Up),
            's' | 'j' => Some(Action::Down),
            'y' => Some(Action::Menu),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    South,

    East,

    North,

    West,

    Start,

    Select,
    LeftBumper,
    RightBumper,

    Guide,
    DPadLeft,
    DPadRight,
    DPadUp,
    DPadDown,
}

impl Button {
    pub const fn direction(self) -> Option<Direction> {
        match self {
            Button::DPadLeft => Some(Direction::Left),
            Button::DPadRight => Some(Direction::Right),
            Button::DPadUp => Some(Direction::Up),
            Button::DPadDown => Some(Direction::Down),
            _ => None,
        }
    }
}

impl Action {
    pub const fn of_button(button: Button) -> Option<Action> {
        match button {
            Button::South => Some(Action::Accept),
            Button::East => Some(Action::Back),
            Button::North => Some(Action::Menu),
            Button::Start => Some(Action::Submit),
            Button::LeftBumper => Some(Action::Previous),
            Button::RightBumper => Some(Action::Next),
            Button::DPadLeft => Some(Action::Left),
            Button::DPadRight => Some(Action::Right),
            Button::DPadUp => Some(Action::Up),
            Button::DPadDown => Some(Action::Down),
            Button::Guide | Button::West | Button::Select => None,
        }
    }
}

pub const POLL_INTERVAL: Duration = Duration::from_millis(8);

pub const INITIAL_REPEAT_DELAY: Duration = Duration::from_millis(350);

pub const REPEAT_INTERVAL: Duration = Duration::from_millis(90);

pub const STICK_ENGAGE: f32 = 0.55;

pub const STICK_RELEASE: f32 = 0.35;

pub fn stick_is_pushed(x: f32, y: f32) -> bool {
    x.abs() >= STICK_ENGAGE || y.abs() >= STICK_ENGAGE
}

pub const SCROLL_STEP: f32 = 15.0;

pub const TAP_SLOP: f32 = 24.0;

#[derive(Debug, Clone, Copy, Default)]
pub struct Wheel {
    carried: f32,
}

impl Wheel {
    pub fn notches(&mut self, notches: f32) -> i32 {
        let total = self.carried + notches;
        let whole = total.trunc();
        self.carried = total - whole;
        whole as i32
    }

    pub fn distance(&mut self, pixels: f32) -> i32 {
        self.notches(pixels / SCROLL_STEP)
    }

    pub fn reset(&mut self) {
        self.carried = 0.0;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Axis {
    #[default]
    Centred,
    Negative,
    Positive,
}

impl Axis {
    pub fn update(self, value: f32) -> Axis {
        match self {
            Axis::Centred => {
                if value <= -STICK_ENGAGE {
                    Axis::Negative
                } else if value >= STICK_ENGAGE {
                    Axis::Positive
                } else {
                    Axis::Centred
                }
            }
            Axis::Negative if value > -STICK_RELEASE => Axis::Centred,
            Axis::Positive if value < STICK_RELEASE => Axis::Centred,
            held => held,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Held {
    active: bool,
    next: Option<Duration>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Repeat {
    x: Axis,
    y: Axis,
    held: [Held; Direction::COUNT],
}

impl Repeat {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update(
        &mut self,
        now: Duration,
        pressed: [bool; Direction::COUNT],
        stick: (f32, f32),
        out: &mut Vec<Action>,
    ) {
        let mut pressed = pressed;
        self.x = self.x.update(stick.0);
        self.y = self.y.update(stick.1);
        pressed[Direction::Left.index()] |= self.x == Axis::Negative;
        pressed[Direction::Right.index()] |= self.x == Axis::Positive;
        pressed[Direction::Up.index()] |= self.y == Axis::Positive;
        pressed[Direction::Down.index()] |= self.y == Axis::Negative;

        for direction in Direction::ALL {
            let index = direction.index();
            let held = &mut self.held[index];
            if !pressed[index] {
                *held = Held::default();
                continue;
            }
            if !held.active {
                held.active = true;
                held.next = Some(now + INITIAL_REPEAT_DELAY);
                out.push(direction.action());
                continue;
            }
            if held.next.is_some_and(|due| now >= due) {
                held.next = Some(now + REPEAT_INTERVAL);
                out.push(direction.action());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(
        repeat: &mut Repeat,
        millis: u64,
        pressed: [bool; Direction::COUNT],
        stick: (f32, f32),
    ) -> Vec<Action> {
        let mut out = Vec::new();
        repeat.update(Duration::from_millis(millis), pressed, stick, &mut out);
        out
    }

    #[test]
    fn a_held_direction_fires_once_and_then_repeats() {
        let mut repeat = Repeat::default();
        let mut down = [false; Direction::COUNT];
        down[Direction::Down.index()] = true;

        assert_eq!(step(&mut repeat, 0, down, (0.0, 0.0)), [Action::Down]);
        assert!(step(&mut repeat, 349, down, (0.0, 0.0)).is_empty());
        assert_eq!(step(&mut repeat, 350, down, (0.0, 0.0)), [Action::Down]);
        assert!(step(&mut repeat, 439, down, (0.0, 0.0)).is_empty());
        assert_eq!(step(&mut repeat, 440, down, (0.0, 0.0)), [Action::Down]);

        let none = [false; Direction::COUNT];
        assert!(step(&mut repeat, 450, none, (0.0, 0.0)).is_empty());
        assert_eq!(step(&mut repeat, 451, down, (0.0, 0.0)), [Action::Down]);
    }

    #[test]
    fn a_stick_engages_and_releases_at_different_distances() {
        let mut repeat = Repeat::default();
        let none = [false; Direction::COUNT];

        assert!(step(&mut repeat, 0, none, (0.54, 0.0)).is_empty());
        assert_eq!(step(&mut repeat, 1, none, (0.56, 0.0)), [Action::Right]);

        assert!(step(&mut repeat, 20, none, (0.40, 0.0)).is_empty());

        assert!(step(&mut repeat, 21, none, (0.34, 0.0)).is_empty());
        assert_eq!(step(&mut repeat, 22, none, (0.60, 0.0)), [Action::Right]);
    }

    #[test]
    fn both_axes_can_move_on_one_poll() {
        let mut repeat = Repeat::default();
        let none = [false; Direction::COUNT];
        assert_eq!(
            step(&mut repeat, 0, none, (-0.9, 0.9)),
            [Action::Left, Action::Up]
        );
    }

    #[test]
    fn a_reset_forgets_what_was_held() {
        let mut repeat = Repeat::default();
        let none = [false; Direction::COUNT];
        assert_eq!(step(&mut repeat, 0, none, (0.8, 0.0)), [Action::Right]);
        repeat.reset();
        assert_eq!(step(&mut repeat, 1, none, (0.8, 0.0)), [Action::Right]);
    }

    #[test]
    fn the_keys_mean_what_they_say() {
        assert_eq!(Action::of_key(Key::Left), Some(Action::Left));
        assert_eq!(Action::of_key(Key::Enter), Some(Action::Accept));
        assert_eq!(Action::of_key(Key::Space), Some(Action::Accept));
        assert_eq!(Action::of_key(Key::Escape), Some(Action::Back));
        assert_eq!(Action::of_key(Key::Backspace), Some(Action::Back));
        assert_eq!(Action::of_key(Key::F10), Some(Action::Menu));

        assert_eq!(Action::of_key(Key::Menu), Some(Action::Menu));
        assert_eq!(Action::of_key(Key::Tab), Some(Action::Next));
        assert_eq!(Action::of_key(Key::BackTab), Some(Action::Previous));

        assert_eq!(Action::of_key(Key::Letter('w')), None);
        assert_eq!(Action::of_letter('w'), Some(Action::Up));
        assert_eq!(Action::of_letter('W'), Some(Action::Up));
        assert_eq!(Action::of_letter('Y'), Some(Action::Menu));
        assert_eq!(Action::of_letter('q'), None);
    }

    #[test]
    fn the_guide_button_is_never_an_applications() {
        assert_eq!(Action::of_button(Button::Guide), None);
        assert_eq!(Action::of_button(Button::West), None);
        assert_eq!(Action::of_button(Button::Select), None);
        assert_eq!(Action::of_button(Button::South), Some(Action::Accept));
        assert_eq!(Action::of_button(Button::East), Some(Action::Back));
        assert_eq!(Action::of_button(Button::North), Some(Action::Menu));
        assert_eq!(Action::of_button(Button::Start), Some(Action::Submit));
    }

    #[test]
    fn only_directions_repeat() {
        for action in Action::ALL {
            assert_eq!(action.repeats(), action.direction().is_some(), "{action:?}");
        }
        assert!(!Action::Accept.repeats());
    }

    #[test]
    fn every_action_has_its_own_name_and_finds_its_way_back() {
        let mut names: Vec<&str> = Action::ALL.iter().map(|a| a.name()).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
        for action in Action::ALL {
            assert_eq!(Action::named(action.name()), Some(action));
        }

        assert_eq!(Action::named("guide"), None);
        assert_eq!(Action::named("launch"), None);
        assert_eq!(Action::named("next-screen"), None);
    }

    #[test]
    fn a_direction_walks_a_list_the_same_way_up_as_left() {
        assert_eq!(Direction::Up.step(), -1);
        assert_eq!(Direction::Left.step(), -1);
        assert_eq!(Direction::Down.step(), 1);
        assert_eq!(Direction::Right.step(), 1);
        for direction in Direction::ALL {
            assert_eq!(direction.action().direction(), Some(direction));
        }
    }

    #[test]
    fn a_part_turned_wheel_is_carried_rather_than_lost() {
        let mut wheel = Wheel::default();
        assert_eq!(wheel.notches(0.34), 0);
        assert_eq!(wheel.notches(0.34), 0);
        assert_eq!(wheel.notches(0.34), 1);
        assert_eq!(wheel.notches(-0.5), 0);
        assert_eq!(wheel.notches(-0.6), -1);

        let mut wheel = Wheel::default();
        assert_eq!(wheel.notches(1.0), 1);
        assert_eq!(wheel.notches(-3.0), -3);

        let mut wheel = Wheel::default();
        assert_eq!(wheel.distance(SCROLL_STEP), 1);
        assert_eq!(wheel.distance(SCROLL_STEP / 2.0), 0);
        assert_eq!(wheel.distance(SCROLL_STEP / 2.0), 1);

        let mut wheel = Wheel::default();
        assert_eq!(wheel.notches(0.9), 0);
        wheel.reset();
        assert_eq!(wheel.notches(0.5), 0);
    }

    const _: () = assert!(REPEAT_INTERVAL.as_millis() > 60);
    const _: () = assert!(STICK_RELEASE < STICK_ENGAGE);
}
