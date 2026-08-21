//! Hello, world — a window that shows the whole design language, in it.
//!
//!     cargo run --release                          # the window
//!     cargo run --release -- --shot hello.png 1280x800 Marks
//!     cargo run --release -- --print               # the six answers, as text
//!
//! A greeting is a fair first page because it needs exactly one of everything
//! a design language has to answer: what colour is this, how large, how heavy,
//! how long does it take, what mark stands beside it, what does it sound like.
//! None of those is chosen here. Each is named, and the toolkit answers.
//!
//! The other six pages are the rest of the answer, because most of it is not a
//! number. The water behind everything is the shell's own wallpaper. Every
//! pane bends what is behind it, so the sidebar bends the water and a context
//! menu bends the sidebar, the rows and the words. The ninety-eight marks are
//! beads of water shaded out of their own distance fields.
//!
//! None of that is written here. This application asks for a pane, a button, a
//! mark and a menu by name and `lxb-render` answers with the material — which
//! is the whole point of a design language being a library rather than a
//! document.
//!
//! It is driven from a keyboard, a controller and a pointer at once, and none
//! of the three is a special case: `lxb-input` turns all of them into the same
//! actions, `lxb-render` says what is under the pointer, and `lxb-sound`
//! answers each of them with the same click.
//!
//!     Up / Down     the page          D-pad, or the left stick
//!     Left / Right  within it         the same
//!     Enter         act on it         the bottom face button
//!     Menu / F10    a context menu    the top face button, or a right click
//!     Escape        close, or leave   the right face button

mod tour;

use std::sync::Arc;

use lxb_input::Controls;
use lxb_render::{Spot, Ui};
use lxb_sound::Sounds;
use lxb_toolkit::{
    input::{Action, Key, Wheel},
    metrics::REFERENCE_HEIGHT,
    palette::Role,
    settings::ShellTheme,
    sound::Sound,
    typography::Text,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::ModifiersState,
    platform::wayland::WindowAttributesExtWayland,
    window::{CursorIcon, Window, WindowId},
};

use tour::{Tour, GREETING, PAGES};

/// The stable id, which has to agree with the desktop entry, the executable
/// name and StartupWMClass. See docs/application-development.md.
const APP_ID: &str = "hello-lxb";

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.first().map(String::as_str) {
        Some("--print") => {
            report(
                arguments
                    .get(1)
                    .and_then(|arg| arg.parse().ok())
                    .unwrap_or(REFERENCE_HEIGHT),
            );
            Ok(())
        }
        Some("--shot") => shot(&arguments[1..]),
        None => window(),
        Some(_) => Err("usage: hello-lxb [--print [HEIGHT]]\n\
             \x20                [--shot FILE [WIDTHxHEIGHT] [PAGE] [menu|dialog]]"
            .to_string()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

/// The greeting's six answers as text, in the terminal's own colours.
///
/// Word for word what the C and Python greetings print, so that any two of the
/// three can be run and diffed. `NO_COLOR` turns the painting off.
fn report(height: f32) {
    let theme = ShellTheme::load();
    let palette = theme.accent;
    let plain = if std::env::var_os("NO_COLOR").is_some() {
        ""
    } else {
        "\x1b[0m"
    };
    let ink = |color: lxb_toolkit::color::Srgb, behind: bool| {
        if std::env::var_os("NO_COLOR").is_some() {
            return String::new();
        }
        let [r, g, b] = color.bytes();
        format!("\x1b[{};2;{r};{g};{b}m", if behind { 48 } else { 38 })
    };

    println!(
        "lxb-toolkit {} — hello, at {height:.0}p\n",
        lxb_toolkit::VERSION
    );

    // The greeting itself: the text role over the sky it sits on, ruled
    // underneath in the accent. Three roles, no colours.
    let padded = format!("  {GREETING}  ");
    println!(
        "  {}{}{padded}{plain}",
        ink(palette.color(Role::SkyTop), true),
        ink(palette.color(Role::Text), false)
    );
    println!(
        "  {}{}{plain}",
        ink(palette.color(Role::Accent), true),
        " ".repeat(padded.chars().count())
    );

    println!();
    for (label, value) in Tour::new().answers(height) {
        println!("  {label:<8} {value}");
    }
    let _ = Text::Title;

    println!("\n  The rest of the language: cargo run --release");
}

/// One settled frame, to a PNG. No display, no compositor, no window — the
/// same device, the same shaders and the same passes.
fn shot(arguments: &[String]) -> Result<(), String> {
    let path = arguments
        .first()
        .ok_or("usage: --shot FILE [WIDTHxHEIGHT] [PAGE] [menu|dialog]")?;
    let (width, height) = match arguments.get(1) {
        Some(size) => {
            let (w, h) = size
                .to_lowercase()
                .split_once('x')
                .map(|(w, h)| (w.to_string(), h.to_string()))
                .ok_or_else(|| format!("not a size: {size}"))?;
            (
                w.parse::<u32>()
                    .map_err(|_| format!("not a size: {size}"))?,
                h.parse::<u32>()
                    .map_err(|_| format!("not a size: {size}"))?,
            )
        }
        None => (1280, 800),
    };

    let mut tour = Tour::new();
    if let Some(page) = arguments.get(2) {
        tour.page = PAGES
            .iter()
            .position(|name| name.eq_ignore_ascii_case(page))
            .ok_or_else(|| format!("no such page: {page} ({})", PAGES.join(", ")))?;
    }
    let overlay = arguments.get(3).map(String::as_str);
    if !matches!(overlay, None | Some("menu") | Some("dialog")) {
        return Err(format!(
            "no such overlay: {} (menu, dialog)",
            overlay.unwrap_or_default()
        ));
    }

    let mut ui = Ui::headless(width, height)?;
    // Far enough past every entrance that nothing is still arriving. The first
    // frame is also where the page learns where its own rows went, which is
    // what a menu grows out of — so an overlay is opened between two frames.
    tour.draw(&mut ui, width as f32, height as f32, 10.0);
    let mut pixels = ui.end_to_image()?;
    if let Some(overlay) = overlay {
        match overlay {
            "menu" => tour.open_menu(),
            _ => {
                let _ = tour.act();
            }
        }
        // Settled, rather than caught on its way out.
        for _ in 0..60 {
            tour.advance(1.0 / 60.0);
        }
        tour.draw(&mut ui, width as f32, height as f32, 11.0);
        pixels = ui.end_to_image()?;
    }

    let file = std::fs::File::create(path).map_err(|err| format!("{path}: {err}"))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&pixels))
        .map_err(|err| format!("{path}: {err}"))?;

    println!("{path} — {width}x{height}, {}", PAGES[tour.page]);
    Ok(())
}

/// The tour, in a standard Wayland window.
///
/// Ordinary public winit and wgpu: nothing here uses LineXinBar's private
/// shell protocol, and the same binary runs on any other desktop.
fn window() -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut application = Application::new();
    event_loop
        .run_app(&mut application)
        .map_err(|err| err.to_string())
}

struct Application {
    instance: wgpu::Instance,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    ui: Option<Ui>,
    tour: Tour,
    opened: std::time::Instant,
    last: std::time::Instant,

    /// Every control this is driven from, read as one: the keyboard's keys go
    /// in, the controllers are read here, and one list of actions comes out
    /// with nothing in it saying which sent what.
    controls: Controls,
    /// And the other half of answering them. Silence is a working state, so
    /// nothing here is checked for having worked.
    sounds: Sounds,
    pointer: [f32; 2],
    /// What is left of a wheel that has not turned far enough to move
    /// anything. A touchpad reports fractions of a notch, and rounding each of
    /// them away is a page that never scrolls under a slow drag.
    wheel: Wheel,
    /// Whether Shift is down, which is the only thing that separates the two
    /// directions of Tab.
    shift: bool,
    /// Where a finger went down, so that letting it up somewhere else is a
    /// drag rather than a tap.
    touch: Option<[f32; 2]>,
    /// The shape the cursor is currently wearing, so it is only set when it
    /// changes.
    hand: bool,
}

impl Application {
    fn new() -> Self {
        let controls = Controls::new();
        if let Some(trouble) = controls.trouble() {
            // Said once and never again: controller input is an enhancement,
            // not a startup requirement, and this is the one thing somebody
            // with a dead pad will go looking for.
            eprintln!("no controller input: {trouble}");
        }
        Self {
            instance: lxb_render::instance(),
            window: None,
            surface: None,
            ui: None,
            tour: Tour::new(),
            opened: std::time::Instant::now(),
            last: std::time::Instant::now(),
            controls,
            sounds: Sounds::new(),
            pointer: [0.0; 2],
            wheel: Wheel::default(),
            shift: false,
            touch: None,
            hand: false,
        }
    }

    /// Since the window opened, which is the one clock everything here is
    /// measured against.
    fn now(&self) -> std::time::Duration {
        self.opened.elapsed()
    }

    /// Do one action and answer it.
    ///
    /// The single place a sound is played, so that a move made with a
    /// direction and the same move made with a click cannot end up sounding
    /// different — which is the whole reason it is one function.
    fn act(&mut self, action: Action) {
        let answer = self.tour.on_action(action);
        self.answer(answer);
    }

    fn answer(&mut self, sound: Option<Sound>) {
        if let Some(sound) = sound {
            self.sounds.play(sound);
        }
    }

    /// What is under the pointer, from the frame that is on the screen.
    fn spot(&self) -> Spot {
        let [x, y] = self.pointer;
        self.ui
            .as_ref()
            .map(|ui| ui.at(x, y))
            .unwrap_or(Spot::Nothing)
    }
}

const SURFACE: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(GREETING)
            // A window on LineXinBar is maximised and pinned to the display it
            // launched on; this size is for every other desktop.
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0))
            .with_name(APP_ID, APP_ID);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                eprintln!("no window: {err}");
                event_loop.exit();
                return;
            }
        };
        let surface = match self.instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(err) => {
                eprintln!("no surface: {err}");
                event_loop.exit();
                return;
            }
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
            Err(message) => {
                eprintln!("{message}");
                event_loop.exit();
                return;
            }
        };

        configure(&surface, &ui, size.width, size.height);
        self.window = Some(window);
        self.surface = Some(surface);
        self.ui = Some(ui);
        self.opened = std::time::Instant::now();
        self.last = self.opened;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // The window alone, and by a handle rather than a borrow: almost every
        // arm below wants a method of this application's own, and a borrow of
        // one field held across the whole match would stand in the way of
        // every one of them.
        let Some(window) = self.window.clone() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(ui)) = (self.surface.as_ref(), self.ui.as_ref()) {
                    configure(surface, ui, size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = [position.x as f32, position.y as f32];
                // Pointing at a thing is what selects it inside a panel, and
                // is silent: a pointer swept across the screen must not be a
                // stream of clicks.
                let spot = self.spot();
                self.tour.point_at(spot);
                // And the cursor says whether the click would be worth
                // making, before it is made.
                let hand = spot.pressable();
                if hand != self.hand {
                    self.hand = hand;
                    window.set_cursor(if hand {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::Default
                    });
                }
            }
            WindowEvent::CursorLeft { .. } => {
                // Half a notch of wheel belongs to what the pointer was over,
                // not to whatever it arrives on next.
                self.wheel.reset();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // On the press rather than the release, because that is when
                // a key fires, when a controller's bottom face button fires,
                // and when every other control in this language fires.
                if state == ElementState::Released {
                    return;
                }
                let spot = self.spot();
                let answer = self.tour.press_at(spot, button == MouseButton::Right);
                self.answer(answer);
            }
            // A wheel moves the selection, which is why it is turned into the
            // same two directions a keyboard sends: one notch is one row,
            // whatever this compositor decided a notch is worth in pixels.
            WindowEvent::MouseWheel { delta, .. } => {
                let notches = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => self.wheel.notches(-lines),
                    MouseScrollDelta::PixelDelta(at) => self.wheel.distance(-at.y as f32),
                };
                let action = if notches > 0 {
                    Action::Down
                } else {
                    Action::Up
                };
                for _ in 0..notches.abs() {
                    self.act(action);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift = modifiers.state().contains(ModifiersState::SHIFT);
            }
            // A window that has lost focus has had every control let go of.
            // Without this a direction held across the moment it went away is
            // still held when it comes back, and the list walks under nobody's
            // thumb.
            WindowEvent::Focused(false) => self.controls.release(),
            WindowEvent::KeyboardInput { event, .. } => {
                // The platform's own key repeat is dropped on the floor. The
                // pace a held direction walks a list at is a property of the
                // interface rather than of the keyboard, and `lxb-input`
                // invents the same middle for an arrow key that it invents for
                // a D-pad — see `lxb_toolkit::input::Repeat`.
                if event.repeat {
                    return;
                }
                let down = event.state == ElementState::Pressed;
                let Some(key) = lxb_input::key_of(&event, self.shift) else {
                    return;
                };
                let now = self.now();
                // The letter shorthands — wasd, hjkl, y — which are asked for
                // separately because an application that takes typed text must
                // not bind them. Nothing on this page is typed into.
                if let Key::Letter(letter) = key {
                    if down {
                        if let Some(action) = Action::of_letter(letter) {
                            self.act(action);
                        }
                    }
                    return;
                }
                if let Some(action) = self.controls.key(key, down, now) {
                    self.act(action);
                }
            }
            // A finger selects but does not press: what a tap does is decided
            // when it is lifted, because until then it may yet be a drag.
            WindowEvent::Touch(finger) => {
                use winit::event::TouchPhase;
                let at = [finger.location.x as f32, finger.location.y as f32];
                match finger.phase {
                    TouchPhase::Started => {
                        self.touch = Some(at);
                        self.pointer = at;
                        let spot = self.spot();
                        self.tour.point_at(spot);
                    }
                    TouchPhase::Ended => {
                        let slop = self
                            .ui
                            .as_ref()
                            .map(|ui| ui.s(lxb_toolkit::input::TAP_SLOP))
                            .unwrap_or(lxb_toolkit::input::TAP_SLOP);
                        let Some(from) = self.touch.take() else {
                            return;
                        };
                        if (at[0] - from[0]).hypot(at[1] - from[1]) > slop {
                            return;
                        }
                        self.pointer = at;
                        let spot = self.spot();
                        let answer = self.tour.press_at(spot, false);
                        self.answer(answer);
                    }
                    TouchPhase::Cancelled => self.touch = None,
                    TouchPhase::Moved => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                // An application stopped while it is hidden must not treat the
                // time it was away as one frame; clamp it, as the toolkit's
                // own spring does.
                let dt = (now - self.last).as_secs_f32().clamp(0.0, 0.1);
                self.last = now;

                // Everything the controllers have to say, and the repeats of
                // whichever key is being held — one list, in which nothing
                // says which of the two it came from.
                for action in self.controls.poll(self.now()) {
                    self.act(action);
                }
                // And the one recording that answers nothing, which is turned
                // on and off rather than played.
                let music = self.tour.music();
                self.sounds.music(music);
                // Hot-plugging: a pad plugged in halfway through is a pad the
                // sidebar should say is there.
                self.tour.pads = self.controls.pads();

                self.tour.advance(dt);
                if self.tour.leave {
                    event_loop.exit();
                    return;
                }

                let size = window.inner_size();
                let elapsed = (now - self.opened).as_secs_f32();
                let (Some(surface), Some(ui)) = (self.surface.as_ref(), self.ui.as_mut()) else {
                    return;
                };
                self.tour
                    .draw(ui, size.width as f32, size.height as f32, elapsed);

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
                    // Occluded, timed out, or refused: skip the frame rather
                    // than draw one nobody will see.
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // A frame is asked for continuously, so the controllers are read once
        // a frame — which is a little slower than
        // `lxb_toolkit::input::POLL_INTERVAL`, and is what an application
        // drawing every frame anyway can afford. One that sleeps between
        // frames should wake on the poll interval instead, or a pad will feel
        // tied to a frame rate that has dropped.
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
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
    use std::collections::BTreeSet;

    /// Every spot on a real frame, found the way a pointer finds them: by
    /// being somewhere and asking.
    ///
    /// Swept rather than read out of the renderer, because what is being
    /// tested is exactly that a hand moving over the window can reach these —
    /// a rectangle registered at zero width would pass any test that asked the
    /// list and fail every user.
    fn swept(ui: &Ui, width: u32, height: u32) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                match ui.at(x as f32, y as f32) {
                    Spot::Nothing => {}
                    other => {
                        found.insert(format!("{other:?}"));
                    }
                }
            }
        }
        found
    }

    fn drawn(tour: &mut Tour, ui: &mut Ui, width: u32, height: u32) {
        // Twice: the first frame is where a page learns where its own rows
        // went, and a menu grows out of that.
        for frame in 0..2 {
            tour.draw(ui, width as f32, height as f32, 10.0 + frame as f32);
        }
    }

    /// The complaint this was written for: a pointer that works in a couple of
    /// places and nowhere else. Every page's own items are reachable, on every
    /// page, and so is every row of the list beside them.
    #[test]
    fn every_item_on_every_page_can_be_pointed_at() {
        let (width, height) = (1280, 800);
        let mut ui = Ui::headless(width, height).expect("a device to draw with");
        for (page, name) in PAGES.iter().enumerate() {
            let mut tour = Tour::new();
            tour.page = page;
            drawn(&mut tour, &mut ui, width, height);
            let found = swept(&ui, width, height);

            for row in 0..PAGES.len() {
                let spot = format!("Control({})", 0x100 + row);
                assert!(found.contains(&spot), "{name} has no row {row}");
            }
            // Page 0 is one greeting rather than a list of anything, which is
            // why it is the one page with nothing of its own to point at.
            let items = found
                .iter()
                .filter(|spot| {
                    spot.strip_prefix("Control(")
                        .and_then(|rest| rest.trim_end_matches(')').parse::<u32>().ok())
                        .is_some_and(|id| id >= 0x200)
                })
                .count();
            if page == 0 {
                assert_eq!(items, 0, "Hello has nothing to walk");
            } else {
                assert!(items > 1, "{name} has {items} item(s) to point at");
            }
        }
    }

    /// The marks page is the one with ninety-eight of them, in a grid with
    /// gaps between. Every single one has to be reachable, or the grid is a
    /// dartboard.
    #[test]
    fn all_ninety_eight_marks_can_be_pointed_at() {
        let (width, height) = (1280, 800);
        let mut ui = Ui::headless(width, height).expect("a device to draw with");
        let mut tour = Tour::new();
        tour.page = PAGES.iter().position(|name| *name == "Marks").unwrap();
        drawn(&mut tour, &mut ui, width, height);
        let found = swept(&ui, width, height);
        for mark in 0..lxb_toolkit::assets::GLYPHS.len() {
            assert!(
                found.contains(&format!("Control({})", 0x200 + mark)),
                "mark {mark} cannot be pointed at"
            );
        }
    }

    /// An open menu takes the whole screen: its rows answer, and everything
    /// else is outside it rather than the page.
    #[test]
    fn an_open_menu_owns_the_pointer() {
        let (width, height) = (1280, 800);
        let mut ui = Ui::headless(width, height).expect("a device to draw with");
        let mut tour = Tour::new();
        drawn(&mut tour, &mut ui, width, height);
        tour.open_menu();
        for _ in 0..60 {
            tour.advance(1.0 / 60.0);
        }
        drawn(&mut tour, &mut ui, width, height);

        let found = swept(&ui, width, height);
        assert!(found.contains("OutsidePanel"));
        for row in 0..4 {
            assert!(
                found.contains(&format!("MenuRow {{ row: {row}, aside: false }}")),
                "menu row {row} cannot be pointed at"
            );
        }
        // Nothing of the page behind it is left pointable.
        assert!(
            !found.iter().any(|spot| spot.starts_with("Control(")),
            "the page under an open menu still answers the pointer"
        );
    }

    /// Pointing at a menu row selects it, silently; pressing it carries it
    /// out. The two are never the same event.
    #[test]
    fn a_menu_row_is_selected_by_pointing_and_taken_by_pressing() {
        let (width, height) = (1280, 800);
        let mut ui = Ui::headless(width, height).expect("a device to draw with");
        let mut tour = Tour::new();
        drawn(&mut tour, &mut ui, width, height);
        tour.open_menu();
        for _ in 0..60 {
            tour.advance(1.0 / 60.0);
        }
        drawn(&mut tour, &mut ui, width, height);

        let row_one = Spot::MenuRow {
            row: 1,
            aside: false,
        };
        assert_eq!(tour.menu.selected(), 0);
        tour.point_at(row_one);
        assert_eq!(tour.menu.selected(), 1, "pointing did not select");

        // Row one asks the question, which is what says the press landed.
        assert!(!tour.dialog.is_open());
        assert_eq!(tour.press_at(row_one, false), Some(Sound::Press));
        assert!(tour.dialog.is_open(), "pressing did not act");
        assert!(!tour.menu.is_open());
    }

    /// A press outside an open menu puts it away, and answers with the sound
    /// of a step back rather than of a press that was kept.
    #[test]
    fn a_press_outside_an_open_menu_dismisses_it() {
        let mut tour = Tour::new();
        tour.open_menu();
        assert!(tour.menu.is_open());
        assert_eq!(tour.press_at(Spot::OutsidePanel, false), Some(Sound::Back));
        assert!(!tour.menu.is_open());
    }

    /// The page's own lists move in two steps: the first click carries the
    /// selection to the pointer and sounds like the direction that would have
    /// walked there, and the second acts.
    #[test]
    fn a_list_moves_in_two_steps_under_the_pointer() {
        let mut tour = Tour::new();
        // The palette buttons, which are a list a press really acts on.
        tour.page = 1;
        let second = Spot::Control(0x200 + 1);
        let applied = tour.accent.applied().name;
        assert_eq!(tour.press_at(second, false), Some(Sound::Move));
        assert_eq!(
            tour.accent.applied().name,
            applied,
            "the first click applied a palette rather than pointing at one"
        );
        assert_eq!(tour.press_at(second, false), Some(Sound::Press));
        assert_ne!(tour.accent.applied().name, applied);
    }

    /// Pressing the page row already stood on is not a press of whatever that
    /// page is pointing at.
    #[test]
    fn a_second_press_on_the_page_already_open_does_nothing() {
        let mut tour = Tour::new();
        let sound = Spot::Control(0x100 + 6);
        assert_eq!(tour.press_at(sound, false), Some(Sound::Move));
        assert_eq!(tour.page, 6);
        assert_eq!(tour.press_at(sound, false), None);
    }

    /// The right button is the pad's top face button, and means what it means
    /// everywhere: a menu about the thing being pointed at, not about whatever
    /// happened to be selected already.
    #[test]
    fn the_right_button_raises_a_menu_about_what_is_under_it() {
        let mut tour = Tour::new();
        assert_eq!(
            tour.press_at(Spot::Control(0x100 + 3), true),
            Some(Sound::Press)
        );
        assert!(tour.menu.is_open());
        assert_eq!(tour.page, 3, "the menu was raised about another page");
    }

    /// One action, whichever control sent it, and one sound for it. A move
    /// that landed nowhere is answered by nothing.
    #[test]
    fn an_action_is_the_same_action_whoever_sent_it() {
        let mut tour = Tour::new();
        assert_eq!(tour.on_action(Action::Down), Some(Sound::Move));
        assert_eq!(tour.page, 1);
        assert_eq!(tour.on_action(Action::Next), Some(Sound::Move));
        assert_eq!(tour.page, 2);
        // Nothing to walk on the greeting, so Left and Right land nowhere.
        tour.page = 0;
        assert_eq!(tour.on_action(Action::Left), None);
        assert_eq!(tour.on_action(Action::Right), None);
    }

    /// The Sound page answers a press with the recording the row is about,
    /// which is the one page where the press *is* the sound.
    #[test]
    fn the_sound_page_plays_the_row_it_is_standing_on() {
        let mut tour = Tour::new();
        tour.page = 6;
        assert_eq!(tour.on_action(Action::Accept), Some(Sound::ALL[0]));
        assert_eq!(tour.on_action(Action::Right), Some(Sound::Move));
        assert_eq!(tour.on_action(Action::Accept), Some(Sound::ALL[1]));

        // The loop is turned on and off rather than played.
        for _ in 0..Sound::ALL.len() {
            if Sound::ALL[tour.selected_sound()].loops() {
                break;
            }
            let _ = tour.on_action(Action::Right);
        }
        assert!(!tour.music());
        assert_eq!(tour.on_action(Action::Accept), None);
        assert!(tour.music());
    }
}
