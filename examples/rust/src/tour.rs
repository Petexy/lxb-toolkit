//! The eight pages, and what the keyboard is doing to them.
//!
//! Every drawn thing here is one call. There is no shader in this file, no
//! pipeline, no atlas and no pass: `lxb-render` owns all of that, because
//! every application would otherwise own an identical copy of it.

use std::path::PathBuf;

use lxb_render::{Align, ContextMenu, Dialog, Entry, Files, Press, Pressing, Selection, Spot, Ui};
use lxb_toolkit::{
    accent::Accent,
    assets, control,
    input::{Action, Key},
    material::{Overlay, Surface},
    menu,
    metrics::{capsule_radius, Metric},
    motion,
    palette::{Role, PALETTES},
    picker::{Purpose as PickerPurpose, Selection as PickerSelection},
    settings::{IconStyle, ShellTheme},
    sound::Sound,
    typography::{Face, Text},
};

pub const PAGES: [&str; 8] = [
    "Hello", "Colour", "Material", "Marks", "Type", "Motion", "Sound", "Picker",
];
pub const GREETING: &str = "Hello, world";

/// The four questions the Picker page can put, in the order it offers them:
/// what is being asked for, what the columns list, the row that asks it, and
/// what a new file is called to begin with.
///
/// A save arrives already named because that is the case worth showing — it is
/// the one purpose whose column opens on the row that answers, and a picture of
/// it with an empty name would be a picture of the other rule.
pub const PICKER_PURPOSES: [(PickerPurpose, PickerSelection, &str, &str); 5] = [
    (
        PickerPurpose::OneFile,
        PickerSelection::File,
        "Choose a file",
        "",
    ),
    (
        PickerPurpose::ManyFiles,
        PickerSelection::File,
        "Choose some files",
        "",
    ),
    (
        PickerPurpose::AFolder,
        PickerSelection::Folder,
        "Choose a folder",
        "",
    ),
    (
        PickerPurpose::ANewFile,
        PickerSelection::File,
        "Choose somewhere to save",
        "untitled.txt",
    ),
    // The one question with a kind of file in force, which is what the panel's
    // Types row exists for: an application asking for images shows a folder of
    // images, and without that row nothing says the rest of the disk is one
    // press away.
    (
        PickerPurpose::OneFile,
        PickerSelection::Image,
        "Choose an image",
        "",
    ),
];

/// The mark that stands beside it, and the noise a press on it makes.
pub const MARK: &str = "launch";
pub const SOUND: Sound = Sound::Press;

/// Where a pointer landing on the tour lands.
///
/// Two numbered ranges rather than one, so the sidebar's page list and the
/// page's own items can never be mistaken for each other — a click on row
/// three of the list and a click on the third mark are two different things
/// and are asked about separately.
const PAGE_SPOT: u32 = 0x100;
const ITEM_SPOT: u32 = 0x200;

/// Six of the thirty-four durations, each a different kind of arrival.
const SHOWN: [(&str, f32, &str); 6] = [
    ("panel", motion::duration::PANEL, "a panel arrives"),
    ("flight", motion::duration::FLIGHT, "a card flies"),
    (
        "menu-flight",
        motion::duration::MENU_FLIGHT,
        "a menu grows out",
    ),
    (
        "accent-change",
        motion::duration::ACCENT_CHANGE,
        "the palette travels",
    ),
    (
        "launch-open",
        motion::duration::LAUNCH_OPEN,
        "an application opens",
    ),
    (
        "sidebar-slide",
        motion::duration::SIDEBAR_SLIDE,
        "the guide slides in",
    ),
];

pub struct Tour {
    pub theme: ShellTheme,
    pub accent: Accent,
    pub icons: IconStyle,
    marks: Vec<&'static str>,

    pub page: usize,
    cursor: [usize; PAGES.len()],
    cursor_since: f32,
    entered: f32,
    pub elapsed: f32,
    /// The display this frame is being drawn at, for the questions a menu has
    /// to ask about the screen rather than about itself.
    height: f32,

    pub menu: ContextMenu,
    pub dialog: Dialog,
    /// The same built-in in-window chooser that lxb-app exposes through
    /// Page::pick. This tour drives the renderer directly, so it owns the
    /// state itself and still draws the toolkit's real Lattice picker.
    pub files: Files,
    picker_root: PathBuf,
    picked: Vec<PathBuf>,

    /// The press the chosen control is part-way through. A press is a
    /// journey in this language, not a state: the key coming up does not stop
    /// it, and it is watched to the end.
    pressing: Pressing,
    /// The lit capsule on the page list, which glides down the sidebar rather
    /// than jumping between rows.
    sidebar_light: Selection,
    /// The one on the page itself. Cleared when the page changes: a light that
    /// has never been on this page appears where it is asked for rather than
    /// flying in from the last page's row.
    light: Selection,
    lit_page: usize,
    spring_at: f64,
    spring_speed: f64,
    pub leave: bool,
    /// How many controllers are connected, so that a pad the machine cannot
    /// see is visibly a pad the machine cannot see rather than a dead button.
    pub pads: usize,
    rows: [[f32; 4]; PAGES.len()],
    /// Whether the one looping recording is playing. Turned on and off rather
    /// than started, because it is the one sound here that answers nothing.
    music: bool,
}

impl Tour {
    pub fn new() -> Self {
        // An application reads the shell's theme. It does not pick a palette
        // of its own: five applications each fond of their own blue is exactly
        // the thing a design language exists to prevent. What it may do — what
        // the Colour page does on purpose — is show what moving the shell's
        // own accent setting does.
        let theme = ShellTheme::load();
        let accent = Accent::new(theme.accent.name).unwrap_or_else(Accent::default_accent);
        Self {
            icons: theme.icons,
            theme,
            accent,
            marks: assets::glyph_names().collect(),
            page: 0,
            cursor: [0; PAGES.len()],
            cursor_since: 0.0,
            entered: 0.0,
            elapsed: 0.0,
            height: 0.0,
            menu: ContextMenu::default(),
            dialog: Dialog::default(),
            files: Files::default(),
            picker_root: picker_root(),
            picked: Vec::new(),
            pressing: Pressing::default(),
            sidebar_light: Selection::default(),
            light: Selection::default(),
            lit_page: usize::MAX,
            spring_at: 0.0,
            spring_speed: 0.0,
            leave: false,
            pads: 0,
            rows: [[0.0; 4]; PAGES.len()],
            music: false,
        }
    }

    // -- motion ------------------------------------------------------------

    fn arrived(&self, start: f32, over: f32) -> f32 {
        motion::ease(((self.elapsed - start) / over).clamp(0.0, 1.0))
    }

    /// The shell's own entry choreography: a lead, then one stagger between
    /// each row and the next, over each row's own slide.
    fn staggered(&self, index: f32) -> f32 {
        self.arrived(
            self.entered + motion::duration::ENTRY_LEAD + index * motion::duration::ENTRY_STAGGER,
            motion::duration::ENTRY_SLIDE,
        )
    }

    /// Never put a question to the desktop, whatever the session has.
    pub fn keep_its_own_questions(&mut self) {
        self.files.own_questions();
    }

    /// Whether a question is standing somewhere else, waiting to be answered.
    pub fn asking_the_desktop(&self) -> bool {
        self.files.asking_the_desktop()
    }

    pub fn advance(&mut self, dt: f32) {
        if let Some(chosen) = self.files.answered() {
            self.picked = chosen;
        }
        self.files.advance(dt);
        self.accent.advance(dt);
        self.menu.advance(dt);
        self.dialog.advance(dt);

        self.pressing.advance(dt);
        let target = if (self.elapsed / 1.6) as i64 % 2 == 1 {
            1.0
        } else {
            0.0
        };
        let (at, speed) = motion::spring(
            self.spring_at,
            self.spring_speed,
            target,
            motion::CARD_SPRING,
            dt as f64,
        );
        self.spring_at = at;
        self.spring_speed = speed;
    }

    fn count(&self) -> usize {
        match self.page {
            1 => PALETTES.len(),
            2 => Surface::ALL.len(),
            3 => self.marks.len(),
            4 => Text::ALL.len(),
            5 => SHOWN.len(),
            6 => Sound::ALL.len(),
            7 => PICKER_PURPOSES.len(),
            _ => 1,
        }
    }

    fn press(&self, chosen: bool) -> Press {
        if chosen {
            self.pressing.state(true)
        } else {
            Press::Resting
        }
    }

    /// Press whatever is selected. Whatever the key does afterwards, the
    /// control goes down and springs back over its own journey.
    pub fn press_down(&mut self) {
        self.pressing.press();
    }

    // -- the whole window --------------------------------------------------

    pub fn draw(&mut self, ui: &mut Ui, width: f32, height: f32, elapsed: f32) {
        self.elapsed = elapsed;
        self.height = height;
        ui.begin(
            width,
            height,
            elapsed,
            &self.accent,
            self.theme.wallpaper,
            self.theme.particles,
            self.theme.icons,
        );

        let inset = ui.m(Metric::PanelInset);
        let gap = ui.m(Metric::Gap);
        let sidebar = ui.m(Metric::MenuWidth).min(width * 0.34);
        let tall = height - 2.0 * inset;

        self.sidebar(ui, [inset, inset, sidebar, tall]);

        let x = inset + sidebar + gap;
        let content = [x, inset, width - x - inset, tall];
        ui.pane(content, Overlay::Dialog);
        let pad = ui.m(Metric::PanelPadding);
        self.page(
            ui,
            content[0] + pad,
            content[1] + pad,
            content[2] - 2.0 * pad,
        );

        ui.context_menu(&mut self.menu);
        ui.dialog(&mut self.dialog);
        self.files.hand(self.pads > 0);
        self.files.draw(ui);
    }

    fn sidebar(&mut self, ui: &mut Ui, rect: [f32; 4]) {
        ui.pane(rect, Overlay::Dialog);

        let [x, y, width, height] = rect;
        let pad = ui.m(Metric::PanelPadding);
        let row = ui.m(Metric::RowHeight);
        let gap = ui.m(Metric::Gap);
        let inner = width - 2.0 * pad;
        let mut top = y + pad;

        let version = format!("lxb-toolkit {}", lxb_toolkit::VERSION);
        let line = ui.line(Text::Label);
        ui.label(
            [x + pad, top, inner, line],
            Text::Label,
            &version,
            Role::TextSoft,
            Align::Left,
        );
        top += line + gap;

        // Where each row sits, worked out before any of them is drawn: the
        // light is laid down first, and it needs to know where it is going.
        let air = ui.s(control::PADDING);
        for (index, at) in self.rows.iter_mut().enumerate() {
            *at = [x + pad, top + index as f32 * row, inner, row];
        }
        let chip = |at: [f32; 4]| [at[0], at[1] + air, at[2], at[3] - 2.0 * air];
        let lit = chip(self.rows[self.page.min(PAGES.len() - 1)]);
        let dt = ui.dt();
        let at = self.sidebar_light.glide(lit, dt);
        ui.lit(at, capsule_radius(at[3]), Role::Accent, inner, 1.0);

        for (index, name) in PAGES.iter().enumerate() {
            let entered = self.arrived(
                motion::duration::ENTRY_LEAD + index as f32 * motion::duration::ENTRY_STAGGER,
                motion::duration::ENTRY_SLIDE,
            );
            let at = self.rows[index];
            // Where the row went, so a pointer over it can be answered. The
            // settled rectangle rather than the one still sliding in: a row
            // arriving is a row about to be here.
            ui.spot(PAGE_SPOT + index as u32, at);
            let chosen = index == self.page;
            let tint = ui.tinted(if chosen { Role::Text } else { Role::TextSoft }, entered);
            ui.label_tinted(
                [
                    at[0] + ui.m(Metric::RowPadding) + (1.0 - entered) * gap,
                    at[1],
                    at[2],
                    at[3],
                ],
                Text::Body,
                name,
                tint,
                Align::Left,
            );
        }

        // The keyboard's names, because a keyboard is what is certainly
        // there. Each of them is one action, and the pad sends the same ones.
        let pads = match self.pads {
            0 => "no pad".to_string(),
            1 => "1 pad".to_string(),
            many => format!("{many} pads"),
        };
        let keys = [
            ("Up / Down", "the page".to_string()),
            ("Left / Right", "within it".to_string()),
            ("Enter", "act on it".to_string()),
            ("Menu / F10", "a context menu".to_string()),
            ("Escape", "close, or leave".to_string()),
            ("Controller", pads),
        ];
        let step = ui.line(Text::Caption);
        let foot = y + height - pad - step * keys.len() as f32;
        let mut column: f32 = 0.0;
        for (key, _) in &keys {
            column = column.max(ui.measure(Text::Caption, key));
        }
        for (index, (key, what)) in keys.iter().enumerate() {
            let at = foot + index as f32 * step;
            let bright = ui.tinted(Role::TextSoft, 0.85);
            let quiet = ui.tinted(Role::TextSoft, 0.55);
            ui.label_tinted(
                [x + pad, at, inner, step],
                Text::Caption,
                key,
                bright,
                Align::Left,
            );
            ui.label_tinted(
                [x + pad + column + gap, at, inner, step],
                Text::Caption,
                what,
                quiet,
                Align::Left,
            );
        }
    }

    /// The page's own name, ruled underneath in the accent.
    fn head(
        &mut self,
        ui: &mut Ui,
        at: [f32; 3],
        title: &str,
        mark: Option<&str>,
        display: bool,
    ) -> f32 {
        let [x, y, width] = at;
        let role = if display { Text::Display } else { Text::Title };
        let icon = ui.m(Metric::ItemIcon);
        let gap = ui.m(Metric::Gap);
        let row_pad = ui.m(Metric::RowPadding);
        let head = if mark.is_some() {
            icon.max(ui.line(role))
        } else {
            ui.line(role)
        };

        let mut left = x;
        if let Some(name) = mark {
            ui.icon(
                [left, y + (head - icon) / 2.0, icon, icon],
                name,
                self.icons,
            );
            left += icon + gap;
        }
        ui.label([left, y, width, head], role, title, Role::Text, Align::Left);

        // The same rule the CSS greeting draws under its own.
        let rule = ui.s(6.0);
        ui.rule([x, y + head + row_pad, width, rule], Role::Accent);
        head + row_pad + rule + row_pad
    }
}

// --- the pages -------------------------------------------------------------

impl Tour {
    /// Lay the page's light down at `at`, gliding rather than jumping, and
    /// return where it got to.
    ///
    /// Always before the rows it lights: the light is one object crossing the
    /// panel, and anything drawn after it hands its own face over as it
    /// arrives.
    fn glide(&mut self, ui: &mut Ui, at: [f32; 4], over: f32, strength: f32) -> [f32; 4] {
        let dt = ui.dt();
        let at = self.light.glide(at, dt);
        ui.lit(at, capsule_radius(at[3]), Role::Accent, over, strength);
        at
    }

    fn page(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        // A light that has never been on this page appears where it is asked
        // for rather than flying in from the last page's row.
        if self.lit_page != self.page {
            self.light.clear();
            self.lit_page = self.page;
        }
        match self.page {
            0 => self.hello(ui, x, y, width),
            1 => self.colour(ui, x, y, width),
            2 => self.material(ui, x, y, width),
            3 => self.marks(ui, x, y, width),
            4 => self.type_(ui, x, y, width),
            5 => self.motion(ui, x, y, width),
            6 => self.sound(ui, x, y, width),
            _ => self.picker_page(ui, x, y, width),
        }
    }

    /// The six questions a greeting has to ask, and what comes back.
    pub fn answers(&self, height: f32) -> Vec<(&'static str, String)> {
        let palette = self.theme.accent;
        vec![
            (
                "theme",
                format!(
                    "{}, {} wallpaper, {} marks",
                    palette.name,
                    self.theme.wallpaper.name(),
                    self.theme.icons.name()
                ),
            ),
            (
                "colour",
                format!(
                    "text {} over sky-top {}",
                    palette.color(Role::Text).hex(),
                    palette.color(Role::SkyTop).hex()
                ),
            ),
            (
                "size",
                format!(
                    "title {:.0}px {}, in a {:.0}px row",
                    Text::Title.on(height),
                    if Text::Title.face() == Face::Bold {
                        "bold"
                    } else {
                        "regular"
                    },
                    Metric::RowHeight.on(height)
                ),
            ),
            (
                "motion",
                format!(
                    "a panel arrives over {:.0}ms, and never linearly",
                    motion::duration::PANEL * 1000.0
                ),
            ),
            (
                "mark",
                format!(
                    "{MARK}, drawn in a {}-unit square",
                    assets::glyph_box(MARK).unwrap_or(0)
                ),
            ),
            (
                "sound",
                format!(
                    "{}, {:.1} KiB of Ogg Vorbis",
                    SOUND.name(),
                    SOUND.bytes().map(<[u8]>::len).unwrap_or(0) as f32 / 1024.0
                ),
            ),
        ]
    }

    fn hello(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        let mut top = y + self.head(ui, [x, y, width], GREETING, Some(MARK), true);
        let gap = ui.m(Metric::Gap);
        let column = ui.m(Metric::ColumnSpacing);
        let step = ui.line(Text::Body);

        let rows = self.answers(ui.height());
        for (index, (label, value)) in rows.iter().enumerate() {
            let entered = self.staggered(index as f32);
            let slid = (1.0 - entered) * gap;
            let at = top + index as f32 * step;
            let bright = ui.tinted(Role::Text, entered);
            let quiet = ui.tinted(Role::TextSoft, entered);
            ui.label_tinted(
                [x + slid, at, column, step],
                Text::Body,
                label,
                bright,
                Align::Left,
            );
            ui.label_tinted(
                [x + column + slid, at, width, step],
                Text::Caption,
                value,
                quiet,
                Align::Left,
            );
        }

        top += rows.len() as f32 * step + gap;
        top += ui.paragraph(
            [x, top, width, 0.0],
            "Every value on this page was named rather than picked, and every one \
             of them depends on the height it was asked at — resize the window and \
             read them again. Enter asks a question; the six other pages are the \
             rest of what the toolkit answers.",
            Role::TextSoft,
        ) + gap;
        ui.paragraph(
            [x, top, width, 0.0],
            "Everything here is the shell's own material: the water behind it is \
             the wallpaper shader, every pane bends what is behind it through the \
             glass shader, and the mark above is a distance field shaded into a \
             bead of water by the glyph shader. This page asks for them by name — \
             ui.pane, ui.icon, ui.button — and never touches one.",
            Role::AccentSoft,
        );
    }

    fn colour(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        let mut top = y + self.head(ui, [x, y, width], "Colour", None, false);
        let gap = ui.m(Metric::Gap);
        let step = ui.line(Text::Body) * 1.35;
        let chip = ui.m(Metric::Tile);
        let column = (width - gap) / 2.0;
        let per_column = Role::ALL.len().div_ceil(2);

        for (index, role) in Role::ALL.iter().enumerate() {
            let entered = self.staggered(index as f32 * 0.5);
            let left = x + (index / per_column) as f32 * (column + gap);
            let at = top + (index % per_column) as f32 * step;
            let height = chip * 0.42;
            ui.card(
                [
                    left + (1.0 - entered) * gap,
                    at + (step - height) / 2.0,
                    chip,
                    height,
                ],
                Surface::Control,
                *role,
                entered,
            );
            let bright = ui.tinted(Role::Text, entered);
            let quiet = ui.tinted(Role::TextSoft, entered);
            ui.label_tinted(
                [left + chip + gap, at, column, step],
                Text::Body,
                role.name(),
                bright,
                Align::Left,
            );
            let hex = self.accent.shown().color(*role).srgb().hex();
            ui.label_tinted(
                [left, at, column, step],
                Text::Caption,
                &hex,
                quiet,
                Align::Right,
            );
        }

        top += per_column as f32 * step + gap;

        // One capsule per palette, with one light between all of them. Worked
        // out before any is drawn, because the light is laid down first and the
        // button it has arrived over hands its own face to it — which is what
        // lets the light glide across the row instead of each button colouring
        // itself in. It glides between rows the same way once there is more
        // than one, which there is: a capsule that would hang off the column
        // starts the next row instead, and one wider than the whole column
        // stays where it is because there is nowhere better for it to go.
        let capsule = ui.s(menu::ROW - 2.0 * control::PADDING);
        let mut rects = [[0.0f32; 4]; PALETTES.len()];
        let mut left = x;
        let mut line = top;
        let mut rows = 1.0f32;
        for (index, palette) in PALETTES.iter().enumerate() {
            let room = ui.measure(Text::Label, palette.name) + 2.0 * ui.m(Metric::RowPadding);
            if left > x && left + room > x + width {
                left = x;
                line += capsule + gap;
                rows += 1.0;
            }
            rects[index] = [left, line, room, capsule];
            left += room + gap;
        }
        let chosen = self.cursor[self.page].min(rects.len() - 1);
        self.glide(ui, rects[chosen], width, 1.0);
        for (index, palette) in PALETTES.iter().enumerate() {
            ui.spot(ITEM_SPOT + index as u32, rects[index]);
            ui.button_over(
                rects[index],
                palette.name,
                self.press(index == chosen),
                width,
            );
        }

        top += rows * (capsule + gap);
        let note = format!(
            "{} is what the shell has. Left and Right choose, Enter applies — and \
             nothing here is repainted when it does: the fourteen colours above \
             travel to their new values over accent-change, {:.0}ms, in linear \
             light, and the water behind the panes travels with them. An \
             application should read the shell's accent at startup and leave it \
             alone; this page moves it to show what the library does when the \
             shell's own setting is moved.",
            self.accent.applied().name,
            motion::duration::ACCENT_CHANGE * 1000.0
        );
        top += ui.paragraph([x, top, width, 0.0], &note, Role::TextSoft) + gap;
        let about = format!(
            "Those five are one control each, and a control is three things: a \
             halo, the lit capsule that glides onto the chosen one, and each \
             button's own face over the top — which fades out as the light \
             arrives, so the lit one is the one place the capsule shows \
             through. Walk the row and watch it cross rather than jump. Enter \
             presses it, over guide-press, {:.0}ms: down by a seventh of \
             itself, back past its own size, and settled — whatever the key \
             does in the meantime.",
            motion::duration::GUIDE_PRESS * 1000.0
        );
        ui.paragraph([x, top, width, 0.0], &about, Role::AccentSoft);
    }

    fn material(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        let mut top = y + self.head(ui, [x, y, width], "Material", None, false);
        let gap = ui.m(Metric::Gap);
        let cuts = Surface::ALL;
        let card = (width - (cuts.len() as f32 - 1.0) * gap) / cuts.len() as f32;
        let tall = ui.m(Metric::RowHeight) * 2.4;
        let row = ui.line(Text::Body);
        let caption = ui.line(Text::Caption);
        let pad = ui.m(Metric::RowPadding);

        for (index, surface) in cuts.iter().enumerate() {
            let glass = surface.glass();
            let entered = self.staggered(index as f32);
            let chosen = index == self.cursor[self.page];
            let left = x + index as f32 * (card + gap);
            ui.spot(ITEM_SPOT + index as u32, [left, top, card, tall]);

            // Each card is drawn in its own cut, over whatever it is standing
            // on — the material rather than a picture of it.
            ui.card(
                [left, top, card, tall],
                *surface,
                Role::Glass,
                glass.frost * 0.5,
            );
            if chosen {
                ui.card([left, top, card, tall], *surface, Role::AccentSoft, 0.18);
            }

            let mut name = surface.name().to_string();
            name[..1].make_ascii_uppercase();
            let bright = ui.tinted(Role::Text, entered);
            ui.label_tinted(
                [left + pad, top + pad, card, row],
                Text::Body,
                &name,
                bright,
                Align::Left,
            );
            for (line, (label, value)) in [
                ("depth", format!("{:.0}px", glass.depth)),
                ("frost", format!("{:.2}", glass.frost)),
                ("gloss", format!("{:.2}", glass.gloss)),
                ("curve", format!("{:.0}", glass.curve)),
            ]
            .iter()
            .enumerate()
            {
                let at = top + pad + row + line as f32 * caption;
                let quiet = ui.tinted(Role::TextSoft, entered);
                let bright = ui.tinted(Role::Text, entered);
                ui.label_tinted(
                    [left + pad, at, card, caption],
                    Text::Caption,
                    label,
                    quiet,
                    Align::Left,
                );
                ui.label_tinted(
                    [left, at, card - pad, caption],
                    Text::Caption,
                    value,
                    bright,
                    Align::Right,
                );
            }
        }

        top += tall + gap;
        top += ui.paragraph(
            [x, top, width, 0.0],
            "Three cuts, and everything is made of one of them: a compact slab for \
             what must carry text over anything, a nearly clear lozenge for what \
             you can act on, and a broad shallow sheet for a pane this size. Each \
             card above is that cut, over the page it is standing on — so the \
             Panel card is nearly opaque, the Control card nearly clear, and the \
             difference is the material rather than an alpha.",
            Role::TextSoft,
        ) + gap;
        ui.paragraph(
            [x, top, width, 0.0],
            "The two panes on this screen are the fourth thing: the layered \
             menu-and-dialog material, which is the Sidebar cut with a stain, two \
             lights under it and a hairline over it. Every one of them refracts \
             what is behind it — the water, and then the page.",
            Role::AccentSoft,
        );
    }

    fn marks(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        let mut top = y + self.head(ui, [x, y, width], "Marks", None, false);
        let gap = ui.m(Metric::Gap);
        let resting = ui.m(Metric::ItemIcon);
        let focused = ui.m(Metric::ItemIconFocused);
        let cell = resting + gap;
        let columns = (((width + gap) / cell).floor() as usize).max(1);
        let chosen = self.cursor[self.page];
        // A mark at rest and a mark the cursor is on. The jump between them is
        // the selection: nothing else about it changes size.
        let grew = self.arrived(self.cursor_since, motion::duration::FLIGHT);

        // The selected one last, because it grows over its neighbours.
        for index in (0..self.marks.len())
            .filter(|i| *i != chosen)
            .chain(Some(chosen))
        {
            let row = index / columns;
            let column = index % columns;
            let cx = x + column as f32 * cell + resting / 2.0;
            let cy = top + row as f32 * cell + resting / 2.0;
            let entered = self.staggered(index as f32 * 0.12);
            // The cell rather than the mark: a grid with gaps a pointer falls
            // through is a grid that is hard to hit, and the gap belongs to
            // the mark beside it as surely as its own middle does.
            ui.spot(
                ITEM_SPOT + index as u32,
                [
                    x + column as f32 * cell,
                    top + row as f32 * cell,
                    cell,
                    cell,
                ],
            );
            let size = if index == chosen {
                let size = resting + (focused - resting) * grew;
                ui.glow(
                    [cx - size, cy - size * 0.8, size * 2.0, size * 1.6],
                    Role::Glow,
                    0.5 * grew,
                );
                size
            } else {
                resting
            };
            ui.icon_tinted(
                [cx - size / 2.0, cy - size / 2.0, size, size],
                self.marks[index],
                self.icons,
                Role::Text,
                entered,
            );
        }

        top += self.marks.len().div_ceil(columns) as f32 * cell + gap;

        let name = self.marks[chosen];
        let line = ui.line(Text::Body);
        ui.label(
            [x, top, width, line],
            Text::Body,
            name,
            Role::Text,
            Align::Left,
        );
        let where_ = format!(
            "{} of {} · a {}-unit square · {}",
            chosen + 1,
            self.marks.len(),
            assets::glyph_box(name).unwrap_or(0),
            self.icons.name()
        );
        ui.label(
            [x, top, width, line],
            Text::Caption,
            &where_,
            Role::TextSoft,
            Align::Right,
        );

        top += line + gap;
        top += ui.paragraph(
            [x, top, width, 0.0],
            "Every one of them is shape source, not a picture: one \
             silhouette, rasterised at four times its cell, measured into a signed \
             distance field, and shaded from that. Left and Right walk them; the \
             menu changes the style.",
            Role::TextSoft,
        ) + gap;
        ui.paragraph(
            [x, top, width, 0.0],
            "This is the Default material: the wall, the reflection, the colour \
             split at the edges and each mark's own contact shadow all fall out of \
             that one field. Simple is the flat branch of the same shader. Neither \
             is the SVG — drawing that directly gets the silhouette and none of \
             this.",
            Role::AccentSoft,
        );
    }

    fn type_(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        let mut top = y + self.head(ui, [x, y, width], "Type", None, false);
        let gap = ui.m(Metric::Gap);

        // Rows of different heights, so where the light is going has to be
        // worked out before any of them is drawn.
        let chosen = self.cursor[self.page].min(Text::ALL.len() - 1);
        let mut lit = [x, top, width, ui.line(Text::ALL[0])];
        let mut at = top;
        for (index, text) in Text::ALL.iter().enumerate() {
            let step = ui.line(*text);
            if index == chosen {
                lit = [x, at, width, step];
            }
            at += step;
        }
        self.glide(ui, lit, width, 0.5);

        for (index, text) in Text::ALL.iter().enumerate() {
            let entered = self.staggered(index as f32);
            let step = ui.line(*text);
            ui.spot(ITEM_SPOT + index as u32, [x, top, width, step]);
            let bright = ui.tinted(Role::Text, entered);
            ui.label_tinted(
                [x + (1.0 - entered) * gap, top, width, step],
                *text,
                GREETING,
                bright,
                Align::Left,
            );
            let about = format!(
                "{}  {:.0}px  {}",
                text.name(),
                text.on(ui.height()),
                if text.face() == Face::Bold {
                    "bold"
                } else {
                    "regular"
                }
            );
            let quiet = ui.tinted(Role::TextSoft, entered);
            ui.label_tinted(
                [x, top, width, step],
                Text::Caption,
                &about,
                quiet,
                Align::Right,
            );
            top += step;
        }

        top += gap;
        let note = format!(
            "One family, Regular and Bold, carried in the library itself because \
             the machine this runs on may have no font service at all — the \
             renderer took the two faces straight out of the library before it \
             drew a word. Leading is {} of the size for the whole scale, never per \
             size: leading that changed per size would make two paragraphs at two \
             sizes read as two typographies. The pixel figures above are this \
             window's height, not the authored ones.",
            Text::LINE
        );
        ui.paragraph([x, top, width, 0.0], &note, Role::TextSoft);
    }

    fn motion(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        let mut top = y + self.head(ui, [x, y, width], "Motion", None, false);
        let gap = ui.m(Metric::Gap);
        let step = ui.m(Metric::RowHeight) * 0.62;
        let column = ui.m(Metric::ColumnSpacing);
        let dot = gap * 1.4;
        let rest = 0.55;
        const SPRING_NOTE: &str = "critically damped · never overshoots";

        let mut note_room = ui.measure(Text::Caption, SPRING_NOTE);
        let about: Vec<String> = SHOWN
            .iter()
            .map(|(_, seconds, what)| format!("{:.0}ms · {what}", seconds * 1000.0))
            .collect();
        for line in &about {
            note_room = note_room.max(ui.measure(Text::Caption, line));
        }
        note_room += gap;
        let track_x = x + column;
        let track_w = (width - column - note_room).max(gap);

        for (index, (name, seconds, _)) in SHOWN.iter().enumerate() {
            let entered = self.staggered(index as f32);
            let row = top + index as f32 * step;
            let chosen = index == self.cursor[self.page];
            ui.spot(ITEM_SPOT + index as u32, [x, row, width, step]);
            let tint = ui.tinted(if chosen { Role::Text } else { Role::TextSoft }, entered);
            ui.label_tinted([x, row, column, step], Text::Body, name, tint, Align::Left);

            // The same journey twice: the toolkit's easing, and the straight
            // line it is never allowed to be.
            let phase = ((self.elapsed % (seconds + rest)) / seconds).clamp(0.0, 1.0);
            ui.card(
                [track_x, row + step / 2.0 - dot / 8.0, track_w, dot / 4.0],
                Surface::Control,
                Role::GlassRaised,
                0.55 * entered,
            );
            let faint = ui.tinted(Role::TextSoft, 0.30 * entered);
            let straight = track_x + (track_w - dot) * phase;
            ui.chip(
                [
                    straight + dot * 0.1,
                    row + step / 2.0 - dot * 0.4,
                    dot * 0.8,
                    dot * 0.8,
                ],
                faint,
            );
            let lit = ui.tinted(Role::Accent, entered);
            let eased = track_x + (track_w - dot) * motion::ease(phase);
            ui.chip([eased, row + step / 2.0 - dot / 2.0, dot, dot], lit);

            let quiet = ui.tinted(Role::TextSoft, entered);
            ui.label_tinted(
                [x, row, width, step],
                Text::Caption,
                &about[index],
                quiet,
                Align::Right,
            );
        }

        top += SHOWN.len() as f32 * step;

        ui.label(
            [x, top, column, step],
            Text::Body,
            "spring",
            Role::TextSoft,
            Align::Left,
        );
        ui.card(
            [track_x, top + step / 2.0 - dot / 8.0, track_w, dot / 4.0],
            Surface::Control,
            Role::GlassRaised,
            0.55,
        );
        let soft = ui.tinted(Role::AccentSoft, 1.0);
        ui.chip(
            [
                track_x + (track_w - dot) * (self.spring_at as f32).clamp(0.0, 1.0),
                top + step / 2.0 - dot / 2.0,
                dot,
                dot,
            ],
            soft,
        );
        ui.label(
            [x, top, width, step],
            Text::Caption,
            SPRING_NOTE,
            Role::TextSoft,
            Align::Right,
        );
        top += step + gap;

        ui.paragraph(
            [x, top, width, 0.0],
            "Thirty-four durations carry a name apiece, because a number typed \
             into two places is two numbers. The faint dot is the same journey run \
             linearly, which is the one thing motion in this language is never \
             allowed to be — nothing starts at full speed and nothing stops dead. \
             The spring is for a target that moves while it is being chased; it \
             accelerates from rest and settles without ever crossing.",
            Role::TextSoft,
        );
    }

    fn sound(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        let mut top = y + self.head(ui, [x, y, width], "Sound", None, false);
        let gap = ui.m(Metric::Gap);
        let step = ui.line(Text::Body) * 1.3;

        // The light first, and it goes down with the row it is on rather than
        // staying put while the row sinks underneath it.
        let chosen = self.cursor[self.page].min(Sound::ALL.len() - 1);
        let lit = motion::pressed(
            [x, top + chosen as f32 * step, width, step],
            self.pressing.through(),
        );
        self.glide(ui, lit, width, 1.0);

        for (index, sound) in Sound::ALL.iter().enumerate() {
            let entered = self.staggered(index as f32 * 0.5);
            let row = top + index as f32 * step;
            ui.spot(ITEM_SPOT + index as u32, [x, row, width, step]);
            let chosen = index == self.cursor[self.page];
            // Everything on the row goes down with the light on it, or the
            // words read as a hole opening behind the row.
            let rect = motion::pressed(
                [x, row, width, step],
                chosen.then(|| self.pressing.through()).flatten(),
            );
            let row = rect[1];
            let bytes = sound.bytes().unwrap_or_default();
            let tint = ui.tinted(if chosen { Role::Text } else { Role::TextSoft }, entered);
            let pad = ui.m(Metric::RowPadding);
            ui.label_tinted(
                [x + pad + (1.0 - entered) * gap, row, width, step],
                Text::Body,
                sound.name(),
                tint,
                Align::Left,
            );
            let mut about = if bytes.len() >= 1024 * 1024 {
                format!("{:.1} MiB", bytes.len() as f32 / (1024.0 * 1024.0))
            } else {
                format!("{:.1} KiB", bytes.len() as f32 / 1024.0)
            };
            if !sound.used_by_shell() {
                about.push_str("  ·  the shell never plays it");
            }
            let quiet = ui.tinted(Role::TextSoft, entered);
            ui.label_tinted(
                [x, row, width - pad, step],
                Text::Caption,
                &about,
                quiet,
                Align::Right,
            );
        }

        top += Sound::ALL.len() as f32 * step + gap;
        let note = "Fourteen recordings, Ogg Vorbis, carried in the library. Enter \
             plays the selected one — the last row loops and is turned on and off \
             instead, because it is the one sound here that answers nothing. Two \
             of them are kept for compatibility and the shell never plays them.";
        top += ui.paragraph([x, top, width, 0.0], note, Role::TextSoft) + gap;
        let about = format!(
            "The library says which recording answers which press and when \
             nothing should be heard at all; lxb-sound opens the machine's output \
             and puts the clip on it. A {:.0}ms rest is the shortest gap left \
             between two copies of one recording, so that a held direction walks \
             this list to a run of clicks rather than to one very loud one — \
             identical samples laid over each other add in phase. Every click you \
             have heard walking these pages came from here.",
            lxb_toolkit::sound::REST * 1000.0
        );
        ui.paragraph([x, top, width, 0.0], &about, Role::AccentSoft);
    }

    fn picker_page(&mut self, ui: &mut Ui, x: f32, y: f32, width: f32) {
        let mut top = y + self.head(
            ui,
            [x, y, width],
            "File and folder picker",
            Some("file-folder"),
            true,
        );
        let gap = ui.m(Metric::Gap);
        top += ui.paragraph(
            [x, top, width, 0.0],
            "One call opens a centred Lattice window covering roughly seventy percent of \
             this application. Folder columns recede along the trail while strong frost and \
             depth put this page behind it. A on Search opens its controller keyboard; \
             Start finishes; B or its hide key returns without losing the query. Four \
             questions, and each grows the head rows it needs: New folder wherever \
             something may be written, a name above Save here, and one row that hands \
             over everything ticked. An application asks lxb-app rather than this, and \
             lxb-app puts the question to the desktop's own chooser when there is one.",
            Role::TextSoft,
        ) + gap;

        let labels: Vec<&str> = PICKER_PURPOSES
            .iter()
            .map(|(_, _, label, _)| *label)
            .collect();
        let row = ui.m(Metric::RowHeight) * 0.74;
        let chosen = self.cursor[self.page].min(labels.len() - 1);
        let lit = motion::pressed(
            [x, top + chosen as f32 * (row + gap), width, row],
            self.pressing.through(),
        );
        self.glide(ui, lit, width, 1.0);
        for (index, label) in labels.iter().enumerate() {
            let rect = [x, top + index as f32 * (row + gap), width, row];
            ui.spot(ITEM_SPOT + index as u32, rect);
            ui.button(rect, label, self.press(index == chosen));
        }
        top += labels.len() as f32 * (row + gap) + gap;

        let result = match self.picked.as_slice() {
            _ if self.asking_the_desktop() => {
                "The question is with the desktop's own chooser, in a window of its \
                 own. This one only draws it where the session has no portal to ask."
                    .to_string()
            }
            [] => "No choice yet — the fixture is safe to explore.".to_string(),
            [one] => format!("Last choice: {}", one.display()),
            many => format!(
                "Last choice: {} files — {}",
                many.len(),
                many.iter()
                    .filter_map(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        ui.paragraph([x, top, width, 0.0], &result, Role::AccentSoft);
    }
}

fn picker_root() -> PathBuf {
    std::env::var_os("LXB_TOUR_FILES")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Keep the same clean fixture spelling C and Python use. Leaving
            // a literal `rust/..` here makes the picker build one phantom
            // ancestor column, so the three tours no longer show the same
            // settled Lattice frame.
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("the Rust example has an examples parent")
                .join("tour-files")
        })
}

// --- input -----------------------------------------------------------------

impl Tour {
    /// One action, and the sound it is owed.
    ///
    /// Nothing here asks which control sent it. That is the whole of what
    /// makes a keyboard, a controller and a wheel one interface rather than
    /// three: a direction is a direction, and the row it lands on answers the
    /// same way whichever hand moved it.
    ///
    /// The answer is a sound rather than nothing, because a move that landed
    /// and a move that pushed into an edge are two different things and the
    /// ear is where the difference is said. A press that starts nothing stays
    /// silent, and so does a move that landed nowhere.
    pub fn on_action(&mut self, action: Action) -> Option<Sound> {
        // Innermost first. A panel that is up owns every action, which is what
        // makes it a panel: a question about leaving must not be answerable by
        // something meant for the page behind it.
        if self.files.is_open() {
            return self.files.act(action);
        }
        if self.dialog.is_open() {
            return self.on_dialog(action);
        }
        if self.menu.is_open() {
            return self.on_menu(action);
        }
        match action {
            // Up and Down are always the page, and so are the shoulders: they
            // are the sideways move a controller has, and the pages are the
            // one list this application is.
            Action::Up | Action::Previous => self.move_page(-1),
            Action::Down | Action::Next => self.move_page(1),
            // Left and Right are always within it.
            Action::Left => self.walk(-1),
            Action::Right => self.walk(1),
            Action::Accept | Action::Submit => {
                self.press_down();
                self.act()
            }
            Action::Menu => {
                self.open_menu();
                Some(Sound::Press)
            }
            // Nothing left to step out of, so this is the way out — asked
            // rather than taken, and a question arriving is a press this
            // screen kept rather than a step back out of anything.
            Action::Back => {
                self.ask();
                Some(Sound::Press)
            }
        }
    }

    /// What the last question was answered with.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn picked(&self) -> &[PathBuf] {
        &self.picked
    }

    /// Whether the chooser this program draws is the one answering.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn picker_open(&self) -> bool {
        self.files.is_open()
    }

    /// One key, as the platform delivered it. Answers whether the chooser
    /// wanted it.
    pub fn picker_key(&mut self, key: Option<Key>, text: Option<&str>) -> bool {
        self.files.key(key, text)
    }

    fn on_dialog(&mut self, action: Action) -> Option<Sound> {
        match action {
            Action::Left => self.dialog.step(-1),
            Action::Right => self.dialog.step(1),
            Action::Accept | Action::Submit => return self.choose_answer(),
            Action::Back => {
                self.dialog.close();
                return Some(Sound::Back);
            }
            // A question is modal: everything else is swallowed rather than
            // reaching the page behind it.
            _ => return None,
        }
        Some(Sound::Move)
    }

    fn on_menu(&mut self, action: Action) -> Option<Sound> {
        match action {
            Action::Up => self.menu.step(-1),
            Action::Down => self.menu.step(1),
            // Sideways is onto the row's own button and off it again, and the
            // rows here have none — so it is a move that landed nowhere,
            // which is answered by nothing.
            Action::Left | Action::Right => {
                let delta = if action == Action::Left { -1 } else { 1 };
                return self.menu.step_aside(delta).then_some(Sound::Move);
            }
            Action::Accept | Action::Submit => return self.choose_menu_row(),
            // Back closes it, and so does the button that raised it: the one
            // control that opens a menu is the one that puts it away.
            Action::Back | Action::Menu => {
                self.menu.close();
                return Some(Sound::Back);
            }
            _ => return None,
        }
        Some(Sound::Move)
    }

    /// The pointer resting at a spot: select what is under it where selecting
    /// is what pointing at a thing means, and say nothing while doing it.
    ///
    /// Silent on purpose. Hovering is not a move — a pointer swept across the
    /// screen would otherwise be a stream of clicks — and the page's own lists
    /// do not follow it at all: they move in two steps, so that a folder never
    /// opens under a pointer that was only crossing the screen.
    pub fn point_at(&mut self, spot: Spot) {
        if self.files.is_open() {
            // A directory row is deliberately inert under hover. This is the
            // same two-step pointer gesture as the shell lattice: click once
            // to bring focus over, then again to activate.
            self.files.point_at(spot);
            return;
        }
        if self.dialog.is_open() {
            self.dialog.point_at(spot);
            return;
        }
        if self.menu.is_open() {
            self.menu.point_at(spot);
        }
    }

    /// A button going down over `spot`. `menu` is the right one, which is the
    /// same control as the pad's top face button and means the same thing.
    ///
    /// On the way down rather than the way up, like every other control in
    /// this language: a row activated on the release fires twice on a chord.
    pub fn press_at(&mut self, spot: Spot, menu: bool) -> Option<Sound> {
        // A menu is *about* something, and the something has to be the thing
        // that was pointed at — one raised over whatever happened to be
        // selected already would be a list of things to do to something the
        // user is not pointing at. So the selection is carried there first,
        // and then the same action the pad's button sends is sent.
        if self.files.is_open() {
            return self.files.press_at(spot, menu);
        }
        if menu {
            self.aim_at(spot);
            return self.on_action(Action::Menu);
        }
        match spot {
            Spot::DialogButton(_) if self.dialog.point_at(spot) => {
                self.dialog.press();
                self.choose_answer()
            }
            Spot::MenuRow { .. } if self.menu.point_at(spot) => {
                self.menu.press();
                self.choose_menu_row()
            }
            // Outside an open panel is where a press dismisses it, which is
            // what a press outside a panel has meant on every desktop there
            // has ever been.
            Spot::OutsidePanel => {
                self.menu.close();
                Some(Sound::Back)
            }
            _ if self.dialog.is_open() || self.menu.is_open() => None,
            // The page's own lists move in two steps: the first press carries
            // the selection to the pointer — and sounds, because a click that
            // puts the selection on a row is the same move as the direction
            // that would have walked there — and the second presses what has
            // arrived.
            //
            // The list of pages beside them is the exception, and it is the
            // same rule read the other way: going to a page *is* what pressing
            // its row does, so a second press on the row already stood on is
            // not a second thing to do. It certainly is not a press of
            // whatever the page it named happens to be pointing at.
            Spot::Control(_) if self.aim_at(spot) => Some(Sound::Move),
            Spot::Control(id) if id >= ITEM_SPOT => {
                self.press_down();
                self.act()
            }
            Spot::Control(_) => None,
            _ => None,
        }
    }

    /// Put the selection outright on whatever is under the pointer, wherever
    /// pointing at a thing is not already what selects it. Answers whether it
    /// moved.
    fn aim_at(&mut self, spot: Spot) -> bool {
        if self.files.is_open() {
            return false;
        }
        if self.dialog.is_open() {
            return false;
        }
        if self.menu.is_open() {
            let was = (self.menu.selected(), self.menu.on_aside());
            self.menu.point_at(spot);
            return was != (self.menu.selected(), self.menu.on_aside());
        }
        let Spot::Control(id) = spot else {
            return false;
        };
        if (PAGE_SPOT..ITEM_SPOT).contains(&id) {
            return self.choose((id - PAGE_SPOT) as usize);
        }
        let item = (id - ITEM_SPOT) as usize;
        if item >= self.count() || self.cursor[self.page] == item {
            return false;
        }
        self.cursor[self.page] = item;
        self.cursor_since = self.elapsed;
        true
    }

    /// Which page the tour is on. Answers whether it moved: a list walked into
    /// its own end has not, and an edge somebody has pushed into is answered
    /// by nothing happening.
    pub fn move_page(&mut self, delta: isize) -> Option<Sound> {
        let pages = PAGES.len() as isize;
        let page = ((self.page as isize + delta).rem_euclid(pages)) as usize;
        self.choose(page).then_some(Sound::Move)
    }

    /// Left and Right are always within the page.
    pub fn walk(&mut self, delta: isize) -> Option<Sound> {
        let total = self.count() as isize;
        if total <= 1 {
            return None;
        }
        let cursor = &mut self.cursor[self.page];
        *cursor = ((*cursor as isize + delta).rem_euclid(total)) as usize;
        self.cursor_since = self.elapsed;
        Some(Sound::Move)
    }

    /// Go to a page. Answers whether that was a move.
    pub fn choose(&mut self, page: usize) -> bool {
        if page >= PAGES.len() || page == self.page {
            return false;
        }
        self.page = page;
        self.entered = self.elapsed;
        self.cursor_since = self.elapsed;
        true
    }

    pub fn open_menu(&mut self) {
        // A menu is about the thing it was raised over, so it is titled with
        // what that thing is and grows out of that thing's own rectangle.
        let entries = vec![
            Entry::new(format!(
                "{} marks",
                if self.icons == IconStyle::Default {
                    "Simple"
                } else {
                    "Default"
                }
            ))
            .glyph("setting-icons"),
            Entry::new("Ask a question").glyph("setting-info"),
            Entry::new("Copy this page's values")
                .glyph("copy")
                .disabled(),
            // Its own band, because leaving is not one of the things above it.
            Entry::new("Close").glyph("arrow-left").group(1),
        ];
        // How many rows this display has room for, which is the one thing the
        // panel cannot work out for itself.
        self.menu.set_window(lxb_toolkit::menu::rows_that_fit(
            self.height,
            lxb_toolkit::menu::ROW,
        ));
        self.menu.open_at(
            self.rows[self.page],
            Some(PAGES[self.page].to_string()),
            entries,
        );
    }

    /// Carry out the chosen row of the menu.
    ///
    /// Read before it is carried out, because carrying it out is what makes
    /// the answer untrue: closing the menu is the first thing every one of
    /// these does.
    fn choose_menu_row(&mut self) -> Option<Sound> {
        let chosen = self.menu.selected();
        let allowed = self.menu.chosen().is_some();
        self.menu.close();
        match chosen {
            0 => {
                self.icons = if self.icons == IconStyle::Default {
                    IconStyle::Simple
                } else {
                    IconStyle::Default
                }
            }
            1 => self.ask(),
            // Copy is disabled, and Close is the row that only closes.
            _ => {}
        }
        allowed.then_some(Sound::Press)
    }

    fn choose_answer(&mut self) -> Option<Sound> {
        if self.dialog.selected() == 1 {
            self.leave = true;
        }
        self.dialog.close();
        Some(Sound::Press)
    }

    /// Enter, on whatever is selected.
    pub fn act(&mut self) -> Option<Sound> {
        if self.files.is_open() {
            return self.files.act(Action::Accept);
        }
        if self.dialog.is_open() {
            return self.choose_answer();
        }
        if self.menu.is_open() {
            return self.choose_menu_row();
        }
        match self.page {
            0 => {
                self.ask();
                Some(Sound::Press)
            }
            1 => {
                let name = PALETTES[self.cursor[self.page].min(PALETTES.len() - 1)].name;
                self.accent.preview(name);
                self.accent.commit(name);
                Some(Sound::Press)
            }
            // The Sound page answers with the recording the row is about,
            // instead of with the press that would ordinarily answer it. It is
            // the one page where the press *is* the sound.
            6 => {
                let sound = Sound::ALL[self.cursor[self.page].min(Sound::ALL.len() - 1)];
                if sound.loops() {
                    // The one recording that is not an answer to anything is
                    // not started by a press either: it is turned on and off.
                    self.music = !self.music;
                    return None;
                }
                Some(sound)
            }
            7 => {
                let asked = self.cursor[self.page].min(PICKER_PURPOSES.len() - 1);
                self.open_picker(asked).then_some(Sound::Press)
            }
            // A press that starts nothing stays silent.
            _ => None,
        }
    }

    /// Whether the looping recording should be playing.
    pub fn music(&self) -> bool {
        self.music
    }

    /// Which recording the Sound page is standing on.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn selected_sound(&self) -> usize {
        self.cursor[6].min(Sound::ALL.len() - 1)
    }

    /// Raise the panel's own menu, for a headless picture of it.
    ///
    /// Only ever after the panel has been drawn once: the menu hangs off the
    /// word "Options" on the legend, and where that word is is something only
    /// the drawing knows.
    pub fn open_picker_menu(&mut self) -> bool {
        self.files.open_menu()
    }

    /// Put one of the Picker page's questions.
    ///
    /// Public for the headless screenshot path; regular interaction reaches it
    /// by pressing one of the controls on that page. Where it is *answered* is
    /// not this program's business: `Files` puts the question to whatever
    /// chooser the session already has, and draws one here only when there is
    /// none to put it to.
    pub fn open_picker(&mut self, which: usize) -> bool {
        if self.menu.is_open() || self.dialog.is_open() {
            return false;
        }
        let asked = PICKER_PURPOSES.get(which).unwrap_or(&PICKER_PURPOSES[0]);
        self.files.ask(asked.0, asked.1, &self.picker_root, asked.3)
    }

    fn ask(&mut self) {
        self.dialog.ask(
            "Leave the tour?",
            "Nothing is closed yet. This is the general dialog: the same layered \
             pane as the context menu, differing only in its width, its position, \
             the scrim behind it and how it arrives.",
            vec!["Stay".to_string(), "Leave".to_string()],
            // A question opens on its safe answer.
            0,
        );
    }
}
