use std::{path::PathBuf, sync::Arc};

pub use lxb_input;
pub use lxb_portal as portal;
pub use lxb_portal::{Awake, Hold};
pub use lxb_render;
pub use lxb_sound;
pub use lxb_toolkit;

use lxb_input::Controls;
pub use lxb_render::{monotonic_now_ns, Fit, WallpaperClock};
use lxb_render::{
    Align, Cadence, ContextMenu, Dialog, Entry, Files, Mark, Next, Press, Pressing, Selection,
    Spot, Ui,
};
use lxb_sound::{Level, Sounds};
use lxb_toolkit::{
    accent::Accent,
    input::{Action, Key, Wheel, SCROLL_STEP, TAP_SLOP},
    material::Overlay,
    metrics::Metric,
    palette::Role,
    settings::ShellTheme,
    sound::Sound,
    typography::Text,
};

pub use lxb_toolkit::picker::{Purpose as PickerPurpose, Selection as PickerSelection};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::wayland::{ActiveEventLoopExtWayland, WindowAttributesExtWayland},
    window::{CursorIcon, Window, WindowId},
};

const SURFACE: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Typed {
    Wrote(String),
    Rubbed,
}

/// A wheel or touchpad gesture that has already been put into a page's
/// actions, said again here with the one thing an action cannot carry: where
/// the pointer was.
///
/// The directions themselves arrive through [`Page::actions`] like any other,
/// so a page that ignores this still scrolls. Read it to answer the question
/// only a pointer raises — *which* of the page's lists the user meant.
///
/// Positive `steps` move down and negative ones move up; `from` is where the
/// first of them sits in this frame's actions, so the two can be walked
/// together in the order they really happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scroll {
    pub spot: Spot,
    pub steps: i32,
    pub from: usize,
}

impl Scroll {
    /// Whether the action at this place in the frame came from this gesture.
    pub fn covers(&self, index: usize) -> bool {
        index >= self.from && index < self.from + self.steps.unsigned_abs() as usize
    }
}

/// The one finger a page follows, and what it has done so far.
///
/// A touchscreen is neither a mouse nor a wheel, but what it is *for* here is
/// the same as a wheel: a list moved by hand. So a drag is turned into the
/// directions a wheel sends — the toolkit's rule that **a gesture is
/// directions like any other control** — and every page that already scrolls
/// scrolls under a finger without knowing that a finger exists. A finger that
/// stayed where it was put is a tap instead, and presses what it is on.
///
/// One finger, the first one down, and every other is ignored until it lifts.
/// Nothing here is a pinch or a two-finger anything, and a second finger
/// arriving in the middle of a drag would only fight the first.
#[derive(Debug, Clone, Copy)]
struct Finger {
    id: u64,
    /// Where it went down, in physical pixels. What says whether it has
    /// wandered far enough to have stopped being a tap.
    from: [f32; 2],
    /// How far down it was when its travel was last paid out in steps.
    last: f32,
    /// What it went down on. **Every step of the drag belongs to that**, not
    /// to whatever it has since slid over: a list moving under a finger takes
    /// its rows out from under it, and a gesture that changed lists halfway
    /// would be one nobody could aim.
    spot: Spot,
    /// Whether it has travelled far enough to be a drag, in which case
    /// letting go of it presses nothing.
    dragged: bool,
}

pub struct App {
    app_id: String,
    title: String,
    size: (f64, f64),

    page: bool,

    driven: bool,
    own_questions: bool,
    low_end: bool,

    marks: Vec<(String, Mark)>,
}

impl App {
    pub fn new(app_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            title: title.into(),
            size: (1280.0, 800.0),
            page: true,
            driven: false,
            own_questions: false,
            low_end: false,
            marks: Vec::new(),
        }
    }

    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.size = (width, height);
        self
    }

    pub fn plain(mut self) -> Self {
        self.page = false;
        self
    }

    pub fn driven(mut self) -> Self {
        self.driven = true;
        self
    }

    pub fn own_file_questions(mut self) -> Self {
        self.own_questions = true;
        self
    }

    /// Count the session as low-end hardware whatever the shell's Settings
    /// say, as an application's own "low-end" switch would: the window is
    /// drawn as it is there — the plain materials, the wallpaper held at one
    /// instant, the frames paced, every light standing on what is chosen at
    /// once — and [`Page::low_end`] answers true.
    ///
    /// A picture of it ([`App::shot`], [`App::shot_after`]) is drawn so too:
    /// every frame at the one instant it is given, so nothing timed by the
    /// frame moves, while a menu, a question and a press still move on a
    /// sixtieth of a second between frames, as they do in the window there.
    pub fn low_end(mut self) -> Self {
        self.low_end = true;
        self
    }

    /// A mark of the application's own: an SVG drawing of a shape, white
    /// where the mark is and cut through where it is open, saying
    /// `lxb:shape` — the way every mark of the toolkit is drawn. From then on
    /// it is drawn by `name` wherever a mark of the toolkit's is
    /// ([`Page::icon`], [`Page::head`], `Ui::icon`), in the same material:
    /// the bead's wall and sheen, the ring round every opening and the
    /// shadow, lit by the same lamp, so it stands beside them as one of the
    /// set.
    ///
    /// Measured here, so a drawing that is refused is refused at once: one
    /// that does not say it is a shape, does not parse or has nothing in it,
    /// and a name the toolkit already draws a mark by. Room is kept for
    /// [`lxb_render::OWN_MARKS`] of them; the same name given again takes the
    /// new drawing.
    pub fn mark(mut self, name: impl Into<String>, svg: &[u8]) -> Result<Self, String> {
        self.add_mark(name, svg)?;
        Ok(self)
    }

    /// [`App::mark`], for an application kept in a variable rather than
    /// built in one go — and kept whole when a drawing is refused.
    pub fn add_mark(&mut self, name: impl Into<String>, svg: &[u8]) -> Result<(), String> {
        let name = name.into();
        if name.is_empty() {
            return Err("a mark needs a name".into());
        }
        if lxb_toolkit::assets::glyph(&name).is_some() {
            return Err(format!("{name} is one of the toolkit's own marks"));
        }
        let mark = Mark::measure(svg).map_err(|err| format!("{name}: {err}"))?;
        if let Some((_, kept)) = self.marks.iter_mut().find(|(other, _)| *other == name) {
            *kept = mark;
        } else if self.marks.len() < lxb_render::OWN_MARKS {
            self.marks.push((name, mark));
        } else {
            return Err(format!(
                "there is room for {} marks of an application's own",
                lxb_render::OWN_MARKS
            ));
        }
        Ok(())
    }

    fn add_marks(&self, ui: &mut Ui) -> Result<(), String> {
        for (name, mark) in &self.marks {
            ui.add_mark(name, mark)?;
        }
        Ok(())
    }

    pub fn shot(
        self,
        path: impl AsRef<std::path::Path>,
        width: u32,
        height: u32,
        seconds: f32,
        page: impl FnMut(&mut Page),
    ) -> Result<(), String> {
        self.picture(path.as_ref(), width, height, seconds, None, page)
    }

    /// A picture of the window `frames` frames after its first, each drawn a
    /// sixtieth of a second after the one before, with whatever is on its
    /// way somewhere — a menu unfolding, a question coming up, the page's own
    /// motion — caught where it has got to.
    ///
    /// Where [`App::shot`] is a check of a page at rest (its menus are put
    /// where they are going before the frame that is kept), this is one of a
    /// page in motion: the page is handed one frame after another, as in a
    /// window drawn sixty times a second, `Page::seconds` moving on by a
    /// sixtieth each time, and the menu, the question and the press moving on
    /// by as much between them. The last frame is the one written. A page
    /// that counts the frames it is handed can do something on one of them —
    /// open a menu, turn a page — and be photographed so long after it.
    pub fn shot_after(
        self,
        path: impl AsRef<std::path::Path>,
        width: u32,
        height: u32,
        seconds: f32,
        frames: u32,
        page: impl FnMut(&mut Page),
    ) -> Result<(), String> {
        self.picture(path.as_ref(), width, height, seconds, Some(frames), page)
    }

    fn picture(
        self,
        path: &std::path::Path,
        width: u32,
        height: u32,
        seconds: f32,
        moving: Option<u32>,
        mut page: impl FnMut(&mut Page),
    ) -> Result<(), String> {
        const FRAME: f32 = 1.0 / 60.0;
        let mut ui = Ui::headless(width, height)?;
        self.add_marks(&mut ui)?;
        ui.set_low_end(self.low_end);
        let theme = ShellTheme::load().drawn(self.low_end);
        let accent = Accent::new(theme.accent.name).unwrap_or_else(Accent::default_accent);
        let mut state = Interaction {
            driven: self.driven,
            low_end: self.low_end,
            // Asked here as well as in the window's own constructor, because a
            // shot is a picture of the window: a legend that was in one and not
            // the other would make the two disagree about what the application
            // looks like.
            hints: Some(says_what_the_buttons_do()),
            ..Interaction::default()
        };
        state.files.own_questions();
        let mut sounds = Sounds::silent();
        let mut pixels = Vec::new();
        // A settled picture is two frames at one instant, everything raised
        // on the first put where it is going before the second; a moving one
        // is as many frames as it asks for after the first, a sixtieth of a
        // second apart.
        let (count, between) = match moving {
            None => (2, 60),
            Some(frames) => (frames.saturating_add(1), 1),
        };
        for frame in 0..count {
            if frame > 0 {
                for _ in 0..between {
                    state.menu.advance(FRAME);
                    state.dialog.advance(FRAME);
                    state.files.advance(FRAME);
                    state.pressing.advance(FRAME);
                }
            }
            let at = match moving {
                Some(_) if !self.low_end => seconds + frame as f32 * FRAME,
                _ => seconds,
            };
            let rect = start(
                &mut ui,
                &self,
                &accent,
                &theme,
                width as f32,
                height as f32,
                at,
            );
            let mut frame = Page {
                ui: &mut ui,
                state: &mut state,
                sounds: &mut sounds,
                icons: theme.icons,
                rect,
                top: rect[1],
                controls: 0,
                lit: false,
            };
            let drawn = {
                page(&mut frame);
                frame.controls
            };
            settle(&mut state, drawn);
            ui.context_menu(&mut state.menu);
            ui.dialog(&mut state.dialog);
            state.files.draw(&mut ui);
            pixels = ui.end_to_image()?;
        }

        let file =
            std::fs::File::create(path).map_err(|err| format!("{}: {err}", path.display()))?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .and_then(|mut writer| writer.write_image_data(&pixels))
            .map_err(|err| format!("{err}"))
    }

    pub fn run(self, page: impl FnMut(&mut Page)) -> Result<(), String> {
        let theme = ShellTheme::load();
        let wallpaper = WallpaperClock::from_environment(theme.accent.name)
            .unwrap_or_else(WallpaperClock::local);

        let event_loop = EventLoop::new().map_err(|err| format!("no event loop: {err}"))?;

        event_loop.set_control_flow(ControlFlow::Poll);
        let mut runtime = Runtime::new(self, page, theme, wallpaper);
        event_loop
            .run_app(&mut runtime)
            .map_err(|err| format!("{err}"))?;
        runtime.trouble.map_or(Ok(()), Err)
    }
}

fn start(
    ui: &mut Ui,
    app: &App,
    accent: &Accent,
    theme: &ShellTheme,
    width: f32,
    height: f32,
    seconds: f32,
) -> [f32; 4] {
    ui.begin(
        width,
        height,
        seconds,
        accent,
        theme.wallpaper,
        theme.particles,
        theme.icons,
    );
    if !app.page {
        return [0.0, 0.0, width, height];
    }
    let inset = ui.m(Metric::PanelInset);
    let pad = ui.m(Metric::PanelPadding);
    let pane = [inset, inset, width - 2.0 * inset, height - 2.0 * inset];
    ui.pane(pane, Overlay::Dialog);
    [
        pane[0] + pad,
        pane[1] + pad,
        pane[2] - 2.0 * pad,
        pane[3] - 2.0 * pad,
    ]
}

/// Whether Escape closes the window, rather than reaching the page as
/// [`Action::Back`].
///
/// **Only for a page that does not steer itself.** Back is what Escape means,
/// and a driven page is one that has said it will handle every action —
/// including that one, which on any page with more than one screen is the way
/// back to the last. Closing the window from under it made going back
/// impossible: every detail page, every screen behind a press, read Back and
/// not one of them ever saw it.
///
/// A page that is not driven has the toolkit walk its controls for it and has
/// no notion of a screen to go back to, so there Escape is still the way out.
///
/// `over` is whether a dialog, a menu or the file chooser is up. Those take
/// Escape for themselves whoever is steering.
fn escape_closes(driven: bool, over: bool) -> bool {
    !driven && !over
}

/// What one action does where the window stands: taken by the file chooser,
/// a question or a menu standing over the page, handed to a driven page in
/// [`Page::actions`], or walking the flow's light. The window's keys, pads,
/// pointer and fingers come here, and so does [`Page::act`].
fn act(state: &mut Interaction, sounds: &mut Sounds, action: Action) {
    if state.files.is_open() {
        if let Some(sound) = state.files.act(action) {
            sounds.play(sound);
        }
        return;
    }
    if state.dialog.is_open() {
        let sound = match action {
            Action::Left => {
                state.dialog.step(-1);
                Some(Sound::Move)
            }
            Action::Right => {
                state.dialog.step(1);
                Some(Sound::Move)
            }
            Action::Accept | Action::Submit => {
                state.dialog.press();
                state.answered = Some(state.dialog.selected());
                state.dialog.close();
                Some(Sound::Press)
            }
            Action::Back => {
                state.dialog.close();
                Some(Sound::Back)
            }
            _ => None,
        };
        if let Some(sound) = sound {
            sounds.play(sound);
        }
        return;
    }
    if state.menu.is_open() {
        let sound = match action {
            Action::Up => {
                state.menu.step(-1);
                Some(Sound::Move)
            }
            Action::Down => {
                state.menu.step(1);
                Some(Sound::Move)
            }
            Action::Accept | Action::Submit => {
                state.menu.press();
                state.chose = Some(state.menu.selected());
                state.menu.close();
                Some(Sound::Press)
            }

            Action::Back | Action::Menu => {
                state.menu.close();
                Some(Sound::Back)
            }
            _ => None,
        };
        if let Some(sound) = sound {
            sounds.play(sound);
        }
        return;
    }

    if state.driven {
        state.arrived.push(action);
        return;
    }
    match action {
        Action::Accept | Action::Submit => {
            if state.count > 0 {
                state.pressing.press();
                state.fired = Some(state.focused);
                sounds.play(Sound::Press);
            }
        }

        _ => {
            if let Some(direction) = action.direction() {
                let back = matches!(
                    direction,
                    lxb_toolkit::input::Direction::Up | lxb_toolkit::input::Direction::Left
                );
                walk(state, sounds, if back { -1 } else { 1 });
            }
        }
    }
}

fn walk(state: &mut Interaction, sounds: &mut Sounds, delta: isize) {
    if state.count == 0 {
        return;
    }
    let last = state.count - 1;
    let want = (state.focused as isize + delta).clamp(0, last as isize) as usize;
    if want != state.focused {
        state.focused = want;
        sounds.play(Sound::Move);
    }
}

fn settle(state: &mut Interaction, drawn: usize) {
    if let Some(answer) = state.files.answered() {
        state.picked_files = answer;
    }
    state.count = drawn;
    if drawn > 0 {
        state.focused = state.focused.min(drawn - 1);
    }

    state.fired = None;
    state.pointer_moved = false;
    state.hovered = None;

    state.arrived.clear();
    state.scrolled.clear();
    state.typed.clear();
    // What the page asked for becomes what is in force until it is asked
    // again. See `Interaction::taking_text`.
    state.taking_text = state.asked_text;
    state.asked_text = false;
    state.taking_keys = state.asked_keys;
    state.asked_keys = false;
    state.bars = std::mem::take(&mut state.asked_bars);
}

#[derive(Default)]
struct Interaction {
    awake: lxb_portal::Awake,
    again_within: Option<std::time::Duration>,

    focused: usize,

    count: usize,

    flow_light: Selection,

    light: Selection,

    pressing: Pressing,

    fired: Option<usize>,
    pointer: [f32; 2],
    /// Whether the pointer is over the window and is what the user last
    /// reached for. A key or a pad takes this away until the pointer moves
    /// again, so a light the hands moved is not pulled back under a pointer
    /// that is only resting. See [`Page::pointer`].
    hovering: bool,
    /// Whether the pointer moved since the last frame, which is the only time
    /// hovering may move a light.
    pointer_moved: bool,
    /// The control it moved onto, read off the frame it was moving over.
    hovered: Option<u32>,

    focus_rect: [f32; 4],

    hand: bool,
    /// The bar a press still held down took hold of, the shell's `held_bar`.
    ///
    /// The one gesture a press and a release cannot describe between them: a
    /// bar taken hold of and moved. See [`Interaction::take_hold`].
    held: Option<u32>,
    /// The controls a press takes hold of: those the page followed with
    /// [`Page::dragging`] on the frame last drawn, which is the frame a press
    /// lands on. What a bar is, to an application.
    bars: Vec<u32>,
    /// What the page asked [`Page::dragging`] about on the frame being drawn
    /// now, which becomes [`Interaction::bars`] when that frame ends.
    asked_bars: Vec<u32>,
    menu: ContextMenu,
    dialog: Dialog,
    files: Files,

    chose: Option<usize>,
    answered: Option<usize>,
    picked_files: Vec<PathBuf>,

    leaving: bool,

    arrived: Vec<Action>,
    scrolled: Vec<Scroll>,

    typed: Vec<Typed>,
    /// Whether text is being taken **now**, between one frame and the next,
    /// which is when a key actually arrives.
    ///
    /// Two flags rather than one. A page says what it wants while it is being
    /// drawn, and the frame ends the moment it returns; a single flag cleared
    /// at the end of the frame was therefore false at every instant a key
    /// could have been pressed, and nothing typed ever reached a page.
    taking_text: bool,
    /// What the page asked for on the frame being drawn now, which becomes
    /// what is in force when that frame ends.
    asked_text: bool,
    /// Whether a keyboard's typing is taken as text **now** with nothing said
    /// to the compositor (`Page::taking_keys`), and what the page asked for
    /// on the frame being drawn: two flags, as `taking_text` is.
    taking_keys: bool,
    asked_keys: bool,
    /// Where on the page the field being typed into is, in points.
    ///
    /// Told to the compositor along with the fact that there *is* one, so an
    /// on-screen keyboard has somewhere to stand clear of.
    text_at: [f32; 4],

    /// Whether a pad is what the user last reached for, rather than a keyboard
    /// or the pointer.
    ///
    /// Watched rather than chosen, the way the shell watches it: an
    /// application that says what its buttons do has to draw a picture of the
    /// thing the hands are actually on. Seeded from what the shell last wrote
    /// down, and from whether there is a pad at all where it has written
    /// nothing.
    pad_in_hand: bool,

    /// Whether this application says what its buttons do at all.
    ///
    /// The shell's own switch, read out of the file it writes it to — see
    /// `lxb_toolkit::settings::button_hints`. Written unless it says otherwise,
    /// which covers a machine that has never had this shell on it as much as a
    /// settings file older than the key.
    ///
    /// Read once, as the accent and the icon style are: it is a preference
    /// somebody sets on a settings page rather than something that moves under
    /// the hands, unlike [`Interaction::pad_in_hand`] above it. An application
    /// left running across a change to it says what it said when it started,
    /// and the next one started says the new thing.
    ///
    /// `None` until it has been asked, and read as *written*, which is what
    /// keeps the derived default honest: a page drawn without ever consulting a
    /// settings file is a page with its legends on, exactly as it was before
    /// there was a setting at all.
    hints: Option<bool>,

    /// Whether the toolkit counts this session as low-end hardware, as the
    /// window decided when it opened. See [`Page::low_end`].
    low_end: bool,

    /// What the window knows of LineXinBar's on-screen keyboard standing over
    /// it. See [`Page::shells_board`].
    board: ShellsBoard,

    /// Whether Shift is held, as the keyboard's modifiers were last reported:
    /// what turns Tab back. See [`Interaction::key_of`].
    shift: bool,

    driven: bool,
}

/// What a window knows of LineXinBar's on-screen keyboard standing over it,
/// by the shell's own rules for it.
///
/// The shell raises its board over a field of an application the moment the
/// field says it takes text — Wayland's text input enabled — while a
/// controller is in hand, once a field, however the field was chosen: a pad,
/// a click, a finger. It draws the board along the foot of the display and
/// says nothing back about where it went or that it went away. What a window
/// is sent as the board goes is the board letting go of the keyboard: a
/// report of the keyboard's modifiers saying nothing is held, standing alone,
/// whichever way it was put away — its own hide key, B, Start after the Enter
/// it types, a key of the person's own keyboard after that key is passed on.
/// The same report heads the board's first key after it rises, with the
/// board's keymap, so one a key follows in the same pass is that key's; one
/// after a report that held something is a modifier coming up; and one after
/// the keyboard arrives at the window is the seat saying what is held. While
/// the board is up the shell keeps every pad from applications, so a pad's
/// press reaching the window means it is not up.
#[derive(Debug, Default)]
struct ShellsBoard {
    /// Whether the window is under LineXinBar at all: decided as it opens.
    watched: bool,
    up: bool,
    /// Whether the last report of the keyboard's modifiers held anything.
    held: bool,
    /// A report saying nothing is held after one that held nothing either,
    /// no key after it yet in this pass of events: the board letting go,
    /// unless a key follows.
    letting_go: bool,
    /// The keyboard arrived at the window in this pass: the report that comes
    /// with it is the seat's.
    arriving: bool,
}

impl ShellsBoard {
    fn watching(watched: bool) -> Self {
        Self {
            watched,
            ..Self::default()
        }
    }

    /// The window's text input enabled, which the shell hears as a field of
    /// it taking the cursor: the board rises where it has a controller in
    /// hand, as it last wrote down.
    fn enabled(&mut self, controller_in_hand: bool) {
        self.up = self.watched && controller_in_hand;
        self.letting_go = false;
    }

    /// The window's text input disabled, or left with the keyboard: the
    /// shell's field is gone, and the board with it.
    fn disabled(&mut self) {
        self.up = false;
        self.letting_go = false;
    }

    fn focused(&mut self, focused: bool) {
        if focused {
            self.arriving = true;
        } else {
            self.disabled();
        }
    }

    fn modifiers(&mut self, held: bool) {
        let was = std::mem::replace(&mut self.held, held);
        if std::mem::take(&mut self.arriving) {
            return;
        }
        if self.up && !held && !was {
            self.letting_go = true;
        }
    }

    fn key(&mut self) {
        self.letting_go = false;
    }

    fn pad(&mut self) {
        self.disabled();
    }

    /// A pass of events over: a report nothing followed was the board letting
    /// go of the keyboard.
    fn settle(&mut self) {
        if std::mem::take(&mut self.letting_go) {
            self.up = false;
        }
        self.arriving = false;
    }

    fn up(&self) -> bool {
        self.up
    }
}

/// Whether a window is under LineXinBar, whose on-screen keyboard is watched
/// for: `desktop` (`XDG_CURRENT_DESKTOP`, a `:`-separated list the session and
/// its compositor set) names it, compared without regard to case as the shell
/// reads it, and the window is a Wayland client — the board answers Wayland's
/// text input only.
fn under_linexinbar(desktop: Option<&str>, wayland: bool) -> bool {
    wayland
        && desktop.is_some_and(|desktops| {
            desktops
                .split(':')
                .any(|name| name.trim().eq_ignore_ascii_case("LineXinBar"))
        })
}

impl Interaction {
    /// The pointer arriving at `at` over `spot`, which is read off the frame
    /// it was moving over. Answers whether it moved.
    ///
    /// **Hovering is selecting**, as it is in the shell, for whatever stands
    /// still: the flow's own controls take the light the moment the pointer
    /// moves onto them, silently, and a driven page is told what it moved
    /// onto and decides for itself. A press holding a bar is a drag, not a
    /// hover, and moving the light under it would take the selection out from
    /// under the handle it is carrying; a press on anything else holds
    /// nothing, and the pointer goes on hovering with the button down.
    fn pointer_to(&mut self, at: [f32; 2], spot: Spot) -> bool {
        let moved = at != self.pointer;
        self.pointer = at;
        if !moved || self.held.is_some() {
            return false;
        }
        self.hovering = true;
        self.pointer_moved = true;
        self.hovered = match spot {
            Spot::Control(id) => Some(id),
            _ => None,
        };
        if let Some(index) = self.hovered.map(|id| id as usize) {
            if !self.driven && index < self.count {
                self.focused = index;
            }
        }
        true
    }

    /// The left button going down over `spot`: what it takes hold of, if
    /// anything.
    ///
    /// As in the shell, only a bar is held — a control the page follows with
    /// [`Page::dragging`] — and every motion until the button comes up is that
    /// press asking for another value. A press on anything else is over the
    /// moment it lands.
    fn take_hold(&mut self, spot: Spot) {
        self.held = match spot {
            Spot::Control(id) if self.bars.contains(&id) => Some(id),
            _ => None,
        };
    }

    /// A button coming up. Letting go is the end of the press, and the only
    /// thing it is the end of: what a bar was left at was set on the way
    /// there. Only the left button takes hold, so only it lets go.
    fn let_go(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.held = None;
        }
    }

    /// The pointer leaving the window. Nothing is hovered until it moves
    /// again, and on Wayland a bar's hold ends with it, as a leave ends one in
    /// the shell: a button held there keeps the pointer on the window wherever
    /// it travels, so a leave is the hold taken away, and the release goes to
    /// whatever has the pointer now. X tells a window it was left in the
    /// middle of a drag and goes on sending it the motion and the release, so
    /// there a leave is only the pointer outside, and the drag goes on.
    fn pointer_left(&mut self, on_wayland: bool) {
        self.pointer_away();
        if on_wayland {
            self.held = None;
        }
    }

    /// The pointer gone from the window, or a key, a pad or a finger reached
    /// for instead: nothing is hovered until the pointer moves again.
    fn pointer_away(&mut self) {
        self.hovering = false;
        self.pointer_moved = false;
        self.hovered = None;
    }

    /// Whether a menu, a dialog or the file chooser stands over the page,
    /// which is then not what the pointer is over.
    fn covered(&self) -> bool {
        self.menu.is_open() || self.dialog.is_open() || self.files.busy()
    }

    /// Whether a key that writes something arrives as text now: a field
    /// takes text (`Page::taking_text`), or takes a keyboard's typing with
    /// nothing said to the compositor (`Page::taking_keys`).
    fn takes_typing(&self) -> bool {
        self.taking_text || self.taking_keys
    }

    /// The keyboard's modifiers, as the window was last told of them: Shift
    /// is kept for the keys that follow, and LineXinBar's board reads the
    /// same report as it always has (`ShellsBoard::modifiers`).
    fn modifiers_reported(&mut self, held: winit::keyboard::ModifiersState) {
        self.shift = held.shift_key();
        self.board.modifiers(!held.is_empty());
    }

    /// The key a keyboard's press names, read with Shift as it was last
    /// reported held: Shift+Tab is BackTab, which walks back, as a left
    /// bumper does. No other key reads differently with Shift
    /// (`lxb_input::key_of_logical`), and what a key types is the event's
    /// own text, never this.
    fn key_of(&self, logical: &winit::keyboard::Key) -> Option<Key> {
        lxb_input::key_of_logical(logical, self.shift)
    }
}

pub struct Page<'a> {
    ui: &'a mut Ui,
    state: &'a mut Interaction,
    sounds: &'a mut Sounds,
    icons: lxb_toolkit::settings::IconStyle,

    rect: [f32; 4],
    top: f32,

    controls: usize,

    lit: bool,
}

impl Page<'_> {
    pub fn ui(&mut self) -> &mut Ui {
        self.ui
    }

    pub fn width(&self) -> f32 {
        self.ui.width()
    }

    pub fn height(&self) -> f32 {
        self.ui.height()
    }

    pub fn seconds(&self) -> f32 {
        self.ui.seconds()
    }

    pub fn icons(&self) -> lxb_toolkit::settings::IconStyle {
        self.icons
    }

    pub fn at(&self, x: f32, y: f32) -> Spot {
        self.ui.at(x, y)
    }

    pub fn metric(&self, metric: Metric) -> f32 {
        self.ui.m(metric)
    }

    pub fn scaled(&self, reference: f32) -> f32 {
        self.ui.s(reference)
    }

    pub fn line(&self, text: Text) -> f32 {
        self.ui.line(text)
    }

    pub fn measure(&mut self, text: Text, string: &str) -> f32 {
        self.ui.measure(text, string)
    }

    pub fn cursor(&self) -> [f32; 4] {
        [
            self.rect[0],
            self.top,
            self.rect[2],
            (self.rect[1] + self.rect[3] - self.top).max(0.0),
        ]
    }

    pub fn set_cursor(&mut self, rect: [f32; 4]) {
        self.rect = rect;
        self.top = rect[1];
    }

    pub fn taking_text(&mut self, taking: bool) {
        self.state.asked_text = taking;
    }

    /// Have what a keyboard types between this frame and the next arrive as
    /// text (`typed`) — its letters written, Backspace rubbing one out —
    /// with nothing said to the compositor: no text input enabled, so no
    /// on-screen keyboard is summoned. For a field the light stands in while
    /// a controller is in hand, which LineXinBar answers a field taking text
    /// by raising its keyboard over: a keyboard reached for types into it
    /// from its first key, and nothing rises unasked. `taking_text` says
    /// both, and is what a field being typed into says.
    pub fn taking_keys(&mut self, taking: bool) {
        self.state.asked_keys = taking;
    }

    /// Where the field being typed into is, for whatever wants to stand clear
    /// of it. Said with `taking_text`, and only meant while that is true.
    pub fn text_at(&mut self, rect: [f32; 4]) {
        self.state.text_at = rect;
    }

    /// Where LineXinBar's on-screen keyboard stands over this window now —
    /// [`lxb_toolkit::board::rect`] for the window's size — or `None` where it
    /// is not up, by everything the window has been told of it.
    ///
    /// The shell raises its board over a field the moment the field takes
    /// text ([`Page::taking_text`]) with a controller in hand, however the
    /// field was chosen — a pad, a click, a finger on a handheld's screen —
    /// and never moves it out of the field's way. A field being typed into
    /// belongs above the rectangle's top for as long as this says it. It goes
    /// on the frame after the board is put away (its own hide key, B, Start,
    /// a key of the person's own keyboard), after the field stops taking
    /// text, and after a pad's press reaches the window. `None` everywhere
    /// but under LineXinBar, and wherever the shell last saw a keyboard in
    /// the person's hands, where it raises nothing by itself.
    pub fn shells_board(&self) -> Option<[f32; 4]> {
        self.state
            .board
            .up()
            .then(|| lxb_toolkit::board::rect(self.width(), self.height()))
    }

    /// Whether a pad is what the user last reached for, rather than a keyboard
    /// or the pointer.
    ///
    /// What a legend asks before it draws a picture of a button: the same act
    /// is South on a pad and Enter on a keyboard, and there is no wording that
    /// names both without naming neither. See
    /// `lxb_toolkit::settings::controller_in_hand`, which is where this starts
    /// from before anything has been touched.
    pub fn pad_in_hand(&self) -> bool {
        self.state.pad_in_hand
    }

    /// Whether the toolkit counts this session as low-end hardware: the
    /// shell's own low-end mode where Settings has said one way or the other
    /// (`low-end-mode` in its settings file), and otherwise whether the
    /// window is drawn in software.
    ///
    /// What a page that moves things itself asks — a page giving way to the
    /// next, a column stepping in — before it moves them: where this is true
    /// it puts them where they are going at once. The toolkit itself draws a
    /// still window here once a second rather than every frame, and holds the
    /// wallpaper at one instant. Decided once, as the window opens, and true
    /// whatever Settings say for an application built with [`App::low_end`].
    /// A picture (`App::shot`) answers false unless it was asked for of such
    /// an application: it is drawn as the window is drawn on hardware that is
    /// not low-end.
    pub fn low_end(&self) -> bool {
        self.state.low_end
    }

    pub fn typed(&mut self) -> std::vec::Drain<'_, Typed> {
        self.state.typed.drain(..)
    }

    pub fn actions(&mut self) -> std::vec::Drain<'_, Action> {
        self.state.arrived.drain(..)
    }

    /// Hand the window `action` as a key or a controller would: a question,
    /// a menu or the file chooser standing over the page takes it, a driven
    /// page finds it among [`Page::actions`] still to be read, and otherwise
    /// it walks or presses the flow's light.
    ///
    /// What a picture of something being answered presses with
    /// ([`App::shot_after`]): the window answers its own question and menus,
    /// and nothing else can reach them. A page asks it before it reads its
    /// actions.
    pub fn act(&mut self, action: Action) {
        act(self.state, self.sounds, action);
    }

    /// The wheel and touchpad gestures behind some of this frame's actions.
    ///
    /// Every gesture is already in [`Page::actions`]; this says which of them
    /// came from a pointer and what it was over. See [`Scroll`].
    pub fn scrolls(&mut self) -> std::vec::Drain<'_, Scroll> {
        self.state.scrolled.drain(..)
    }

    /// How far a wheel or touchpad moved over one of this page's own spots
    /// this frame. Positive is down.
    ///
    /// The question [`Page::scrolls`] answers, asked the way a page asks about
    /// a press — which is the shape the C and Python bindings can carry. It
    /// takes nothing away: the directions are in [`Page::actions`] either way,
    /// so a page reads this to decide *what* they move, not whether to move.
    pub fn scrolled(&self, id: u32) -> i32 {
        self.state
            .scrolled
            .iter()
            .filter(|scroll| scroll.spot == Spot::Control(id))
            .map(|scroll| scroll.steps)
            .sum()
    }

    pub fn focus(&mut self, index: usize) {
        self.state.focused = index;
    }

    pub fn focused(&self) -> usize {
        self.state.focused
    }

    fn lay_the_light(&mut self) {
        if self.lit {
            return;
        }
        self.lit = true;

        if self.state.count == 0 {
            return;
        }
        let at = self
            .ui
            .glide(&mut self.state.flow_light, self.state.focus_rect);

        let strength = match self.state.pressing.through() {
            Some(through) => 1.0 - through * 0.35,
            None => 1.0,
        };
        self.ui.selection(at, strength);
    }

    pub fn glide(&mut self, rect: [f32; 4], strength: f32) -> [f32; 4] {
        let at = self.ui.glide(&mut self.state.light, rect);
        self.ui.selection(at, strength);
        at
    }

    /// The light carried to `rect` as [`Page::glide`] carries it — or put
    /// there at once, `appears`, as [`Page::place`] puts it — and drawn in
    /// that rectangle's own shape, its corners rounded by `radius` rather
    /// than a capsule's: a card or a picture is lit as itself. It is the one
    /// light the capsule is, so a page that lights its rows with one and its
    /// cards with the other has it travel between them. Answers where it is
    /// drawn this frame.
    pub fn light_shaped(
        &mut self,
        rect: [f32; 4],
        radius: f32,
        strength: f32,
        appears: bool,
    ) -> [f32; 4] {
        if appears {
            self.state.light.clear();
        }
        let at = self.ui.glide(&mut self.state.light, rect);
        self.ui
            .lit(at, radius, lxb_toolkit::control::LIT_ROLE, at[2], strength);
        at
    }

    pub fn place(&mut self, rect: [f32; 4], strength: f32) -> [f32; 4] {
        self.state.light.clear();
        self.glide(rect, strength)
    }

    pub fn press(&self, lit: bool) -> Press {
        self.state.pressing.state(lit)
    }

    /// Where the pointer is while a press that took hold of this control is
    /// still held down, in the page's own space — `None` when nothing is being
    /// dragged from there.
    ///
    /// Asking is what makes `id` a bar. A press on a control the page asked
    /// about on the frame it landed on takes hold of it, and every motion
    /// until the button comes up is that drag, not a hover. A press on any
    /// other control holds nothing, and the pointer goes on hovering with the
    /// button down, as it does in the shell, where only a bar is held. So ask
    /// on every frame the bar is drawn.
    ///
    /// Only a pointer drags. A finger on the same control drags the list
    /// instead, which is what a finger does everywhere else on the page and
    /// what somebody holding a touchscreen expects of it; see `Finger`.
    pub fn dragging(&mut self, id: u32) -> Option<[f32; 2]> {
        if !self.state.asked_bars.contains(&id) {
            self.state.asked_bars.push(id);
        }
        (self.state.held == Some(id)).then_some(self.state.pointer)
    }

    /// Where the pointer rests over the page, in the page's own space.
    ///
    /// `None` once it has left the window, while a menu, a dialog or the file
    /// chooser stands over the page, from the moment a key, a pad or a finger
    /// is reached for until the pointer moves again, and while a press that
    /// took hold of a bar is held down: that is a drag, which
    /// [`Page::dragging`] follows, and not a hover. A press held on any other
    /// control leaves it answering.
    pub fn pointer(&self) -> Option<[f32; 2]> {
        (self.state.hovering && self.state.held.is_none() && !self.state.covered())
            .then_some(self.state.pointer)
    }

    /// Whether the pointer moved over the page since the last frame.
    ///
    /// The only time hovering may move a light: one moved by the pad stays
    /// where the pad put it under a pointer that is resting.
    pub fn pointer_moved(&self) -> bool {
        self.state.pointer_moved && self.pointer().is_some()
    }

    /// The control the pointer moved onto since the last frame, by the number
    /// it was drawn with — `None` on a frame it did not move, and over
    /// nothing of the page's own.
    ///
    /// Read off the frame the pointer was moving over, so it can be asked
    /// before anything is drawn. A page lights what stands still — a row, a
    /// button, a card — with it, and leaves alone what moves under the
    /// pointer, such as a list that scrolls to its selection, where a first
    /// click carries the light instead.
    pub fn hovered(&self) -> Option<u32> {
        self.state.hovered.filter(|_| self.pointer_moved())
    }

    pub fn pressed(&mut self, id: u32) -> bool {
        if self.state.fired == Some(id as usize) {
            self.state.fired = None;
            true
        } else {
            false
        }
    }

    pub fn quit(&mut self) {
        self.state.leaving = true;
    }

    pub fn keep_awake(&mut self, hold: lxb_portal::Hold) {
        self.state.awake.hold(hold);
    }

    pub fn redraw_within(&mut self, wait: std::time::Duration) {
        self.state.again_within = Some(self.state.again_within.map_or(wait, |had| had.min(wait)));
    }

    pub fn play(&mut self, sound: Sound) {
        self.sounds.play(sound);
    }

    pub fn set_volume(&mut self, value: f32, muted: bool) {
        self.sounds.set_level(Level { value, muted });
    }

    pub fn title(&mut self, text: &str) {
        self.run_of(Text::Display, text, Role::Text);
    }

    pub fn heading(&mut self, text: &str) {
        self.run_of(Text::Title, text, Role::Text);
    }

    pub fn text(&mut self, text: &str) {
        let width = self.rect[2];
        let used = self
            .ui
            .paragraph([self.rect[0], self.top, width, 0.0], text, Role::Text);
        self.top += used + self.gap_size();
    }

    pub fn note(&mut self, text: &str) {
        let width = self.rect[2];
        let used = self
            .ui
            .paragraph([self.rect[0], self.top, width, 0.0], text, Role::TextSoft);
        self.top += used + self.gap_size();
    }

    pub fn icon(&mut self, name: &str) {
        let size = self.ui.m(Metric::ItemIcon);
        self.ui
            .icon([self.rect[0], self.top, size, size], name, self.icons);
        self.top += size + self.gap_size();
    }

    pub fn picture(&mut self, path: impl AsRef<std::path::Path>, height: f32) -> bool {
        let path = path.as_ref();
        let width = self.rect[2];
        let drawn = self.ui.picture(
            [self.rect[0], self.top, width, height],
            0.0,
            path,
            Fit::Contain,
            1.0,
        );
        self.top += height + self.gap_size();
        drawn
    }

    pub fn head(&mut self, icon: &str, text: &str) {
        let mark = self.ui.m(Metric::ItemIcon);
        let gap = self.ui.m(Metric::Gap);
        let line = self.ui.line(Text::Display).max(mark);
        self.ui.icon(
            [self.rect[0], self.top + (line - mark) * 0.5, mark, mark],
            icon,
            self.icons,
        );
        self.ui.label(
            [self.rect[0] + mark + gap, self.top, self.rect[2], line],
            Text::Display,
            text,
            Role::Text,
            Align::Left,
        );
        self.top += line + self.gap_size();
    }

    pub fn rule(&mut self) {
        let thick = self.ui.s(1.0).max(1.0);
        let gap = self.gap_size();
        self.top += gap * 0.5;
        self.ui.rule(
            [self.rect[0], self.top, self.rect[2], thick],
            Role::AccentSoft,
        );
        self.top += thick + gap * 0.5;
    }

    pub fn gap(&mut self) {
        self.top += self.gap_size();
    }

    pub fn button(&mut self, label: &str) -> bool {
        let room = self.ui.measure(Text::Label, label) + 2.0 * self.ui.m(Metric::RowPadding);
        let height = self.control_height();
        let rect = [self.rect[0], self.top, room, height];
        self.top += height + self.gap_size();
        self.pressable(rect, |ui, rect, press| ui.button(rect, label, press))
    }

    pub fn row(&mut self, label: &str) -> bool {
        self.row_showing(label, None)
    }

    pub fn row_value(&mut self, label: &str, value: &str) -> bool {
        self.row_showing(label, Some(value))
    }

    pub fn item(&mut self, label: &str) -> bool {
        let height = self.ui.m(Metric::RowHeight);
        let rect = [self.rect[0], self.top, self.rect[2], height];
        self.top += height;
        let pad = self.ui.m(Metric::RowPadding);
        self.pressable(rect, |ui, rect, press| {
            let lit = press.lit();
            let ink = if lit { Role::Text } else { Role::TextSoft };
            ui.label(
                [rect[0] + pad, rect[1], rect[2] - 2.0 * pad, rect[3]],
                Text::Body,
                label,
                ink,
                Align::Left,
            );
        })
    }

    fn row_showing(&mut self, label: &str, value: Option<&str>) -> bool {
        let height = self.ui.m(Metric::RowHeight);
        let rect = [self.rect[0], self.top, self.rect[2], height];
        self.top += height;
        self.pressable(rect, |ui, rect, press| ui.row(rect, label, value, press))
    }

    /// Say where the light is standing, for a page that draws its own
    /// controls.
    ///
    /// [`Page`]'s own rows and buttons record this as they are drawn, and
    /// [`Page::menu`] grows out of it. A page that lays out its own cards —
    /// which is most of what a page with more than a column of rows on it does
    /// — is invisible to that, and its menu grew out of the top-left corner of
    /// the window instead of out of the thing it was raised over.
    ///
    /// Give it the rectangle the light is really drawn in, on the frame it is
    /// drawn: the light glides, and a menu that grew from where the light was
    /// going rather than from where it is reads as arriving from nowhere.
    pub fn light_at(&mut self, rect: [f32; 4]) {
        self.state.focus_rect = rect;
    }

    pub fn menu(&mut self, title: Option<&str>, commands: &[&str]) {
        self.menu_marked(title, commands, usize::MAX);
    }

    /// The same menu, with the one command already in force wearing the
    /// language's own `chosen` mark.
    ///
    /// A menu of alternatives that does not say which one you are on is a menu
    /// somebody has to press to find out. `marked` is an index into
    /// `commands`; anything past the end marks nothing, which is what
    /// [`Page::menu`] passes for a menu of acts rather than of answers.
    ///
    /// It opens with the light on the marked row, as the shell opens a list
    /// of values: the question is what this is set to, and the light arriving
    /// on the answer is half of the reply — and Accept at once keeps it.
    pub fn menu_marked(&mut self, title: Option<&str>, commands: &[&str], marked: usize) {
        self.raise_menu(title, commands, marked, marked);
    }

    /// The same menu of acts, marking nothing, with the light on the command
    /// at `row` as it opens rather than on its first.
    ///
    /// What a menu a person comes back to does: one that opened another
    /// menu from one of its rows and is raised again once that is answered
    /// opens with the light on the row they went in by — the one just
    /// changed — as the shell's Settings come back to the row a page was
    /// opened from. A row past the end is the first.
    pub fn menu_on(&mut self, title: Option<&str>, commands: &[&str], row: usize) {
        self.raise_menu(title, commands, usize::MAX, row);
    }

    fn raise_menu(&mut self, title: Option<&str>, commands: &[&str], marked: usize, from: usize) {
        if self.state.menu.is_open() || self.state.dialog.is_open() || self.state.files.busy() {
            return;
        }
        let anchor = self.state.focus_rect;
        let entries = commands
            .iter()
            .enumerate()
            .map(|(at, name)| {
                let entry = Entry::new(*name);
                if at == marked {
                    entry.glyph("chosen")
                } else {
                    entry
                }
            })
            .collect();
        let rows = lxb_toolkit::menu::rows_of_that_fit(
            &vec![lxb_toolkit::menu::Row::command(1); commands.len()],
            title.map(|_| 1),
            self.ui.height(),
        );
        self.state.menu.set_window(rows);
        if self
            .state
            .menu
            .open_selecting(anchor, title.map(str::to_string), entries, from)
        {
            self.sounds.play(Sound::Press);
        }
    }

    pub fn chose(&mut self) -> Option<usize> {
        self.state.chose.take()
    }

    /// Ask a question, modal until it is answered.
    ///
    /// The body keeps the lines it is given: a line break is a line break on
    /// the pane and a line that fits is written as it came. Only a line too
    /// wide is broken again, at a space, and never at a no-break space, inside
    /// quotation marks or before a lone "·" or "/".
    pub fn ask(&mut self, title: &str, body: &str, answers: &[&str]) {
        if self.state.dialog.is_open() || self.state.menu.is_open() || self.state.files.busy() {
            return;
        }
        self.state.dialog.ask(
            title.to_string(),
            body.to_string(),
            answers.iter().map(|answer| answer.to_string()).collect(),
            0,
        );
        self.sounds.play(Sound::Press);
    }

    pub fn answered(&mut self) -> Option<usize> {
        self.state.answered.take()
    }

    pub fn pick(
        &mut self,
        selection: PickerSelection,
        directory: impl AsRef<std::path::Path>,
    ) -> bool {
        self.ask_for(PickerPurpose::of(selection), selection, directory, "")
    }

    pub fn pick_many(
        &mut self,
        selection: PickerSelection,
        directory: impl AsRef<std::path::Path>,
    ) -> bool {
        self.ask_for(PickerPurpose::ManyFiles, selection, directory, "")
    }

    pub fn save(&mut self, name: &str, directory: impl AsRef<std::path::Path>) -> bool {
        self.ask_for(
            PickerPurpose::ANewFile,
            PickerSelection::File,
            directory,
            name,
        )
    }

    fn ask_for(
        &mut self,
        purpose: PickerPurpose,
        selection: PickerSelection,
        directory: impl AsRef<std::path::Path>,
        name: &str,
    ) -> bool {
        if self.state.menu.is_open() || self.state.dialog.is_open() || self.state.files.busy() {
            return false;
        }
        let opened = self.state.files.ask(purpose, selection, directory, name);
        if opened {
            self.sounds.play(Sound::Press);
        }
        opened
    }

    pub fn picked(&mut self) -> Option<PathBuf> {
        self.picked_files().into_iter().next()
    }

    pub fn picked_files(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.state.picked_files)
    }

    pub fn picked_next(&mut self) -> Option<PathBuf> {
        if self.state.picked_files.is_empty() {
            return None;
        }
        Some(self.state.picked_files.remove(0))
    }

    fn gap_size(&self) -> f32 {
        self.ui.m(Metric::Gap)
    }

    fn run_of(&mut self, text: Text, string: &str, role: Role) {
        let line = self.ui.line(text);
        self.ui.label(
            [self.rect[0], self.top, self.rect[2], line],
            text,
            string,
            role,
            Align::Left,
        );
        self.top += line + self.gap_size();
    }

    fn control_height(&self) -> f32 {
        self.ui
            .s(lxb_toolkit::menu::ROW - 2.0 * lxb_toolkit::control::PADDING)
    }

    fn pressable(&mut self, rect: [f32; 4], draw: impl FnOnce(&mut Ui, [f32; 4], Press)) -> bool {
        self.lay_the_light();
        let index = self.controls;
        self.controls += 1;
        let lit = index == self.state.focused;
        if lit {
            self.state.focus_rect = rect;
        }
        self.ui.spot(index as u32, rect);
        draw(self.ui, rect, self.state.pressing.state(lit));

        if self.state.fired == Some(index) {
            self.state.fired = None;
            true
        } else {
            false
        }
    }
}

struct Runtime<F> {
    app: App,
    page: F,
    instance: wgpu::Instance,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    ui: Option<Ui>,

    theme: ShellTheme,
    accent: Accent,

    state: Interaction,
    controls: Controls,
    /// What is left of a wheel or touchpad gesture that has not travelled far
    /// enough to move one control yet. Keeping the fraction is what makes a
    /// slow two-finger scroll work instead of rounding every small event away.
    wheel: Wheel,
    /// The finger on the screen, and the same carried fraction for it. See
    /// [`Finger`].
    finger: Option<Finger>,
    touch: Wheel,
    sounds: Sounds,

    /// What the window was last told about a field being typed into. See
    /// `Runtime::draw`.
    text_allowed: bool,
    text_shown: [f32; 4],

    opened: std::time::Instant,
    last: std::time::Instant,
    wallpaper: WallpaperClock,

    low_end: bool,
    next_frame: std::time::Instant,
    next_poll: std::time::Instant,
    touched: std::time::Instant,
    cadence: Cadence,
    answer_asked: bool,

    trouble: Option<String>,
}

impl<F: FnMut(&mut Page)> Runtime<F> {
    fn new(app: App, page: F, theme: ShellTheme, wallpaper: WallpaperClock) -> Self {
        let accent = Accent::new(theme.accent.name).unwrap_or_else(Accent::default_accent);
        let controls = Controls::new();
        if let Some(trouble) = controls.trouble() {
            eprintln!("no controller input: {trouble}");
        }
        let now = std::time::Instant::now();
        let driven = app.driven;
        let own_questions = app.own_questions;
        let title = app.title.clone();
        Self {
            app,
            page,
            instance: lxb_render::instance(),
            window: None,
            surface: None,
            ui: None,
            theme,
            accent,
            state: {
                let mut state = Interaction {
                    driven,
                    // What the shell last saw the user reach for, or — where
                    // it has written nothing, because this is not a shell
                    // session — whether there is a pad plugged in at all.
                    pad_in_hand: lxb_toolkit::settings::controller_in_hand()
                        .unwrap_or(controls.pads() > 0),
                    // On unless the shell says otherwise, which is the shell's
                    // own default: a console is the one kind of machine nobody
                    // arrives at already knowing which button does what.
                    hints: Some(says_what_the_buttons_do()),
                    ..Interaction::default()
                };
                if own_questions {
                    state.files.own_questions();
                }
                state.awake.named(title.clone(), title);
                state
            },
            controls,
            wheel: Wheel::default(),
            finger: None,
            touch: Wheel::default(),
            sounds: Sounds::new(),
            text_allowed: false,
            text_shown: [0.0; 4],
            opened: now,
            last: now,
            wallpaper,
            low_end: false,
            next_frame: now,
            next_poll: now,
            touched: now,
            cadence: Cadence::default(),
            answer_asked: false,
            trouble: None,
        }
    }

    fn now(&self) -> std::time::Duration {
        self.opened.elapsed()
    }

    fn spot(&self) -> Spot {
        self.spot_at(self.state.pointer)
    }

    fn spot_at(&self, [x, y]: [f32; 2]) -> Spot {
        self.ui.as_ref().map_or(Spot::Nothing, |ui| ui.at(x, y))
    }

    /// The overlays follow whatever is pointing at them, so that what a press
    /// would reach is lit before it is pressed. A finger points as a pointer
    /// does, which is why this is not written into `CursorMoved`.
    fn point_here(&mut self, spot: Spot) {
        if self.state.files.is_open() {
            self.state.files.point_at(spot);
        } else if self.state.dialog.is_open() {
            self.state.dialog.point_at(spot);
        } else if self.state.menu.is_open() {
            self.state.menu.point_at(spot);
        }
    }

    /// A press landing somewhere, whatever pressed it. A tap is the same act
    /// as a left click and has to reach the same places, overlays included —
    /// written twice, the two would have drifted the first time either grew a
    /// case.
    fn press_here(&mut self, spot: Spot) {
        if self.state.files.is_open() {
            if let Some(sound) = self.state.files.press_at(spot, false) {
                self.sounds.play(sound);
            }
            return;
        }
        match spot {
            Spot::MenuRow { .. } | Spot::DialogButton(_) => self.act(Action::Accept),

            Spot::OutsidePanel => self.act(Action::Back),
            Spot::Control(index) => {
                let index = index as usize;
                if index < self.state.count {
                    self.state.focused = index;
                }
                if self.state.driven {
                    self.state.pressing.press();
                    self.state.fired = Some(index);
                    self.sounds.play(Sound::Press);
                } else if index < self.state.count {
                    self.act(Action::Accept);
                }
            }
            Spot::Nothing => {}
            _ => {}
        }
    }

    /// How far a finger drags to move a list by one step.
    ///
    /// A row of the design language, rather than the wheel's own
    /// [`SCROLL_STEP`]: a finger is pulling the list itself and a page's lists
    /// move a row at a time, so a drag the height of a row moves a row, and
    /// what is under the finger stays roughly under it. A wheel is a
    /// different instrument and keeps its own smaller notch — five to a row —
    /// because a wheel is turned rather than dragged.
    fn finger_step(&self) -> f32 {
        self.ui
            .as_ref()
            .map(|ui| ui.m(Metric::RowHeight))
            .unwrap_or(SCROLL_STEP)
    }

    /// A finger arriving, moving, or lifting. See [`Finger`].
    fn finger(&mut self, touch: winit::event::Touch) {
        let at = [touch.location.x as f32, touch.location.y as f32];
        match touch.phase {
            TouchPhase::Started => {
                if self.finger.is_some() {
                    return;
                }
                self.state.pad_in_hand = false;
                self.state.pointer_away();
                self.state.pointer = at;
                let spot = self.spot();
                self.point_here(spot);
                self.touch.reset();
                self.finger = Some(Finger {
                    id: touch.id,
                    from: at,
                    last: at[1],
                    spot,
                    dragged: false,
                });
            }
            TouchPhase::Moved => {
                let Some(finger) = self.finger.filter(|one| one.id == touch.id) else {
                    return;
                };
                self.state.pointer = at;
                let slop = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.s(TAP_SLOP))
                    .unwrap_or(TAP_SLOP);
                let wandered = (at[0] - finger.from[0]).hypot(at[1] - finger.from[1]) > slop;
                let step = self.finger_step();
                let steps = steps_of_drag(&mut self.touch, finger.last, at[1], step);
                if let Some(finger) = self.finger.as_mut() {
                    finger.last = at[1];
                    finger.dragged |= wandered;
                }
                if steps == 0 {
                    return;
                }
                let from = self.state.arrived.len();
                for action in actions_of_steps(steps) {
                    self.act(action);
                }
                if self.state.arrived.len() > from {
                    self.state.scrolled.push(Scroll {
                        spot: finger.spot,
                        steps,
                        from,
                    });
                }
            }
            TouchPhase::Ended => {
                let Some(finger) = self.finger.filter(|one| one.id == touch.id) else {
                    return;
                };
                self.finger = None;
                self.state.pointer = at;
                // A finger that travelled was a drag, and a drag presses
                // nothing however it ends: the list it moved is the whole of
                // what it did.
                if finger.dragged {
                    return;
                }
                let spot = self.spot();
                self.press_here(spot);
            }
            // The compositor has taken the sequence away — a gesture it
            // decided was its own. Nothing was pressed, and nothing is now.
            TouchPhase::Cancelled => self.finger = None,
        }
    }

    fn poll_pads(&mut self) -> bool {
        let actions = self.controls.poll(self.now());
        let any = !actions.is_empty();
        for action in actions {
            self.state.board.pad();
            self.state.pad_in_hand = true;
            self.state.pointer_away();
            self.act(action);
        }
        any
    }

    fn act(&mut self, action: Action) {
        act(&mut self.state, &mut self.sounds, action);
    }

    fn draw(&mut self, width: f32, height: f32, elapsed: f32) {
        // Every event of this pass has arrived before it is drawn.
        self.state.board.settle();
        let Some(ui) = self.ui.as_mut() else {
            return;
        };
        let rect = start(
            ui,
            &self.app,
            &self.accent,
            &self.theme,
            width,
            height,
            elapsed,
        );

        let mut page = Page {
            ui,
            state: &mut self.state,
            sounds: &mut self.sounds,
            icons: self.theme.icons,
            rect,
            top: rect[1],
            controls: 0,
            lit: false,
        };
        (self.page)(&mut page);
        let drawn = page.controls;
        let asked_text = self.state.asked_text;
        let text_at = self.state.text_at;
        settle(&mut self.state, drawn);

        // **Saying it to the window is what summons an on-screen keyboard.**
        // `taking_text` is this toolkit's own business and no compositor can
        // see it; `set_ime_allowed` is `zwp_text_input_v3`, which is the only
        // thing in Wayland that says a field inside a window has the cursor,
        // and it is what LineXinBar's keyboard waits for. Said only when it
        // changes: enabling a text input twice is a protocol error in some
        // compositors and a wasted round trip in the rest.
        if asked_text != self.text_allowed {
            self.text_allowed = asked_text;
            if let Some(window) = self.window.as_ref() {
                window.set_ime_allowed(asked_text);
            }
        }
        if asked_text && text_at != self.text_shown {
            self.text_shown = text_at;
            if let Some(window) = self.window.as_ref() {
                window.set_ime_cursor_area(
                    winit::dpi::LogicalPosition::new(text_at[0], text_at[1]),
                    winit::dpi::LogicalSize::new(text_at[2], text_at[3]),
                );
            }
        }

        ui.context_menu(&mut self.state.menu);
        ui.dialog(&mut self.state.dialog);
        self.state.files.hand(self.controls.pads() > 0);
        self.state
            .files
            .say_what_the_buttons_do(self.state.hints.unwrap_or(true));
        self.state.files.draw(ui);
    }
}

/// Whether this application writes what its buttons do.
///
/// The shell's own switch — Settings > System > Button hints — out of the file
/// the shell writes it to, and its own default where nothing has been written:
/// a machine that has never had this shell on it, and a settings file older
/// than the key, are both "nobody has said", and neither is somebody asking for
/// a panel with no legend on it. See `lxb_toolkit::settings::button_hints`.
fn says_what_the_buttons_do() -> bool {
    lxb_toolkit::settings::button_hints().unwrap_or(true)
}

/// What a letter means to a window where no field takes it as text: `y`,
/// the letter printed on a pad's top face button, is that button — the
/// context menu — as LineXinBar reads it beside the Menu key and F10. The
/// shell's other letter shorthands, `wasd` and `hjkl`, are left to an
/// application that asks `Action::of_letter` for them: a list walked by
/// letters would walk under a person typing the first letters of a name
/// before the field takes them.
fn letter_press(key: Key) -> Option<Action> {
    match key {
        Key::Letter(letter) => Action::of_letter(letter).filter(|action| *action == Action::Menu),
        _ => None,
    }
}

/// Whether an event wakes a low-end loop that is waiting between frames.
///
/// Everything a hand does, and the window changing size. A pointer moving, or
/// arriving or leaving, is in it because a page lights what it hovers: a
/// light that waited for the next still frame to follow the pointer would be
/// a second behind it.
fn wakes(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::CursorEntered { .. }
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::Touch(_)
            | WindowEvent::Ime(_)
            | WindowEvent::Resized(_)
    )
}

/// Normalize traditional wheel lines and touchpad pixels into signed steps.
/// [`Wheel`] carries their fractional remainder between events so both feel
/// deliberate at low speed.
fn steps_of_wheel(wheel: &mut Wheel, delta: MouseScrollDelta) -> i32 {
    const MAX_STEPS_PER_EVENT: i32 = 12;

    let steps = match delta {
        MouseScrollDelta::LineDelta(_, lines) => wheel.notches(-lines),
        MouseScrollDelta::PixelDelta(at) => wheel.distance(-at.y as f32),
    };
    steps.clamp(-MAX_STEPS_PER_EVENT, MAX_STEPS_PER_EVENT)
}

/// A finger's travel turned into the same steps a wheel sends.
///
/// **Pulled down is the list going up.** What a finger does is move the page
/// under it, so dragging down brings back what was above. Every touchscreen
/// agrees about this, and it is the opposite of what the two numbers say on
/// their own — `was` and `now` are how far *down the screen* the finger was
/// and is, and down the screen is further into nothing.
///
/// The fraction is carried the way a slow wheel's is, so a finger crossing
/// half a step twice moves one step rather than none.
fn steps_of_drag(touch: &mut Wheel, was: f32, now: f32, step: f32) -> i32 {
    touch.notches((was - now) / step)
}

fn actions_of_steps(steps: i32) -> Vec<Action> {
    let action = if steps > 0 { Action::Down } else { Action::Up };
    vec![action; steps.unsigned_abs() as usize]
}

impl<F: FnMut(&mut Page)> ApplicationHandler for Runtime<F> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.app.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.app.size.0,
                self.app.size.1,
            ))
            .with_name(&self.app.app_id, &self.app.app_id);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => return self.give_up(event_loop, format!("no window: {err}")),
        };
        self.instance = lxb_render::instance_for(window.clone());
        let surface = match self.instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(err) => return self.give_up(event_loop, format!("no surface: {err}")),
        };
        let size = window.inner_size();
        let mut ui = match pollster::block_on(Ui::new(
            &self.instance,
            Some(&surface),
            SURFACE,
            size.width,
            size.height,
        )) {
            Ok(ui) => ui,
            Err(message) => return self.give_up(event_loop, message),
        };
        if let Err(message) = self.app.add_marks(&mut ui) {
            return self.give_up(event_loop, message);
        }
        configure(&surface, &ui, size.width, size.height);
        self.low_end = self.app.low_end || self.theme.low_end_on(ui.software());
        self.state.low_end = self.low_end;
        ui.set_low_end(self.low_end);
        self.theme = self.theme.drawn(self.low_end);
        self.state.board = ShellsBoard::watching(under_linexinbar(
            std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref(),
            event_loop.is_wayland(),
        ));
        self.window = Some(window);
        self.surface = Some(surface);
        self.ui = Some(ui);
        self.opened = std::time::Instant::now();
        self.last = self.opened;
        self.wallpaper.restart();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else {
            return;
        };
        if self.low_end && wakes(&event) {
            let now = std::time::Instant::now();
            self.touched = now;
            self.next_frame = now;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Focused(false) => {
                self.state.board.focused(false);
                self.controls.release();
                self.wheel.reset();
                // Whatever was being held is not being held now: the release
                // that would have said so is going somewhere else.
                self.state.held = None;
                self.finger = None;
            }
            WindowEvent::Focused(true) => self.state.board.focused(true),
            // What LineXinBar's board hears as a field of this window taking
            // the cursor and giving it up, and what it sends as it lets go of
            // the keyboard. See `ShellsBoard`.
            WindowEvent::Ime(Ime::Enabled) => self
                .state
                .board
                .enabled(lxb_toolkit::settings::controller_in_hand().unwrap_or(true)),
            WindowEvent::Ime(Ime::Disabled) => self.state.board.disabled(),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.state.modifiers_reported(modifiers.state())
            }
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(ui)) = (self.surface.as_ref(), self.ui.as_ref()) {
                    configure(surface, ui, size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let at = [position.x as f32, position.y as f32];
                let spot = self.spot_at(at);
                self.state.pointer_to(at, spot);
                // A button still down on a bar is a value still being set,
                // wherever the pointer has since travelled: the drag is all
                // this motion is about, as it is in the shell.
                if self.state.held.is_some() {
                    return;
                }
                self.point_here(spot);

                let hand = spot.pressable();
                if hand != self.state.hand {
                    self.state.hand = hand;
                    window.set_cursor(if hand {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::Default
                    });
                }
            }
            WindowEvent::CursorLeft { .. } => {
                // A partial gesture belongs to the thing it began over, not
                // to whatever the pointer enters next.
                self.wheel.reset();
                self.state.pointer_left(event_loop.is_wayland());
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state != ElementState::Pressed {
                    self.state.let_go(button);
                    return;
                }
                self.state.pad_in_hand = false;
                self.state.hovering = true;

                if button == MouseButton::Right {
                    if self.state.files.is_open() {
                        if let Some(sound) = self.state.files.press_at(self.spot(), true) {
                            self.sounds.play(sound);
                        }
                        return;
                    }
                    if let Spot::Control(index) = self.spot() {
                        let index = index as usize;
                        if index < self.state.count {
                            self.state.focused = index;
                        }
                    }
                    self.act(Action::Menu);
                    return;
                }
                if button != MouseButton::Left {
                    return;
                }
                let spot = self.spot();
                self.state.take_hold(spot);
                self.press_here(spot);
            }
            WindowEvent::Touch(touch) => self.finger(touch),
            WindowEvent::MouseWheel { delta, .. } => {
                self.state.pad_in_hand = false;
                let steps = steps_of_wheel(&mut self.wheel, delta);
                if steps == 0 {
                    return;
                }
                // **A wheel is directions, like every other control.** It goes
                // the same way a key and a pad go — through `act`, into the
                // same queue, in the order it arrived — so a page that has
                // never heard of a wheel still scrolls, in any of the three
                // languages, and a gesture cannot overtake a press that came
                // before it.
                //
                // What a wheel has that the others do not is somewhere it was
                // pointed. That is recorded beside the directions rather than
                // instead of them, so a page with two lists can move the one
                // under the hand. `from` is where they landed in the queue,
                // which is what keeps the two accounts of one frame agreeing.
                let from = self.state.arrived.len();
                for action in actions_of_steps(steps) {
                    self.act(action);
                }
                let queued = self.state.arrived.len() - from;
                if queued > 0 {
                    self.state.scrolled.push(Scroll {
                        spot: self.spot(),
                        steps,
                        from,
                    });
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }
                let down = event.state == ElementState::Pressed;
                let key = self.state.key_of(&event.logical_key);
                if down {
                    self.state.board.key();
                    self.state.pad_in_hand = false;
                    self.state.pointer_away();
                }

                if down && self.state.files.key(key, event.text.as_deref()) {
                    return;
                }

                if down && self.state.takes_typing() {
                    if key == Some(Key::Backspace) {
                        self.state.typed.push(Typed::Rubbed);
                        return;
                    }
                    let written: String = event
                        .text
                        .as_deref()
                        .unwrap_or_default()
                        .chars()
                        .filter(|letter| !letter.is_control())
                        .collect();
                    if !written.is_empty() {
                        self.state.typed.push(Typed::Wrote(written));
                        return;
                    }
                }

                let Some(key) = key else {
                    return;
                };

                let over = self.state.dialog.is_open()
                    || self.state.menu.is_open()
                    || self.state.files.is_open();
                if key == Key::Escape && down && escape_closes(self.state.driven, over) {
                    event_loop.exit();
                    return;
                }
                let now = self.now();
                if let Some(action) = self.controls.key(key, down, now) {
                    self.act(action);
                } else if let Some(action) = letter_press(key).filter(|_| down) {
                    self.act(action);
                }
            }
            WindowEvent::RedrawRequested => {
                if !self.low_end {
                    self.poll_pads();
                }
                let now = std::time::Instant::now();
                if self.low_end {
                    if std::mem::take(&mut self.answer_asked) {
                        self.cadence.answered(now);
                    }
                    if self.moving(now) || self.cadence.moved() {
                        if let Next::At(beat) = self.cadence.next(now) {
                            self.next_frame = beat;
                            return;
                        }
                    }
                }

                let dt = (now - self.last).as_secs_f32().clamp(0.0, 0.1);
                self.last = now;
                self.accent.advance(dt);
                self.state.pressing.advance(dt);
                self.state.menu.advance(dt);
                self.state.dialog.advance(dt);
                self.state.files.advance(dt);

                let size = window.inner_size();
                let elapsed = if self.low_end {
                    lxb_toolkit::settings::STILL_WALLPAPER_AT
                } else {
                    self.wallpaper.elapsed_secs()
                };
                self.draw(size.width as f32, size.height as f32, elapsed);
                if self.state.leaving {
                    event_loop.exit();
                    return;
                }

                let (Some(surface), Some(ui)) = (self.surface.as_ref(), self.ui.as_mut()) else {
                    return;
                };
                use wgpu::CurrentSurfaceTexture as Acquired;
                match surface.get_current_texture() {
                    Acquired::Success(frame) | Acquired::Suboptimal(frame) => {
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        if let Err(message) = ui.end(&view) {
                            eprintln!("{message}");
                        }
                        if self.low_end {
                            window.pre_present_notify();
                        }
                        ui.queue.present(frame);
                    }
                    Acquired::Outdated | Acquired::Lost => {
                        configure(surface, ui, size.width, size.height)
                    }
                    _ => {}
                }
                if self.low_end {
                    let moving = self.moving(now);
                    self.cadence.drew(now, moving);
                    self.cadence.set_refresh(
                        window
                            .current_monitor()
                            .and_then(|monitor| monitor.refresh_rate_millihertz())
                            .unwrap_or(0),
                    );
                    if moving {
                        window.request_redraw();
                        self.answer_asked = true;
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.state.board.settle();
        self.sounds.rest(std::time::Instant::now());
        if !self.low_end {
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return;
        }
        let now = std::time::Instant::now();
        if now >= self.next_poll {
            if self.poll_pads() {
                self.touched = now;
                self.next_frame = now;
            }
            self.next_poll = now + LOW_END_POLL;
        }
        if self.moving(now) || self.cadence.moved() {
            match self.cadence.next(now) {
                Next::Now if !self.answer_asked => {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    self.next_frame = now + LOW_END_STILL;
                }
                Next::At(beat) => self.next_frame = beat,
                _ => self.next_frame = now + LOW_END_STILL,
            }
        } else if now >= self.next_frame {
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            let asked = self.state.again_within.take().unwrap_or(LOW_END_STILL);
            self.next_frame = now + LOW_END_STILL.min(asked.max(self.cadence.period()));
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame.min(self.next_poll)));
    }
}

const LOW_END_POLL: std::time::Duration = std::time::Duration::from_millis(16);
const LOW_END_STILL: std::time::Duration = std::time::Duration::from_secs(1);
const LOW_END_SETTLING: std::time::Duration = std::time::Duration::from_secs(1);

impl<F> Runtime<F> {
    fn moving(&self, now: std::time::Instant) -> bool {
        now.saturating_duration_since(self.touched) < LOW_END_SETTLING || self.accent.moving()
    }
}

impl<F> Runtime<F> {
    fn give_up(&mut self, event_loop: &ActiveEventLoop, trouble: String) {
        self.trouble = Some(trouble);
        event_loop.exit();
    }
}

fn configure(surface: &wgpu::Surface<'_>, ui: &Ui, width: u32, height: u32) {
    surface.configure(
        &ui.device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: SURFACE,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Headless {
        ui: Ui,
        state: Interaction,
        app: App,
        theme: ShellTheme,
        accent: Accent,
        sounds: Sounds,
    }

    impl Headless {
        fn new() -> Option<Self> {
            let ui = Ui::headless(1280, 800).ok()?;
            let theme = ShellTheme::load();
            let accent = Accent::new(theme.accent.name).unwrap_or_else(Accent::default_accent);
            Some(Self {
                ui,
                state: Interaction::default(),
                app: App::new("com.example.Test", "Test"),
                theme,
                accent,
                sounds: Sounds::silent(),
            })
        }

        fn draw(&mut self, page: impl FnOnce(&mut Page)) -> usize {
            self.draw_at(10.0, page)
        }

        fn draw_at(&mut self, seconds: f32, page: impl FnOnce(&mut Page)) -> usize {
            let rect = start(
                &mut self.ui,
                &self.app,
                &self.accent,
                &self.theme,
                1280.0,
                800.0,
                seconds,
            );
            let mut frame = Page {
                ui: &mut self.ui,
                state: &mut self.state,
                sounds: &mut self.sounds,
                icons: self.theme.icons,
                rect,
                top: rect[1],
                controls: 0,
                lit: false,
            };
            page(&mut frame);
            let drawn = frame.controls;
            settle(&mut self.state, drawn);
            drawn
        }
    }

    macro_rules! headless {
        ($what:literal) => {
            match Headless::new() {
                Some(headless) => headless,
                None => {
                    eprintln!("no adapter: {} was not checked", $what);
                    return;
                }
            }
        };
    }

    /// The one thing about a touchscreen that cannot be read off the numbers.
    #[test]
    fn a_finger_pulled_down_takes_the_list_up() {
        let mut touch = Wheel::default();
        let row = 76.0;

        // Dragged a row's height down the screen: the list comes back up.
        assert_eq!(steps_of_drag(&mut touch, 100.0, 100.0 + row, row), -1);
        assert_eq!(actions_of_steps(-1), [Action::Up]);

        // And pushed back up the screen, which is the list going on.
        assert_eq!(steps_of_drag(&mut touch, 100.0 + row, 100.0, row), 1);
        assert_eq!(actions_of_steps(1), [Action::Down]);

        // Half a row twice is one step, not none: a finger is never still and
        // its travel arrives in whatever pieces the screen reports.
        assert_eq!(steps_of_drag(&mut touch, 0.0, row * 0.5, row), 0);
        assert_eq!(steps_of_drag(&mut touch, row * 0.5, row, row), -1);
    }

    #[test]
    fn a_wheel_and_a_touchpad_send_the_same_navigation_actions() {
        let mut wheel = Wheel::default();
        assert_eq!(
            actions_of_steps(steps_of_wheel(
                &mut wheel,
                MouseScrollDelta::LineDelta(0.0, 2.0),
            )),
            [Action::Up, Action::Up]
        );
        assert_eq!(
            actions_of_steps(steps_of_wheel(
                &mut wheel,
                MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, -30.0)),
            )),
            [Action::Down, Action::Down]
        );
    }

    #[test]
    fn small_wheel_movements_are_carried_until_they_make_one_step() {
        let mut wheel = Wheel::default();
        for lines in [0.34, 0.34] {
            assert_eq!(
                steps_of_wheel(&mut wheel, MouseScrollDelta::LineDelta(0.0, lines),),
                0
            );
        }
        assert_eq!(
            steps_of_wheel(&mut wheel, MouseScrollDelta::LineDelta(0.0, 0.34)),
            -1
        );
        wheel.reset();
        assert_eq!(
            steps_of_wheel(&mut wheel, MouseScrollDelta::LineDelta(0.0, 0.5),),
            0
        );
    }

    #[test]
    fn an_extreme_wheel_event_is_bounded() {
        let mut wheel = Wheel::default();
        assert_eq!(
            steps_of_wheel(&mut wheel, MouseScrollDelta::LineDelta(0.0, -10_000.0),),
            12
        );
    }

    #[test]
    fn a_driven_page_is_told_where_a_gesture_pointed_and_where_its_actions_landed() {
        let mut headless = headless!("a driven page's scroll event");
        // A press, then a wheel over one of the page's own lists: the wheel's
        // directions go into the queue behind the press, and the gesture says
        // so.
        headless.state.arrived.push(Action::Accept);
        headless.state.arrived.extend([Action::Down; 3]);
        headless.state.scrolled.push(Scroll {
            spot: Spot::Control(0x7000),
            steps: 3,
            from: 1,
        });

        let mut actions = Vec::new();
        let mut received = Vec::new();
        let mut over = 0;
        headless.draw(|page| {
            over = page.scrolled(0x7000);
            actions.extend(page.actions());
            received.extend(page.scrolls());
        });
        assert_eq!(
            actions,
            [Action::Accept, Action::Down, Action::Down, Action::Down],
            "a gesture was taken out of the actions rather than said twice"
        );
        assert_eq!(
            received,
            [Scroll {
                spot: Spot::Control(0x7000),
                steps: 3,
                from: 1,
            }]
        );
        assert_eq!(over, 3, "the same gesture, asked for the way C asks");
        assert!(!received[0].covers(0), "the press was blamed on the wheel");
        assert!(received[0].covers(1) && received[0].covers(3));
        assert!(!received[0].covers(4));
    }

    #[test]
    fn a_gesture_over_nothing_of_the_pages_own_is_still_a_direction() {
        let mut headless = headless!("a scroll over no spot of the page's");
        headless.state.arrived.push(Action::Up);
        headless.state.scrolled.push(Scroll {
            spot: Spot::Nothing,
            steps: -1,
            from: 0,
        });

        let mut actions = Vec::new();
        let mut over = 0;
        headless.draw(|page| {
            over = page.scrolled(0x7000);
            actions.extend(page.actions());
        });
        assert_eq!(actions, [Action::Up]);
        assert_eq!(over, 0, "a gesture was credited to a list it missed");
    }

    #[test]
    fn the_phase_a_window_opens_on_is_the_phase_the_wallpaper_is_drawn_at() {
        let mut headless = headless!("the phase a window opens on");
        let accent = headless.theme.accent.name;
        let carried = match (lxb_toolkit::handoff::boot_id(), crate::monotonic_now_ns()) {
            (Ok(boot_id), Ok(now_ns)) => {
                let record = lxb_toolkit::handoff::Handoff::of(
                    boot_id,
                    now_ns,
                    42_000_000_000,
                    accent,
                    None,
                    None,
                )
                .expect("a canonical accent and this machine's boot id")
                .encode();
                WallpaperClock::from_record(&record, accent)
                    .expect("a record written a moment ago on this boot")
                    .elapsed_secs()
            }
            _ => {
                eprintln!("no clock: the phase a window opens on was not checked");
                return;
            }
        };
        assert!(carried >= 42.0, "{carried}");

        headless.draw_at(carried, |page| {
            page.focus(0);
            page.item("Hello");
        });
        assert_eq!(
            headless.ui.seconds(),
            carried,
            "the window drew its wallpaper at its own clock, not the one it was handed"
        );

        headless.draw_at(0.0, |page| {
            page.focus(0);
            page.item("Hello");
        });
        assert_eq!(headless.ui.seconds(), 0.0);
    }

    #[test]
    fn escape_reaches_a_page_that_steers_itself() {
        assert!(
            escape_closes(false, false),
            "a page the toolkit walks for has no screen to go back to, so \
             Escape is its way out"
        );
        assert!(
            !escape_closes(true, false),
            "a driven page reads Back itself, and the window closing from \
             under it is what made going back impossible"
        );
        for driven in [true, false] {
            assert!(
                !escape_closes(driven, true),
                "a dialog, a menu and the chooser take Escape for themselves"
            );
        }
    }

    #[test]
    fn a_click_is_answered_by_the_control_it_landed_on() {
        let mut headless = headless!("a click on a page that drives itself");
        headless.state.driven = true;

        headless.draw(|page| {
            page.focus(0);
            for name in ["Hello", "Colour", "Material", "Marks"] {
                page.item(name);
            }
        });

        headless.state.focused = 2;
        headless.state.fired = Some(2);

        let mut pressed = Vec::new();
        headless.draw(|page| {
            page.focus(0);
            for (index, name) in ["Hello", "Colour", "Material", "Marks"]
                .into_iter()
                .enumerate()
            {
                if page.item(name) {
                    pressed.push(index);
                }
            }
        });

        assert_eq!(
            pressed,
            vec![2],
            "the row the pointer landed on is the row that answers, \
             however the page has moved its light since"
        );
    }

    /// Where the middle of control `index` was drawn on the last frame.
    fn middle_of(ui: &Ui, index: u32) -> [f32; 2] {
        let mut y = 0.0;
        while y < 800.0 {
            let mut x = 0.0;
            while x < 1280.0 {
                if ui.at(x, y) == Spot::Control(index) {
                    return [x + 8.0, y + 8.0];
                }
                x += 4.0;
            }
            y += 4.0;
        }
        panic!("control {index} was not drawn");
    }

    fn four_rows(page: &mut Page) {
        for name in ["Hello", "Colour", "Material", "Marks"] {
            page.item(name);
        }
    }

    type Told = (Option<[f32; 2]>, bool, Option<u32>);

    fn told(page: &Page) -> Told {
        (page.pointer(), page.pointer_moved(), page.hovered())
    }

    #[test]
    fn a_driven_page_is_told_what_the_pointer_moved_onto_and_where_it_rests() {
        let mut headless = headless!("hovering over a driven page");
        headless.state.driven = true;
        headless.draw(four_rows);

        let at = middle_of(&headless.ui, 1);
        let spot = headless.ui.at(at[0], at[1]);
        assert!(headless.state.pointer_to(at, spot));

        let mut heard = None;
        headless.draw(|page| {
            heard = Some(told(page));
            four_rows(page);
        });
        assert_eq!(
            heard,
            Some((Some(at), true, Some(1))),
            "asked before anything was drawn, the page still hears what the \
             pointer moved onto"
        );
        assert_eq!(
            headless.state.focused, 0,
            "a driven page moves its own light"
        );

        headless.draw(|page| {
            heard = Some(told(page));
            four_rows(page);
        });
        assert_eq!(
            heard,
            Some((Some(at), false, None)),
            "a pointer that rests is still there, and has moved onto nothing"
        );

        assert!(!headless.state.pointer_to(at, spot), "the same place again");
        headless.draw(|page| heard = Some(told(page)));
        assert_eq!(heard, Some((Some(at), false, None)));
    }

    #[test]
    fn a_key_or_a_pad_takes_hovering_away_until_the_pointer_moves() {
        let mut headless = headless!("hovering given up to the hands");
        headless.state.driven = true;
        headless.draw(four_rows);
        let at = middle_of(&headless.ui, 2);
        let spot = headless.ui.at(at[0], at[1]);
        headless.state.pointer_to(at, spot);

        headless.state.pointer_away();
        let mut heard = None;
        headless.draw(|page| {
            heard = Some(told(page));
            four_rows(page);
        });
        assert_eq!(
            heard,
            Some((None, false, None)),
            "the pad moved the light, and the resting pointer must not fight it"
        );

        assert!(!headless.state.pointer_to(at, spot));
        headless.draw(|page| {
            heard = Some(told(page));
            four_rows(page);
        });
        assert_eq!(heard, Some((None, false, None)), "it has not moved yet");

        let on = [at[0] + 2.0, at[1]];
        assert!(headless.state.pointer_to(on, spot));
        headless.draw(|page| heard = Some(told(page)));
        assert_eq!(heard, Some((Some(on), true, Some(2))), "now it has");
    }

    #[test]
    fn the_flows_light_follows_the_pointer() {
        let mut headless = headless!("hovering over the flow");
        headless.draw(four_rows);
        assert_eq!(headless.state.focused, 0);

        let at = middle_of(&headless.ui, 3);
        let spot = headless.ui.at(at[0], at[1]);
        headless.state.pointer_to(at, spot);
        assert_eq!(headless.state.focused, 3, "hovering is selecting");

        headless.state.pointer_to([at[0], 799.0], Spot::Nothing);
        assert_eq!(
            headless.state.focused, 3,
            "off every control, the light stays where it was"
        );
    }

    const BAR: u32 = 0x400;

    /// Four rows and a bar beside them, which the page follows with
    /// `dragging` — what makes it a bar.
    fn rows_and_a_bar(page: &mut Page) -> Option<[f32; 2]> {
        four_rows(page);
        let dragged = page.dragging(BAR);
        page.ui().spot(BAR, [1180.0, 120.0, 24.0, 560.0]);
        dragged
    }

    #[test]
    fn a_press_held_on_a_plain_control_goes_on_hovering() {
        for driven in [false, true] {
            let mut headless = headless!("a press held over the flow");
            headless.state.driven = driven;
            headless.draw(|page| {
                rows_and_a_bar(page);
            });
            let from = middle_of(&headless.ui, 0);
            headless
                .state
                .pointer_to(from, headless.ui.at(from[0], from[1]));

            headless.state.take_hold(Spot::Control(0));
            assert_eq!(headless.state.held, None, "a row is not a bar");

            let at = middle_of(&headless.ui, 2);
            let spot = headless.ui.at(at[0], at[1]);
            assert!(headless.state.pointer_to(at, spot));
            assert_eq!(headless.state.hovered, Some(2));
            assert_eq!(
                headless.state.focused,
                if driven { 0 } else { 2 },
                "with the button still down, hovering is still selecting"
            );

            let mut heard = None;
            let mut dragged = None;
            headless.draw(|page| {
                heard = Some(told(page));
                dragged = page.dragging(0);
                rows_and_a_bar(page);
            });
            assert_eq!(
                heard,
                Some((Some(at), true, Some(2))),
                "a driven page hears the pointer move with a button down"
            );
            assert_eq!(dragged, None, "nothing is dragged from a row");
        }
    }

    #[test]
    fn a_press_held_on_a_bar_is_a_drag_and_not_a_hover() {
        let mut headless = headless!("a drag over the flow");
        headless.draw(|page| {
            rows_and_a_bar(page);
        });
        let at = middle_of(&headless.ui, 2);
        let spot = headless.ui.at(at[0], at[1]);

        headless.state.hovering = true;
        headless.state.take_hold(Spot::Control(BAR));
        assert_eq!(headless.state.held, Some(BAR), "the bar is taken hold of");
        assert!(!headless.state.pointer_to(at, spot));
        assert_eq!(headless.state.focused, 0, "the light left what was held");
        assert_eq!(headless.state.pointer, at, "and the drag still follows it");
        assert_eq!(headless.state.hovered, None);

        let mut heard = None;
        let mut dragged = None;
        headless.draw(|page| {
            heard = Some(told(page));
            dragged = rows_and_a_bar(page);
        });
        assert_eq!(
            heard,
            Some((None, false, None)),
            "a page measuring its own rows is not told of a drag as a hover"
        );
        assert_eq!(dragged, Some(at), "the drag is told where it is");

        headless.state.let_go(MouseButton::Right);
        assert_eq!(
            headless.state.held,
            Some(BAR),
            "another button coming up is not the press ending"
        );
        headless.state.let_go(MouseButton::Left);
        assert_eq!(headless.state.held, None);
        headless.draw(|page| heard = Some(told(page)));
        assert_eq!(
            heard,
            Some((Some(at), false, None)),
            "let go, the pointer rests where the drag ended"
        );
    }

    #[test]
    fn only_a_control_followed_on_the_frame_pressed_is_taken_hold_of() {
        let mut headless = headless!("what a press takes hold of");
        headless.draw(|page| {
            rows_and_a_bar(page);
        });
        headless.draw(four_rows);
        headless.state.take_hold(Spot::Control(BAR));
        assert_eq!(
            headless.state.held, None,
            "a bar the frame pressed on did not follow is a bar no longer"
        );

        headless.draw(|page| {
            rows_and_a_bar(page);
        });
        for elsewhere in [
            Spot::Nothing,
            Spot::OutsidePanel,
            Spot::MenuRow {
                row: 0,
                aside: false,
            },
        ] {
            headless.state.take_hold(elsewhere);
            assert_eq!(headless.state.held, None, "{elsewhere:?}");
        }

        headless.state.take_hold(Spot::Control(BAR));
        assert_eq!(headless.state.held, Some(BAR));
        headless.state.take_hold(Spot::Control(1));
        assert_eq!(
            headless.state.held, None,
            "a press elsewhere holds nothing, whatever was held before"
        );
    }

    #[test]
    fn a_leave_lets_go_of_a_bar_on_wayland_and_not_on_x() {
        let mut state = Interaction {
            bars: vec![BAR],
            hovering: true,
            ..Interaction::default()
        };
        state.take_hold(Spot::Control(BAR));

        state.pointer_left(false);
        assert_eq!(state.held, Some(BAR), "X goes on sending the drag");
        assert!(!state.hovering, "and nothing is hovered");

        state.pointer_left(true);
        assert_eq!(
            state.held, None,
            "on Wayland the release goes to whatever has the pointer now"
        );
    }

    #[test]
    fn nothing_on_the_page_is_hovered_under_a_menu() {
        let mut headless = headless!("hovering under a menu");
        headless.state.driven = true;
        headless.draw(four_rows);
        let at = middle_of(&headless.ui, 1);
        headless.draw(|page| {
            four_rows(page);
            page.menu(None, &["One", "Two"]);
        });
        assert!(headless.state.menu.is_open());

        headless.state.pointer_to(at, Spot::Control(1));
        let mut heard = None;
        headless.draw(|page| heard = Some(told(page)));
        assert_eq!(heard, Some((None, false, None)));
    }

    #[test]
    fn a_menu_of_alternatives_opens_on_the_one_in_force() {
        let mut headless = headless!("a menu of alternatives");
        headless.draw(|page| {
            four_rows(page);
            page.menu_marked(Some("Spare memory"), &["Off", "Small", "Large"], 0);
        });
        assert_eq!(headless.state.menu.selected(), 0);
        headless.state.menu.close();

        headless.draw(|page| {
            four_rows(page);
            page.menu_marked(Some("Spare memory"), &["Off", "Small", "Large"], 2);
        });
        assert!(headless.state.menu.is_open());
        assert_eq!(
            headless.state.menu.selected(),
            2,
            "Accept at once keeps what is in force rather than choosing Off"
        );
        headless.state.menu.close();

        headless.draw(|page| {
            four_rows(page);
            page.menu(None, &["Copy", "Rename", "Delete"]);
        });
        assert_eq!(
            headless.state.menu.selected(),
            0,
            "a menu of acts opens at its top"
        );
    }

    #[test]
    fn a_moving_picture_hands_the_page_a_frame_every_sixtieth_of_a_second() {
        let path =
            std::env::temp_dir().join(format!("lxb-app-shot-after-{}.png", std::process::id()));
        let mut clock = Vec::new();
        let taken = App::new("com.example.Test", "Test").plain().shot_after(
            &path,
            64,
            48,
            10.0,
            3,
            |page| clock.push(page.seconds()),
        );
        if let Err(trouble) = taken {
            eprintln!("no adapter: a moving picture was not checked ({trouble})");
            return;
        }
        let _ = std::fs::remove_file(&path);
        assert_eq!(clock.len(), 4, "the first frame and three after it");
        for (index, seconds) in clock.iter().enumerate() {
            let wanted = 10.0 + index as f32 / 60.0;
            assert!(
                (seconds - wanted).abs() < 1e-4,
                "frame {index} was drawn at {seconds}, not {wanted}"
            );
        }

        let mut settled = Vec::new();
        App::new("com.example.Test", "Test")
            .plain()
            .shot(&path, 64, 48, 10.0, |page| settled.push(page.seconds()))
            .expect("a settled picture where a moving one was taken");
        let _ = std::fs::remove_file(&path);
        assert_eq!(settled, [10.0, 10.0], "a settled picture is one instant");
    }

    #[test]
    fn a_page_is_told_whether_the_session_is_counted_low_end() {
        let mut headless = headless!("telling a page it is low-end");
        let mut told = None;
        headless.draw(|page| told = Some(page.low_end()));
        assert_eq!(told, Some(false), "a page is not low-end until it is said");

        headless.state.low_end = true;
        headless.draw(|page| told = Some(page.low_end()));
        assert_eq!(told, Some(true));
    }

    #[test]
    fn a_picture_of_a_low_end_window_is_drawn_at_one_instant() {
        let path = std::env::temp_dir().join(format!(
            "lxb-app-low-end-picture-{}.png",
            std::process::id()
        ));
        let mut told = Vec::new();
        let taken = App::new("com.example.Test", "Test")
            .plain()
            .driven()
            .low_end()
            .shot_after(&path, 64, 48, 10.0, 3, |page| {
                told.push((page.seconds(), page.low_end()));
            });
        let _ = std::fs::remove_file(&path);
        if let Err(trouble) = taken {
            eprintln!("no adapter: a low-end picture was not checked ({trouble})");
            return;
        }
        assert_eq!(
            told,
            [(10.0, true); 4],
            "a low-end window holds its clock still and says it is low-end"
        );
    }

    const A_DISC: &[u8] = b"<svg data-lxb-material=\"lxb:shape\" \
        xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 32 32\">\
        <circle cx=\"16\" cy=\"16\" r=\"10\" fill=\"#ffffff\"/></svg>";

    #[test]
    fn a_mark_of_the_applications_own_is_refused_at_once_and_the_application_kept() {
        let mut app = App::new("com.example.Test", "Test");
        assert!(
            app.add_mark("launch", A_DISC).is_err(),
            "a toolkit mark's name"
        );
        assert!(app.add_mark("", A_DISC).is_err(), "no name");
        assert!(app.add_mark("app-blank", b"<svg/>").is_err(), "not a shape");
        app.add_mark("app-disc", A_DISC).expect("a shape");
        app.add_mark("app-disc", A_DISC).expect("drawn again");
        assert_eq!(app.marks.len(), 1, "a name given again took a second place");
        let app = app.mark("app-ring", A_DISC).expect("chained");
        assert_eq!(app.marks.len(), 2);
        assert!(app.mark("logo", A_DISC).is_err());
    }

    #[test]
    fn a_picture_draws_the_applications_own_marks() {
        let path = std::env::temp_dir().join(format!(
            "lxb-app-own-mark-picture-{}.png",
            std::process::id()
        ));
        let mut known = Vec::new();
        let taken = App::new("com.example.Test", "Test")
            .mark("app-disc", A_DISC)
            .expect("a shape")
            .shot(&path, 64, 48, 10.0, |page| {
                page.icon("app-disc");
                known.push(page.ui().has_mark("app-disc"));
            });
        let _ = std::fs::remove_file(&path);
        if let Err(trouble) = taken {
            eprintln!("no adapter: an application's mark was not checked ({trouble})");
            return;
        }
        assert_eq!(known, [true, true], "the picture did not have the mark");
    }

    #[test]
    fn a_page_can_answer_the_question_over_it_as_a_key_would() {
        let mut headless = headless!("a press handed to the window");
        headless.state.driven = true;
        headless.draw(|page| page.ask("Erase?", "It goes.", &["Go back", "Erase"]));
        assert!(headless.state.dialog.is_open());

        headless.draw(|page| page.act(Action::Right));
        assert_eq!(headless.state.dialog.selected(), 1, "Right moved to Erase");
        let mut read = Vec::new();
        headless.draw(|page| {
            page.act(Action::Accept);
            read.extend(page.actions());
        });
        assert!(read.is_empty(), "the question took the press, not the page");
        assert!(!headless.state.dialog.is_open());
        let mut answer = None;
        headless.draw(|page| answer = page.answered());
        assert_eq!(answer, Some(1));

        headless.draw(|page| {
            page.act(Action::Down);
            read.extend(page.actions());
        });
        assert_eq!(
            read,
            [Action::Down],
            "with nothing over it a driven page reads it"
        );
    }

    #[test]
    fn a_menu_come_back_to_opens_on_the_row_it_was_left_by() {
        let mut headless = headless!("a menu opening on a row");
        headless.state.driven = true;
        let rows = ["Storage", "Spare memory", "Your files"];
        headless.draw(|page| page.menu_on(Some("Options"), &rows, 1));
        assert!(headless.state.menu.is_open());
        assert_eq!(
            headless.state.menu.selected(),
            1,
            "the light opens on the row"
        );
        headless.draw(|page| page.act(Action::Accept));
        let mut chose = None;
        headless.draw(|page| chose = page.chose());
        assert_eq!(
            chose,
            Some(1),
            "Accept at once answers the row the light opened on"
        );

        headless.draw(|page| page.menu_on(None, &rows, 9));
        assert_eq!(
            headless.state.menu.selected(),
            0,
            "past the end is the first row"
        );
    }

    #[test]
    fn a_pointer_moving_arriving_or_leaving_wakes_a_low_end_loop() {
        use winit::event::DeviceId;
        let device_id = DeviceId::dummy();
        assert!(wakes(&WindowEvent::CursorMoved {
            device_id,
            position: winit::dpi::PhysicalPosition::new(10.0, 20.0),
        }));
        assert!(wakes(&WindowEvent::CursorEntered { device_id }));
        assert!(wakes(&WindowEvent::CursorLeft { device_id }));
        assert!(!wakes(&WindowEvent::Focused(true)));
    }

    #[test]
    fn the_flows_light_travels_between_its_controls() {
        let mut headless = headless!("the flow's light");
        let rows = |page: &mut Page| {
            for name in ["Hello", "Colour", "Material"] {
                page.item(name);
            }
        };

        headless.draw_at(10.0, rows);
        let first = headless.state.focus_rect;

        headless.state.focused = 2;
        headless.draw_at(10.016, rows);
        let third = headless.state.focus_rect;
        assert_ne!(first[1], third[1], "the two rows are not in the same place");

        headless.draw_at(10.032, rows);
        headless.draw_at(10.048, rows);
        let at = headless
            .state
            .flow_light
            .rect()
            .expect("the light is somewhere");
        assert!(
            at[1] > first[1] && at[1] < third[1],
            "the light is between the row it left and the row it is going to, \
             not on either: left {first:?}, going to {third:?}, at {at:?}"
        );
    }

    /// Low-end hardware draws every frame at one instant (the wallpaper held
    /// still), so a light timed by the frame never moved: the flow's stayed
    /// on the control it was first drawn on, and so did a driven page's.
    #[test]
    fn on_low_end_hardware_a_pages_lights_stand_where_they_are_going() {
        let mut headless = headless!("a page's lights on low-end hardware");
        let still = lxb_toolkit::settings::STILL_WALLPAPER_AT;
        headless.ui.set_low_end(true);
        let rows = |page: &mut Page| {
            for name in ["Hello", "Colour", "Material"] {
                page.item(name);
            }
        };
        headless.draw_at(still, rows);
        headless.state.focused = 2;
        // The flow's light is laid under its controls, before the one it is
        // going to is drawn, so it sets off a frame after the light is moved.
        headless.draw_at(still, rows);
        headless.draw_at(still, rows);
        assert_eq!(
            headless.state.flow_light.rect(),
            Some(headless.state.focus_rect),
            "the flow's light was left where it was first drawn"
        );

        headless.state.driven = true;
        let (here, there) = ([40.0, 60.0, 300.0, 70.0], [40.0, 160.0, 300.0, 70.0]);
        let mut at = [0.0; 4];
        headless.draw_at(still, |page| at = page.glide(here, 1.0));
        assert_eq!(at, here);
        headless.draw_at(still, |page| at = page.glide(there, 1.0));
        assert_eq!(
            at, there,
            "a driven page's light was left on the row it left"
        );

        headless.ui.set_low_end(false);
        headless.draw_at(still, |page| at = page.glide(here, 1.0));
        assert_eq!(at, there, "a frame of no time moved a gliding light");
    }

    /// A card lit as itself is lit by the page's one light: put on it at once
    /// where it appears, and the capsule sets off from it to the next row
    /// rather than appearing there, as it would from a row of its own.
    #[test]
    fn a_light_in_a_cards_shape_is_the_pages_one_light() {
        let mut headless = headless!("a light in a card's shape");
        headless.state.driven = true;
        let (card, row) = ([40.0, 60.0, 300.0, 200.0], [40.0, 400.0, 300.0, 70.0]);
        let mut at = [0.0; 4];
        headless.draw_at(10.0, |page| at = page.light_shaped(card, 24.0, 1.0, true));
        assert_eq!(
            at, card,
            "a light that appears stands where it is asked for"
        );

        for seconds in [10.016, 10.032, 10.048] {
            headless.draw_at(seconds, |page| at = page.glide(row, 1.0));
        }
        assert!(
            at[1] > card[1] && at[1] < row[1],
            "the capsule did not set off from the card: left {card:?}, going \
             to {row:?}, at {at:?}"
        );

        headless.draw_at(10.064, |page| {
            at = page.light_shaped(card, 24.0, 1.0, false)
        });
        assert!(
            at[1] > card[1] && at[1] < row[1],
            "the card's light did not travel back from where the capsule had \
             got to: at {at:?}"
        );

        // On its way back, put there at once: on the card that same frame.
        headless.draw_at(10.080, |page| at = page.light_shaped(card, 24.0, 1.0, true));
        assert_eq!(
            at, card,
            "a light that appears did not stand on the card at once"
        );
    }

    #[test]
    fn a_control_a_page_drew_itself_can_show_a_press() {
        let mut headless = headless!("a press on a self-drawn control");
        headless.state.driven = true;

        headless.draw(|page| {
            assert_eq!(
                page.press(true),
                Press::Focused,
                "at rest, the lit one is lit"
            );
            assert_eq!(page.press(false), Press::Resting);
        });

        headless.state.pressing.press();

        headless.draw(|page| {
            assert!(
                matches!(page.press(true), Press::Going(_)),
                "the control the light is on is part-way through the press"
            );
            assert_eq!(
                page.press(false),
                Press::Resting,
                "and every other control is left alone"
            );
        });
    }

    #[test]
    fn a_page_that_draws_its_own_controls_is_answered_on_its_own_numbers() {
        const ITEM: u32 = 0x200;
        let mut headless = headless!("a press on a page's own spot");
        headless.state.driven = true;

        headless.draw(|page| {
            page.item("the only numbered control");
            let rect = [10.0, 400.0, 120.0, 40.0];
            page.ui().spot(ITEM + 3, rect);
        });

        headless.state.fired = Some((ITEM + 3) as usize);

        let (mut mine, mut other) = (false, false);
        headless.draw(|page| {
            page.item("the only numbered control");
            mine = page.pressed(ITEM + 3);
            other = page.pressed(ITEM + 4);
        });

        assert!(mine, "the spot the press names has to answer it");
        assert!(!other, "and no other spot may");
        assert!(
            !headless.state.driven || headless.state.fired.is_none(),
            "the press is answered once and then gone"
        );
    }

    #[test]
    fn the_flow_lays_itself_out_down_the_page() {
        let mut headless = headless!("the flow's layout");
        let mut tops = Vec::new();
        headless.draw(|page| {
            for _ in 0..4 {
                tops.push(page.cursor()[1]);
                page.text("a line of the page");
            }
            tops.push(page.cursor()[1]);
        });
        for pair in tops.windows(2) {
            assert!(
                pair[1] > pair[0],
                "the flow did not move on: {} then {}",
                pair[0],
                pair[1]
            );
        }

        let inset = headless.ui.m(Metric::PanelInset);
        assert!(tops[0] >= inset, "the flow started outside the pane");
    }

    #[test]
    fn controls_are_numbered_by_the_order_they_are_read_in() {
        let mut headless = headless!("how controls are numbered");
        let drawn = headless.draw(|page| {
            assert!(!page.button("first"));
            assert!(!page.row("second"));
            assert!(!page.row_value("third", "on"));
        });
        assert_eq!(drawn, 3);
        assert_eq!(headless.state.count, 3);

        let mut found = [false; 3];
        let mut x = 0.0;
        while x < 1280.0 {
            let mut y = 0.0;
            while y < 800.0 {
                if let Spot::Control(id) = headless.ui.at(x, y) {
                    found[id as usize] = true;
                }
                y += 4.0;
            }
            x += 4.0;
        }
        assert_eq!(found, [true; 3], "a control nobody can point at");
    }

    #[test]
    fn a_control_answers_one_press_once() {
        let mut headless = headless!("what a press answers");
        headless.draw(|page| {
            page.button("only");
        });

        headless.state.fired = Some(0);
        let mut answered = 0;
        headless.draw(|page| {
            if page.button("only") {
                answered += 1;
            }
        });
        assert_eq!(answered, 1, "the press was not reported");

        headless.draw(|page| {
            if page.button("only") {
                answered += 1;
            }
        });
        assert_eq!(answered, 1, "the press was reported twice");
    }

    #[test]
    fn a_press_on_a_control_that_has_gone_is_dropped() {
        let mut headless = headless!("a press on a control that has gone");
        headless.draw(|page| {
            page.button("one");
            page.button("two");
            page.button("three");
        });
        headless.state.focused = 2;
        headless.state.fired = Some(2);

        let mut answered = 0;
        headless.draw(|page| {
            if page.button("one") {
                answered += 1;
            }
        });
        assert_eq!(answered, 0, "a press landed on a control it was not for");

        assert_eq!(headless.state.focused, 0);
    }

    #[test]
    fn a_save_falls_back_to_the_pages_own_chooser_and_answers_with_a_new_path() {
        let mut headless = headless!("a save with no desktop to ask");
        headless.state.files.own_questions();
        let directory =
            std::env::temp_dir().join(format!("lxb-app-save-{}-{}", std::process::id(), line!()));
        std::fs::create_dir_all(&directory).expect("a directory to save into");

        headless.draw(|page| {
            assert!(page.save("notes.txt", &directory));
            assert!(
                !page.save("again.txt", &directory),
                "a second question cannot cover the first"
            );
        });
        assert!(headless.state.files.is_open());

        assert!(headless.state.files.act(Action::Accept).is_some());
        assert!(!headless.state.files.is_open());
        headless.draw(|_| {});

        headless.draw(|page| {
            assert_eq!(page.picked(), Some(directory.join("notes.txt")));
            assert_eq!(page.picked(), None, "the answer is one-shot");
        });
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn many_files_come_back_one_at_a_time_and_then_stop() {
        let mut headless = headless!("an answer of several files");
        headless.state.picked_files = vec![PathBuf::from("/tmp/one"), PathBuf::from("/tmp/two")];

        headless.draw(|page| {
            assert_eq!(page.picked_next(), Some(PathBuf::from("/tmp/one")));
            assert_eq!(page.picked_next(), Some(PathBuf::from("/tmp/two")));
            assert_eq!(page.picked_next(), None);
        });

        headless.state.picked_files = vec![PathBuf::from("/tmp/one"), PathBuf::from("/tmp/two")];
        headless.draw(|page| {
            assert_eq!(
                page.picked_files(),
                vec![PathBuf::from("/tmp/one"), PathBuf::from("/tmp/two")]
            );
            assert!(page.picked_files().is_empty());
        });
    }

    /// The letter y raises the context menu where no field takes it, as the
    /// Menu key, F10 and the shell's own y do; no other letter is a press.
    #[test]
    fn the_letter_y_is_the_context_menu_and_no_other_letter_is_a_press() {
        assert_eq!(letter_press(Key::Letter('y')), Some(Action::Menu));
        assert_eq!(letter_press(Key::Letter('Y')), Some(Action::Menu));
        for letter in ['w', 'a', 's', 'd', 'h', 'j', 'k', 'l', 'q', 'ż'] {
            assert_eq!(letter_press(Key::Letter(letter)), None, "{letter}");
        }
        assert_eq!(letter_press(Key::Menu), None, "the controls' own");
    }

    /// A page that takes a keyboard's typing quietly takes it from the end
    /// of the frame that asked, as `taking_text` does, for as long as it
    /// goes on asking — and never as text the compositor is told of, which
    /// is `taking_text` alone (what summons an on-screen keyboard).
    #[test]
    fn keys_are_taken_quietly_from_the_frame_that_asked() {
        let mut state = Interaction::default();
        assert!(!state.takes_typing());
        state.asked_keys = true;
        settle(&mut state, 0);
        assert!(state.takes_typing());
        assert!(
            !state.taking_text && !state.asked_text,
            "a quiet field was said to the compositor"
        );
        settle(&mut state, 0);
        assert!(
            !state.takes_typing(),
            "a page that stopped asking still took keys"
        );
        state.asked_text = true;
        settle(&mut state, 0);
        assert!(state.takes_typing(), "a field taking text takes keys too");
    }

    /// The shell's board rises over a field taking text where the shell has
    /// a controller in hand, whatever chose the field; where it last saw a
    /// keyboard it raises nothing, and nowhere but under LineXinBar is it
    /// watched for.
    #[test]
    fn the_shells_board_rises_over_a_field_taking_text_with_a_controller_in_hand() {
        let mut board = ShellsBoard::watching(true);
        assert!(!board.up());
        board.enabled(true);
        board.settle();
        assert!(board.up(), "a field taking text raised no board");
        board.disabled();
        assert!(!board.up(), "the board outlived its field");
        board.enabled(false);
        assert!(!board.up(), "a board rose over a keyboard in hand");
        let mut elsewhere = ShellsBoard::watching(false);
        elsewhere.enabled(true);
        assert!(
            !elsewhere.up(),
            "a board was watched for outside LineXinBar"
        );
    }

    /// Put away by its own hide key, by B or by Start, the board lets go of
    /// the keyboard, and what reaches the window is a report of nothing held
    /// and nothing after it (seen live: `wl_keyboard.keymap` and
    /// `modifiers(…, 0, 0, 0, 0)`, then nothing). Start's Enter comes first.
    #[test]
    fn the_board_letting_go_of_the_keyboard_is_the_board_put_away() {
        let mut board = ShellsBoard::watching(true);
        board.enabled(true);
        board.modifiers(false);
        board.settle();
        assert!(!board.up(), "the band stayed after the board's hide key");

        board.enabled(true);
        board.key();
        board.modifiers(false);
        board.settle();
        assert!(!board.up(), "Start's Enter kept the board up after it");
    }

    /// The board's first key after it rises carries the board's keymap and
    /// the same report ahead of it; a modifier the board held round a key
    /// comes up after it; the keyboard arriving at the window brings the
    /// seat's report. None of them is the board going.
    #[test]
    fn a_report_that_belongs_to_a_key_or_the_keyboards_arrival_keeps_the_board_up() {
        let mut board = ShellsBoard::watching(true);
        board.enabled(true);
        board.modifiers(false);
        board.key();
        board.settle();
        assert!(board.up(), "the board's first letter put it away");

        board.modifiers(true);
        board.key();
        board.modifiers(false);
        board.settle();
        assert!(board.up(), "Ctrl coming up put the board away");

        board.focused(false);
        assert!(!board.up(), "the board outlived the keyboard leaving");
        board.focused(true);
        board.enabled(true);
        board.modifiers(false);
        board.settle();
        assert!(
            board.up(),
            "the seat's report put a board offered again away"
        );
    }

    /// Shift+Tab walks back, with Shift as the keyboard last reported it. The
    /// window handed every key on as though Shift were not held, so on a
    /// keyboard Shift+Tab was Tab in every application: in the Linexin
    /// Installer's Partitions it went on to the next disk, not the one before.
    #[test]
    fn shift_tab_walks_back_with_shift_as_the_keyboard_last_reported_it() {
        use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey};
        let tab = WinitKey::Named(NamedKey::Tab);
        let mut state = Interaction::default();
        assert_eq!(state.key_of(&tab), Some(Key::Tab));
        state.modifiers_reported(ModifiersState::SHIFT);
        assert_eq!(state.key_of(&tab), Some(Key::BackTab), "Shift+Tab");
        assert_eq!(
            state.key_of(&tab).and_then(Action::of_key),
            Some(Action::Previous)
        );
        state.modifiers_reported(ModifiersState::SHIFT | ModifiersState::CONTROL);
        assert_eq!(state.key_of(&tab), Some(Key::BackTab), "Ctrl beside it");
        state.modifiers_reported(ModifiersState::CONTROL);
        assert_eq!(state.key_of(&tab), Some(Key::Tab), "Ctrl alone");
        state.modifiers_reported(ModifiersState::empty());
        assert_eq!(state.key_of(&tab), Some(Key::Tab), "Shift let go");
        assert_eq!(
            state.key_of(&tab).and_then(Action::of_key),
            Some(Action::Next)
        );
    }

    /// Shift reaches no other key's action: with it held, every key names
    /// the same key — a letter as the keyboard reports it, capital or not —
    /// so every action, the letter y's menu and Backspace's rubbing out are
    /// as they were. And LineXinBar's board reads the same report as before:
    /// Shift or Ctrl coming up after a key is not the board going, a report
    /// of nothing held after nothing is.
    #[test]
    fn shift_changes_no_other_key_and_the_board_reads_the_report_as_before() {
        use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey};
        let loose = Interaction::default();
        let mut held = Interaction::default();
        held.modifiers_reported(ModifiersState::SHIFT);
        let named = [
            NamedKey::ArrowLeft,
            NamedKey::ArrowRight,
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
            NamedKey::Enter,
            NamedKey::Space,
            NamedKey::Escape,
            NamedKey::Backspace,
            NamedKey::ContextMenu,
            NamedKey::F10,
            NamedKey::Shift,
            NamedKey::Home,
        ];
        let characters = ["a", "A", "y", "Y", "1", "!", " ", "ż", "Ż"];
        let others = named.into_iter().map(WinitKey::Named).chain(
            characters
                .into_iter()
                .map(|text| WinitKey::Character(text.into())),
        );
        for key in others {
            let (with, without) = (held.key_of(&key), loose.key_of(&key));
            assert_eq!(with, without, "{key:?}");
            assert_eq!(
                with.and_then(Action::of_key),
                without.and_then(Action::of_key),
                "{key:?}"
            );
            assert_eq!(
                with.and_then(letter_press),
                without.and_then(letter_press),
                "{key:?}"
            );
        }

        let mut state = Interaction {
            board: ShellsBoard::watching(true),
            ..Interaction::default()
        };
        state.board.enabled(true);
        for modifier in [ModifiersState::SHIFT, ModifiersState::CONTROL] {
            state.modifiers_reported(modifier);
            state.board.key();
            state.modifiers_reported(ModifiersState::empty());
            state.board.settle();
            assert!(
                state.board.up(),
                "{modifier:?} coming up put the board away"
            );
        }
        state.modifiers_reported(ModifiersState::empty());
        state.board.settle();
        assert!(!state.board.up(), "the board letting go was not heard");
    }

    /// While the board is up the shell keeps every pad from applications: a
    /// pad's press reaching the window is a board already gone.
    #[test]
    fn a_pads_press_reaching_the_window_is_the_board_gone() {
        let mut board = ShellsBoard::watching(true);
        board.enabled(true);
        board.pad();
        assert!(!board.up());
    }

    /// The page is told where the board stands only while it is up, by the
    /// shell's own rule for the window's size.
    #[test]
    fn a_page_is_told_where_the_shells_board_stands_while_it_is_up() {
        let mut headless = headless!("where the shell's board stands");
        let mut told = None;
        headless.draw(|page| told = Some(page.shells_board()));
        assert_eq!(told, Some(None));
        headless.state.board = ShellsBoard::watching(true);
        headless.state.board.enabled(true);
        headless.draw(|page| told = Some(page.shells_board()));
        assert_eq!(told, Some(Some(lxb_toolkit::board::rect(1280.0, 800.0))));
    }

    #[test]
    fn a_window_is_under_linexinbar_where_the_desktop_names_it_on_wayland() {
        assert!(under_linexinbar(Some("LineXinBar"), true));
        assert!(under_linexinbar(Some("GNOME: linexinbar"), true));
        assert!(
            !under_linexinbar(Some("LineXinBar"), false),
            "an X11 window"
        );
        assert!(!under_linexinbar(Some("KDE"), true));
        assert!(!under_linexinbar(None, true));
    }

    #[test]
    fn the_built_in_picker_is_a_page_lattice_and_returns_one_path() {
        let mut headless = headless!("the page's built-in picker");
        headless.state.files.own_questions();
        let directory = std::env::temp_dir();

        headless.draw(|page| {
            assert!(page.pick(PickerSelection::Folder, &directory));
            assert!(
                !page.pick(PickerSelection::Folder, &directory),
                "a second picker cannot cover the first"
            );
            page.menu(Some("not while picking"), &["No"]);
            page.ask("not while picking", "No", &["No"]);
        });

        assert!(headless.state.files.is_open());
        assert!(!headless.state.menu.is_open());
        assert!(!headless.state.dialog.is_open());

        while headless.state.files.act(Action::Up).is_some() {}
        assert!(headless.state.files.act(Action::Down).is_some());
        assert!(headless.state.files.act(Action::Accept).is_some());
        assert!(!headless.state.files.is_open());
        headless.draw(|_| {});

        headless.draw(|page| {
            assert_eq!(page.picked(), Some(directory.clone()));
            assert_eq!(page.picked(), None, "the result is one-shot");
        });
    }
}
