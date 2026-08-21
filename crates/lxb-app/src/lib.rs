use std::sync::Arc;

pub use lxb_input;
pub use lxb_render;
pub use lxb_sound;
pub use lxb_toolkit;

use lxb_input::Controls;
use lxb_render::{Align, ContextMenu, Dialog, Entry, Press, Pressing, Selection, Spot, Ui};
use lxb_sound::{Level, Sounds};
use lxb_toolkit::{
    accent::Accent,
    input::{Action, Key},
    material::Overlay,
    metrics::Metric,
    palette::Role,
    settings::ShellTheme,
    sound::Sound,
    typography::Text,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::wayland::WindowAttributesExtWayland,
    window::{CursorIcon, Window, WindowId},
};

const SURFACE: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

pub struct App {
    app_id: String,
    title: String,
    size: (f64, f64),

    page: bool,

    driven: bool,
}

impl App {
    pub fn new(app_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            title: title.into(),
            size: (1280.0, 800.0),
            page: true,
            driven: false,
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
        let mut sounds = Sounds::silent();
        let mut pixels = Vec::new();
        for frame in 0..2 {
            if frame == 1 {
                for _ in 0..60 {
                    state.menu.advance(1.0 / 60.0);
                    state.dialog.advance(1.0 / 60.0);
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
        let event_loop = EventLoop::new().map_err(|err| format!("no event loop: {err}"))?;

        event_loop.set_control_flow(ControlFlow::Poll);
        let mut runtime = Runtime::new(self, page);
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

fn settle(state: &mut Interaction, drawn: usize) {
    state.count = drawn;
    if drawn > 0 {
        state.focused = state.focused.min(drawn - 1);
    }

    state.fired = None;

    state.arrived.clear();
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
    menu: ContextMenu,
    dialog: Dialog,

    chose: Option<usize>,
    answered: Option<usize>,

    leaving: bool,

    arrived: Vec<Action>,

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

    pub fn actions(&mut self) -> std::vec::Drain<'_, Action> {
        self.state.arrived.drain(..)
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

    pub fn menu(&mut self, title: Option<&str>, commands: &[&str]) {
        if self.state.menu.is_open() {
            return;
        }
        let anchor = self.state.focus_rect;
        let entries = commands.iter().map(|name| Entry::new(*name)).collect();
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
        if self.state.dialog.is_open() {
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
    sounds: Sounds,

    opened: std::time::Instant,
    last: std::time::Instant,

    trouble: Option<String>,
}

impl<F: FnMut(&mut Page)> Runtime<F> {
    fn new(app: App, page: F) -> Self {
        let theme = ShellTheme::load();
        let accent = Accent::new(theme.accent.name).unwrap_or_else(Accent::default_accent);
        let controls = Controls::new();
        if let Some(trouble) = controls.trouble() {
            eprintln!("no controller input: {trouble}");
        }
        let now = std::time::Instant::now();
        let driven = app.driven;
        Self {
            app,
            page,
            instance: lxb_render::instance(),
            window: None,
            surface: None,
            ui: None,
            theme,
            accent,
            state: Interaction {
                driven,
                ..Interaction::default()
            },
            controls,
            sounds: Sounds::new(),
            opened: now,
            last: now,
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

    fn act(&mut self, action: Action) {
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
        settle(&mut self.state, drawn);

        ui.context_menu(&mut self.state.menu);
        ui.dialog(&mut self.state.dialog);
    }
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
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Focused(false) => self.controls.release(),
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(ui)) = (self.surface.as_ref(), self.ui.as_ref()) {
                    configure(surface, ui, size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.state.pointer = [position.x as f32, position.y as f32];

                let spot = self.spot();
                self.state.dialog.point_at(spot);
                self.state.menu.point_at(spot);

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
            WindowEvent::MouseInput { state, button, .. } => {
                if state != ElementState::Pressed {
                    return;
                }

                if button == MouseButton::Right {
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
                match self.spot() {
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
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }
                let down = event.state == ElementState::Pressed;
                let Some(key) = lxb_input::key_of(&event, false) else {
                    return;
                };

                if key == Key::Escape
                    && down
                    && !self.state.dialog.is_open()
                    && !self.state.menu.is_open()
                {
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
                    self.act(action);
                }
                let now = std::time::Instant::now();

                let dt = (now - self.last).as_secs_f32().clamp(0.0, 0.1);
                self.last = now;
                self.accent.advance(dt);
                self.state.pressing.advance(dt);
                self.state.menu.advance(dt);
                self.state.dialog.advance(dt);

                let size = window.inner_size();
                let elapsed = (now - self.opened).as_secs_f32();
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
}
