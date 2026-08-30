use std::{path::PathBuf, sync::Arc};

pub use lxb_input;
pub use lxb_portal as portal;
pub use lxb_render;
pub use lxb_sound;
pub use lxb_toolkit;

use lxb_input::Controls;
pub use lxb_render::{monotonic_now_ns, Fit, WallpaperClock};
use lxb_render::{Align, ContextMenu, Dialog, Entry, Files, Press, Pressing, Selection, Spot, Ui};
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
    event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::wayland::WindowAttributesExtWayland,
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

    pub fn shot(
        self,
        path: impl AsRef<std::path::Path>,
        width: u32,
        height: u32,
        seconds: f32,
        mut page: impl FnMut(&mut Page),
    ) -> Result<(), String> {
        let mut ui = Ui::headless(width, height)?;
        let theme = ShellTheme::load();
        let accent = Accent::new(theme.accent.name).unwrap_or_else(Accent::default_accent);
        let mut state = Interaction {
            driven: self.driven,
            ..Interaction::default()
        };
        state.files.own_questions();
        let mut sounds = Sounds::silent();
        let mut pixels = Vec::new();
        for frame in 0..2 {
            if frame == 1 {
                for _ in 0..60 {
                    state.menu.advance(1.0 / 60.0);
                    state.dialog.advance(1.0 / 60.0);
                    state.files.advance(1.0 / 60.0);
                    state.pressing.advance(1.0 / 60.0);
                }
            }
            let rect = start(
                &mut ui,
                &self,
                &accent,
                &theme,
                width as f32,
                height as f32,
                seconds,
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

        let file = std::fs::File::create(path.as_ref())
            .map_err(|err| format!("{}: {err}", path.as_ref().display()))?;
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
    ui.begin(width, height, seconds, accent, theme.wallpaper, theme.icons);
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

fn settle(state: &mut Interaction, drawn: usize) {
    if let Some(answer) = state.files.answered() {
        state.picked_files = answer;
    }
    state.count = drawn;
    if drawn > 0 {
        state.focused = state.focused.min(drawn - 1);
    }

    state.fired = None;

    state.arrived.clear();
    state.scrolled.clear();
    state.typed.clear();
    // What the page asked for becomes what is in force until it is asked
    // again. See `Interaction::taking_text`.
    state.taking_text = state.asked_text;
    state.asked_text = false;
}

#[derive(Default)]
struct Interaction {
    focused: usize,

    count: usize,

    flow_light: Selection,

    light: Selection,

    pressing: Pressing,

    fired: Option<usize>,
    pointer: [f32; 2],

    focus_rect: [f32; 4],

    hand: bool,
    /// Which control a press that is still held down began on.
    ///
    /// The one gesture a press and a release cannot describe between them: a
    /// bar taken hold of and moved. See [`Page::dragging`].
    held: Option<u32>,
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

    driven: bool,
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

    /// Where the field being typed into is, for whatever wants to stand clear
    /// of it. Said with `taking_text`, and only meant while that is true.
    pub fn text_at(&mut self, rect: [f32; 4]) {
        self.state.text_at = rect;
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

    pub fn typed(&mut self) -> std::vec::Drain<'_, Typed> {
        self.state.typed.drain(..)
    }

    pub fn actions(&mut self) -> std::vec::Drain<'_, Action> {
        self.state.arrived.drain(..)
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
        let dt = self.ui.dt();
        let at = self.state.flow_light.glide(self.state.focus_rect, dt);

        let strength = match self.state.pressing.through() {
            Some(through) => 1.0 - through * 0.35,
            None => 1.0,
        };
        self.ui.selection(at, strength);
    }

    pub fn glide(&mut self, rect: [f32; 4], strength: f32) -> [f32; 4] {
        let dt = self.ui.dt();
        let at = self.state.light.glide(rect, dt);
        self.ui.selection(at, strength);
        at
    }

    pub fn place(&mut self, rect: [f32; 4], strength: f32) -> [f32; 4] {
        self.state.light.clear();
        self.glide(rect, strength)
    }

    pub fn press(&self, lit: bool) -> Press {
        self.state.pressing.state(lit)
    }

    /// Where the pointer is while a press that began on this control is still
    /// held down, in the page's own space — `None` when nothing is being
    /// dragged from there.
    ///
    /// Only a pointer drags. A finger on the same control drags the list
    /// instead, which is what a finger does everywhere else on the page and
    /// what somebody holding a touchscreen expects of it; see `Finger`.
    pub fn dragging(&self, id: u32) -> Option<[f32; 2]> {
        (self.state.held == Some(id)).then_some(self.state.pointer)
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
    pub fn menu_marked(&mut self, title: Option<&str>, commands: &[&str], marked: usize) {
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
            .open_at(anchor, title.map(str::to_string), entries)
        {
            self.sounds.play(Sound::Press);
        }
    }

    pub fn chose(&mut self) -> Option<usize> {
        self.state.chose.take()
    }

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
                    ..Interaction::default()
                };
                if own_questions {
                    state.files.own_questions();
                }
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
            trouble: None,
        }
    }

    fn now(&self) -> std::time::Duration {
        self.opened.elapsed()
    }

    fn spot(&self) -> Spot {
        let [x, y] = self.state.pointer;
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

    fn act(&mut self, action: Action) {
        if self.state.files.is_open() {
            if let Some(sound) = self.state.files.act(action) {
                self.sounds.play(sound);
            }
            return;
        }
        if self.state.dialog.is_open() {
            let sound = match action {
                Action::Left => {
                    self.state.dialog.step(-1);
                    Some(Sound::Move)
                }
                Action::Right => {
                    self.state.dialog.step(1);
                    Some(Sound::Move)
                }
                Action::Accept | Action::Submit => {
                    self.state.dialog.press();
                    self.state.answered = Some(self.state.dialog.selected());
                    self.state.dialog.close();
                    Some(Sound::Press)
                }
                Action::Back => {
                    self.state.dialog.close();
                    Some(Sound::Back)
                }
                _ => None,
            };
            if let Some(sound) = sound {
                self.sounds.play(sound);
            }
            return;
        }
        if self.state.menu.is_open() {
            let sound = match action {
                Action::Up => {
                    self.state.menu.step(-1);
                    Some(Sound::Move)
                }
                Action::Down => {
                    self.state.menu.step(1);
                    Some(Sound::Move)
                }
                Action::Accept | Action::Submit => {
                    self.state.menu.press();
                    self.state.chose = Some(self.state.menu.selected());
                    self.state.menu.close();
                    Some(Sound::Press)
                }

                Action::Back | Action::Menu => {
                    self.state.menu.close();
                    Some(Sound::Back)
                }
                _ => None,
            };
            if let Some(sound) = sound {
                self.sounds.play(sound);
            }
            return;
        }

        if self.state.driven {
            self.state.arrived.push(action);
            return;
        }
        match action {
            Action::Accept | Action::Submit => {
                if self.state.count > 0 {
                    self.state.pressing.press();
                    self.state.fired = Some(self.state.focused);
                    self.sounds.play(Sound::Press);
                }
            }

            _ => {
                if let Some(direction) = action.direction() {
                    let back = matches!(
                        direction,
                        lxb_toolkit::input::Direction::Up | lxb_toolkit::input::Direction::Left
                    );
                    self.walk(if back { -1 } else { 1 });
                }
            }
        }
    }

    fn walk(&mut self, delta: isize) {
        if self.state.count == 0 {
            return;
        }
        let last = self.state.count - 1;
        let want = (self.state.focused as isize + delta).clamp(0, last as isize) as usize;
        if want != self.state.focused {
            self.state.focused = want;
            self.sounds.play(Sound::Move);
        }
    }

    fn draw(&mut self, width: f32, height: f32, elapsed: f32) {
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
        self.state.files.draw(ui);
    }
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
        let surface = match self.instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(err) => return self.give_up(event_loop, format!("no surface: {err}")),
        };
        let size = window.inner_size();
        let ui = match pollster::block_on(Ui::new(
            &self.instance,
            Some(&surface),
            SURFACE,
            size.width,
            size.height,
        )) {
            Ok(ui) => ui,
            Err(message) => return self.give_up(event_loop, message),
        };
        configure(&surface, &ui, size.width, size.height);
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
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Focused(false) => {
                self.controls.release();
                self.wheel.reset();
                // Whatever was being held is not being held now: the release
                // that would have said so is going somewhere else.
                self.state.held = None;
                self.finger = None;
            }
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(ui)) = (self.surface.as_ref(), self.ui.as_ref()) {
                    configure(surface, ui, size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.state.pointer = [position.x as f32, position.y as f32];

                let spot = self.spot();
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
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state != ElementState::Pressed {
                    // Letting go is half of the one gesture a press cannot
                    // describe on its own. See [`Page::dragging`].
                    self.state.held = None;
                    return;
                }
                self.state.pad_in_hand = false;

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
                if let Spot::Control(id) = spot {
                    self.state.held = Some(id);
                }
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
                let key = lxb_input::key_of(&event, false);
                if down {
                    self.state.pad_in_hand = false;
                }

                if down && self.state.files.key(key, event.text.as_deref()) {
                    return;
                }

                if down && self.state.taking_text {
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
                }
            }
            WindowEvent::RedrawRequested => {
                for action in self.controls.poll(self.now()) {
                    // A pad said something, so a pad is what is in hand.
                    self.state.pad_in_hand = true;
                    self.act(action);
                }
                let now = std::time::Instant::now();

                let dt = (now - self.last).as_secs_f32().clamp(0.0, 0.1);
                self.last = now;
                self.accent.advance(dt);
                self.state.pressing.advance(dt);
                self.state.menu.advance(dt);
                self.state.dialog.advance(dt);
                self.state.files.advance(dt);

                let size = window.inner_size();
                let elapsed = self.wallpaper.elapsed_secs();
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
                        ui.queue.present(frame);
                    }
                    Acquired::Outdated | Acquired::Lost => {
                        configure(surface, ui, size.width, size.height)
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
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
