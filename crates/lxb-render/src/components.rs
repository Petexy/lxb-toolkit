use lxb_toolkit::{
    control,
    material::{modal, optics, Overlay, Surface},
    menu,
    metrics::{capsule_radius, Metric},
    motion,
    palette::Role,
    picker::{
        EntryKind, Picker, Purpose as PickerPurpose, Selection as PickerSelection,
        Sort as PickerSort,
    },
    settings::IconStyle,
    typography::{Face, Text},
};

use xkbcommon::xkb;

use crate::renderer::{Quad, Run, Ui, OVER, PANE, SOFTEN};
use crate::{Align, Fit, Spot};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Press {
    #[default]
    Resting,

    Focused,

    Pressed,

    Going(f32),
}

impl Press {
    pub fn lit(self) -> bool {
        self != Press::Resting
    }

    pub fn through(self) -> Option<f32> {
        match self {
            Press::Resting | Press::Focused => None,
            Press::Pressed => Some(motion::PRESS_DOWN),
            Press::Going(t) => Some(t),
        }
    }
}

impl From<bool> for Press {
    fn from(lit: bool) -> Self {
        if lit {
            Press::Focused
        } else {
            Press::Resting
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Pressing {
    at: Option<f32>,
}

impl Pressing {
    pub fn press(&mut self) {
        self.at = Some(0.0);
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        let Some(at) = self.at else {
            return false;
        };
        let at = at + dt / motion::duration::GUIDE_PRESS;
        self.at = (at < 1.0).then_some(at);
        self.at.is_some()
    }

    pub fn through(self) -> Option<f32> {
        self.at
    }

    pub fn state(self, lit: bool) -> Press {
        match self.at {
            Some(at) if lit => Press::Going(at),
            _ => Press::from(lit),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Selection {
    at: Option<[f32; 4]>,
    speed: [f32; 4],
}

impl Selection {
    pub fn glide(&mut self, target: [f32; 4], dt: f32) -> [f32; 4] {
        let Some(current) = self.at else {
            self.speed = [0.0; 4];
            self.at = Some(target);
            return target;
        };
        let mut next = [0.0; 4];
        for (index, slot) in next.iter_mut().enumerate() {
            let (at, moving) = motion::spring(
                current[index] as f64,
                self.speed[index] as f64,
                target[index] as f64,
                motion::HIGHLIGHT_SPRING,
                dt as f64,
            );
            *slot = at as f32;
            self.speed[index] = moving as f32;
        }
        self.at = Some(next);
        next
    }

    pub fn rect(self) -> Option<[f32; 4]> {
        self.at
    }

    pub fn gliding(self) -> bool {
        self.speed.iter().any(|speed| speed.abs() > 0.5)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Ui {
    /// Blur and fade the ends of a scrolling viewport away into the page.
    ///
    /// A post-composite effect: it samples the complete page after both its
    /// surfaces and its words have been drawn — so a card, its mark and its
    /// names soften as one picture — and stays behind menus and dialogs. The
    /// middle of the viewport is untouched.
    ///
    /// **It ends in whatever is behind the page's own content** — the panes it
    /// drew over the ground, or the bare ground where it drew none. Blur and
    /// translucency grow together towards each end, and the last of it is
    /// exactly what would be there if the list were not, so a list stops
    /// without anything to stop at. Ending at a colour instead leaves a band
    /// that stops dead, which reads as a shadow however carefully the colour
    /// is chosen.
    ///
    /// `band` is the feather in points. `top` and `bottom` are how much really
    /// continues past each end: an end with nothing beyond it is nought and is
    /// left crisp, and an end part of the way there fades over a *narrower*
    /// band rather than a fainter one, because a fade that stopped short of
    /// the ground would stop at a line.
    pub fn soft_edges(&mut self, viewport: [f32; 4], band: f32, top: f32, bottom: f32) {
        if viewport[2] <= 0.0 || viewport[3] <= 0.0 || band <= 0.0 || (top <= 0.0 && bottom <= 0.0)
        {
            return;
        }
        let band = band.min(viewport[3] * 0.5);
        self.quad(
            SOFTEN,
            Quad::soft_vertical_edges(viewport, band, top, bottom),
        );
    }

    pub fn pane(&mut self, rect: [f32; 4], overlay: Overlay) {
        self.pane_at(rect, overlay, 1.0, PANE);
    }

    pub(crate) fn pane_at(&mut self, rect: [f32; 4], overlay: Overlay, alpha: f32, layer: usize) {
        let material = overlay.material();
        let [x, y, width, height] = rect;
        let radius = self.s(material.radius);
        let under = layer.saturating_sub(1);

        let head = self
            .s(material.header_height)
            .min(material.header_height_share * height);
        let foot = self
            .s(material.foot_height)
            .min(material.foot_height_share * height);
        let inset = self.s(material.light_inset);

        let header = self.tinted(material.header_role, material.header_light * alpha);
        self.quad(
            under,
            Quad::light(
                [
                    x + material.header_x * width,
                    y + inset,
                    material.header_width * width,
                    head,
                ],
                header,
            ),
        );
        let footer = self.tinted(material.foot_role, material.foot_light * alpha);
        self.quad(
            under,
            Quad::light(
                [
                    x + material.foot_x * width,
                    y + height - inset - foot,
                    material.foot_width * width,
                    foot,
                ],
                footer,
            ),
        );

        let rim = self.s(material.rim_width).max(1.0);
        let edge = self.tinted(material.rim_role, material.rim * alpha);
        self.quad(
            layer,
            Quad::solid(
                [x - rim, y - rim, width + 2.0 * rim, height + 2.0 * rim],
                radius + rim,
                edge,
            ),
        );

        let tint = self.tinted(material.stain_role, material.stain * alpha);
        let scale = self.scale;
        self.quad(
            layer,
            Quad::glass(rect, radius, tint, Surface::Sidebar.glass(), scale),
        );
    }

    pub fn card(&mut self, rect: [f32; 4], surface: Surface, role: Role, alpha: f32) {
        let tint = self.tinted(role, alpha);
        let scale = self.scale;
        let radius = self.m(Metric::CardRadius);
        let layer = self.layer();
        self.quad(
            layer,
            Quad::glass(rect, radius, tint, surface.glass(), scale),
        );
    }

    pub fn lit(&mut self, rect: [f32; 4], radius: f32, role: Role, over: f32, strength: f32) {
        self.lit_tall(
            rect,
            radius,
            role,
            over,
            rect[3] * control::GLOW_HEIGHT,
            strength,
        );
    }

    pub fn lit_tall(
        &mut self,
        rect: [f32; 4],
        radius: f32,
        role: Role,
        over: f32,
        tall: f32,
        strength: f32,
    ) {
        if strength <= 0.0 {
            return;
        }
        let layer = self.layer();
        let seconds = self.scene.time;
        let glow = self.tinted(role, control::glow_alpha(seconds) * strength);
        self.quad(
            layer,
            Quad::light(control::glow_rect_tall(rect, over, tall), glow),
        );

        // **`strength` fades the quad, and does not merely stain it.** A glass
        // quad's material is not scaled by its own tint: the refraction, the
        // gloss and the rim are all there at a tint alpha of a thousandth, and
        // a light asked for at almost nothing came out as a hard ring that then
        // went out in one frame. Putting it in `Quad::faded` — `shape[3]`, the
        // one channel the glass branch of the shader multiplies through — is
        // what `Ui::control` has always done with its own. Nothing moves at
        // full strength, which is what everything but a page crossing asks for.
        let tint = self.tinted(role, control::lit_alpha(seconds));
        let scale = self.scale;
        self.quad(
            layer,
            Quad::glass(rect, radius, tint, control::lit(), scale).faded(strength),
        );
        self.light = Some(rect);
    }

    pub fn control(
        &mut self,
        rect: [f32; 4],
        radius: f32,
        press: Press,
        over: f32,
        alpha: f32,
    ) -> [f32; 4] {
        let sunk = motion::pressed(rect, press.through());
        let arrived = match self.light {
            Some(light) => control::arrival(light, rect),

            None if press.lit() => {
                self.lit(sunk, radius, control::LIT_ROLE, over, alpha);
                self.light = None;
                1.0
            }
            None => 0.0,
        };
        let layer = self.layer();
        let tint = self.tinted(control::CHIP_ROLE, control::CHIP_TINT);
        let scale = self.scale;
        self.quad(
            layer,
            Quad::glass(sunk, radius, tint, control::chip(), scale).faded(alpha * (1.0 - arrived)),
        );
        sunk
    }

    pub fn control_out(&mut self, rect: [f32; 4], radius: f32, alpha: f32) {
        let layer = self.layer();
        let tint = self.tinted(control::OUT_ROLE, control::OUT * alpha);
        let width = self.s(control::OUT_WIDTH).max(1.0);
        self.quad(layer, Quad::outline(rect, radius, tint, width));
    }

    pub fn button(&mut self, rect: [f32; 4], label: &str, press: impl Into<Press>) {
        self.button_over(rect, label, press, rect[2]);
    }

    pub fn button_over(&mut self, rect: [f32; 4], label: &str, press: impl Into<Press>, over: f32) {
        let press = press.into();
        let radius = capsule_radius(rect[3]);
        let sunk = self.control(rect, radius, press, over, 1.0);

        let lit = press.lit();
        let ink = self.tinted(
            Role::Text,
            if lit {
                control::INK
            } else {
                control::INK_QUIET
            },
        );
        self.label_weighted(sunk, Text::Label, label, ink, Align::Centre, lit);
    }

    pub fn row(
        &mut self,
        rect: [f32; 4],
        name: &str,
        value: Option<&str>,
        press: impl Into<Press>,
    ) {
        let press = press.into();
        let sunk = self.control(rect, capsule_radius(rect[3]), press, rect[2], 1.0);
        let pad = self.m(Metric::RowPadding);
        let [x, y, width, height] = sunk;
        let ink = if press.lit() {
            Role::Text
        } else {
            Role::TextSoft
        };
        match value {
            None => self.label(
                [x + pad, y, width - 2.0 * pad, height],
                Text::Body,
                name,
                ink,
                Align::Left,
            ),
            Some(value) => {
                let top = self.line(Text::Body);
                self.label(
                    [x + pad, y, width - 2.0 * pad, top],
                    Text::Body,
                    name,
                    ink,
                    Align::Left,
                );
                self.label(
                    [x + pad, y + top, width - 2.0 * pad, height - top],
                    Text::Caption,
                    value,
                    Role::TextSoft,
                    Align::Left,
                );
            }
        }
    }

    pub fn selection(&mut self, rect: [f32; 4], strength: f32) {
        self.lit(
            rect,
            capsule_radius(rect[3]),
            control::LIT_ROLE,
            rect[2],
            strength,
        );
    }

    pub fn chip(&mut self, rect: [f32; 4], tint: [f32; 4]) {
        let radius = capsule_radius(rect[3].min(rect[2]));
        let layer = self.layer();
        self.quad(layer, Quad::solid(rect, radius, tint));
    }

    pub fn glow(&mut self, rect: [f32; 4], role: Role, amount: f32) {
        let tint = self.tinted(role, amount);
        let layer = self.layer();
        self.quad(layer, Quad::light(rect, tint));
    }

    pub fn rule(&mut self, rect: [f32; 4], role: Role) {
        let tint = self.role(role);
        let layer = self.layer();
        self.quad(layer, Quad::solid(rect, 0.0, tint));
    }

    /// How wide a bar down the edge of a list is drawn.
    ///
    /// One number for every application, because it is a control as much as a
    /// mark: somebody who has learnt to catch one has learnt to catch them
    /// all. Wide enough to take hold of, and no wider — the bar is beside the
    /// reading, not part of it.
    pub fn scroll_bar_width(&self) -> f32 {
        self.s(SCROLL_BAR)
    }

    /// A bar down the edge of a list: how much of it is on the screen, where
    /// in it that is, and something for a pointer to take hold of. Answers
    /// where the thumb came out, which is what a page marks as its spot.
    ///
    /// `at` is how far down the list the top of what is showing has reached
    /// and `run` is the share of the list that is showing, both from nought to
    /// one. Rows rather than pixels, because rows are what a page knows about
    /// its own list.
    ///
    /// **Nothing is drawn for a pad.** A controller has no use for it — the
    /// light is already saying where in the list it is, and a bar beside it
    /// would be a control nothing on the pad can reach. That is the page's
    /// call, not this one's: ask `Page::pad_in_hand`.
    ///
    /// The thumb is never shorter than the track is wide, twice over, or a
    /// list of a thousand would offer a pointer nothing to catch.
    pub fn scroll_bar(&mut self, track: [f32; 4], at: f32, run: f32, lit: bool) -> [f32; 4] {
        let thumb = scroll_thumb(track, at, run);
        self.chip(track, self.tinted(Role::Glass, SCROLL_TROUGH));
        self.chip(
            thumb,
            self.tinted(
                if lit { Role::Accent } else { Role::TextSoft },
                if lit { 1.0 } else { SCROLL_THUMB },
            ),
        );
        thumb
    }

    pub fn picture(
        &mut self,
        rect: [f32; 4],
        radius: f32,
        path: impl AsRef<std::path::Path>,
        fit: Fit,
        opacity: f32,
    ) -> bool {
        let Some(shown) = self.thumbnail(path.as_ref()) else {
            return false;
        };
        let Some((rect, cell)) = fitted(shown, rect, fit) else {
            return false;
        };
        let layer = self.layer();
        self.quad(layer, Quad::image(rect, radius, cell, opacity));
        true
    }

    pub fn picture_aspect(&mut self, path: impl AsRef<std::path::Path>) -> Option<f32> {
        self.thumbnail(path.as_ref())
            .map(|shown| shown.aspect)
            .filter(|aspect| aspect.is_finite() && *aspect > 0.0)
    }

    pub fn icon(&mut self, rect: [f32; 4], name: &str, style: IconStyle) {
        self.icon_tinted(rect, name, style, Role::Text, 1.0);
    }

    pub fn icon_tinted(
        &mut self,
        rect: [f32; 4],
        name: &str,
        style: IconStyle,
        role: Role,
        alpha: f32,
    ) {
        let Some(index) = self.mark_index(name) else {
            return;
        };
        let tint = self.tinted(role, alpha);
        let cell = self.cell(index);

        let layer = self.layer();
        self.quad(
            layer,
            Quad {
                rect,
                shape: [
                    0.0,
                    crate::renderer::KIND_GLYPH,
                    if style == IconStyle::Simple { 1.0 } else { 0.0 },
                    1.0,
                ],
                tint,
                material: [0.0, 0.0, Surface::Control.glass().gloss, 0.0],
                cell,
                cut: crate::renderer::NO_CUT,
            },
        );
    }

    pub fn label(&mut self, rect: [f32; 4], text: Text, string: &str, role: Role, align: Align) {
        self.label_tinted(rect, text, string, self.tinted(role, 1.0), align);
    }

    pub fn label_tinted(
        &mut self,
        rect: [f32; 4],
        text: Text,
        string: &str,
        tint: [f32; 4],
        align: Align,
    ) {
        self.label_weighted(rect, text, string, tint, align, text.face() == Face::Bold);
    }

    pub fn label_weighted(
        &mut self,
        rect: [f32; 4],
        text: Text,
        string: &str,
        tint: [f32; 4],
        align: Align,
        bold: bool,
    ) {
        self.label_weighted_clipped(rect, text, string, tint, align, bold, None);
    }

    #[allow(clippy::too_many_arguments)]
    fn label_weighted_clipped(
        &mut self,
        rect: [f32; 4],
        text: Text,
        string: &str,
        tint: [f32; 4],
        align: Align,
        bold: bool,
        clip: Option<[f32; 4]>,
    ) {
        self.label_weighted_sized_clipped(
            rect,
            text,
            string,
            tint,
            align,
            bold,
            self.size(text),
            clip,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn label_weighted_sized_clipped(
        &mut self,
        rect: [f32; 4],
        _text: Text,
        string: &str,
        tint: [f32; 4],
        align: Align,
        bold: bool,
        size: f32,
        clip: Option<[f32; 4]>,
    ) {
        if string.is_empty() {
            return;
        }
        let [x, y, width, height] = rect;
        let measured = self.shaped_width(string, size, bold);
        let left = match align {
            Align::Left => x,
            Align::Centre => x + (width - measured) / 2.0,
            Align::Right => x + width - measured,
        };
        let layer = self.layer();
        self.run(
            layer,
            Run {
                text: string.to_string(),
                left,
                top: y + (height - size * Text::LINE) / 2.0,
                width,
                lines: 1,
                size,
                bold,
                tint,
                clip,
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn label_stacked(
        &mut self,
        at: [f32; 2],
        width: f32,
        text: Text,
        string: &str,
        tint: [f32; 4],
        bold: bool,
        lines: u8,
    ) {
        if string.is_empty() || width <= 0.0 {
            return;
        }
        let size = self.size(text);
        let layer = self.layer();
        self.run(
            layer,
            Run {
                text: string.to_string(),
                left: at[0],
                top: at[1],
                width,
                lines: lines.max(1),
                size,
                bold,
                tint,
                clip: None,
            },
        );
    }

    pub fn paragraph(&mut self, rect: [f32; 4], string: &str, role: Role) -> f32 {
        let [x, y, width, _] = rect;
        let step = self.line(Text::Caption);
        let lines = self.wrap(Text::Caption, string, width);
        for (index, line) in lines.iter().enumerate() {
            self.label(
                [x, y + index as f32 * step, width, step],
                Text::Caption,
                line,
                role,
                Align::Left,
            );
        }
        step * lines.len() as f32
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub label: String,

    pub detail: Option<String>,

    pub stamp: Option<String>,

    pub glyph: Option<&'static str>,

    pub aside: Option<&'static str>,

    pub group: u8,

    pub enabled: bool,

    pub grave: bool,

    pub reading: bool,

    pub lines: u8,
    pub detail_lines: u8,
}

impl Entry {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: None,
            stamp: None,
            glyph: None,
            aside: None,
            group: 0,
            enabled: true,
            grave: false,
            reading: false,
            lines: 1,
            detail_lines: 1,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn stamp(mut self, stamp: impl Into<String>) -> Self {
        self.stamp = Some(stamp.into());
        self
    }

    pub fn glyph(mut self, glyph: &'static str) -> Self {
        self.glyph = Some(glyph);
        self
    }

    pub fn aside(mut self, glyph: &'static str) -> Self {
        self.aside = Some(glyph);
        self
    }

    pub fn group(mut self, group: u8) -> Self {
        self.group = group;
        self
    }

    pub fn grave(mut self) -> Self {
        self.grave = true;
        self
    }

    pub fn reading(mut self) -> Self {
        self.reading = true;
        self
    }

    pub fn lines(mut self, label: u8, detail: u8) -> Self {
        self.lines = label.max(1);
        self.detail_lines = detail.max(1);
        self
    }

    fn shape(&self) -> menu::Row {
        menu::Row {
            stacked: self.detail.is_some(),
            stamp: self.stamp.is_some(),
            aside: self.aside.is_some(),
            reading: self.reading,
            group: self.group,
            lines: self.lines,
            detail_lines: self.detail_lines,
        }
    }
}

#[derive(Debug, Default)]
pub struct ContextMenu {
    entries: Vec<Entry>,
    title: Option<String>,

    title_lines: u8,
    anchor: [f32; 4],
    selected: usize,

    on_aside: bool,

    first: usize,
    window: usize,

    extra: f32,
    open: bool,

    out: f32,

    unfolded: f32,

    light: Selection,

    pressing: Pressing,
    aside_pressing: Pressing,
}

impl ContextMenu {
    pub fn open_at(
        &mut self,
        anchor: [f32; 4],
        title: Option<String>,
        entries: Vec<Entry>,
    ) -> bool {
        if !entries.iter().any(|entry| entry.enabled) {
            return false;
        }
        self.selected = entries.iter().position(|entry| entry.enabled).unwrap_or(0);
        self.window = self.window.max(1).min(entries.len().max(1));
        self.entries = entries;
        self.title_lines = self.title_lines.max(1);
        self.title = title;
        self.anchor = anchor;
        self.on_aside = false;
        self.first = 0;
        self.open = true;
        self.unfolded = 0.0;

        self.light.clear();
        self.pressing = Pressing::default();
        self.aside_pressing = Pressing::default();
        true
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn press(&mut self) {
        if self.on_aside {
            self.aside_pressing.press();
        } else {
            self.pressing.press();
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn showing(&self) -> bool {
        self.out > 0.0
    }

    pub fn set_window(&mut self, rows: usize) {
        self.window = rows.max(1);
        self.hold_selection();
    }

    pub fn widen(&mut self, extra: f32) {
        self.extra = extra.max(0.0);
    }

    pub fn step(&mut self, delta: isize) {
        let total = self.entries.len() as isize;
        if total == 0 {
            return;
        }
        for step in 1..=total {
            let at = (self.selected as isize + delta * step).rem_euclid(total) as usize;
            if self.entries[at].enabled {
                self.selected = at;

                self.unfolded = 0.0;
                self.on_aside = false;
                self.hold_selection();
                return;
            }
        }
    }

    pub fn step_aside(&mut self, delta: isize) -> bool {
        let has_button = self
            .entries
            .get(self.selected)
            .is_some_and(|entry| entry.aside.is_some());
        let wanted = delta > 0;
        if !has_button || self.on_aside == wanted {
            return false;
        }
        self.on_aside = wanted;
        true
    }

    pub fn on_aside(&self) -> bool {
        self.on_aside
    }

    pub fn point_at(&mut self, spot: crate::Spot) -> bool {
        let crate::Spot::MenuRow { row, aside } = spot else {
            return false;
        };
        if !self.entries.get(row).is_some_and(|entry| entry.enabled) {
            return false;
        }

        let aside = aside
            && self
                .entries
                .get(row)
                .is_some_and(|entry| entry.aside.is_some());
        if self.selected != row {
            self.selected = row;

            self.unfolded = 0.0;
            self.hold_selection();
        }
        self.on_aside = aside;
        true
    }

    fn hold_selection(&mut self) {
        let window = self.window.max(1).min(self.entries.len().max(1));
        let last = self.entries.len().saturating_sub(window);
        if self.selected < self.first {
            self.first = self.selected;
        } else if self.selected + 1 > self.first + window {
            self.first = self.selected + 1 - window;
        }
        self.first = self.first.min(last);
    }

    pub fn scrolled_above(&self) -> bool {
        self.first > 0
    }

    pub fn scrolled_below(&self) -> bool {
        self.first + self.window.max(1) < self.entries.len()
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn chosen(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        let pressing = self.pressing.advance(dt) | self.aside_pressing.advance(dt);

        let opening = self.open
            && self.out >= 1.0
            && self
                .entries
                .get(self.selected)
                .is_some_and(|entry| entry.enabled && (entry.lines > 1 || entry.detail_lines > 1));
        let unfold = dt / motion::duration::MENU_UNFOLD;
        let was = self.unfolded;
        self.unfolded = if opening {
            (self.unfolded + unfold).min(1.0)
        } else {
            (self.unfolded - unfold).max(0.0)
        };
        let unfolding = self.unfolded != was;

        let target = if self.open { 1.0 } else { 0.0 };
        let step = dt / motion::duration::MENU_FLIGHT;
        if (self.out - target).abs() <= step {
            self.out = target;
            return pressing || unfolding || self.light.gliding();
        }
        self.out += step * (target - self.out).signum();
        true
    }

    pub fn travelled(&self) -> f32 {
        motion::ease(self.out.clamp(0.0, 1.0))
    }

    pub fn unfolded(&self) -> f32 {
        motion::ease(self.unfolded.clamp(0.0, 1.0))
    }
}

impl Ui {
    pub fn context_menu(&mut self, state: &mut ContextMenu) {
        if !state.showing() {
            return;
        }
        let out = state.travelled();
        let shape = menu::CONTEXT;
        let scale = self.scale;
        let icons = self.icons;
        let rows: Vec<menu::Row> = state.entries.iter().map(Entry::shape).collect();
        let title_lines = state.title.as_ref().map(|_| state.title_lines.max(1));
        let window = if state.window == 0 {
            menu::rows_of_that_fit(&rows, title_lines, self.height())
        } else {
            state.window
        };
        let layout = menu::Layout::new(
            &rows,
            title_lines,
            state.anchor,
            [self.width(), self.height()],
            state.first,
            window,
            state.selected,
            state.unfolded(),
            state.extra,
        );
        let panel = layout.rect;
        if panel[2] <= 0.0 || panel[3] <= 0.0 {
            return;
        }

        self.mark_spot(Spot::OutsidePanel, [0.0, 0.0, self.width(), self.height()]);
        for index in layout.first..(layout.first + layout.visible).min(state.entries.len()) {
            if let Some(line) = layout.row(index) {
                self.mark_spot(
                    Spot::MenuRow {
                        row: index,
                        aside: false,
                    },
                    line,
                );
            }

            if let Some(button) = layout.aside(index) {
                self.mark_spot(
                    Spot::MenuRow {
                        row: index,
                        aside: true,
                    },
                    button,
                );
            }
        }

        let opening = state.is_open();
        let arrived = shape.shown(out, opening);
        let grown = menu::growing(state.anchor, panel, out);
        self.recede(out, state.anchor);
        self.recede_behind(grown, 1.0 + (shape.dim - 1.0) * out, arrived);

        // Under the pane and no further. A menu is not modal — the page it is
        // over is still in play, and frosting all of it would say otherwise —
        // but the words on the pane still have to be read over whatever the
        // page happens to have put beneath them, which on an application's
        // page is anything at all. `grown` rather than `panel` because the
        // pane is drawn at `panel` and flown to `grown` below, and a frost
        // that stayed where the pane was going would sit off to one side for
        // the whole of the opening.
        let material = Overlay::ContextMenu.material();
        // The pane's own corner as it will be flown to, so the frost ends
        // exactly where the pane does and not a point outside it.
        let corner = self.s(material.radius) * (grown[2] / panel[2].max(1.0));
        self.frost_behind(grown, corner, arrived);

        self.in_overlay = true;
        let pane_at = self.mark(OVER);
        self.pane_at(panel, Overlay::ContextMenu, 1.0, OVER);
        let inside_at = self.mark(OVER);

        let pad = self.s(menu::LABEL_PADDING);
        let margin = self.s(menu::MARGIN);
        let text_x = panel[0] + margin + pad;
        let text_w = (panel[2] - (margin + pad) * 2.0).max(0.0);

        if let Some(name) = state.title.clone() {
            let size = self.s(shape.title_size);
            let tint = self.tinted(Role::TextSoft, 0.85);

            let top = panel[1] + self.s(menu::MARGIN + shape.title * 0.42) - size * 0.5;
            self.label_stacked(
                [text_x, top],
                text_w,
                Text::Title,
                &name,
                tint,
                true,
                state.title_lines.max(1),
            );
            let rule = self.tinted(Role::AccentSoft, 0.16);
            let under = panel[1]
                + self.s(menu::MARGIN
                    + shape.title * 0.78
                    + menu::title_growth(state.title_lines.max(1)));
            self.quad(
                OVER,
                Quad::solid(
                    [
                        panel[0] + margin,
                        under,
                        panel[2] - margin * 2.0,
                        scale.max(1.0),
                    ],
                    0.0,
                    rule,
                ),
            );
        }

        for rule in layout.separators() {
            let tint = self.tinted(Role::AccentSoft, 0.16);
            self.quad(OVER, Quad::solid(rule, 0.0, tint));
        }

        let pulse = motion::pulse(self.seconds());
        let lit = layout
            .highlight(state.on_aside)
            .map(|target| state.light.glide(target, self.dt()));
        if let Some([hx, hy, hw, hh]) = lit {
            let role = match state.entries.get(state.selected) {
                Some(entry) if entry.grave => Role::Danger,
                _ => control::LIT_ROLE,
            };

            let glow_h = hh + self.s(shape.row) * shape.glow_reach;
            let halo = self.tinted(role, control::GLOW + control::GLOW_PULSE * pulse);
            self.quad(
                OVER,
                Quad::light(
                    [
                        hx + hw * 0.5 - panel[2] * 0.62,
                        hy + hh * 0.5 - glow_h * 0.5,
                        panel[2] * control::GLOW_WIDTH,
                        glow_h,
                    ],
                    halo,
                ),
            );

            let through = if state.on_aside {
                state.aside_pressing.through()
            } else {
                state.pressing.through()
            };
            let sunk = motion::pressed([hx, hy, hw, hh], through);
            let radius = match state.entries.get(state.selected) {
                Some(entry) if state.on_aside => menu::aside_radius(&entry.shape(), self.height()),
                Some(entry) => menu::chip_radius(&entry.shape(), sunk[3], self.height()),
                None => sunk[3] * 0.5,
            };
            let tint = self.tinted(role, control::LIT + control::LIT_PULSE * pulse);
            self.quad(
                OVER,
                Quad::glass(sunk, radius, tint, Surface::Control.glass(), scale),
            );
        }

        for index in layout.first..(layout.first + layout.visible).min(state.entries.len()) {
            let Some(line) = layout.row(index) else {
                continue;
            };
            let Some(chip) = layout.chip(index) else {
                continue;
            };
            let entry = &state.entries[index];
            let row = entry.shape();
            let focused = index == state.selected;

            let handed_over = match (focused && !state.on_aside, lit) {
                (true, Some(highlight)) => control::arrival(highlight, chip),
                _ => 0.0,
            };
            let through = if focused && !state.on_aside {
                state.pressing.through()
            } else {
                None
            };
            let sunk = motion::pressed(chip, through);
            let radius = menu::chip_radius(&row, sunk[3], self.height());
            if entry.enabled {
                let tint = self.tinted(control::CHIP_ROLE, control::CHIP_TINT);
                self.quad(
                    OVER,
                    Quad::glass(sunk, radius, tint, control::chip(), scale)
                        .faded(1.0 - handed_over),
                );
            } else {
                let tint = self.tinted(control::OUT_ROLE, control::OUT);
                self.quad(
                    OVER,
                    Quad::outline(sunk, radius, tint, self.s(control::OUT_WIDTH).max(1.0)),
                );
            }

            if let (Some(mark), Some(rect)) = (entry.aside, layout.aside(index)) {
                let on_it = focused && state.on_aside;
                let button = motion::pressed(
                    rect,
                    if on_it {
                        state.aside_pressing.through()
                    } else {
                        None
                    },
                );
                let tint = self.tinted(control::CHIP_ROLE, control::ASIDE_TINT);
                let handed = match (on_it, lit) {
                    (true, Some(highlight)) => control::arrival(highlight, rect),
                    _ => 0.0,
                };
                self.quad(
                    OVER,
                    Quad::glass(
                        button,
                        menu::aside_radius(&row, self.height()),
                        tint,
                        control::chip(),
                        scale,
                    )
                    .faded(1.0 - handed),
                );

                let glyph = button[2] * shape.aside_glyph;
                self.icon_tinted(
                    [
                        button[0] + (button[2] - glyph) * 0.5,
                        button[1] + (button[3] - glyph) * 0.5,
                        glyph,
                        glyph,
                    ],
                    mark,
                    icons,
                    Role::Text,
                    if on_it {
                        control::MARK
                    } else {
                        control::MARK_QUIET
                    },
                );
            }

            let (label_grown, detail_grown) = layout.opened(index);

            let settled = sunk[3] - label_grown - detail_grown;

            let mut label_x = sunk[0] + pad;
            let mut label_w = (line[0] + line[2]) - pad - label_x;
            if let Some(name) = entry.glyph {
                let glyph = settled * shape.glyph;
                self.icon_tinted(
                    [label_x, sunk[1] + (settled - glyph) * 0.5, glyph, glyph],
                    name,
                    icons,
                    Role::Text,
                    if entry.enabled { 0.9 } else { 0.35 },
                );
                label_x += glyph + pad * 0.5;
                label_w -= glyph + pad * 0.5;
            }

            let stamp_box = if entry.stamp.is_some() {
                self.s(menu::STAMP_ROOM)
            } else {
                0.0
            };
            let label_box = self.s(menu::LABEL_LINE) + label_grown;
            let detail_box = match entry.detail {
                Some(_) => self.s(menu::DETAIL_LINE) + detail_grown,
                None => 0.0,
            };
            let stack = sunk[1] + (sunk[3] - (stamp_box + label_box + detail_box)) * 0.5;
            if let Some(stamp) = entry.stamp.clone() {
                let tint = self.tinted(Role::TextSoft, if focused { 0.85 } else { 0.6 });
                self.label_stacked(
                    [label_x, stack],
                    label_w.max(0.0),
                    Text::Caption,
                    &stamp,
                    tint,
                    false,
                    1,
                );
            }

            let tint = if entry.enabled {
                self.tinted(
                    Role::Text,
                    if focused {
                        control::INK
                    } else {
                        control::INK_QUIET
                    },
                )
            } else if entry.reading {
                self.tinted(Role::Text, 0.92)
            } else {
                self.tinted(Role::TextSoft, 0.38)
            };
            let label = entry.label.clone();
            self.label_stacked(
                [label_x, stack + stamp_box],
                label_w.max(0.0),
                Text::Body,
                &label,
                tint,
                focused && entry.enabled,
                menu::lines_in(label_grown, self.s(menu::LABEL_LINE)),
            );
            if let Some(detail) = entry.detail.clone() {
                let tint = match (entry.enabled, focused) {
                    (true, true) => self.tinted(Role::Text, 0.94),
                    (true, false) => self.tinted(Role::TextSoft, 0.72),
                    (false, _) => self.tinted(Role::TextSoft, 0.3),
                };
                self.label_stacked(
                    [label_x, stack + stamp_box + label_box],
                    label_w.max(0.0),
                    Text::Caption,
                    &detail,
                    tint,
                    false,
                    menu::lines_in(detail_grown, self.s(menu::DETAIL_LINE)),
                );
            }
        }

        let arrow = self.s(shape.scroll_arrow);
        let strip = self.s(shape.scroll_strip);
        for (showing, name, y) in [
            (
                state.scrolled_above(),
                "arrow-up",
                panel[1] + layout.rows_top() - strip * 0.5 - arrow * 0.5,
            ),
            (
                state.scrolled_below(),
                "arrow-down",
                panel[1] + panel[3] - margin - strip * 0.5 - arrow * 0.5,
            ),
        ] {
            if !showing {
                continue;
            }
            self.icon_tinted(
                [panel[0] + (panel[2] - arrow) * 0.5, y, arrow, arrow],
                name,
                icons,
                Role::Text,
                0.55,
            );
        }

        let factor = grown[2] / panel[2];
        let offset = [grown[0] - panel[0] * factor, grown[1] - panel[1] * factor];
        self.flew(OVER, pane_at, factor, offset);
        self.faded(OVER, pane_at, Some(inside_at), arrived);
        self.faded(OVER, inside_at, None, shape.content_shown(out, opening));

        self.light = None;
        self.in_overlay = false;
    }

    fn picker_furniture(
        &mut self,
        state: &FilePicker,
        panel: [f32; 4],
        scale: f32,
        foot: f32,
        title: &str,
        out: f32,
    ) {
        if out <= 0.01 {
            return;
        }
        let margin = PICKER_MARGIN * scale;
        let head = PICKER_HEAD * scale;
        let room = (panel[2] - margin * 2.0).max(0.0);
        let left = panel[0] + margin;
        let heading = PICKER_HEADING * scale;

        let ink = self.tinted(Role::Text, 0.98 * out);
        self.label_weighted_sized_clipped(
            [
                left,
                panel[1] + (head - heading) * 0.5 - heading * 0.12,
                room,
                heading * 1.4,
            ],
            Text::Title,
            title,
            ink,
            Align::Left,
            true,
            heading,
            None,
        );

        let rule = (1.0 * scale).max(1.0);
        let hair = self.tinted(Role::TextSoft, PICKER_RULE * out);
        let foot_top = panel[1] + panel[3] - foot;
        for y in [panel[1] + head - rule, foot_top] {
            self.quad(
                crate::renderer::OVER,
                Quad::solid([left, y, room, rule], 0.0, hair),
            );
        }

        let where_line = PICKER_WHERE_LINE * scale;
        let where_band = PICKER_WHERE * scale;
        let place = state.where_it_is();
        let place = self.cut_from_the_front(&place, where_line, room);
        let ink = self.tinted(Role::Text, 0.82 * out);
        self.label_weighted_sized_clipped(
            [
                left,
                foot_top - where_band * 0.5 - where_line * 0.5 - where_line * 0.12,
                room,
                where_line * 1.4,
            ],
            Text::Body,
            &place,
            ink,
            Align::Left,
            false,
            where_line,
            None,
        );

        let said = state.narrowed_to();
        let line = PICKER_SHOWING_LINE * scale;
        let ink = self.tinted(Role::TextSoft, 0.78 * out);
        self.label_weighted_sized_clipped(
            [
                left,
                foot_top + foot * 0.5 - line * 0.5 - line * 0.12,
                state.legend_left.max(0.0),
                line * 1.4,
            ],
            Text::Caption,
            &said,
            ink,
            Align::Left,
            false,
            line,
            None,
        );
    }

    fn cut_from_the_front(&mut self, text: &str, size: f32, room: f32) -> String {
        if room <= 0.0 || self.shaped_width(text, size, false) <= room {
            return text.to_string();
        }
        let mut from = 0;
        while from < text.len() {
            from += 1;
            while from < text.len() && !text.is_char_boundary(from) {
                from += 1;
            }
            let shorter = format!("…{}", &text[from..]);
            if self.shaped_width(&shorter, size, false) <= room {
                return shorter;
            }
        }
        "…".to_string()
    }

    fn picker_legend(
        &mut self,
        state: &mut FilePicker,
        panel: [f32; 4],
        scale: f32,
        foot: f32,
        out: f32,
    ) {
        if out <= 0.01 {
            return;
        }
        // Turned off for the session, so this panel says nothing either — the
        // shell's own legends are gone on the same answer, and a file question
        // that went on drawing pad buttons over a shell that had stopped would
        // be the one screen the setting did not reach. The anchor the panel's
        // menu grows out of is left at the middle of the foot, which is where
        // it starts: with no Options pair to pin it to, a menu out of the
        // corner would climb out of nothing.
        if !state.writes_what_the_buttons_do() {
            // The line on the left gets the whole foot, there being nothing on
            // the right of it any more, and the menu grows out of the middle of
            // that foot — the same fallback the row itself uses when it has no
            // Options pair to pin the anchor to.
            state.legend_left = (panel[2] - PICKER_MARGIN * scale * 2.0).max(0.0);
            state.menu_anchor = [
                panel[0] + panel[2] * 0.5,
                panel[1] + panel[3] - foot * 0.5,
                1.0,
                1.0,
            ];
            return;
        }
        let glyph = PICKER_HINT_GLYPH * scale;
        let size = PICKER_HINT_LABEL * scale;
        let gap = PICKER_HINT_GAP * scale;
        let step = PICKER_HINT_STEP * scale;
        let middle = panel[1] + panel[3] - foot * 0.5;
        let mut right = panel[0] + panel[2] - PICKER_HINT_EDGE * scale;
        let ink = self.tinted(Role::TextSoft, 0.86 * out);
        let mark = 0.90 * out;
        let mut anchor = [panel[0] + panel[2] * 0.5, middle, 1.0, 1.0];
        let mut leftmost = right;

        for (word, name) in picker_hints(state.purpose(), state.pad).into_iter().rev() {
            let width = self.shaped_width(word, size, false);
            right -= width;
            self.label_weighted_sized_clipped(
                [right, middle - size * 0.72, width, size * 1.44],
                Text::Caption,
                word,
                ink,
                Align::Left,
                false,
                size,
                None,
            );
            right -= gap + glyph;
            self.icon_tinted(
                [right, middle - glyph * 0.5, glyph, glyph],
                name,
                self.icons,
                Role::Text,
                mark,
            );
            if word == lxb_toolkit::i18n::text("options") {
                anchor = [right, middle - glyph * 0.5, glyph + gap + width, glyph];
            }
            leftmost = right;
            right -= step;
            if right <= panel[0] {
                break;
            }
        }
        state.menu_anchor = anchor;
        state.legend_left =
            (leftmost - PICKER_HINT_STEP * scale - (panel[0] + PICKER_MARGIN * scale)).max(0.0);
    }

    fn picker_search_keyboard(
        &mut self,
        state: &mut FilePicker,
        panel: [f32; 4],
        picker_scale: f32,
        content_clip: [f32; 4],
    ) {
        let arrived = state.keyboard.travelled();
        if arrived <= 0.01 {
            return;
        }
        let layout = picker_keyboard_layout(panel, picker_scale);
        let board = layout.panel_rect(arrived);
        if picker_intersection(board, content_clip).is_none() {
            return;
        }
        let scale = layout.scale;
        let radius = PICKER_KEYBOARD_RADIUS * scale;
        let pulse = motion::pulse(self.seconds());

        let mut slab = Surface::Panel.glass();
        slab.frost = 0.95;
        slab.gloss = 1.0;
        slab.curve = 0.0;
        self.quad(
            OVER,
            Quad::glass(
                board,
                Overlay::Dialog.material().radius * scale,
                self.tinted(Role::Glass, 0.52),
                slab,
                scale,
            ),
        );

        let (selected_row, selected_column) = state.keyboard.selected_position();
        let selected_rect = layout.key_rect(selected_row, selected_column, arrived);
        let lit = selected_rect;
        for row in 0..PICKER_KEYBOARD_ROWS {
            for (column, key) in picker_keyboard_row_keys(row).into_iter().enumerate() {
                let rect = layout.key_rect(row, column, arrived);
                let Some(visible) = picker_intersection(rect, content_clip) else {
                    continue;
                };
                let selected = (row, column) == (selected_row, selected_column);
                let latch = state.keyboard.latched(key);
                let held = latch.is_on();
                if state.keyboard.is_open() {
                    self.mark_spot(Spot::PickerKey { row, column }, visible);
                }
                // The selection's halo, which is drawn outside the key and so
                // survives whatever fills it.
                if selected {
                    let glow = rect[3] * 2.2;
                    let light = [
                        lit[0] + lit[2] * 0.5 - glow * 0.5,
                        lit[1] + lit[3] * 0.5 - glow * 0.5,
                        glow,
                        glow,
                    ];
                    self.quad(
                        OVER,
                        Quad::light(light, self.tinted(Role::Accent, 0.30 + 0.08 * pulse)),
                    );
                }
                // One fill per key, the selected one included. It used to be a
                // branch that asked `selected` first, so a modifier armed or
                // locked from the board showed nothing at all until the cursor
                // was walked off it — and the cursor stands on the key that was
                // just pressed. That reads as a key needing two presses to come
                // back off.
                //
                // And a held key is drawn *darker* rather than as a brighter
                // cast of the accent, which is the colour the cursor is drawn
                // in: two readings competing to mean two things. A key holding
                // the board down is a key pressed into the panel, so it is the
                // panel's own near-black glass, and twice as much of it locked
                // as armed.
                let (role, alpha) = match latch {
                    PickerKeyboardLatch::Locked => (Role::Glass, 0.72),
                    PickerKeyboardLatch::Once => (Role::Glass, 0.40),
                    PickerKeyboardLatch::Off if selected => (Role::Accent, 0.52 + 0.05 * pulse),
                    PickerKeyboardLatch::Off if key.is_character() => (Role::GlassRaised, 0.10),
                    PickerKeyboardLatch::Off => (Role::GlassRaised, 0.17),
                };
                let mut key_glass = Surface::Control.glass();
                // A pressed key does not catch the light a raised one does, and
                // the selected key catches all of it.
                if !(selected && !held) {
                    key_glass.gloss = 0.45;
                }
                self.quad(
                    OVER,
                    Quad::glass(rect, radius, self.tinted(role, alpha), key_glass, scale),
                );
                // The cursor's own rim, and only on a key whose fill a latch has
                // taken over. Everywhere else the fill *is* the selection.
                if selected && held {
                    self.quad(
                        OVER,
                        Quad::outline(
                            rect,
                            radius,
                            self.tinted(Role::AccentSoft, 0.62 + 0.10 * pulse),
                            2.0 * scale,
                        ),
                    );
                }

                if key.is_close() {
                    self.quad(
                        OVER,
                        Quad::outline(
                            rect,
                            radius,
                            self.tinted(Role::AccentSoft, if selected { 0.55 } else { 0.34 }),
                            1.5 * scale,
                        ),
                    );
                }

                if let Some(glyph) = key.glyph() {
                    let mark = rect[3] * 0.46;
                    self.icon_tinted(
                        [
                            rect[0] + rect[2] * 0.5 - mark * 0.5,
                            rect[1] + rect[3] * 0.5 - mark * 0.5,
                            mark,
                            mark,
                        ],
                        glyph,
                        self.icons,
                        Role::Text,
                        if selected { 1.0 } else { 0.82 },
                    );
                    continue;
                }

                let label = key.label(state.keyboard.level());
                let size = if row == 0 {
                    PICKER_KEYBOARD_FUNCTION_CAP * scale
                } else if key.is_character() {
                    26.0 * scale
                } else {
                    17.0 * scale
                };
                self.label_weighted_sized_clipped(
                    rect,
                    Text::Caption,
                    &label,
                    self.tinted(Role::Text, if selected || held { 1.0 } else { 0.82 }),
                    Align::Centre,
                    selected,
                    size,
                    Some(content_clip),
                );
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Dialog {
    title: String,
    body: String,
    choices: Vec<String>,
    selected: usize,
    open: bool,
    out: f32,

    light: Selection,

    pressing: Pressing,
}

impl Dialog {
    pub fn ask(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        choices: Vec<String>,
        from: usize,
    ) {
        self.title = title.into();
        self.body = body.into();
        self.selected = from.min(choices.len().saturating_sub(1));
        self.choices = choices;
        self.open = true;
        self.light.clear();
        self.pressing = Pressing::default();
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn press(&mut self) {
        self.pressing.press();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn showing(&self) -> bool {
        self.out > 0.0
    }

    pub fn step(&mut self, delta: isize) {
        let total = self.choices.len() as isize;
        if total > 0 {
            self.selected = (self.selected as isize + delta).rem_euclid(total) as usize;
        }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn point_at(&mut self, spot: crate::Spot) -> bool {
        let crate::Spot::DialogButton(index) = spot else {
            return false;
        };
        if index >= self.choices.len() {
            return false;
        }
        self.selected = index;
        true
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        let pressing = self.pressing.advance(dt);
        let target = if self.open { 1.0 } else { 0.0 };
        let step = dt / motion::duration::MENU_FLIGHT;
        if (self.out - target).abs() <= step {
            self.out = target;
            return pressing || self.light.gliding();
        }
        self.out += step * (target - self.out).signum();
        true
    }

    pub fn travelled(&self) -> f32 {
        motion::ease(self.out.clamp(0.0, 1.0))
    }
}

impl Ui {
    /// Lay the page back over itself out of the blur pyramid, so a modal
    /// surface has something calm underneath it.
    ///
    /// A dialog's own pane is the sidebar's clearer cut, and over the shell's
    /// own quiet surfaces that is exactly right. Over an application's page it
    /// is not: a page is free to be a photograph, and words set over a
    /// photograph at any translucency are words nobody can read. So what
    /// changes is not the pane but what the pane is looking at — the page goes
    /// into the blur pyramid as far as `modal::FROST`, which is where nothing
    /// on it can be read at body size and it is still recognisably itself.
    ///
    /// It draws on **SOFTEN**, which is the layer for exactly this: an effect
    /// that has to sample the complete page — its pictures and its words, not
    /// just the surfaces under them — and still sit behind every overlay. A
    /// page cannot do it, because everything a page draws lands on CONTROL and
    /// glass there samples the layer *beneath* the page.
    ///
    /// It must be SOFTEN and not the overlay layer, even though the dialog is
    /// drawn there. **A pane of glass fetches what is behind it out of the
    /// source texture, not out of the picture being painted.** A frost drawn
    /// beside the dialog on the overlay layer therefore frosted the whole
    /// window except the one place it was for: the pane read the page from the
    /// source, sharp and bright, and laid it back over the frost. On SOFTEN
    /// the frost is in that source, so the pane refracts a page already calm.
    ///
    /// It is pushed after `recede_behind`, which fades this layer along with
    /// the rest of the page. The frost is not part of the page it is dimming.
    ///
    /// `rect` is what to frost, and **nothing outside it may be touched.** A
    /// dialog is modal and frosts the whole window; a context menu is not, and
    /// frosts only what is under its own pane — the page around one is still
    /// in play and must stay legible. Give the rectangle and the corner the
    /// pane will really be drawn with on this frame.
    ///
    /// It once grew the rectangle a little, so that the refraction at a pane's
    /// edges — which samples a short way outside the pane — would find frost
    /// there rather than the sharp page. It cannot: the stain is nearly black,
    /// so the grown ring is a dark halo hanging outside the pane with nothing
    /// over it. What is under the glass is the glass's business; a rim of
    /// darkened page beyond it is a shadow, and this language does not draw
    /// shadows.
    pub(crate) fn frost_behind(&mut self, rect: [f32; 4], radius: f32, out: f32) {
        if out <= 0.0 || rect[2] <= 0.0 || rect[3] <= 0.0 {
            return;
        }
        let stain = self.tinted(Role::Glass, modal::STAIN);
        let lod = modal::FROST * optics::BLUR_LEVELS;
        self.quad(SOFTEN, Quad::frost(rect, radius, lod, stain).faded(out));
    }

    pub fn dialog(&mut self, state: &mut Dialog) {
        if !state.showing() {
            return;
        }
        let out = state.travelled();
        let width = self.m(Metric::DialogWidth).min(self.width() * 0.7);
        let pad = self.m(Metric::PanelPadding);
        let gap = self.m(Metric::Gap);
        let capsule = self.m(Metric::RowHeight) * 0.6;

        let body = self.wrap(Text::Caption, &state.body, width - 2.0 * pad);
        let height = 2.0 * pad
            + self.line(Text::Title)
            + gap
            + self.line(Text::Caption) * body.len() as f32
            + gap
            + capsule;
        let x = (self.width() - width) / 2.0;
        let y = (self.height() - height) / 2.0;

        let scale = 0.96 + 0.04 * out;
        let panel = [
            x + width * (1.0 - scale) / 2.0,
            y + height * (1.0 - scale) / 2.0,
            width * scale,
            height * scale,
        ];

        let dim = self.m(Metric::DialogDim);

        self.recede(out, [panel[0], panel[1], panel[2], panel[3]]);
        self.recede_behind(panel, 1.0 + (dim - 1.0) * out, out);

        self.mark_spot(Spot::Nothing, [0.0, 0.0, self.width(), self.height()]);

        self.frost_behind([0.0, 0.0, self.width(), self.height()], 0.0, out);

        self.in_overlay = true;
        self.pane_at(panel, Overlay::Dialog, out, OVER);

        let mut top = panel[1] + pad;
        let bright = self.tinted(Role::Text, out);
        let line = self.line(Text::Title);
        self.label_tinted(
            [panel[0] + pad, top, panel[2] - 2.0 * pad, line],
            Text::Title,
            &state.title,
            bright,
            Align::Left,
        );
        top += line + gap;
        let caption = self.line(Text::Caption);
        for (index, line) in body.iter().enumerate() {
            // White made quiet by how much of it there is, never by hue.
            // `Role::TextSoft` is a pale lavender: over the shell's own dark
            // surfaces it passes for a soft white, and over the bright patch
            // of somebody's screenshot showing through the pane it comes back
            // as what it really is — lavender on grey, at almost no contrast,
            // in the middle of a sentence. The answers below already quiet
            // themselves this way; the body was the one part of a dialog that
            // did not.
            let quiet = self.tinted(Role::Text, control::INK_QUIET * out);
            self.label_tinted(
                [
                    panel[0] + pad,
                    top + index as f32 * caption,
                    panel[2] - 2.0 * pad,
                    caption,
                ],
                Text::Caption,
                line,
                quiet,
                Align::Left,
            );
        }
        top += caption * body.len() as f32 + gap;

        let mut answers = vec![[0.0f32; 4]; state.choices.len()];
        let mut right = panel[0] + panel[2] - pad;
        for (index, label) in state.choices.iter().enumerate().rev() {
            let room = self.measure(Text::Label, label) + 2.0 * self.m(Metric::RowPadding);
            let left = right - room;
            answers[index] = [left, top, room, capsule];
            right = left - gap;
        }

        for (index, rect) in answers.iter().enumerate() {
            self.mark_spot(Spot::DialogButton(index), *rect);
        }

        if let Some(chosen) = answers.get(state.selected).copied() {
            let radius = capsule_radius(chosen[3]);
            let seconds = self.scene.time;
            let lit = state.light.glide(chosen, self.dt);
            let glow = self.tinted(Role::Glow, control::glow_alpha(seconds) * out);
            self.quad(OVER, Quad::light(control::glow_rect(lit, panel[2]), glow));
            let tint = self.tinted(control::LIT_ROLE, control::answer_alpha(seconds) * out);
            let scale = self.scale;
            self.quad(OVER, Quad::glass(lit, radius, tint, control::lit(), scale));
            self.light = Some(lit);
        }

        for (index, label) in state.choices.iter().enumerate() {
            let press = if index == state.selected {
                state.pressing.state(true)
            } else {
                Press::Resting
            };
            let rect = answers[index];
            let sunk = self.control(rect, capsule_radius(rect[3]), press, panel[2], out);
            let lit = press.lit();
            let ink = self.tinted(
                Role::Text,
                out * if lit {
                    control::INK
                } else {
                    control::INK_QUIET
                },
            );
            self.label_weighted(sunk, Text::Label, label, ink, Align::Centre, lit);
        }

        self.light = None;
        self.in_overlay = false;
    }
}

/// How wide a bar down the edge of a list is, in reference points, and how
/// solid its two halves are.
///
/// The trough is barely there — its work is to say how far the thumb can go,
/// not to draw a line down the page — and the thumb is ink until a hand takes
/// hold of it, at which point it is the accent like every other control that
/// is in use.
const SCROLL_BAR: f32 = 8.0;

/// The shortest a thumb is drawn, as a multiple of the bar's own width.
///
/// A store's Everything shelf is a thousand lines deep in a window that holds
/// eight, which is a thumb of five points — a control nobody could take hold
/// of, and one that would say "nearly nothing is showing" by disappearing.
const LEAST_THUMB: f32 = 3.0;

/// Where the thumb of a bar comes out on its track. See [`Ui::scroll_bar`],
/// which is this drawn; kept apart from the drawing so that it can be read
/// back without a screen.
fn scroll_thumb(track: [f32; 4], at: f32, run: f32) -> [f32; 4] {
    let least = (track[2] * LEAST_THUMB).min(track[3]);
    let long = (track[3] * run.clamp(0.0, 1.0)).clamp(least, track[3]);
    [
        track[0],
        track[1] + (track[3] - long) * at.clamp(0.0, 1.0),
        track[2],
        long,
    ]
}

const SCROLL_TROUGH: f32 = 0.5;
const SCROLL_THUMB: f32 = 0.75;

const PICKER_CROSS_X: f32 = 0.34;
const PICKER_CROSS_Y: f32 = 0.38;

const PICKER_WINDOW_WIDTH: f32 = 0.8;
const PICKER_WINDOW_HEIGHT: f32 = 0.8;
const PICKER_MARGIN: f32 = 24.0;
const PICKER_HEAD: f32 = 78.0;
const PICKER_WHERE: f32 = 38.0;
const PICKER_HEADING: f32 = 26.0;
const PICKER_WHERE_LINE: f32 = 22.0;
const PICKER_SHOWING_LINE: f32 = 20.0;
const PICKER_RULE: f32 = 0.16;
const PICKER_DISC: f32 = std::f32::consts::SQRT_2 * 1.04;
const PICKER_ITEM_PREVIEW: f32 = PICKER_DISC * 0.8;
const PICKER_CATEGORY_DISC: f32 = 1.30;
const PICKER_DEPTH_SHRINK: f32 = 0.86;
const PICKER_DEPTH_HAZE: f32 = 0.72;
const PICKER_DEPTH_FLOOR_SCALE: f32 = 0.50;
const PICKER_DEPTH_FLOOR_HAZE: f32 = 0.35;
const PICKER_SCRIM: f32 = 0.28;
const PICKER_PAGE_DIM: f32 = 0.74;
const PICKER_PAGE_FROST: f32 = 0.56;
const PICKER_WINDOW_FROST: f32 = 0.98;
const PICKER_WINDOW_BASE: f32 = 0.94;
const PICKER_WINDOW_STAIN: f32 = 0.18;
const PICKER_WINDOW_GLASS: f32 = 0.20;
const PICKER_FILE_INK: f32 = 0.45;
const PICKER_GAP_ABOVE: f32 = 168.0;
const PICKER_GAP_BELOW: f32 = 227.4;
const PICKER_COLUMN_GONE_BY: f32 = 0.35;
const PICKER_KEYBOARD_ROWS: usize = 6;
const PICKER_KEYBOARD_COLUMNS: f32 = 15.0;
const PICKER_KEYBOARD_KEY_WIDTH: f32 = 62.0;
const PICKER_KEYBOARD_KEY_HEIGHT: f32 = 56.0;
const PICKER_KEYBOARD_GAP: f32 = 8.0;
const PICKER_KEYBOARD_RADIUS: f32 = 13.0;
const PICKER_KEYBOARD_FUNCTION_CAP: f32 = 14.0;
const PICKER_KEYBOARD_PAD: f32 = 20.0;
const PICKER_KEYBOARD_BOTTOM: f32 = 34.0;
const PICKER_KEYBOARD_FLIGHT: f32 = 0.24;
const PICKER_FOOT: f32 = 58.0;
const PICKER_HINT_GLYPH: f32 = 34.0;
const PICKER_HINT_LABEL: f32 = 19.0;
const PICKER_HINT_GAP: f32 = 8.0;
const PICKER_HINT_STEP: f32 = 24.0;
const PICKER_HINT_EDGE: f32 = 30.0;
const PICKER_TICK: f32 = 0.5;

fn ticked_row(label: String, in_force: bool) -> Entry {
    let row = Entry::new(label);
    if in_force {
        row.glyph("chosen")
    } else {
        row
    }
}

fn picker_hints(purpose: PickerPurpose, pad: bool) -> Vec<(&'static str, &'static str)> {
    let one = |label, on_a_pad, otherwise| (label, if pad { on_a_pad } else { otherwise });
    let mut hints = vec![one(
        if purpose.takes_several() {
            lxb_toolkit::i18n::text("choose")
        } else {
            lxb_toolkit::i18n::text("select")
        },
        "pad-south",
        "key-space",
    )];
    if purpose.answers_with_a_head_row() {
        hints.push(one(
            lxb_toolkit::i18n::text("approve"),
            "pad-start",
            "key-enter",
        ));
    }
    hints.push(one(
        lxb_toolkit::i18n::text("options"),
        "pad-north",
        "mouse-right",
    ));
    hints.push(one(
        lxb_toolkit::i18n::text("cancel"),
        "pad-east",
        "key-escape",
    ));
    hints
}

fn picker_title(purpose: PickerPurpose, selection: PickerSelection) -> &'static str {
    match purpose {
        PickerPurpose::OneFile => match selection {
            PickerSelection::File => lxb_toolkit::i18n::text("choose-a-file"),
            PickerSelection::Image => lxb_toolkit::i18n::text("choose-an-image"),
            PickerSelection::Scenery => lxb_toolkit::i18n::text("choose-scenery"),
            PickerSelection::Folder => lxb_toolkit::i18n::text("choose-a-folder"),
        },
        asked => asked.asking(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerHeadRow {
    NewFolder,
    Name,
    Answer,
    SearchField,
    SearchClear,
}

impl PickerHeadRow {
    const fn is_typed_into(self) -> bool {
        matches!(self, Self::NewFolder | Self::Name | Self::SearchField)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerStanding {
    Open,
    Behind,
    Leaving,
}

#[derive(Debug)]
struct PickerLevel {
    purpose: PickerPurpose,
    picker: Picker,
    head: Vec<PickerHeadRow>,
    head_focused: Option<usize>,
    making: String,
    position: f32,
    speed: f32,
}

impl PickerLevel {
    fn new(
        purpose: PickerPurpose,
        selection: PickerSelection,
        directory: impl AsRef<std::path::Path>,
        named: bool,
    ) -> Self {
        let picker = Picker::new(selection, directory);
        let mut level = Self {
            purpose,
            picker,
            head: Vec::new(),
            head_focused: None,
            making: String::new(),
            position: 0.0,
            speed: 0.0,
        };
        level.head = level.heads();
        level.head_focused = level.opens_on(named);
        level.position = level
            .focused_row()
            .or_else(|| level.picker.selected().map(|row| row + level.head.len()))
            .unwrap_or(0) as f32;
        level
    }

    fn heads(&self) -> Vec<PickerHeadRow> {
        let mut rows = Vec::new();
        if self.purpose.makes_folders() && lxb_toolkit::picker::writable(self.picker.location()) {
            rows.push(PickerHeadRow::NewFolder);
        }
        if matches!(self.purpose, PickerPurpose::ANewFile) {
            rows.push(PickerHeadRow::Name);
        }
        if self.purpose.answers_with_a_head_row() && self.picker.accessible() {
            rows.push(PickerHeadRow::Answer);
        }
        if self.picker.can_search() {
            rows.push(PickerHeadRow::SearchField);
            if !self.picker.query().is_empty() {
                rows.push(PickerHeadRow::SearchClear);
            }
        }
        rows
    }

    fn opens_on(&self, named: bool) -> Option<usize> {
        if matches!(self.purpose, PickerPurpose::ANewFile) {
            let wanted = if named {
                PickerHeadRow::Answer
            } else {
                PickerHeadRow::Name
            };
            if let Some(at) = self.head.iter().position(|row| *row == wanted) {
                return Some(at);
            }
        }
        if self.picker.entries().is_empty() && !self.head.is_empty() {
            let last = self.head.len() - 1;
            return Some(
                self.head
                    .iter()
                    .rposition(|row| *row != PickerHeadRow::Answer)
                    .unwrap_or(last),
            );
        }
        None
    }

    fn refresh_head(&mut self) {
        self.head = self.heads();
        let count = self.head_count();
        let invalid = self.head_focused.is_some_and(|row| row >= count);
        if invalid {
            self.head_focused = (count > 0).then_some(count - 1);
        } else if self.picker.selected().is_none() && count > 0 && self.head_focused.is_none() {
            self.head_focused = Some(count - 1);
        }
    }

    fn row_count(&self) -> usize {
        self.picker.entries().len() + self.head_count()
    }

    fn focused_row(&self) -> Option<usize> {
        self.head_focused
            .filter(|row| self.head_row(*row).is_some())
            .or_else(|| self.picker.selected().map(|row| row + self.head_count()))
    }

    fn head_count(&self) -> usize {
        self.head.len()
    }

    fn head_row(&self, row: usize) -> Option<PickerHeadRow> {
        self.head.get(row).copied()
    }

    fn focused_head(&self) -> Option<PickerHeadRow> {
        self.head_focused.and_then(|row| self.head_row(row))
    }

    fn move_one(&mut self, direction: isize) -> bool {
        if let Some(head) = self
            .head_focused
            .filter(|row| self.head_row(*row).is_some())
        {
            if direction > 0 && head + 1 < self.head_count() {
                self.head_focused = Some(head + 1);
                return true;
            }
            if direction > 0 && !self.picker.entries().is_empty() {
                self.head_focused = None;
                let _ = self.picker.select(0);
                return true;
            }
            if direction < 0 && head > 0 {
                self.head_focused = Some(head - 1);
                return true;
            }
            return false;
        }

        let Some(selected) = self.picker.selected() else {
            if self.head_count() > 0 {
                self.head_focused = Some(self.head_count() - 1);
                return true;
            }
            return false;
        };
        if direction < 0 && selected == 0 && self.head_count() > 0 {
            self.head_focused = Some(self.head_count() - 1);
            return true;
        }
        self.picker.move_selection(direction)
    }

    fn select_row(&mut self, row: usize) -> bool {
        if self.head_row(row).is_some() {
            let changed = self.head_focused != Some(row);
            self.head_focused = Some(row);
            return changed;
        }
        let entry = row.saturating_sub(self.head_count());
        let valid = entry < self.picker.entries().len();
        let was_head = self.head_focused;
        let changed = self.picker.select(entry);
        if valid {
            self.head_focused = None;
        }
        changed || (was_head.is_some() && valid)
    }

    fn focus_child(&mut self, path: &std::path::Path) -> bool {
        if let Some(index) = self
            .picker
            .entries()
            .iter()
            .position(|entry| entry.path == path)
        {
            let _ = self.picker.select(index);
            self.head_focused = None;
            self.position = self.focused_row().unwrap_or(0) as f32;
            self.speed = 0.0;
            return true;
        }
        false
    }

    fn advance(&mut self, dt: f32) -> bool {
        let target = self.focused_row().unwrap_or(0) as f32;
        let (position, speed) = motion::spring(
            self.position as f64,
            self.speed as f64,
            target as f64,
            motion::HIGHLIGHT_SPRING,
            dt as f64,
        );
        self.position = position as f32;
        self.speed = speed as f32;
        (self.position - target).abs() > 0.001 || self.speed.abs() > 0.001
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerKeyboardStroke {
    Character(char),
    Space,
    Backspace,
    Enter,
    Inert,
    /// A dead key: a keysym with no character of its own that the *layout* put
    /// on the alphabet, which is how a French or a German keyboard reaches its
    /// accented letters.
    ///
    /// It is on the board because a key that is on the keyboard and missing
    /// from the picture of it is a picture that is wrong, and it shows the
    /// accent a real keycap shows. Typing it does nothing: this board types
    /// into a field rather than through a keymap, so there is nothing here for
    /// an accent to combine with — and the letters it would have made are on
    /// the AltGr face of this board anyway, where they can be typed directly.
    Dead(u32),
}

/// Which face of the board is showing.
///
/// xkb's first four shift levels, which is what a keyboard's four-level type
/// is. The board reaches them with two keys: Shift, and AltGr where the layout
/// has anything on the far two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PickerKeyboardLevel {
    #[default]
    Plain,
    Shift,
    AltGr,
    AltGrShift,
}

impl PickerKeyboardLevel {
    fn of(shifted: bool, altgr: bool) -> Self {
        match (altgr, shifted) {
            (false, false) => Self::Plain,
            (false, true) => Self::Shift,
            (true, false) => Self::AltGr,
            (true, true) => Self::AltGrShift,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Plain => 0,
            Self::Shift => 1,
            Self::AltGr => 2,
            Self::AltGrShift => 3,
        }
    }
}

/// What one character key types, on each face the board can show.
///
/// Four answers rather than the two a keycap is printed with, because a layout
/// keeps its accented letters on the far two: `ą` is AltGr and `a` on a Polish
/// keyboard, and a board offering only the near pair is a board a Pole cannot
/// search their own files with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PickerKeyboardCap {
    levels: [Option<PickerKeyboardStroke>; 4],
}

impl PickerKeyboardCap {
    /// A key with a plain and a shifted character and nothing on AltGr, which
    /// is how the board's own fallback arrangement is written.
    const fn letter(plain: char, shifted: char) -> Self {
        Self {
            levels: [
                Some(PickerKeyboardStroke::Character(plain)),
                Some(PickerKeyboardStroke::Character(shifted)),
                None,
                None,
            ],
        }
    }

    fn at(self, level: PickerKeyboardLevel) -> Option<PickerKeyboardStroke> {
        self.levels[level.index()]
    }

    /// What is printed on it on one face.
    fn printed(self, level: PickerKeyboardLevel) -> String {
        match self.at(level) {
            Some(PickerKeyboardStroke::Character(character)) => character.to_string(),
            // The accent a dead key carries, which is what a real keycap shows.
            Some(PickerKeyboardStroke::Dead(raw)) => {
                picker_keyboard_dead_mark(raw).unwrap_or("").to_string()
            }
            Some(PickerKeyboardStroke::Space) => " ".to_string(),
            Some(_) | None => String::new(),
        }
    }

    fn has_altgr(self) -> bool {
        self.levels[2].is_some() || self.levels[3].is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerKeyboardArrow {
    Left,
    Down,
    Up,
    Right,
}

impl PickerKeyboardArrow {
    const ALL: [Self; 4] = [Self::Left, Self::Down, Self::Up, Self::Right];

    fn glyph(self) -> &'static str {
        match self {
            Self::Left => "arrow-left",
            Self::Down => "arrow-down",
            Self::Up => "arrow-up",
            Self::Right => "arrow-right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerKeyboardKey {
    /// A character key: what it types on each of the four faces the board can
    /// show. See [`PickerKeyboardCap`].
    Character(PickerKeyboardCap),
    Named(&'static str, PickerKeyboardStroke),
    Arrow(PickerKeyboardArrow),
    Shift,
    Caps,
    Control,
    Alt,
    /// AltGr: the third and fourth faces of the board, where most layouts keep
    /// their accented letters and their currency signs. On the board only where
    /// the layout has something on those faces.
    AltGr,
    Close,
}

impl PickerKeyboardKey {
    fn label(self, level: PickerKeyboardLevel) -> String {
        match self {
            Self::Character(cap) => cap.printed(level),
            Self::Named(label, _) => label.to_string(),
            Self::Arrow(_) | Self::Close => String::new(),
            Self::Shift => "Shift".to_string(),
            Self::Caps => "Caps".to_string(),
            Self::Control => "Ctrl".to_string(),
            Self::Alt => "Alt".to_string(),
            Self::AltGr => "AltGr".to_string(),
        }
    }

    fn glyph(self) -> Option<&'static str> {
        match self {
            Self::Arrow(arrow) => Some(arrow.glyph()),
            Self::Close => Some("keyboard-hide"),
            _ => None,
        }
    }

    fn is_character(self) -> bool {
        matches!(self, Self::Character(..))
    }

    fn is_close(self) -> bool {
        self == Self::Close
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PickerKeyboardLatch {
    #[default]
    Off,
    Once,
    Locked,
}

impl PickerKeyboardLatch {
    fn pressed(self) -> Self {
        match self {
            Self::Off => Self::Once,
            Self::Once => Self::Locked,
            Self::Locked => Self::Off,
        }
    }

    fn spent(self) -> Self {
        match self {
            Self::Once => Self::Off,
            other => other,
        }
    }

    fn is_on(self) -> bool {
        self != Self::Off
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerKeyboardPress {
    Type(PickerKeyboardStroke),
    Shifted,
    /// The key has nothing on the face the board is showing, so the press did
    /// nothing at all — the armed modifier included, because the user is still
    /// reaching for the key it was armed for.
    Nothing,
    Close,
}

#[derive(Debug)]
struct PickerKeyboard {
    open: bool,
    out: f32,
    row: usize,
    column: usize,
    shift: PickerKeyboardLatch,
    control: PickerKeyboardLatch,
    alt: PickerKeyboardLatch,
    altgr: PickerKeyboardLatch,
}

impl Default for PickerKeyboard {
    fn default() -> Self {
        Self {
            open: false,
            out: 0.0,

            row: 3,
            column: 1,
            shift: PickerKeyboardLatch::Off,
            control: PickerKeyboardLatch::Off,
            alt: PickerKeyboardLatch::Off,
            altgr: PickerKeyboardLatch::Off,
        }
    }
}

impl PickerKeyboard {
    fn open(&mut self) {
        self.open = true;
        self.row = 3;
        self.column = 1;
        self.shift = PickerKeyboardLatch::Off;
        self.control = PickerKeyboardLatch::Off;
        self.alt = PickerKeyboardLatch::Off;
        self.altgr = PickerKeyboardLatch::Off;
    }

    fn close(&mut self) -> bool {
        let was_open = self.open;
        self.open = false;
        was_open
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn showing(&self) -> bool {
        self.out > 0.0
    }

    fn travelled(&self) -> f32 {
        self.out.clamp(0.0, 1.0)
    }

    fn advance(&mut self, dt: f32) -> bool {
        let target = if self.open { 1.0 } else { 0.0 };
        let step = dt / PICKER_KEYBOARD_FLIGHT;
        if (self.out - target).abs() <= step {
            self.out = target;
        } else {
            self.out += step * (target - self.out).signum();
            return true;
        }
        false
    }

    /// Which face the board is showing, which is what every cap on it says and
    /// what the next press will type.
    fn level(&self) -> PickerKeyboardLevel {
        PickerKeyboardLevel::of(self.shift.is_on(), self.altgr.is_on())
    }

    fn selected_position(&self) -> (usize, usize) {
        let count = picker_keyboard_row_keys(self.row).len();
        (self.row, self.column.min(count.saturating_sub(1)))
    }

    fn selected(&self) -> PickerKeyboardKey {
        let (row, column) = self.selected_position();
        picker_keyboard_row_keys(row)[column]
    }

    fn move_by(&mut self, horizontal: isize, vertical: isize) -> bool {
        if !self.open {
            return false;
        }
        let before = self.selected_position();
        if horizontal != 0 {
            let (row, column) = self.selected_position();
            let count = picker_keyboard_row_keys(row).len() as isize;
            self.column = (column as isize + horizontal).rem_euclid(count) as usize;
        } else if vertical != 0 {
            let step = vertical.signum();
            for _ in 0..vertical.unsigned_abs() {
                let (row, column) = self.selected_position();
                let next_row =
                    (row as isize + step).rem_euclid(PICKER_KEYBOARD_ROWS as isize) as usize;
                self.row = next_row;
                self.column = picker_keyboard_nearest_column(row, column, next_row);
            }
        }
        self.selected_position() != before
    }

    fn select(&mut self, row: usize, column: usize) -> bool {
        if !self.open
            || row >= PICKER_KEYBOARD_ROWS
            || column >= picker_keyboard_row_keys(row).len()
        {
            return false;
        }
        let changed = self.selected_position() != (row, column);
        self.row = row;
        self.column = column;
        changed
    }

    fn latched(&self, key: PickerKeyboardKey) -> PickerKeyboardLatch {
        match key {
            PickerKeyboardKey::Shift => self.shift,
            PickerKeyboardKey::Control => self.control,
            PickerKeyboardKey::Alt => self.alt,
            PickerKeyboardKey::AltGr => self.altgr,
            PickerKeyboardKey::Caps if self.shift == PickerKeyboardLatch::Locked => {
                PickerKeyboardLatch::Locked
            }
            _ => PickerKeyboardLatch::Off,
        }
    }

    fn press(&mut self) -> PickerKeyboardPress {
        match self.selected() {
            PickerKeyboardKey::Shift => {
                self.shift = self.shift.pressed();
                PickerKeyboardPress::Shifted
            }
            PickerKeyboardKey::Control => {
                self.control = self.control.pressed();
                PickerKeyboardPress::Shifted
            }
            PickerKeyboardKey::Alt => {
                self.alt = self.alt.pressed();
                PickerKeyboardPress::Shifted
            }
            PickerKeyboardKey::AltGr => {
                self.altgr = self.altgr.pressed();
                PickerKeyboardPress::Shifted
            }
            PickerKeyboardKey::Caps => {
                self.shift = match self.shift {
                    PickerKeyboardLatch::Locked => PickerKeyboardLatch::Off,
                    _ => PickerKeyboardLatch::Locked,
                };
                PickerKeyboardPress::Shifted
            }
            PickerKeyboardKey::Close => PickerKeyboardPress::Close,
            PickerKeyboardKey::Named(_, stroke) => {
                self.spend();
                PickerKeyboardPress::Type(stroke)
            }
            PickerKeyboardKey::Arrow(_) => {
                self.spend();
                PickerKeyboardPress::Type(PickerKeyboardStroke::Inert)
            }
            PickerKeyboardKey::Character(cap) => {
                // Read before spending: it is this press the armed shift is for.
                match cap.at(self.level()) {
                    Some(stroke) => {
                        self.spend();
                        PickerKeyboardPress::Type(stroke)
                    }
                    // Nothing on this face, so nothing happens — the latches
                    // included. A blank cap that spent the AltGr the user had
                    // just armed would take the modifier away for the key they
                    // were actually reaching for.
                    None => PickerKeyboardPress::Nothing,
                }
            }
        }
    }

    fn spend(&mut self) {
        self.shift = self.shift.spent();
        self.control = self.control.spent();
        self.alt = self.alt.spent();
        self.altgr = self.altgr.spent();
    }
}

fn picker_keyboard_row_scale(row: usize) -> f32 {
    if row == 0 {
        0.56
    } else {
        1.0
    }
}

/// The character rows a board with no layout to read shows.
///
/// The fallback and not the board: the caps follow whatever keyboard this
/// machine is configured for — see [`picker_keyboard_note_layout`] — and this
/// is what is drawn until that has been read, and if it will not compile.
const PICKER_KEYBOARD_NUMBER_ROW: (&str, &str) = ("`1234567890-=", "~!@#$%^&*()_+");
const PICKER_KEYBOARD_UPPER_ROW: (&str, &str) = ("qwertyuiop[]", "QWERTYUIOP{}");
const PICKER_KEYBOARD_HOME_ROW: (&str, &str) = ("asdfghjkl;'", "ASDFGHJKL:\"");
const PICKER_KEYBOARD_LOWER_ROW: (&str, &str) = ("zxcvbnm,./", "ZXCVBNM<>?");

/// The X11 keycode of every character key on the board, by row.
///
/// What makes the caps follow the layout: a keymap answers "what does this key
/// produce" about a *keycode*, so the board's ANSI positions have to be named
/// in the only language xkb has for them. X11's numbering, which is evdev's
/// plus eight.
///
/// The counts are fixed here — thirteen, thirteen, eleven, ten — which is what
/// stops a layout changing the width of a row. Every row but the function row
/// comes to exactly [`PICKER_KEYBOARD_COLUMNS`], and a keymap has no say in it.
const PICKER_KEYBOARD_KEYCODES: [&[u32]; 4] = [
    // <TLDE> and <AE01>..<AE12>
    &[49, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21],
    // <AD01>..<AD12>, then <BKSL> — which the row draws last and wider.
    &[24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 51],
    // <AC01>..<AC11>
    &[38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48],
    // <AB01>..<AB10>
    &[52, 53, 54, 55, 56, 57, 58, 59, 60, 61],
];

/// The caps of the four character rows, in [`PICKER_KEYBOARD_KEYCODES`] order.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PickerKeyboardArrangement {
    rows: [Vec<PickerKeyboardCap>; 4],
    /// Whether anything on it is reached with AltGr, which decides whether the
    /// board draws that key at all. An American board has no AltGr, and one
    /// that drew a dead key would be a key doing nothing on the layout most
    /// people use.
    altgr: bool,
}

/// What this machine's keyboards are set to, as caps this board can print.
///
/// Read once, on the first board drawn, and kept: reading a keymap is a file
/// opened and a grammar parsed, and the picker's board is built fresh every
/// frame it is on screen.
static PICKER_KEYBOARD_CAPS: std::sync::OnceLock<Option<PickerKeyboardArrangement>> =
    std::sync::OnceLock::new();

/// The caps of one character row: the layout's, or the ANSI/US fallback.
///
/// `row` is the board's own row number — 1 to 4 — which is
/// [`PICKER_KEYBOARD_KEYCODES`]'s index plus one.
fn picker_keyboard_caps(row: usize) -> Vec<PickerKeyboardCap> {
    if let Some(held) = picker_keyboard_arrangement() {
        if let Some(caps) = row.checked_sub(1).and_then(|index| held.rows.get(index)) {
            return caps.clone();
        }
    }
    let (plain, shifted) = match row {
        1 => PICKER_KEYBOARD_NUMBER_ROW,
        2 => PICKER_KEYBOARD_UPPER_ROW,
        3 => PICKER_KEYBOARD_HOME_ROW,
        _ => PICKER_KEYBOARD_LOWER_ROW,
    };
    let mut caps: Vec<PickerKeyboardCap> = plain
        .chars()
        .zip(shifted.chars())
        .map(|(plain, shifted)| PickerKeyboardCap::letter(plain, shifted))
        .collect();
    // The backslash, which the fallback rows above do not carry because it is
    // the one character key drawn at a width of its own.
    if row == 2 {
        caps.push(PickerKeyboardCap::letter('\\', '|'));
    }
    caps
}

/// Whether the board draws an AltGr key at all.
fn picker_keyboard_altgr_on_the_board() -> bool {
    picker_keyboard_arrangement().is_some_and(|held| held.altgr)
}

/// This machine's arrangement, compiled on first use.
///
/// The first of the layouts this machine names that xkbcommon will compile, in
/// the order [`picker_keyboard_layouts`] believes them. The first of them and
/// not simply the first one named, because the answer standing in front is a
/// setting: a file naming an arrangement this machine's xkeyboard-config does
/// not have must not cost the board the machine's own keyboard behind it.
fn picker_keyboard_arrangement() -> Option<&'static PickerKeyboardArrangement> {
    PICKER_KEYBOARD_CAPS
        .get_or_init(|| {
            picker_keyboard_layouts()
                .into_iter()
                .find_map(|(layout, variant)| picker_keyboard_note_layout(&layout, &variant))
        })
        .as_ref()
}

/// What this machine's keyboard is set to, as xkb layouts and variants, in the
/// order they deserve to be believed.
///
/// **The shell's own setting first.** It is the answer somebody gave on
/// Settings > Input > Keyboard > Keyboard layout, and an application reads it
/// out of the file rather than being handed a copy of it at startup — see
/// [`lxb_toolkit::settings::keyboard_layout`]. Nothing else here is a decision
/// anybody made about this board.
///
/// Then `XKB_DEFAULT_LAYOUT`, which is what libxkbcommon itself honours and what
/// a compositor that has been told a layout exports for its clients. Under this
/// shell it agrees with the setting above; under any other desktop it is the
/// only one of the two that will be there.
///
/// Then the system's own X11 keyboard configuration, which is what
/// `localectl set-x11-keymap` writes and what every session on this machine
/// starts from. Then `us` — not a guess about the machine but the same default
/// libxkbcommon has, and the reason this list always ends in something that
/// compiles.
fn picker_keyboard_layouts() -> Vec<(String, String)> {
    let mut named = Vec::new();
    named.extend(lxb_toolkit::settings::keyboard_layout());
    if let Ok(layout) = std::env::var("XKB_DEFAULT_LAYOUT") {
        if !layout.trim().is_empty() {
            let variant = std::env::var("XKB_DEFAULT_VARIANT").unwrap_or_default();
            named.push((layout.trim().to_string(), variant.trim().to_string()));
        }
    }
    named.extend(picker_keyboard_configured_layout());
    named.push(("us".to_string(), String::new()));
    // Two places saying the same thing is the ordinary case under this shell,
    // and compiling that keymap twice to find out it still will not build is
    // the one cost worth taking off a board drawn on first use.
    named.dedup();
    named
}

/// The layout out of the system's X11 keyboard configuration.
///
/// Only the first `XkbLayout`, and only the first of a comma-separated list:
/// xkb can hold several layouts at once with a key to switch between them, and
/// this board has no such key — so it offers the one the machine comes up in
/// rather than pretending to offer all of them.
fn picker_keyboard_configured_layout() -> Option<(String, String)> {
    for path in [
        "/etc/X11/xorg.conf.d/00-keyboard.conf",
        "/etc/X11/xorg.conf.d/90-keyboard.conf",
        "/etc/X11/xorg.conf",
    ] {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let option = |name: &str| {
            text.lines()
                .filter_map(|line| {
                    let rest = line.trim().strip_prefix("Option")?.trim_start();
                    let rest = rest.strip_prefix(&format!("\"{name}\""))?.trim_start();
                    Some(rest.trim_matches('"').split(',').next()?.trim().to_string())
                })
                .find(|value| !value.is_empty())
        };
        if let Some(layout) = option("XkbLayout") {
            return Some((layout, option("XkbVariant").unwrap_or_default()));
        }
    }
    None
}

/// Compile a layout and read the four character rows off it.
///
/// The four faces the board can show are xkb's first four shift levels, which
/// is what a keyboard's four-level type is: plain, Shift, AltGr, and both.
///
/// A key with nothing on a level gets nothing rather than falling back to its
/// plain character. A cap that showed `a` on the AltGr face and typed `a` when
/// pressed would be a key ignoring the modifier the user is holding.
fn picker_keyboard_note_layout(layout: &str, variant: &str) -> Option<PickerKeyboardArrangement> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        layout,
        variant,
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )?;
    let mut rows: [Vec<PickerKeyboardCap>; 4] = Default::default();
    let mut altgr = false;
    for (row, keycodes) in PICKER_KEYBOARD_KEYCODES.iter().enumerate() {
        for keycode in *keycodes {
            let cap = picker_keyboard_cap_of(&keymap, *keycode);
            altgr |= cap.has_altgr();
            rows[row].push(cap);
        }
    }
    // A keymap that compiled but says nothing about the alphabet is not an
    // arrangement — it is a layout this board cannot show, and the US fallback
    // is a better board than one with a blank home row.
    rows[2]
        .iter()
        .any(|cap| cap.at(PickerKeyboardLevel::Plain).is_some())
        .then_some(PickerKeyboardArrangement { rows, altgr })
}

/// What one key of the keymap types on each of the board's four faces.
fn picker_keyboard_cap_of(keymap: &xkb::Keymap, keycode: u32) -> PickerKeyboardCap {
    let key = xkb::Keycode::new(keycode);
    let mut levels = [None; 4];
    for (index, slot) in levels.iter_mut().enumerate() {
        // One keysym or none. A level bound to several — which xkb allows and
        // almost nothing uses — is one keycap's worth of typing here.
        *slot = keymap
            .key_get_syms_by_level(key, 0, index as u32)
            .first()
            .copied()
            .and_then(picker_keyboard_stroke_of);
    }
    PickerKeyboardCap { levels }
}

/// One keysym as this board would type it.
///
/// A character where it has one — which is nearly all of them — and the keysym
/// itself where it has not, which is the dead keys.
fn picker_keyboard_stroke_of(keysym: xkb::Keysym) -> Option<PickerKeyboardStroke> {
    if let Some(character) =
        char::from_u32(xkb::keysym_to_utf32(keysym)).filter(|c| !c.is_control())
    {
        return Some(PickerKeyboardStroke::Character(character));
    }
    picker_keyboard_dead_mark(keysym.raw())
        .is_some()
        .then_some(PickerKeyboardStroke::Dead(keysym.raw()))
}

/// The accent printed on a dead key's cap.
///
/// The one place the board cannot ask the keymap what to print, because a dead
/// key has no character. What a real keycap shows is the accent itself. A dead
/// key this table does not know is left off the board rather than shown blank:
/// a cap with nothing on it is a key nobody can find out the meaning of.
fn picker_keyboard_dead_mark(raw: u32) -> Option<&'static str> {
    let name = xkb::keysym_get_name(xkb::Keysym::new(raw));
    Some(match name.strip_prefix("dead_")? {
        "grave" => "`",
        "acute" => "´",
        "circumflex" => "^",
        "tilde" | "perispomeni" => "~",
        "macron" => "¯",
        "breve" => "˘",
        "abovedot" => "˙",
        "diaeresis" => "¨",
        "abovering" => "˚",
        "doubleacute" => "˝",
        "caron" => "ˇ",
        "cedilla" => "¸",
        "ogonek" => "˛",
        "iota" => "ͅ",
        "belowdot" => "̣",
        "hook" => "̉",
        "horn" => "̛",
        "stroke" => "̶",
        "abovecomma" | "psili" => "᾿",
        "abovereversedcomma" | "dasia" => "῾",
        "doublegrave" => "̏",
        "belowring" => "̥",
        "belowmacron" => "̱",
        "belowcircumflex" => "̭",
        "belowtilde" => "̰",
        "belowbreve" => "̮",
        "belowdiaeresis" => "̤",
        "invertedbreve" => "̑",
        "belowcomma" => "̦",
        "currency" => "¤",
        "greek" => "µ",
        _ => return None,
    })
}

fn picker_keyboard_row_spans(row: usize) -> Vec<(PickerKeyboardKey, f32)> {
    use PickerKeyboardKey::{Alt, AltGr, Arrow, Caps, Character, Close, Control, Named, Shift};

    let mut keys = Vec::new();
    match row {
        0 => {
            let span = PICKER_KEYBOARD_COLUMNS / 13.0;
            keys.push((Named("Esc", PickerKeyboardStroke::Inert), span));
            for label in [
                "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
            ] {
                keys.push((Named(label, PickerKeyboardStroke::Inert), span));
            }
        }
        1 => {
            keys.extend(
                picker_keyboard_caps(row)
                    .into_iter()
                    .map(|cap| (Character(cap), 1.0)),
            );
            keys.push((Named("Back", PickerKeyboardStroke::Backspace), 2.0));
        }
        2 => {
            keys.push((Named("Tab", PickerKeyboardStroke::Inert), 1.5));
            // The backslash is the row's last key and is drawn wider, which is
            // where ANSI puts it. It is a character key like the twelve before
            // it, so the layout has its say about what it prints — on a German
            // keyboard that position is `#`.
            let caps = picker_keyboard_caps(row);
            let (letters, wide) = caps.split_at(caps.len().saturating_sub(1));
            keys.extend(letters.iter().map(|cap| (Character(*cap), 1.0)));
            keys.extend(wide.iter().map(|cap| (Character(*cap), 1.5)));
        }
        3 => {
            keys.push((Caps, 1.75));
            keys.extend(
                picker_keyboard_caps(row)
                    .into_iter()
                    .map(|cap| (Character(cap), 1.0)),
            );
            keys.push((Named("Enter", PickerKeyboardStroke::Enter), 2.25));
        }
        4 => {
            keys.push((Shift, 2.25));
            keys.extend(
                picker_keyboard_caps(row)
                    .into_iter()
                    .map(|cap| (Character(cap), 1.0)),
            );
            keys.push((Shift, 2.75));
        }
        _ => {
            keys.push((Control, 1.5));
            keys.push((Alt, 1.5));
            // AltGr takes a key and a half out of the space bar, and only where
            // the layout has something on the faces it reaches. Right of the
            // space bar, which is where a keyboard that has one puts it.
            match picker_keyboard_altgr_on_the_board() {
                true => {
                    keys.push((Named("Space", PickerKeyboardStroke::Space), 4.0));
                    keys.push((AltGr, 1.5));
                }
                false => keys.push((Named("Space", PickerKeyboardStroke::Space), 5.5)),
            }
            keys.extend(PickerKeyboardArrow::ALL.map(|arrow| (Arrow(arrow), 1.0)));
            keys.push((Close, 2.5));
        }
    }
    keys
}

fn picker_keyboard_row_keys(row: usize) -> Vec<PickerKeyboardKey> {
    picker_keyboard_row_spans(row)
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

fn picker_keyboard_row_layout(row: usize) -> Vec<(f32, f32)> {
    let keys = picker_keyboard_row_spans(row);
    let width: f32 = keys.iter().map(|(_, span)| *span).sum();
    let mut at = (PICKER_KEYBOARD_COLUMNS - width) * 0.5;
    keys.into_iter()
        .map(|(_, span)| {
            let start = at;
            at += span;
            (start, span)
        })
        .collect()
}

fn picker_keyboard_nearest_column(from: usize, column: usize, row: usize) -> usize {
    let middle = |row: usize, column: usize| {
        picker_keyboard_row_layout(row)
            .get(column)
            .map(|(start, span)| start + span * 0.5)
            .unwrap_or(PICKER_KEYBOARD_COLUMNS * 0.5)
    };
    let wanted = middle(from, column);
    picker_keyboard_row_layout(row)
        .iter()
        .enumerate()
        .min_by(|(_, (a, aw)), (_, (b, bw))| {
            let distance = |start: f32, span: f32| (start + span * 0.5 - wanted).abs();
            distance(*a, *aw).total_cmp(&distance(*b, *bw))
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PickerMenu {
    #[default]
    Options,
    Kinds,
    Sorts,
}

#[derive(Clone, Copy)]
struct PickerAsking<'a> {
    purpose: PickerPurpose,
    name: &'a str,
    ticked: &'a [std::path::PathBuf],
}

#[derive(Debug, Default)]
pub struct FilePicker {
    purpose: PickerPurpose,
    name: String,
    ticked: Vec<std::path::PathBuf>,
    pad: bool,
    /// Whether the foot names the buttons at all — Settings > System > Button
    /// hints, which is one answer for the whole session and reaches an
    /// application through `lxb_toolkit::settings::button_hints`.
    ///
    /// `None` until somebody says, and read as *written*: a machine that has
    /// never had this shell on it, and a settings file older than the key, both
    /// look like this, and neither is somebody asking for a panel with no
    /// legend on it.
    hints: Option<bool>,
    sort: PickerSort,
    hidden: bool,
    filtered: bool,
    menu: ContextMenu,
    menu_at: PickerMenu,
    menu_anchor: [f32; 4],
    legend_left: f32,
    levels: Vec<PickerLevel>,
    open: usize,
    open_state: bool,
    out: f32,
    depth: f32,
    depth_speed: f32,
    light: Selection,
    pressing: Pressing,
    keyboard: PickerKeyboard,
}

impl FilePicker {
    pub fn open(
        &mut self,
        selection: PickerSelection,
        directory: impl AsRef<std::path::Path>,
    ) -> bool {
        self.open_for(PickerPurpose::of(selection), selection, directory, "")
    }

    pub fn open_for(
        &mut self,
        purpose: PickerPurpose,
        selection: PickerSelection,
        directory: impl AsRef<std::path::Path>,
        name: &str,
    ) -> bool {
        if self.open_state || self.showing() {
            return false;
        }
        let selection = match purpose {
            PickerPurpose::AFolder => PickerSelection::Folder,
            _ if selection.chooses_folder() => PickerSelection::File,
            _ => selection,
        };
        self.purpose = purpose;
        self.name = name.to_string();
        self.ticked.clear();
        self.sort = PickerSort::default();
        self.hidden = false;
        self.filtered = true;
        self.menu.close();
        self.menu_at = PickerMenu::Options;
        let named = !self.name.trim().is_empty();

        let start = Picker::new(selection, directory);
        let mut paths: Vec<_> = start
            .location()
            .ancestors()
            .map(std::path::Path::to_path_buf)
            .collect();
        paths.reverse();
        self.levels = paths
            .iter()
            .map(|path| PickerLevel::new(purpose, selection, path, named))
            .collect();
        if self.levels.is_empty() {
            return false;
        }
        let mut first_visible = 0;
        for index in 0..self.levels.len().saturating_sub(1) {
            let child = self.levels[index + 1].picker.location().to_path_buf();
            if !self.levels[index].focus_child(&child) {
                first_visible = index + 1;
            }
        }
        if first_visible > 0 {
            self.levels.drain(0..first_visible);
        }

        self.open = self.levels.len() - 1;
        self.open_state = true;
        self.out = 0.0;
        self.depth = self.open as f32;
        self.depth_speed = 0.0;
        self.light.clear();
        self.pressing = Pressing::default();
        self.keyboard = PickerKeyboard::default();
        true
    }

    pub fn close(&mut self) {
        self.open_state = false;
        self.keyboard.close();
        self.menu.close();
    }

    pub const fn purpose(&self) -> PickerPurpose {
        self.purpose
    }

    pub fn hand(&mut self, pad: bool) {
        self.pad = pad;
    }

    /// Whether the panel writes what its buttons do. See [`FilePicker::hints`],
    /// where what nothing at all means is written down.
    pub fn say_what_the_buttons_do(&mut self, hints: bool) {
        self.hints = Some(hints);
    }

    fn writes_what_the_buttons_do(&self) -> bool {
        self.hints.unwrap_or(true)
    }

    pub fn menu_is_open(&self) -> bool {
        self.menu.is_open()
    }

    pub fn open_menu(&mut self) -> bool {
        if !self.open_state || self.menu.is_open() {
            return false;
        }
        self.menu_at = PickerMenu::Options;
        let rows = self.menu_rows();
        self.menu.set_window(rows.len().max(1));
        self.menu.open_at(self.menu_anchor, None, rows)
    }

    pub fn close_menu(&mut self) -> bool {
        if !self.menu.is_open() {
            return false;
        }
        self.menu.close();
        true
    }

    pub fn menu_step(&mut self, delta: isize) -> bool {
        if !self.menu.is_open() {
            return false;
        }
        self.menu.step(delta);
        true
    }

    pub fn menu_point_at(&mut self, spot: Spot) -> bool {
        self.menu.is_open() && self.menu.point_at(spot)
    }

    pub fn menu_press(&mut self) -> bool {
        if !self.menu.is_open() {
            return false;
        }
        self.menu.press();
        let row = self.menu.selected();
        match self.menu_at {
            PickerMenu::Options => {
                let mut at = 0;
                if self.offers_kinds() {
                    if row == 0 {
                        return self.step_into(PickerMenu::Kinds);
                    }
                    at = 1;
                }
                if row == at {
                    return self.step_into(PickerMenu::Sorts);
                }
                let hidden = !self.hidden;
                self.hidden = hidden;
                self.each_level(|picker| {
                    picker.show_hidden(hidden);
                });
                self.refresh_menu()
            }
            PickerMenu::Kinds => {
                let filtered = row == 0 && self.offers_kinds();
                self.filtered = filtered;
                let kind = filtered.then_some(0);
                self.each_level(|picker| {
                    picker.show_kind(kind);
                });
                self.refresh_menu()
            }
            PickerMenu::Sorts => {
                let sort = PickerSort::ALL
                    .get(row)
                    .copied()
                    .unwrap_or(PickerSort::NameAscending);
                self.sort = sort;
                self.each_level(|picker| {
                    picker.sort_by(sort);
                });
                self.refresh_menu()
            }
        }
    }

    fn step_into(&mut self, list: PickerMenu) -> bool {
        self.menu_at = list;
        self.refresh_menu()
    }

    fn refresh_menu(&mut self) -> bool {
        let rows = self.menu_rows();
        let standing = self.menu.selected().min(rows.len().saturating_sub(1));
        self.menu.set_window(rows.len().max(1));
        self.menu.open_at(self.menu_anchor, None, rows);
        self.menu.step(standing as isize);
        true
    }

    fn dress(&self, level: &mut PickerLevel) {
        let mut changed = level.picker.sort_by(self.sort);
        changed |= level.picker.show_hidden(self.hidden);
        changed |= level.picker.show_kind(self.filtered.then_some(0));
        if changed {
            level.refresh_head();
            level.head_focused = level.opens_on(!self.name.trim().is_empty());
        }
    }

    fn each_level(&mut self, mut act: impl FnMut(&mut Picker)) {
        for level in &mut self.levels {
            act(&mut level.picker);
            level.refresh_head();
        }
    }

    fn offers_kinds(&self) -> bool {
        self.active_level()
            .is_some_and(|level| !level.picker.kinds().is_empty())
    }

    fn menu_rows(&self) -> Vec<Entry> {
        match self.menu_at {
            PickerMenu::Options => {
                let mut rows = Vec::new();
                if let Some(level) = self.active_level() {
                    if !level.picker.kinds().is_empty() {
                        rows.push(
                            Entry::new(lxb_toolkit::i18n::text("types"))
                                .glyph("file-page")
                                .detail(level.picker.showing()),
                        );
                    }
                }
                let group = u8::from(!rows.is_empty());
                rows.push(
                    Entry::new(lxb_toolkit::i18n::text("sort"))
                        .glyph("sort")
                        .detail(self.sort.label())
                        .group(group),
                );
                rows.push(
                    Entry::new(if self.hidden {
                        lxb_toolkit::i18n::text("hide-hidden-files")
                    } else {
                        lxb_toolkit::i18n::text("show-hidden-files")
                    })
                    .glyph(if self.hidden {
                        "chosen"
                    } else {
                        "setting-info"
                    })
                    .group(group),
                );
                rows
            }
            PickerMenu::Kinds => {
                let mut rows = Vec::new();
                if let Some(level) = self.active_level() {
                    for kind in level.picker.kinds() {
                        rows.push(ticked_row(kind.name, self.filtered));
                    }
                }
                rows.push(ticked_row(
                    lxb_toolkit::i18n::text("everything").to_string(),
                    !self.filtered,
                ));
                rows
            }
            PickerMenu::Sorts => PickerSort::ALL
                .iter()
                .map(|sort| ticked_row(sort.label().to_string(), *sort == self.sort))
                .collect(),
        }
    }

    pub fn named(&self) -> &str {
        &self.name
    }

    pub fn ticked(&self) -> &[std::path::PathBuf] {
        &self.ticked
    }

    pub fn where_it_is(&self) -> String {
        self.location()
            .map(|at| at.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn narrowed_to(&self) -> String {
        if matches!(self.purpose, PickerPurpose::ANewFile) {
            let name = match self.name.trim() {
                "" => lxb_toolkit::i18n::text("untitled"),
                name => name,
            };
            let typing = self
                .active_level()
                .and_then(PickerLevel::focused_head)
                .is_some_and(|row| matches!(row, PickerHeadRow::Name));
            return match typing {
                true => lxb_toolkit::message!("saving-as-typing", "name" => name.to_string()),
                false => lxb_toolkit::message!("saving-as", "name" => name.to_string()),
            };
        }
        let kinds = self
            .active_level()
            .map(|level| level.picker.showing())
            .unwrap_or_else(|| lxb_toolkit::i18n::text("everything").to_string());
        lxb_toolkit::message!("showing-kinds", "kinds" => kinds)
    }

    fn asking(&self) -> PickerAsking<'_> {
        PickerAsking {
            purpose: self.purpose,
            name: &self.name,
            ticked: &self.ticked,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open_state
    }

    pub fn search_keyboard_open(&self) -> bool {
        self.keyboard.is_open()
    }

    pub fn move_search_keyboard(&mut self, horizontal: isize, vertical: isize) -> bool {
        self.keyboard.move_by(horizontal, vertical)
    }

    pub fn press_search_keyboard(&mut self) -> bool {
        if !self.keyboard.is_open() {
            return false;
        }
        match self.keyboard.press() {
            PickerKeyboardPress::Type(PickerKeyboardStroke::Character(character)) => {
                let _ = self.type_text(&character.to_string());
                true
            }
            PickerKeyboardPress::Type(PickerKeyboardStroke::Space) => {
                let _ = self.type_text(" ");
                true
            }
            PickerKeyboardPress::Type(PickerKeyboardStroke::Backspace) => {
                let _ = self.erase_search();
                true
            }
            PickerKeyboardPress::Type(PickerKeyboardStroke::Enter) => self.submit_search_keyboard(),

            // A dead key among the inert ones: this board types into a field
            // rather than through a keymap, so there is nothing here for an
            // accent to combine with. It stays *on* the board because a key
            // that is on the keyboard and missing from the picture of it is a
            // picture that is wrong — and the letters it would have made are on
            // the AltGr face, where they can be typed directly.
            PickerKeyboardPress::Type(
                PickerKeyboardStroke::Inert | PickerKeyboardStroke::Dead(_),
            ) => true,
            PickerKeyboardPress::Shifted | PickerKeyboardPress::Nothing => true,
            PickerKeyboardPress::Close => self.close_search_keyboard(),
        }
    }

    pub fn submit_search_keyboard(&mut self) -> bool {
        if !self.keyboard.is_open() {
            return false;
        }
        self.keyboard.close();
        match self.active_level().and_then(PickerLevel::focused_head) {
            Some(PickerHeadRow::NewFolder) => {
                self.make_the_folder();
            }
            Some(PickerHeadRow::SearchField) => {
                if let Some(level) = self.active_level_mut() {
                    if !level.picker.entries().is_empty() {
                        let _ = level.picker.select(0);
                        level.head_focused = None;
                    }
                }
            }
            _ => {}
        }
        self.light.clear();
        true
    }

    fn make_the_folder(&mut self) -> bool {
        let Some(level) = self.active_level_mut() else {
            return false;
        };
        let name = std::mem::take(&mut level.making);
        let Ok(made) = level.picker.make_folder(&name) else {
            level.refresh_head();
            return false;
        };
        level.refresh_head();
        level.focus_child(&made);
        true
    }

    fn tick(&mut self) -> bool {
        let Some(path) = self
            .active_level()
            .and_then(|level| level.picker.selected_entry())
            .filter(|entry| entry.is_file())
            .map(|entry| entry.path.clone())
        else {
            return false;
        };
        match self.ticked.iter().position(|already| *already == path) {
            Some(at) => {
                self.ticked.remove(at);
            }
            None => self.ticked.push(path),
        }
        true
    }

    pub fn close_search_keyboard(&mut self) -> bool {
        self.keyboard.close()
    }

    pub fn press_search_key(&mut self, row: usize, column: usize) -> bool {
        if !self.keyboard.is_open()
            || row >= PICKER_KEYBOARD_ROWS
            || column >= picker_keyboard_row_keys(row).len()
        {
            return false;
        }
        self.keyboard.select(row, column);
        self.press_search_keyboard()
    }

    pub fn showing(&self) -> bool {
        self.out > 0.0
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        let mut moving = self.pressing.advance(dt) || self.keyboard.advance(dt);
        let target = if self.open_state { 1.0 } else { 0.0 };
        let step = dt / motion::duration::MENU_FLIGHT;
        if (self.out - target).abs() <= step {
            self.out = target;
        } else {
            self.out += step * (target - self.out).signum();
            moving = true;
        }

        if !self.levels.is_empty() {
            let (depth, speed) = motion::spring(
                self.depth as f64,
                self.depth_speed as f64,
                self.open as f64,
                motion::HIGHLIGHT_SPRING,
                dt as f64,
            );
            self.depth = depth as f32;
            self.depth_speed = speed as f32;
            moving |=
                (self.depth - self.open as f32).abs() > 0.001 || self.depth_speed.abs() > 0.001;
            for level in &mut self.levels {
                moving |= level.advance(dt);
            }
            moving |= self.menu.advance(dt);

            if self.levels.len() > self.open + 1
                && (self.depth - self.open as f32).abs() <= 0.001
                && self.depth_speed.abs() <= 0.001
            {
                self.levels.truncate(self.open + 1);
            }
        }

        if !self.open_state && self.out == 0.0 {
            self.levels.clear();
            self.open = 0;
            self.depth = 0.0;
            self.depth_speed = 0.0;
            self.light.clear();
            self.keyboard = PickerKeyboard::default();
        }
        moving
    }

    pub fn travelled(&self) -> f32 {
        motion::ease(self.out.clamp(0.0, 1.0))
    }

    pub fn selection(&self) -> Option<PickerSelection> {
        self.active_level().map(|level| level.picker.selection())
    }

    pub fn location(&self) -> Option<&std::path::Path> {
        self.active_level().map(|level| level.picker.location())
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        if self.keyboard.is_open() {
            return false;
        }
        let direction = delta.signum();
        if direction == 0 {
            return false;
        }
        let mut moved = false;
        for _ in 0..delta.unsigned_abs() {
            let Some(level) = self.active_level_mut() else {
                break;
            };
            if !level.move_one(direction) {
                break;
            }
            moved = true;
        }
        if moved {
            self.light.clear();
        }
        moved
    }

    pub fn enter(&mut self) -> bool {
        if self.keyboard.is_open() {
            return false;
        }
        let Some(selection) = self.selection() else {
            return false;
        };
        let head = self
            .active_level()
            .and_then(|level| level.head_focused.and_then(|row| level.head_row(row)));
        if matches!(head, Some(PickerHeadRow::SearchClear)) {
            let level = self
                .active_level_mut()
                .expect("the active picker level was just found");
            level.picker.search(String::new());
            level.refresh_head();
            self.light.clear();
            return true;
        }
        if head.is_some() {
            return false;
        }
        let path = {
            let Some(level) = self.active_level() else {
                return false;
            };
            let Some(entry) = level.picker.selected_entry() else {
                return false;
            };
            if !entry.is_folder() {
                return false;
            }
            entry.path.clone()
        };

        let named = !self.name.trim().is_empty();
        self.levels.truncate(self.open + 1);
        let mut level = PickerLevel::new(self.purpose, selection, path, named);
        self.dress(&mut level);
        self.levels.push(level);
        self.open += 1;
        self.light.clear();
        true
    }

    pub fn activate(&mut self) -> bool {
        if self.keyboard.is_open() {
            return self.press_search_keyboard();
        }
        match self.active_level().and_then(PickerLevel::focused_head) {
            Some(row) if row.is_typed_into() => {
                self.keyboard.open();
                true
            }
            None if self.purpose.takes_several() => self.tick(),
            _ => self.enter(),
        }
    }

    pub fn leave(&mut self) -> bool {
        if self.keyboard.is_open() {
            return false;
        }
        self.leave_to(1)
    }

    pub fn leave_to(&mut self, steps: usize) -> bool {
        if self.keyboard.is_open() {
            return false;
        }
        if steps == 0 || steps > self.open {
            return false;
        }
        self.open -= steps;
        self.light.clear();
        true
    }

    pub fn choose(&self) -> Option<std::path::PathBuf> {
        self.chose().into_iter().next()
    }

    pub fn chose(&self) -> Vec<std::path::PathBuf> {
        let Some(level) = self.active_level() else {
            return Vec::new();
        };
        match level.focused_head() {
            Some(PickerHeadRow::Answer) => self.answer(),
            Some(_) => Vec::new(),
            None if matches!(self.purpose, PickerPurpose::OneFile) => {
                level.picker.choose().into_iter().collect()
            }
            None => Vec::new(),
        }
    }

    fn answer(&self) -> Vec<std::path::PathBuf> {
        let Some(level) = self
            .active_level()
            .filter(|level| level.picker.accessible())
        else {
            return Vec::new();
        };
        let here = level.picker.location().to_path_buf();
        match self.purpose {
            PickerPurpose::AFolder => vec![here],
            PickerPurpose::ManyFiles => self.ticked.clone(),
            PickerPurpose::ANewFile => match self.name.trim() {
                "" => Vec::new(),
                name => vec![here.join(name)],
            },
            PickerPurpose::OneFile => Vec::new(),
        }
    }

    pub fn press(&mut self) {
        self.pressing.press();
    }

    pub fn type_text(&mut self, text: &str) -> bool {
        let typed: String = text
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        if typed.is_empty() {
            return false;
        }
        let Some(head) = self
            .active_level()
            .and_then(PickerLevel::focused_head)
            .filter(|row| row.is_typed_into())
        else {
            return false;
        };
        match head {
            PickerHeadRow::Name => self.name.push_str(&typed),
            PickerHeadRow::NewFolder => {
                let Some(level) = self.active_level_mut() else {
                    return false;
                };
                level.making.push_str(&typed);
            }
            _ => {
                let Some(level) = self.active_level_mut() else {
                    return false;
                };
                let query = format!("{}{}", level.picker.query(), typed);
                level.picker.search(query);
                level.refresh_head();
            }
        }
        self.light.clear();
        true
    }

    pub fn erase_search(&mut self) -> bool {
        let Some(head) = self
            .active_level()
            .and_then(PickerLevel::focused_head)
            .filter(|row| row.is_typed_into())
        else {
            return false;
        };
        match head {
            PickerHeadRow::Name => {
                if self.name.pop().is_none() {
                    return false;
                }
            }
            PickerHeadRow::NewFolder => {
                let Some(level) = self.active_level_mut() else {
                    return false;
                };
                if level.making.pop().is_none() {
                    return false;
                }
            }
            _ => {
                let Some(level) = self.active_level_mut() else {
                    return false;
                };
                if level.picker.query().is_empty() {
                    return false;
                }
                let mut query = level.picker.query().to_string();
                query.pop();
                level.picker.search(query);
                level.refresh_head();
            }
        }
        self.light.clear();
        true
    }

    pub fn point_at(&mut self, spot: Spot) -> bool {
        if self.keyboard.is_open() {
            return match spot {
                Spot::PickerKey { row, column } => self.keyboard.select(row, column),
                _ => false,
            };
        }
        match spot {
            Spot::PickerRow(row) => {
                let changed = self
                    .active_level_mut()
                    .is_some_and(|level| level.select_row(row));
                if changed {
                    self.light.clear();
                }
                changed
            }
            Spot::PickerTrail(steps) => steps > 0 && steps <= self.open,
            Spot::PickerLeave => self.open > 0,
            _ => false,
        }
    }

    fn active_level(&self) -> Option<&PickerLevel> {
        self.levels.get(self.open)
    }

    fn active_level_mut(&mut self) -> Option<&mut PickerLevel> {
        self.levels.get_mut(self.open)
    }
}

impl Ui {
    pub fn file_picker(&mut self, state: &mut FilePicker) {
        if !state.showing() || state.levels.is_empty() {
            return;
        }

        let out = state.travelled();
        let width = self.width();
        let height = self.height();
        let full = [0.0, 0.0, width, height];
        let panel = picker_window(width, height);
        let picker_height = panel[3];
        let picker_scale = lxb_toolkit::metrics::scale_for(picker_height);
        let focused_icon = Metric::ItemIconFocused.on(picker_height);
        let idle_icon = Metric::ItemIcon.on(picker_height);
        let spacing = Metric::ItemSpacing.on(picker_height);
        let step = picker_column_step(picker_scale, focused_icon);
        let foot = PICKER_FOOT * picker_scale;
        let content_clip = picker_body(panel, picker_scale);
        let cross_x = content_clip[0] + content_clip[2] * PICKER_CROSS_X;
        let cross_y = content_clip[1] + content_clip[3] * PICKER_CROSS_Y;
        let slide = (1.0 - out) * content_clip[2] * (1.0 - PICKER_CROSS_X);
        let title_line = Text::Title.on(picker_height) * Text::LINE;
        let body_line = Text::Body.on(picker_height) * Text::LINE;
        let caption_line = Text::Caption.on(picker_height) * Text::LINE;
        let label_line = Text::Label.on(picker_height) * Text::LINE;

        self.recede(out, panel);
        self.recede_behind(full, 1.0 - PICKER_PAGE_DIM * out, 0.88 * out);
        self.mark_spot(Spot::OutsidePicker, full);

        self.in_overlay = true;
        self.quad(
            OVER,
            Quad::solid(full, 0.0, self.tinted(Role::Glass, PICKER_SCRIM * out)),
        );

        let mut ambient = Surface::Sidebar.glass();
        ambient.frost = PICKER_PAGE_FROST;
        ambient.gloss = 0.45;
        ambient.curve = 0.0;
        self.quad(
            OVER,
            Quad::glass(
                full,
                0.0,
                self.tinted(Role::Glass, 0.18 * out),
                ambient,
                self.scale,
            )
            .faded(0.48 * out),
        );

        let overlay = Overlay::Dialog.material();
        let mut window = overlay.glass;

        window.frost = PICKER_WINDOW_FROST;
        window.gloss = 0.38;
        window.curve = 0.0;
        let radius = picker_scale * overlay.radius;
        self.quad(
            OVER,
            Quad::solid(
                [
                    panel[0] - 1.0,
                    panel[1] - 1.0,
                    panel[2] + 2.0,
                    panel[3] + 2.0,
                ],
                radius + 1.0,
                self.tinted(Role::AccentSoft, 0.06 * out),
            ),
        );
        self.quad(
            OVER,
            Quad::solid(
                panel,
                radius,
                self.tinted(Role::Glass, PICKER_WINDOW_BASE * out),
            ),
        );
        self.quad(
            OVER,
            Quad::solid(
                panel,
                radius,
                self.tinted(Role::AccentDeep, PICKER_WINDOW_STAIN * out),
            ),
        );
        self.quad(
            OVER,
            Quad::glass(
                panel,
                radius,
                self.tinted(Role::AccentDeep, PICKER_WINDOW_STAIN * out),
                window,
                picker_scale,
            )
            .faded(PICKER_WINDOW_GLASS * out),
        );

        let content_at = self.mark(OVER);

        let selection = state.selection().unwrap_or(PickerSelection::File);
        let title = picker_title(state.purpose(), selection);

        let pulse = motion::pulse(self.seconds());
        let asked = state.asking();
        for (index, level) in state.levels.iter().enumerate() {
            let level_depth = index as f32;
            let standing = picker_standing(index, state.open);
            let active = 1.0 - (state.depth - level_depth).abs().min(1.0);
            let arriving = (1.0 - (level_depth - state.depth)).clamp(0.0, 1.0);
            let present = match standing {
                PickerStanding::Leaving => picker_departing(arriving),
                PickerStanding::Open | PickerStanding::Behind => arriving,
            };
            if present <= 0.01 {
                continue;
            }
            let (near, haze) = picker_receded(state.depth - level_depth);
            let x = cross_x + (level_depth - state.depth) * step + slide;
            let clarity = haze * picker_edge_alpha(x, panel[0], panel[0] + panel[2], focused_icon);
            if clarity <= 0.01 {
                continue;
            }
            let text_x = x + (focused_icon * PICKER_DISC * 0.5 + 12.0 * picker_scale) * near;
            let next_x = cross_x + (level_depth + 1.0 - state.depth) * step + slide;
            let text_width = if index < state.open {
                (next_x - focused_icon * PICKER_DISC * 0.5 * near - 10.0 * picker_scale - text_x)
                    .max(0.0)
            } else {
                (panel[0] + panel[2] - text_x - 40.0 * picker_scale).max(0.0)
            };
            let focus = level.focused_row();
            let rows =
                picker_rows_in_view(level.position, level.row_count(), spacing * near, panel[3]);

            for row in rows {
                let trail = matches!(standing, PickerStanding::Behind) && focus == Some(row);
                let offset = row as f32 - level.position;
                let y = picker_row_y(
                    index,
                    offset,
                    cross_y,
                    spacing * near,
                    PICKER_GAP_ABOVE * picker_scale,
                    PICKER_GAP_BELOW * picker_scale,
                );
                if y < panel[1] - spacing || y > panel[1] + panel[3] + spacing {
                    continue;
                }
                let distance = offset.abs();

                let selected = focus == Some(row) && distance < 0.5 && active > 0.5;
                let row_focus = if selected { active.max(0.0) } else { 0.0 };
                let icon = (idle_icon + (focused_icon - idle_icon) * row_focus) * near;
                let row_presence = match standing {
                    _ if trail => present,
                    PickerStanding::Leaving => present,
                    PickerStanding::Behind => picker_departing(active),
                    PickerStanding::Open => active,
                };
                let mut alpha = out
                    * row_presence
                    * clarity
                    * (1.0 - distance / 6.0).clamp(0.0, 1.0)
                    * picker_vertical_alpha(y, panel[1], panel[1] + panel[3], picker_scale);

                if index == 0 && offset > -1.0 && offset < 0.0 {
                    alpha *= 1.0 - 0.7 * (1.0 - (2.0 * offset + 1.0).abs());
                }
                if alpha <= 0.01 {
                    continue;
                }

                let (label, detail, glyph, quiet) = picker_row_content(asked, level, row);
                let preview = picker_row_entry(level, row)
                    .filter(|entry| {
                        entry.kind == EntryKind::File
                            && picker_file_glyph(&entry.path) == "category-images"
                    })
                    .and_then(|entry| self.thumbnail(&entry.path));
                let ink = if quiet && !selected {
                    PICKER_FILE_INK
                } else {
                    1.0
                };
                let reach = [
                    x - focused_icon * PICKER_DISC * 0.5 * near,
                    y - spacing * 0.44,
                    (text_x + text_width - (x - focused_icon * PICKER_DISC * 0.5 * near)).max(icon),
                    spacing * 0.88,
                ];
                if let Some(reach) = picker_intersection(reach, content_clip) {
                    if matches!(standing, PickerStanding::Open) {
                        self.mark_spot(Spot::PickerRow(row), reach);
                    } else if trail {
                        self.mark_spot(Spot::PickerTrail(state.open - index), reach);
                    }
                }

                if selected {
                    let disc_size = icon * PICKER_DISC;
                    let still = [
                        x - disc_size * 0.5,
                        y - disc_size * 0.5,
                        disc_size,
                        disc_size,
                    ];

                    let disc = still;
                    let glow = icon * (2.3 + 0.2 * pulse);
                    self.quad(
                        OVER,
                        Quad::light(
                            [x - glow * 0.5, y - glow * 0.5, glow, glow],
                            self.tinted(Role::Accent, (0.34 + 0.26 * pulse) * alpha),
                        ),
                    );
                    self.quad(
                        OVER,
                        Quad::glass(
                            disc,
                            disc[3] * 0.5,
                            self.tinted(Role::Accent, 0.13 * alpha),
                            Surface::Control.glass(),
                            picker_scale,
                        ),
                    );
                }

                if let Some(preview) = preview {
                    let size = icon * PICKER_ITEM_PREVIEW;
                    self.quad(
                        OVER,
                        Quad::image(
                            [x - size * 0.5, y - size * 0.5, size, size],
                            size * 0.5,
                            picker_round_preview_cell(preview),
                            alpha,
                        ),
                    );
                } else {
                    let glyph_role = if selected {
                        Role::TextSoft
                    } else {
                        Role::AccentSoft
                    };
                    self.icon_tinted(
                        [x - icon * 0.5, y - icon * 0.5, icon, icon],
                        glyph,
                        self.icons,
                        glyph_role,
                        alpha * ink * if selected { 0.90 } else { 0.75 },
                    );
                }
                if asked.purpose.takes_several() {
                    if let Some(entry) = picker_row_entry(level, row) {
                        if asked.ticked.contains(&entry.path) {
                            let tick = icon * PICKER_TICK * near;
                            let band = [text_x + text_width - tick, y - tick * 0.5, tick, tick];
                            if picker_intersection(band, content_clip).is_some() {
                                self.icon_tinted(
                                    band,
                                    "chosen",
                                    self.icons,
                                    Role::Accent,
                                    alpha * ink,
                                );
                            }
                        }
                    }
                }
                if selected {
                    let name_line = title_line * near;
                    self.label_weighted_sized_clipped(
                        [text_x, y - name_line * 0.96, text_width, name_line],
                        Text::Title,
                        &label,
                        self.tinted(Role::Text, alpha * ink),
                        Align::Left,
                        true,
                        Text::Title.on(picker_height) * near,
                        Some(content_clip),
                    );
                    self.label_weighted_sized_clipped(
                        [
                            text_x,
                            y + 3.0 * picker_scale * near,
                            text_width,
                            caption_line * near,
                        ],
                        Text::Caption,
                        &detail,
                        self.tinted(Role::TextSoft, alpha * 0.85 * ink),
                        Align::Left,
                        false,
                        Text::Caption.on(picker_height) * near,
                        Some(content_clip),
                    );
                } else {
                    self.label_weighted_sized_clipped(
                        [text_x, y - body_line * 0.5, text_width, body_line],
                        Text::Body,
                        &label,
                        self.tinted(Role::Text, alpha * 0.62 * if trail { 0.85 } else { ink }),
                        Align::Left,
                        false,
                        Text::Body.on(picker_height),
                        Some(content_clip),
                    );
                }
            }

            if level.row_count() == 0 && index == state.open {
                self.label_weighted_sized_clipped(
                    [
                        text_x,
                        cross_y + PICKER_GAP_BELOW * picker_scale,
                        text_width,
                        body_line,
                    ],
                    Text::Body,
                    level.picker.note(),
                    self.tinted(Role::TextSoft, out * clarity * 0.78),
                    Align::Left,
                    false,
                    Text::Body.on(picker_height),
                    Some(content_clip),
                );
            }
        }

        let prompt_x = cross_x - state.depth * step + slide;
        let prompt_steps = state.depth.max(0.0) + state.depth.clamp(0.0, 1.0);
        let (prompt_near, prompt_ink) = picker_receded(prompt_steps);
        let prompt_alpha = out
            * prompt_ink
            * picker_edge_alpha(prompt_x, panel[0], panel[0] + panel[2], focused_icon);
        if prompt_alpha > 0.01 {
            let icon = Metric::ColumnIconFocused.on(picker_height) * prompt_near;
            let disc = icon * PICKER_CATEGORY_DISC;
            let prompt = [prompt_x - disc * 0.5, cross_y - disc * 0.5, disc, disc];
            if state.open > 0 {
                if let Some(reach) = picker_intersection(prompt, content_clip) {
                    self.mark_spot(Spot::PickerTrail(state.open), reach);
                }
            }
            self.quad(
                OVER,
                Quad::glass(
                    prompt,
                    disc * 0.30,
                    self.tinted(Role::Accent, 0.10 * prompt_alpha),
                    Surface::Control.glass(),
                    picker_scale,
                ),
            );
            self.icon_tinted(
                [prompt_x - icon * 0.5, cross_y - icon * 0.5, icon, icon],
                "file-folder",
                self.icons,
                Role::AccentSoft,
                prompt_alpha * 0.82,
            );
            self.label_weighted_sized_clipped(
                [
                    prompt_x - 130.0 * picker_scale * prompt_near,
                    cross_y + disc * 0.5 + 4.0 * picker_scale,
                    260.0 * picker_scale * prompt_near,
                    label_line * prompt_near,
                ],
                Text::Label,
                title,
                self.tinted(Role::Text, prompt_alpha * 0.78),
                Align::Centre,
                false,
                Text::Label.on(picker_height) * prompt_near,
                Some(content_clip),
            );
        }

        if state.keyboard.showing() {
            self.picker_search_keyboard(state, panel, picker_scale, content_clip);
        }

        self.clipped(OVER, content_at, None, content_clip);

        self.picker_furniture(state, panel, picker_scale, foot, title, out);
        self.picker_legend(state, panel, picker_scale, foot, out);

        if state.menu.showing() {
            let grown = lxb_toolkit::menu::growing(
                state.menu_anchor,
                state.menu_anchor,
                state.menu.travelled(),
            );
            self.cut_text_under(&[crate::renderer::OVER], grown, state.menu.travelled());
            self.context_menu(&mut state.menu);
        }

        self.light = None;
        self.in_overlay = false;
    }
}

fn picker_receded(steps: f32) -> (f32, f32) {
    let steps = steps.max(0.0);
    (
        PICKER_DEPTH_SHRINK
            .powf(steps)
            .max(PICKER_DEPTH_FLOOR_SCALE),
        PICKER_DEPTH_HAZE.powf(steps).max(PICKER_DEPTH_FLOOR_HAZE),
    )
}

fn picker_standing(index: usize, open: usize) -> PickerStanding {
    match index.cmp(&open) {
        std::cmp::Ordering::Less => PickerStanding::Behind,
        std::cmp::Ordering::Equal => PickerStanding::Open,
        std::cmp::Ordering::Greater => PickerStanding::Leaving,
    }
}

fn picker_departing(left: f32) -> f32 {
    motion::ease(((left - PICKER_COLUMN_GONE_BY) / (1.0 - PICKER_COLUMN_GONE_BY)).clamp(0.0, 1.0))
}

fn picker_window(width: f32, height: f32) -> [f32; 4] {
    let picker_width = width * PICKER_WINDOW_WIDTH;
    let picker_height = height * PICKER_WINDOW_HEIGHT;
    [
        (width - picker_width) * 0.5,
        (height - picker_height) * 0.5,
        picker_width,
        picker_height,
    ]
}

fn picker_body(panel: [f32; 4], scale: f32) -> [f32; 4] {
    let [x, y, width, height] = panel;
    let margin = PICKER_MARGIN * scale;
    let head = PICKER_HEAD * scale;
    let foot = (PICKER_FOOT + PICKER_WHERE) * scale;
    [
        x + margin,
        y + head,
        (width - margin * 2.0).max(0.0),
        (height - head - foot).max(0.0),
    ]
}

fn picker_intersection([ax, ay, aw, ah]: [f32; 4], [bx, by, bw, bh]: [f32; 4]) -> Option<[f32; 4]> {
    let x = ax.max(bx);
    let y = ay.max(by);
    let right = (ax + aw).min(bx + bw);
    let bottom = (ay + ah).min(by + bh);
    (right > x && bottom > y).then_some([x, y, right - x, bottom - y])
}

#[derive(Debug, Clone, Copy)]
struct PickerKeyboardLayout {
    panel: [f32; 4],
    scale: f32,
}

impl PickerKeyboardLayout {
    fn lift(self, arrived: f32) -> f32 {
        (1.0 - arrived) * (self.panel[3] + PICKER_KEYBOARD_BOTTOM * self.scale)
    }

    fn panel_rect(self, arrived: f32) -> [f32; 4] {
        [
            self.panel[0],
            self.panel[1] + self.lift(arrived),
            self.panel[2],
            self.panel[3],
        ]
    }

    fn key_rect(self, row: usize, column: usize, arrived: f32) -> [f32; 4] {
        let unit = PICKER_KEYBOARD_KEY_WIDTH * self.scale;
        let gap = PICKER_KEYBOARD_GAP * self.scale;
        let pad = PICKER_KEYBOARD_PAD * self.scale;
        let (start, span) = picker_keyboard_row_layout(row)
            .get(column)
            .copied()
            .unwrap_or((0.0, 1.0));
        let (top, height) = picker_keyboard_row_band(row);
        [
            self.panel[0] + pad + start * unit + gap * 0.5,
            self.panel[1] + pad + top * self.scale + self.lift(arrived),
            span * unit - gap,
            height * self.scale,
        ]
    }
}

fn picker_keyboard_layout(picker: [f32; 4], scale: f32) -> PickerKeyboardLayout {
    let natural = (PICKER_KEYBOARD_KEY_WIDTH * PICKER_KEYBOARD_COLUMNS
        + (PICKER_KEYBOARD_PAD + PICKER_KEYBOARD_BOTTOM) * 2.0)
        * scale;
    let scale = if picker[2] > 0.0 && natural > picker[2] {
        scale * picker[2] / natural
    } else {
        scale
    };
    let width =
        (PICKER_KEYBOARD_KEY_WIDTH * PICKER_KEYBOARD_COLUMNS + PICKER_KEYBOARD_PAD * 2.0) * scale;
    let height = (picker_keyboard_keys_height() + PICKER_KEYBOARD_PAD * 2.0) * scale;
    let x = picker[0] + (picker[2] - width) * 0.5;
    let y = picker[1] + picker[3] - PICKER_KEYBOARD_BOTTOM * scale - height;
    PickerKeyboardLayout {
        panel: [x, y, width, height],
        scale,
    }
}

fn picker_keyboard_row_band(row: usize) -> (f32, f32) {
    let top = (0..row)
        .map(|above| {
            picker_keyboard_row_scale(above) * PICKER_KEYBOARD_KEY_HEIGHT + PICKER_KEYBOARD_GAP
        })
        .sum();
    (
        top,
        picker_keyboard_row_scale(row) * PICKER_KEYBOARD_KEY_HEIGHT,
    )
}

fn picker_keyboard_keys_height() -> f32 {
    let (top, height) = picker_keyboard_row_band(PICKER_KEYBOARD_ROWS - 1);
    top + height
}

fn picker_column_step(scale: f32, focused_icon: f32) -> f32 {
    let margin = focused_icon * PICKER_DISC * 0.5 + 24.0 * scale;
    (1080.0 * 16.0 / 9.0 * PICKER_CROSS_X * scale - margin).max(64.0 * scale)
}

fn picker_row_y(
    level: usize,
    offset: f32,
    cross_y: f32,
    spacing: f32,
    gap_above: f32,
    gap_below: f32,
) -> f32 {
    if level != 0 {
        return cross_y + gap_below + offset * spacing;
    }
    if offset >= 0.0 {
        cross_y + gap_below + offset * spacing
    } else if offset <= -1.0 {
        cross_y - gap_above + (offset + 1.0) * spacing
    } else {
        cross_y + gap_below + offset * (gap_below + gap_above)
    }
}

fn picker_rows_in_view(
    position: f32,
    rows: usize,
    spacing: f32,
    height: f32,
) -> std::ops::Range<usize> {
    if spacing <= 0.0 || !spacing.is_finite() || !position.is_finite() {
        return 0..rows;
    }
    let reach = height / spacing + 3.0;
    let from = (position - reach).clamp(0.0, rows as f32) as usize;
    let to = ((position + reach).clamp(0.0, rows as f32) as usize + 1).min(rows);
    from..to.max(from)
}

fn picker_edge_alpha(x: f32, left: f32, right: f32, focused_icon: f32) -> f32 {
    let band = focused_icon.max(1.0);
    ((x - left + band) / band).clamp(0.0, 1.0) * ((right - x + band) / band).clamp(0.0, 1.0)
}

fn picker_vertical_alpha(y: f32, top: f32, bottom: f32, scale: f32) -> f32 {
    let range = 70.0 * scale;
    ((y - top - 24.0 * scale) / range).clamp(0.0, 1.0)
        * ((bottom - 24.0 * scale - y) / range).clamp(0.0, 1.0)
}

fn picker_row_entry(level: &PickerLevel, row: usize) -> Option<&lxb_toolkit::picker::Entry> {
    if level.head_row(row).is_some() {
        return None;
    }
    level
        .picker
        .entries()
        .get(row.checked_sub(level.head_count())?)
}

fn fitted(
    thumbnail: crate::renderer::Thumbnail,
    [x, y, width, height]: [f32; 4],
    fit: Fit,
) -> Option<([f32; 4], [f32; 4])> {
    if !(width > 0.0 && height > 0.0) {
        return None;
    }
    let picture = if thumbnail.aspect.is_finite() && thumbnail.aspect > 0.0 {
        thumbnail.aspect
    } else {
        1.0
    };
    let room = width / height;
    let [u0, v0, u1, v1] = thumbnail.cell;

    match fit {
        Fit::Contain => {
            let (shown_width, shown_height) = if picture >= room {
                (width, width / picture)
            } else {
                (height * picture, height)
            };
            Some((
                [
                    x + (width - shown_width) * 0.5,
                    y + (height - shown_height) * 0.5,
                    shown_width,
                    shown_height,
                ],
                thumbnail.cell,
            ))
        }
        Fit::Cover => {
            let (keep_x, keep_y) = if picture >= room {
                (room / picture, 1.0)
            } else {
                (1.0, picture / room)
            };
            let edge_x = (1.0 - keep_x) * 0.5;
            let edge_y = (1.0 - keep_y) * 0.5;
            let span_u = u1 - u0;
            let span_v = v1 - v0;
            Some((
                [x, y, width, height],
                [
                    u0 + span_u * edge_x,
                    v0 + span_v * edge_y,
                    u1 - span_u * edge_x,
                    v1 - span_v * edge_y,
                ],
            ))
        }
    }
}

fn picker_round_preview_cell(thumbnail: crate::renderer::Thumbnail) -> [f32; 4] {
    let aspect = if thumbnail.aspect.is_finite() && thumbnail.aspect > 0.0 {
        thumbnail.aspect
    } else {
        1.0
    };
    let (keep_x, keep_y) = if aspect >= 1.0 {
        (1.0 / aspect, 1.0)
    } else {
        (1.0, aspect)
    };
    let edge_x = (1.0 - keep_x) * 0.5;
    let edge_y = (1.0 - keep_y) * 0.5;
    let [u0, v0, u1, v1] = thumbnail.cell;
    let width = u1 - u0;
    let height = v1 - v0;
    [
        u0 + width * edge_x,
        v0 + height * edge_y,
        u1 - width * edge_x,
        v1 - height * edge_y,
    ]
}

fn picker_row_content(
    asked: PickerAsking<'_>,
    level: &PickerLevel,
    row: usize,
) -> (String, String, &'static str, bool) {
    if let Some(head) = level.head_row(row) {
        return match head {
            PickerHeadRow::NewFolder => (
                if level.making.is_empty() {
                    lxb_toolkit::i18n::text("new-folder").to_string()
                } else {
                    level.making.clone()
                },
                lxb_toolkit::i18n::text("make-a-folder-here").to_string(),
                "new-folder",
                false,
            ),
            PickerHeadRow::Name => (
                if asked.name.trim().is_empty() {
                    lxb_toolkit::i18n::text("name").to_string()
                } else {
                    asked.name.to_string()
                },
                lxb_toolkit::i18n::text("what-the-file-will-be-called").to_string(),
                "rename",
                false,
            ),
            PickerHeadRow::Answer => (
                asked
                    .purpose
                    .accept()
                    .unwrap_or_else(|| lxb_toolkit::i18n::text("choose-this"))
                    .to_string(),
                picker_answer_note(asked, level),
                if asked.purpose.takes_several() {
                    "select-multiple"
                } else {
                    "chosen"
                },
                false,
            ),
            PickerHeadRow::SearchField => {
                let query = level.picker.query();
                let detail = if query.is_empty() {
                    lxb_toolkit::i18n::text("search-this-folder-by-name").to_string()
                } else {
                    lxb_toolkit::message!("matching-items", "count" => level.picker.entries().len())
                };
                (
                    if query.is_empty() {
                        lxb_toolkit::i18n::text("search").to_string()
                    } else {
                        query.to_string()
                    },
                    detail,
                    "search",
                    false,
                )
            }
            PickerHeadRow::SearchClear => (
                lxb_toolkit::i18n::text("clear-search").to_string(),
                lxb_toolkit::i18n::text("show-every-item-in-this-folder").to_string(),
                "search-clear",
                false,
            ),
        };
    }

    let entry = &level.picker.entries()[row - level.head_count()];
    let (detail, glyph, quiet) = picker_entry_facts(entry);
    (entry.name.clone(), detail, glyph, quiet)
}

fn picker_answer_note(asked: PickerAsking<'_>, level: &PickerLevel) -> String {
    match asked.purpose {
        PickerPurpose::ManyFiles => match asked.ticked.len() {
            0 => lxb_toolkit::i18n::text("nothing-chosen-yet").to_string(),
            chosen => lxb_toolkit::message!("files-chosen", "count" => chosen),
        },
        PickerPurpose::ANewFile => match asked.name.trim() {
            "" => lxb_toolkit::i18n::text("give-the-file-a-name-first").to_string(),
            name => lxb_toolkit::message!("save-as-name", "name" => name.to_string()),
        },
        _ => level
            .picker
            .location()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| lxb_toolkit::i18n::text("use-this-folder").to_string()),
    }
}

fn picker_entry_facts(entry: &lxb_toolkit::picker::Entry) -> (String, &'static str, bool) {
    match entry.kind {
        EntryKind::Folder => (
            picker_modified_note(&entry.path)
                .unwrap_or_else(|| lxb_toolkit::i18n::text("folder").to_string()),
            "file-folder",
            false,
        ),
        EntryKind::File => {
            let detail = std::fs::metadata(&entry.path)
                .ok()
                .map(|facts| {
                    let modified =
                        facts
                            .modified()
                            .ok()
                            .map(picker_time_note)
                            .unwrap_or_else(|| {
                                lxb_toolkit::i18n::text("modified-date-unknown").to_string()
                            });
                    format!("{} · {modified}", picker_size_note(facts.len()))
                })
                .unwrap_or_else(|| lxb_toolkit::i18n::text("file").to_string());
            (detail, picker_file_glyph(&entry.path), true)
        }
    }
}

fn picker_modified_note(path: &std::path::Path) -> Option<String> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(picker_time_note(modified))
}

fn picker_time_note(modified: std::time::SystemTime) -> String {
    let Ok(seconds) = modified.duration_since(std::time::SystemTime::UNIX_EPOCH) else {
        return lxb_toolkit::i18n::text("modified-date-unknown").to_string();
    };
    let (year, month, day) = picker_civil_date((seconds.as_secs() / 86_400) as i64);
    // The whole date to the catalog at once, month included: the order of the
    // three is the language's, and so is the form of the month — Polish writes
    // *1 stycznia*, which is not the name of the month on its own.
    lxb_toolkit::message!(
        "file-date",
        "day" => day.to_string(),
        "month" => lxb_toolkit::i18n::month(month as usize),
        "year" => year.to_string()
    )
}

fn picker_civil_date(days: i64) -> (i32, u8, u8) {
    let days = days + 719_468;
    let era = (if days >= 0 { days } else { days - 146_096 }) / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_position + 2) / 5 + 1;
    let month = month_position + if month_position < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year as i32, month as u8, day as u8)
}

fn picker_size_note(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1_024.0 && unit + 1 < UNITS.len() {
        value /= 1_024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if value >= 10.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        lxb_toolkit::i18n::decimal(format!("{value:.1} {}", UNITS[unit]))
    }
}

fn picker_file_glyph(path: &std::path::Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some(
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "jxl" | "heic" | "heif" | "bmp"
            | "tif" | "tiff" | "svg" | "psd" | "dng" | "cr2" | "cr3" | "nef" | "arw" | "orf"
            | "raf" | "rw2",
        ) => "category-images",
        Some(
            "mp4" | "m4v" | "mkv" | "webm" | "avi" | "divx" | "mov" | "wmv" | "flv" | "mpg"
            | "mpeg" | "vob" | "ogv" | "3gp" | "m2ts" | "rmvb",
        ) => "category-video",
        Some("mp3" | "ogg" | "oga" | "opus" | "flac" | "wav" | "m4a" | "aac" | "wma") => {
            "category-music"
        }
        _ => "file-page",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bar has to say where in a list it is, and stay worth catching however
    /// long the list behind it gets.
    #[test]
    fn a_bar_stands_where_the_list_does_and_never_shrinks_out_of_reach() {
        let track = [0.0, 100.0, 8.0, 400.0];

        // At the top of a list with a third of it showing.
        let thumb = scroll_thumb(track, 0.0, 1.0 / 3.0);
        assert_eq!(thumb[1], track[1]);
        assert!((thumb[3] - track[3] / 3.0).abs() < 0.5, "{thumb:?}");

        // At the bottom of it, the thumb ends where the track ends.
        let thumb = scroll_thumb(track, 1.0, 1.0 / 3.0);
        assert!(
            (thumb[1] + thumb[3] - (track[1] + track[3])).abs() < 0.5,
            "{thumb:?}"
        );

        // A thousand rows with eight showing: still something to catch.
        let thumb = scroll_thumb(track, 0.5, 0.008);
        assert!(thumb[3] >= track[2] * LEAST_THUMB, "{thumb:?}");

        // A list that fits fills its own track rather than overflowing it,
        // and one asked for past its end stays on it.
        assert_eq!(scroll_thumb(track, 0.0, 1.4)[3], track[3]);
        let thumb = scroll_thumb(track, 1.6, 0.25);
        assert!(
            thumb[1] + thumb[3] <= track[1] + track[3] + 0.5,
            "{thumb:?}"
        );
    }

    struct PickerDirectory {
        path: std::path::PathBuf,
    }

    impl PickerDirectory {
        fn new(name: &str) -> Self {
            use std::sync::atomic::{AtomicUsize, Ordering};

            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "lxb-render-picker-{name}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            std::fs::create_dir_all(&path).expect("a picker test directory");
            Self { path }
        }
    }

    impl Drop for PickerDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    const CONTROL_LAYER: usize = crate::renderer::CONTROL;

    #[test]
    fn a_soft_edge_sits_between_the_complete_page_and_real_overlays() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: the soft edge layer was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        let list = [120.0, 80.0, 640.0, 440.0];
        ui.soft_edges(list, 90.0, 1.0, 0.6);
        ui.soft_edges(list, 90.0, 0.0, 0.0);

        let edges = &ui.scene.layers[SOFTEN].quads;
        assert_eq!(edges.len(), 1, "an end with nothing past it drew a pass");
        assert_eq!(edges[0].rect, list);
        assert_eq!(edges[0].shape[1], crate::renderer::KIND_SOFT_EDGE);
        assert_eq!(edges[0].material[0], 90.0);
        assert_eq!(edges[0].cell[0], 1.0);
        assert_eq!(edges[0].cell[1], 0.6);
        assert!(
            edges[0].tint[3] > 0.0,
            "a soft edge is not tinted, but a quad with no alpha is never \
             pushed at all"
        );
        assert!(
            ui.scene.layers[OVER].quads.is_empty(),
            "a page effect was put in the menu and dialog layer"
        );
    }

    #[test]
    fn only_the_control_the_light_is_on_shows_the_press() {
        let mut pressing = Pressing::default();
        pressing.press();

        assert!(
            matches!(pressing.state(true), Press::Going(_)),
            "the control being pressed has to show it"
        );
        assert_eq!(
            pressing.state(false),
            Press::Resting,
            "a control the light is not on is resting, however many presses are in flight elsewhere"
        );
    }

    fn menu() -> ContextMenu {
        let mut menu = ContextMenu::default();
        assert!(menu.open_at(
            [0.0, 0.0, 100.0, 40.0],
            None,
            vec![
                Entry::new("Information").glyph("setting-info"),
                Entry::new("Copy").disabled(),
                Entry::new("Close").aside("arrow-left"),
            ],
        ));
        menu
    }

    #[test]
    fn pointing_at_a_menu_row_selects_it_and_a_dead_row_does_not() {
        let mut menu = menu();
        assert_eq!(menu.selected(), 0);

        let row = |row, aside| Spot::MenuRow { row, aside };
        assert!(menu.point_at(row(2, false)));
        assert_eq!(menu.selected(), 2);

        assert!(!menu.point_at(row(1, false)));
        assert_eq!(menu.selected(), 2);

        assert!(!menu.point_at(row(9, false)));
        assert!(!menu.point_at(Spot::OutsidePanel));
        assert!(!menu.point_at(Spot::Control(3)));
        assert_eq!(menu.selected(), 2);
    }

    #[test]
    fn a_button_that_is_not_there_cannot_be_pointed_at() {
        let mut menu = menu();
        assert!(menu.point_at(Spot::MenuRow {
            row: 2,
            aside: true
        }));
        assert!(menu.on_aside());

        assert!(menu.point_at(Spot::MenuRow {
            row: 0,
            aside: true
        }));
        assert_eq!(menu.selected(), 0);
        assert!(!menu.on_aside(), "row 0 has no button to be on");
    }

    #[test]
    fn a_row_pointed_at_opens_from_nothing_like_one_walked_to() {
        let mut menu = menu();
        menu.unfolded = 0.7;
        assert!(menu.point_at(Spot::MenuRow {
            row: 2,
            aside: false
        }));
        assert_eq!(menu.unfolded, 0.0);

        menu.unfolded = 0.7;
        assert!(menu.point_at(Spot::MenuRow {
            row: 2,
            aside: false
        }));
        assert_eq!(menu.unfolded, 0.7);
    }

    #[test]
    fn a_dialog_frosts_the_page_where_its_own_pane_can_see_it() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: the dialog's frost was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        let mut dialog = Dialog::default();
        dialog.ask(
            "Remove it?",
            "It will be removed.",
            vec!["Keep it".to_string(), "Remove".to_string()],
            0,
        );
        for _ in 0..60 {
            dialog.advance(1.0 / 60.0);
        }
        ui.dialog(&mut dialog);

        let frost = ui.scene.layers[SOFTEN]
            .quads
            .iter()
            .find(|quad| quad.shape[1] == crate::renderer::KIND_FROST)
            .copied()
            .expect("a dialog laid no frost under itself");
        assert_eq!(
            frost.rect,
            [0.0, 0.0, width as f32, height as f32],
            "the frost is the page, not the panel: a dialog dims all of it"
        );
        assert!(
            frost.material[0] > 1.0,
            "a frost that reaches one level of the pyramid is not a frost"
        );
        assert!(
            frost.material[0] <= lxb_toolkit::material::optics::BLUR_LEVELS,
            "past the last level there is nothing further to sample"
        );
        assert!(
            frost.shape[3] > 0.9,
            "the frost was dimmed along with the page it is there to dim: it \
             must be laid after `recede_behind`, not before it"
        );

        // The one that matters. A pane of glass samples the source texture,
        // not the picture being painted, so a frost beside the dialog on its
        // own layer is a frost the dialog's pane reads straight past — the
        // whole window calm except the rectangle it was for.
        assert!(
            ui.scene.layers[OVER]
                .quads
                .iter()
                .all(|quad| quad.shape[1] != crate::renderer::KIND_FROST),
            "the frost is on the overlay layer, where the dialog's own pane \
             cannot see it"
        );
        assert!(
            ui.scene.layers[OVER]
                .quads
                .iter()
                .any(|quad| quad.shape[1] == crate::renderer::KIND_GLASS),
            "the dialog drew no pane at all"
        );
    }

    #[test]
    fn a_dialog_writes_in_white_ink_only() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: the dialog's ink was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        let mut dialog = Dialog::default();
        dialog.ask(
            "Remove it?",
            "It will be removed. Its settings and files will stay.",
            vec!["Keep it".to_string(), "Remove".to_string()],
            0,
        );
        for _ in 0..60 {
            dialog.advance(1.0 / 60.0);
        }
        ui.dialog(&mut dialog);

        let ink = ui.tinted(Role::Text, 1.0);
        let runs = &ui.scene.layers[OVER].runs;
        assert!(runs.len() >= 4, "a dialog wrote almost nothing: {runs:?}");
        for run in runs {
            // Most palettes cut `Role::TextSoft` as a pale cast of the accent,
            // which is legible only over a ground the language chose itself. A
            // dialog does not know what is behind it — a page may be a
            // photograph — so it quiets its ink with alpha and nothing else.
            assert!(
                run.tint[..3]
                    .iter()
                    .zip(&ink[..3])
                    .all(|(said, want)| (said - want).abs() < 1e-3),
                "a dialog wrote in something other than its ink: {:?} is not \
                 {ink:?}",
                run.tint
            );
            assert!(run.tint[3] > 0.0, "a run with no alpha was pushed at all");
        }
    }

    #[test]
    fn a_dialog_that_has_not_opened_yet_frosts_nothing() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: the dialog's frost was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        let mut dialog = Dialog::default();
        dialog.ask(
            "Remove it?",
            "It will be removed.",
            vec!["Keep it".to_string(), "Remove".to_string()],
            0,
        );
        ui.dialog(&mut dialog);

        assert!(
            [SOFTEN, OVER].iter().all(|&layer| ui.scene.layers[layer]
                .quads
                .iter()
                .all(|quad| quad.shape[1] != crate::renderer::KIND_FROST)),
            "the page snapped out of focus before the dialog had moved"
        );
    }

    #[test]
    fn pointing_at_a_dialog_answer_selects_it() {
        let mut dialog = Dialog::default();
        dialog.ask(
            "Leave?",
            "Nothing is closed yet.",
            vec!["Stay".to_string(), "Leave".to_string()],
            0,
        );
        assert!(dialog.point_at(Spot::DialogButton(1)));
        assert_eq!(dialog.selected(), 1);
        assert!(!dialog.point_at(Spot::DialogButton(9)));
        assert!(!dialog.point_at(Spot::Nothing));
        assert_eq!(dialog.selected(), 1);
    }

    #[test]
    fn no_component_pins_what_it_draws_to_the_page() {
        let source = include_str!("components.rs");
        let drawing = source.split("#[cfg(test)]").next().unwrap_or_default();
        let offenders: Vec<&str> = drawing
            .lines()
            .filter(|line| {
                let line = line.trim();
                !line.starts_with("//") && line.contains("CONTROL")
            })
            .collect();
        assert!(
            offenders.is_empty(),
            "these name the page's layer instead of asking for one:\n{}",
            offenders.join("\n")
        );
    }

    #[test]
    fn what_is_drawn_between_two_marks_is_cut_and_nothing_else_is() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: cutting a range of the scene was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        // A heading, then a list of two rows, then a legend under it. Only the
        // list is cut.
        ui.label(
            [40.0, 10.0, 300.0, 30.0],
            Text::Title,
            "Heading",
            Role::Text,
            Align::Left,
        );
        let from = ui.written();
        for row in 0..2 {
            ui.card(
                [40.0, 60.0 + 80.0 * row as f32, 300.0, 70.0],
                Surface::Control,
                Role::Glass,
                0.4,
            );
        }
        let to = ui.written();
        ui.label(
            [40.0, 260.0, 300.0, 30.0],
            Text::Caption,
            "Legend",
            Role::TextSoft,
            Align::Left,
        );
        let list = [40.0, 60.0, 300.0, 120.0];
        ui.cut_between(from, to, list);

        let quads = &ui.scene.layers[CONTROL_LAYER].quads;
        let cards: Vec<_> = quads
            .iter()
            .filter(|quad| quad.shape[1] == crate::renderer::KIND_GLASS)
            .collect();
        assert_eq!(cards.len(), 2, "the two rows of the list");
        for card in cards {
            assert_eq!(
                card.cut,
                [list[0], list[1], list[0] + list[2], list[1] + list[3]],
                "a row of the list was not cut to it"
            );
        }

        let runs = &ui.scene.layers[CONTROL_LAYER].runs;
        assert_eq!(runs.len(), 2, "the heading and the legend");
        for run in runs {
            assert!(
                run.clip.is_none(),
                "a word outside the two marks was cut with the list"
            );
        }
    }

    #[test]
    fn a_cut_between_marks_that_are_not_a_range_cuts_nothing() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: a backwards cut was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        let from = ui.written();
        ui.card(
            [40.0, 60.0, 300.0, 70.0],
            Surface::Control,
            Role::Glass,
            0.4,
        );
        let to = ui.written();
        // The wrong way round, which is a mistake in the page and not a
        // licence to cut the whole scene.
        ui.cut_between(to, from, [0.0, 0.0, 10.0, 10.0]);
        let quads = &ui.scene.layers[CONTROL_LAYER].quads;
        let card = quads
            .iter()
            .find(|quad| quad.shape[1] == crate::renderer::KIND_GLASS)
            .expect("the card");
        assert_eq!(card.cut, crate::renderer::NO_CUT, "a backwards cut cut");
    }

    #[test]
    fn a_fade_between_marks_takes_the_glass_down_with_the_words() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: a fade was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        // A heading that stays, and a card and a word that go out together.
        ui.label(
            [40.0, 10.0, 300.0, 30.0],
            Text::Title,
            "Heading",
            Role::Text,
            Align::Left,
        );
        let from = ui.written();
        ui.card(
            [40.0, 60.0, 300.0, 70.0],
            Surface::Control,
            Role::Glass,
            0.4,
        );
        ui.label(
            [40.0, 60.0, 300.0, 30.0],
            Text::Body,
            "Leaving",
            Role::Text,
            Align::Left,
        );
        let to = ui.written();
        ui.fade_between(from, to, 0.25);

        let card = ui.scene.layers[CONTROL_LAYER]
            .quads
            .iter()
            .find(|quad| quad.shape[1] == crate::renderer::KIND_GLASS)
            .expect("the card");
        assert!(
            (card.shape[3] - 0.25).abs() < 1e-6,
            "a pane of glass did not fade with the page it was drawn on: a \
             glass quad's material is not scaled by its own tint, so this is \
             the only channel that takes it out"
        );

        let runs = &ui.scene.layers[CONTROL_LAYER].runs;
        assert_eq!(runs.len(), 2, "the heading and the word that is leaving");
        assert!(
            (runs[1].tint[3] - 0.25).abs() < 1e-6,
            "the word between the marks did not fade"
        );
        assert!(
            runs[0].tint[3] > 0.9,
            "a word outside the two marks was faded with them"
        );
    }

    #[test]
    fn a_fade_between_marks_that_are_not_a_range_fades_nothing() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: a backwards fade was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        let from = ui.written();
        ui.card(
            [40.0, 60.0, 300.0, 70.0],
            Surface::Control,
            Role::Glass,
            0.4,
        );
        let to = ui.written();
        // The wrong way round, which is a mistake in the page and not a
        // licence to fade the whole scene.
        ui.fade_between(to, from, 0.0);
        let card = ui.scene.layers[CONTROL_LAYER]
            .quads
            .iter()
            .find(|quad| quad.shape[1] == crate::renderer::KIND_GLASS)
            .expect("the card");
        assert_eq!(card.shape[3], 1.0, "a backwards fade faded");
    }

    #[test]
    fn a_light_at_no_strength_leaves_no_glass_behind_it() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: a fading light was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        // A page crossing away asks for its light at almost nothing. What it
        // must not get is a ring refracting as hard as at full strength.
        ui.selection([40.0, 60.0, 300.0, 70.0], 0.02);
        let lit = ui.scene.layers[CONTROL_LAYER]
            .quads
            .iter()
            .find(|quad| quad.shape[1] == crate::renderer::KIND_GLASS)
            .expect("the light");
        assert!(
            (lit.shape[3] - 0.02).abs() < 1e-6,
            "the light's glass was laid at full strength: its fade is in the \
             tint, which a glass quad's material does not read"
        );
    }

    #[test]
    fn a_mark_inside_a_panel_is_drawn_in_the_panels_layer() {
        let (width, height) = (900u32, 600u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: the layer a mark lands in was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        ui.begin(
            width as f32,
            height as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );

        ui.icon([40.0, 40.0, 32.0, 32.0], "launch", IconStyle::Default);
        let marks_in = |ui: &Ui, layer: usize| {
            ui.scene.layers[layer]
                .quads
                .iter()
                .filter(|quad| quad.shape[1] == crate::renderer::KIND_GLYPH)
                .count()
        };
        assert_eq!(marks_in(&ui, CONTROL_LAYER), 1);
        assert_eq!(marks_in(&ui, OVER), 0);

        let mut menu = ContextMenu::default();
        assert!(menu.open_at(
            [40.0, 200.0, 200.0, 48.0],
            None,
            vec![Entry::new("Information").glyph("setting-info")],
        ));
        for _ in 0..90 {
            menu.advance(1.0 / 60.0);
        }
        ui.context_menu(&mut menu);

        assert_eq!(marks_in(&ui, CONTROL_LAYER), 1, "the page's own mark moved");
        assert_eq!(
            marks_in(&ui, OVER),
            1,
            "the menu's mark was left on the page, under the panel's glass"
        );
    }

    #[test]
    fn nothing_is_hidden_before_the_panel_hiding_it_can_be_seen() {
        let (width, height) = (1920u32, 1080u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: what a folding panel does to the page was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();

        let writing = |ui: &mut Ui, menu: Option<&mut ContextMenu>| {
            ui.begin(
                width as f32,
                height as f32,
                10.0,
                &accent,
                lxb_toolkit::settings::WallpaperStyle::Default,
                false,
                IconStyle::Default,
            );
            let rows = 16;
            for row in 0..rows {
                ui.label(
                    [
                        0.0,
                        height as f32 * row as f32 / rows as f32,
                        width as f32,
                        24.0,
                    ],
                    Text::Body,
                    "a line of the page underneath",
                    Role::Text,
                    Align::Left,
                );
            }
            let out = match menu {
                Some(menu) => {
                    ui.context_menu(menu);
                    menu.travelled()
                }
                None => 0.0,
            };
            let dim = 1.0 + (menu::CONTEXT.dim - 1.0) * out;
            let stepped = 1.0 - menu::DEPTH * out;
            let ink: f32 = [PANE, CONTROL_LAYER]
                .into_iter()
                .flat_map(|layer| ui.scene.layers[layer].runs.iter())
                .map(|run| {
                    let width = match run.clip {
                        Some([_, _, w, _]) => w.min(run.width),
                        None => run.width,
                    };
                    run.tint[3] * width.max(0.0)
                })
                .sum();
            ink / (dim * stepped)
        };

        let bare = writing(&mut ui, None);
        assert!(bare > 0.0, "the page has no writing on it");

        let mut menu = ContextMenu::default();
        assert!(menu.open_at(
            [40.0, 200.0, 200.0, 48.0],
            Some("A window".to_string()),
            vec![
                Entry::new("Information"),
                Entry::new("Copy this page's values"),
                Entry::new("Close"),
            ],
        ));

        for _ in 0..90 {
            menu.advance(1.0 / 60.0);
        }
        let settled = writing(&mut ui, Some(&mut menu));
        assert!(
            settled < bare * 0.95,
            "the settled panel took nothing away, so this measures nothing"
        );

        menu.close();
        let mut folded = ContextMenu::default();
        assert!(folded.open_at(
            [40.0, 200.0, 200.0, 48.0],
            Some("A window".to_string()),
            vec![
                Entry::new("Information"),
                Entry::new("Copy this page's values"),
                Entry::new("Close"),
            ],
        ));
        folded.advance(motion::duration::MENU_FLIGHT / 100.0);
        assert!(
            motion::ease(folded.travelled() / menu::CONTEXT.panel_in) < 0.01,
            "this is meant to read a panel that cannot be seen yet"
        );
        let early = writing(&mut ui, Some(&mut folded));
        assert!(
            early > bare * 0.999,
            "the panel took {:.1}% of the page's writing away before there \
             was anything of it to see",
            (bare - early) / bare * 100.0
        );
    }

    #[test]
    fn a_dismissed_panel_gives_the_page_its_words_back_as_it_fades() {
        let (width, height) = (1920u32, 1080u32);
        let Ok(mut ui) = Ui::headless(width, height) else {
            eprintln!("no adapter: what a dismissed panel gives back was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        let raise = || {
            let mut menu = ContextMenu::default();
            assert!(menu.open_at(
                [40.0, 200.0, 200.0, 48.0],
                Some("A window".to_string()),
                vec![
                    Entry::new("Information"),
                    Entry::new("Copy"),
                    Entry::new("Close"),
                ],
            ));
            menu
        };

        let writing = |ui: &mut Ui, menu: Option<&mut ContextMenu>| {
            ui.begin(
                width as f32,
                height as f32,
                10.0,
                &accent,
                lxb_toolkit::settings::WallpaperStyle::Default,
                false,
                IconStyle::Default,
            );
            let rows = 16;
            for row in 0..rows {
                ui.label(
                    [
                        0.0,
                        height as f32 * row as f32 / rows as f32,
                        width as f32,
                        24.0,
                    ],
                    Text::Body,
                    "a line of the page underneath",
                    Role::Text,
                    Align::Left,
                );
            }
            let out = match menu {
                Some(menu) => {
                    ui.context_menu(menu);
                    menu.travelled()
                }
                None => 0.0,
            };
            let dim = 1.0 + (menu::CONTEXT.dim - 1.0) * out;
            let stepped = 1.0 - menu::DEPTH * out;
            let ink: f32 = [PANE, CONTROL_LAYER]
                .into_iter()
                .flat_map(|layer| ui.scene.layers[layer].runs.iter())
                .map(|run| {
                    let width = match run.clip {
                        Some([_, _, w, _]) => w.min(run.width),
                        None => run.width,
                    };
                    run.tint[3] * width.max(0.0)
                })
                .sum();
            ink / (dim * stepped)
        };

        let half = motion::duration::MENU_FLIGHT * 0.5;
        let mut arriving = raise();
        arriving.advance(half);
        let mut leaving = raise();
        for _ in 0..90 {
            leaving.advance(1.0 / 60.0);
        }
        leaving.close();
        leaving.advance(half);
        assert!((arriving.travelled() - 0.5).abs() < 1e-4);
        assert!((leaving.travelled() - 0.5).abs() < 1e-4);

        let bare = writing(&mut ui, None);
        let out = writing(&mut ui, Some(&mut arriving));
        let back = writing(&mut ui, Some(&mut leaving));
        assert!(
            out < bare * 0.98,
            "the arriving panel covered nothing, so this measures nothing"
        );

        let halfway = (out + bare) / 2.0;
        assert!(
            (back - halfway).abs() < bare * 0.002,
            "a panel half gone gave back {:.0}% of what it took, not half",
            (back - out) / (bare - out) * 100.0
        );
    }

    #[test]
    fn a_menu_of_dead_rows_never_opens() {
        let mut menu = ContextMenu::default();
        assert!(!menu.open_at(
            [0.0, 0.0, 10.0, 10.0],
            None,
            vec![Entry::new("Copy").disabled()],
        ));
        assert!(!menu.is_open());
        assert!(menu.is_empty());
    }

    #[test]
    fn many_files_are_ticked_one_by_one_and_the_head_row_answers_with_all_of_them() {
        let directory = PickerDirectory::new("many");
        for name in ["one.txt", "two.txt"] {
            std::fs::write(directory.path.join(name), b"x").expect("a visible file");
        }

        let mut picker = FilePicker::default();
        assert!(picker.open_for(
            PickerPurpose::ManyFiles,
            PickerSelection::File,
            &directory.path,
            "",
        ));
        assert!(
            picker.chose().is_empty(),
            "a file row is not an answer when several are wanted"
        );
        assert!(picker.activate(), "the file the cursor is on is ticked");
        assert_eq!(picker.ticked().len(), 1);
        assert!(picker.activate(), "a second press takes the tick back");
        assert!(picker.ticked().is_empty());
        assert!(picker.activate());
        assert!(picker.move_selection(1));
        assert!(picker.activate());
        assert_eq!(picker.ticked().len(), 2);

        assert!(picker.move_selection(-3), "Up passes Search to the answer");
        let chose = picker.chose();
        assert_eq!(
            chose,
            vec![
                directory.path.join("one.txt"),
                directory.path.join("two.txt")
            ]
        );
        assert_eq!(picker.choose().as_deref(), Some(chose[0].as_path()));
    }

    #[test]
    fn a_save_is_given_a_name_and_answers_with_a_file_that_is_not_there_yet() {
        let directory = PickerDirectory::new("save");
        std::fs::write(directory.path.join("already.txt"), b"x").expect("a visible file");

        let mut picker = FilePicker::default();
        assert!(picker.open_for(
            PickerPurpose::ANewFile,
            PickerSelection::File,
            &directory.path,
            "",
        ));
        assert_eq!(picker.named(), "", "an unnamed save opens on the name");
        assert!(picker.chose().is_empty(), "a save with no name has none");
        assert!(picker.type_text("notes.txt"));
        assert_eq!(picker.named(), "notes.txt");
        assert!(
            picker.erase_search(),
            "the name is erased a letter at a time"
        );
        assert_eq!(picker.named(), "notes.tx");
        assert!(picker.type_text("t"));

        assert!(picker.move_selection(1), "Down reaches Save here");
        assert_eq!(
            picker.chose(),
            vec![directory.path.join("notes.txt")],
            "a save answers with a path inside the folder being stood in"
        );
        assert!(!directory.path.join("notes.txt").exists());
    }

    #[test]
    fn a_save_that_was_named_by_the_application_opens_on_the_row_that_answers() {
        let directory = PickerDirectory::new("save-named");

        let mut picker = FilePicker::default();
        assert!(picker.open_for(
            PickerPurpose::ANewFile,
            PickerSelection::File,
            &directory.path,
            "report.pdf",
        ));
        assert_eq!(picker.named(), "report.pdf");
        assert_eq!(picker.chose(), vec![directory.path.join("report.pdf")]);
    }

    #[test]
    fn a_folder_is_made_from_the_head_row_and_the_column_steps_onto_it() {
        let directory = PickerDirectory::new("new-folder");

        let mut picker = FilePicker::default();
        assert!(picker.open_for(
            PickerPurpose::AFolder,
            PickerSelection::Folder,
            &directory.path,
            "",
        ));
        assert!(
            picker.chose().is_empty(),
            "an empty folder rests on New folder, not on the row that answers"
        );
        assert!(picker.activate(), "New folder types");
        assert!(picker.search_keyboard_open());
        assert!(picker.type_text("Holiday"));
        assert!(picker.submit_search_keyboard());
        assert!(directory.path.join("Holiday").is_dir());
        assert!(!picker.search_keyboard_open());

        assert!(
            picker.chose().is_empty(),
            "the folder that was made is stood on, not chosen"
        );
        assert!(
            picker.enter(),
            "the folder that was made can be walked into"
        );
        assert_eq!(
            picker.location(),
            Some(directory.path.join("Holiday").as_path())
        );
        assert!(
            picker.chose().is_empty(),
            "an empty folder does not open on the row that answers"
        );
        assert!(picker.move_selection(1), "Down reaches Use this folder");
        assert_eq!(
            picker.chose(),
            vec![directory.path.join("Holiday")],
            "the answer is the folder being stood in"
        );
    }

    #[test]
    fn one_file_keeps_the_head_rows_a_question_from_outside_would_add() {
        let directory = PickerDirectory::new("one-file-head");
        std::fs::write(directory.path.join("only.txt"), b"x").expect("a visible file");

        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        let level = picker.active_level().expect("an open picker has a level");
        assert_eq!(
            level.head,
            vec![PickerHeadRow::SearchField],
            "one file is answered by the listing and needs no answer row"
        );
        assert_eq!(
            picker.chose(),
            vec![directory.path.join("only.txt")],
            "the file the cursor rests on is the answer"
        );
    }

    #[test]
    fn the_foot_says_where_the_walk_is_and_what_it_is_narrowed_to() {
        let directory = PickerDirectory::new("foot");
        std::fs::create_dir(directory.path.join("inside")).expect("a folder");
        std::fs::write(directory.path.join("cover.png"), b"x").expect("an image");

        let mut picker = FilePicker::default();
        assert!(picker.open_for(
            PickerPurpose::OneFile,
            PickerSelection::Image,
            &directory.path,
            "",
        ));
        assert_eq!(
            picker.where_it_is(),
            directory.path.to_string_lossy(),
            "the whole path, not the folder's name"
        );
        assert_eq!(picker.narrowed_to(), "Showing  Images");

        assert!(picker.enter(), "into the folder");
        assert_eq!(
            picker.where_it_is(),
            directory.path.join("inside").to_string_lossy(),
            "the foot follows the walk"
        );

        let mut save = FilePicker::default();
        assert!(save.open_for(
            PickerPurpose::ANewFile,
            PickerSelection::File,
            &directory.path,
            "notes.txt",
        ));
        assert_eq!(
            save.narrowed_to(),
            "Saving as  notes.txt",
            "a save says what it will be called, from wherever in the walk"
        );

        let mut unnamed = FilePicker::default();
        assert!(unnamed.open_for(
            PickerPurpose::ANewFile,
            PickerSelection::File,
            &directory.path,
            "",
        ));
        assert_eq!(
            unnamed.narrowed_to(),
            "Saving as  Untitled|",
            "an unnamed save opens on the name, and the caret says so"
        );
    }

    #[test]
    fn the_panels_own_menu_offers_the_answers_that_are_not_on_the_column() {
        let directory = PickerDirectory::new("menu");
        std::fs::write(directory.path.join("cover.png"), b"x").expect("an image");
        std::fs::write(directory.path.join("notes.txt"), b"x").expect("a file");
        std::fs::write(directory.path.join(".secret"), b"x").expect("a hidden name");

        let mut picker = FilePicker::default();
        assert!(picker.open_for(
            PickerPurpose::OneFile,
            PickerSelection::Image,
            &directory.path,
            "",
        ));
        assert!(!picker.menu_is_open());
        assert!(picker.open_menu());
        assert!(picker.menu_is_open());
        assert!(!picker.open_menu(), "a second press cannot cover the first");

        let labels: Vec<String> = picker
            .menu_rows()
            .iter()
            .map(|row| row.label.clone())
            .collect();
        assert_eq!(labels, ["Types", "Sort", "Show hidden files"]);
        assert_eq!(
            picker.menu_rows()[0].detail.as_deref(),
            Some("Images"),
            "the row says what is in force without being stepped into"
        );

        assert!(picker.close_menu());
        assert!(!picker.menu_is_open());
        assert!(!picker.close_menu());
    }

    #[test]
    fn a_kind_the_application_asked_for_can_be_got_past_from_the_menu() {
        let directory = PickerDirectory::new("menu-kinds");
        std::fs::write(directory.path.join("cover.png"), b"x").expect("an image");
        std::fs::write(directory.path.join("notes.txt"), b"x").expect("a file");

        let mut picker = FilePicker::default();
        assert!(picker.open_for(
            PickerPurpose::OneFile,
            PickerSelection::Image,
            &directory.path,
            "",
        ));
        let listed = |picker: &FilePicker| {
            picker
                .active_level()
                .expect("a level")
                .picker
                .entries()
                .len()
        };
        assert_eq!(listed(&picker), 1, "only the image");

        assert!(picker.open_menu());
        assert!(picker.menu_press(), "Types steps into the kinds");
        let rows = picker.menu_rows();
        assert_eq!(
            rows.iter().map(|row| row.label.clone()).collect::<Vec<_>>(),
            ["Images", "Everything"]
        );
        assert_eq!(rows[0].glyph, Some("chosen"), "the kind in force is ticked");
        assert_eq!(rows[1].glyph, None);

        assert!(picker.menu_step(1), "down to Everything");
        assert!(picker.menu_press());
        assert_eq!(listed(&picker), 2, "the rest of the disk");
        let rows = picker.menu_rows();
        assert_eq!(rows[0].glyph, None, "the tick moved under the hand");
        assert_eq!(rows[1].glyph, Some("chosen"));
        assert!(picker.menu_is_open(), "the row holds the menu open");
    }

    #[test]
    fn the_order_and_the_hidden_names_are_answered_from_the_menu() {
        let directory = PickerDirectory::new("menu-sort");
        std::fs::write(directory.path.join("alpha.txt"), b"x").expect("a file");
        std::fs::write(directory.path.join("beta.txt"), b"x").expect("a file");
        std::fs::write(directory.path.join(".secret"), b"x").expect("a hidden name");

        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        let names = |picker: &FilePicker| {
            picker
                .active_level()
                .expect("a level")
                .picker
                .entries()
                .iter()
                .map(|entry| entry.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&picker), ["alpha.txt", "beta.txt"]);

        assert!(picker.open_menu());
        assert_eq!(picker.menu_rows()[0].label, "Sort");
        assert!(picker.menu_press(), "Sort steps into the orders");
        assert_eq!(picker.menu_rows().len(), PickerSort::ALL.len());
        assert_eq!(picker.menu_rows()[0].glyph, Some("chosen"));

        assert!(picker.menu_step(1), "Name (Z to A)");
        assert!(picker.menu_press());
        assert_eq!(names(&picker), ["beta.txt", "alpha.txt"]);
        assert_eq!(picker.menu_rows()[1].glyph, Some("chosen"));

        assert!(picker.close_menu());
        assert!(picker.open_menu());
        assert!(picker.menu_step(1), "down to Show hidden files");
        assert!(picker.menu_press());
        assert!(names(&picker).contains(&".secret".to_string()));
        assert_eq!(picker.menu_rows()[1].label, "Hide hidden files");
    }

    #[test]
    fn a_folder_picker_keeps_its_answer_separate_from_the_folder_it_enters() {
        let directory = PickerDirectory::new("folder-choice");
        std::fs::create_dir(directory.path.join("inside")).expect("a visible folder");

        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::Folder, &directory.path));
        assert_eq!(picker.choose(), None, "the head row is not focused first");
        assert!(
            picker.enter(),
            "the visible folder opens instead of choosing"
        );
        assert!(picker.leave(), "the picker can return to the parent");
        assert!(picker.move_selection(-1), "Up reaches Select folder");
        assert_eq!(picker.choose().as_deref(), Some(directory.path.as_path()));
    }

    #[test]
    fn a_picker_keeps_a_lattice_trail_for_pointer_backtracking() {
        let directory = PickerDirectory::new("trail");
        let inside = directory.path.join("inside");
        std::fs::create_dir(&inside).expect("a visible folder");

        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        assert!(picker.enter());
        assert_eq!(picker.location(), Some(inside.as_path()));
        assert!(picker.point_at(Spot::PickerTrail(1)));
        assert!(picker.leave_to(1));
        assert_eq!(picker.location(), Some(directory.path.as_path()));
        assert!(!picker.leave_to(0));
    }

    #[test]
    fn a_picker_keeps_the_departing_column_until_its_depth_lands() {
        let directory = PickerDirectory::new("departure");
        std::fs::create_dir(directory.path.join("inside")).expect("a visible folder");

        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        assert!(picker.enter());
        assert!(picker.leave());
        assert!(
            picker.levels.len() > picker.open + 1,
            "the column just left must remain available for the departure flight"
        );
        for _ in 0..600 {
            picker.advance(1.0 / 120.0);
        }
        assert_eq!(
            picker.levels.len(),
            picker.open + 1,
            "the settled path kept an old departing column"
        );
    }

    #[test]
    fn a_file_picker_routes_typed_text_to_its_model_search() {
        let directory = PickerDirectory::new("search");
        std::fs::write(directory.path.join("notes.txt"), b"tour").expect("a visible file");

        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        assert!(picker.move_selection(-1), "Up reaches the Search head row");
        assert!(picker.type_text("notes"));
        assert!(
            picker.move_selection(2),
            "Down passes Clear search to the match"
        );
        assert_eq!(
            picker.choose().as_deref(),
            Some(directory.path.join("notes.txt").as_path())
        );
        assert!(picker.move_selection(-2), "Up returns to the Search field");
        assert!(picker.erase_search());
    }

    #[test]
    fn a_query_keeps_its_field_and_clear_search_row_at_the_column_head() {
        let directory = PickerDirectory::new("search-head");
        std::fs::write(directory.path.join("notes.txt"), b"tour").expect("a visible file");

        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        assert!(picker.move_selection(-1));
        assert!(picker.type_text("note"));
        let asked = picker.asking();
        let level = picker.active_level().expect("an open picker has a level");
        assert_eq!(level.head_count(), 2);
        assert_eq!(picker_row_content(asked, level, 0).0, "note");
        assert_eq!(picker_row_content(asked, level, 1).0, "Clear search");
        assert!(picker.move_selection(1), "Down reaches Clear search");
        assert!(
            picker.enter(),
            "Accept on Clear search restores the directory"
        );
        assert!(picker
            .active_level()
            .expect("the active level remains open")
            .picker
            .query()
            .is_empty());
    }

    #[test]
    fn the_picker_keyboard_keeps_the_shells_ansi_rows_and_spans() {
        let counts: Vec<_> = (0..PICKER_KEYBOARD_ROWS)
            .map(|row| picker_keyboard_row_keys(row).len())
            .collect();
        // The bottom row is eight keys, and nine on a machine whose layout
        // keeps letters behind AltGr — which is the one key on this board the
        // layout is allowed to add. Every other row is fixed, which is what
        // stops a keymap changing the shape of the grid.
        let bottom = 8 + usize::from(picker_keyboard_altgr_on_the_board());
        assert_eq!(counts, vec![13, 14, 14, 13, 12, bottom]);
        for row in 0..PICKER_KEYBOARD_ROWS {
            let width: f32 = picker_keyboard_row_spans(row)
                .iter()
                .map(|(_, span)| span)
                .sum();
            assert!((width - PICKER_KEYBOARD_COLUMNS).abs() < 0.0001);
        }
        assert_eq!(picker_keyboard_row_scale(0), 0.56);
        assert_eq!(picker_keyboard_row_scale(1), 1.0);
        assert_eq!(
            picker_keyboard_row_keys(1).last(),
            Some(&PickerKeyboardKey::Named(
                "Back",
                PickerKeyboardStroke::Backspace,
            ))
        );
        // The arrows begin after Ctrl, Alt, Space and AltGr where there is one.
        let arrows = 3 + usize::from(picker_keyboard_altgr_on_the_board());
        assert_eq!(
            picker_keyboard_row_keys(5)[arrows].glyph(),
            Some("arrow-left")
        );
        assert_eq!(
            picker_keyboard_row_keys(5)
                .last()
                .and_then(|key| key.glyph()),
            Some("keyboard-hide")
        );
    }

    /// The caps come off the layout, and the keys stay where they are.
    ///
    /// German is the clearest pair to check both halves at once: it is QWERTZ,
    /// so the key ANSI prints Y types z and the key it prints Z types y — the
    /// letters swapped and the keys exactly where they were.
    #[test]
    fn picker_keyboard_caps_come_off_the_layout_and_the_keys_stay_put() {
        let Some(german) = picker_keyboard_note_layout("de", "") else {
            // No xkeyboard-config on this machine, which a build container may
            // well not have. Nothing to assert about a layout that is not there.
            return;
        };
        let printed = |row: usize| -> Vec<String> {
            german.rows[row]
                .iter()
                .map(|cap| cap.printed(PickerKeyboardLevel::Plain))
                .collect()
        };
        // The lower row, which is the fourth in keycode order.
        assert_eq!(
            printed(3),
            ["y", "x", "c", "v", "b", "n", "m", ",", ".", "-"]
        );
        // And the upper row's first six, plus the key ANSI prints backslash —
        // `#` on a German board.
        assert_eq!(printed(1)[..6], ["q", "w", "e", "r", "t", "z"]);
        assert_eq!(printed(1)[12], "#");
        // Every row still has exactly the number of keys the grid is built for.
        for (row, keycodes) in PICKER_KEYBOARD_KEYCODES.iter().enumerate() {
            assert_eq!(german.rows[row].len(), keycodes.len());
        }
    }

    /// A layout that keeps letters behind AltGr is marked as needing the key,
    /// and one that does not is not.
    ///
    /// Polish is the case this exists for: it is QWERTY, so without AltGr the
    /// board would look right and be unable to write a single Polish word.
    #[test]
    fn picker_keyboard_altgr_is_read_off_the_layout_that_needs_it() {
        let Some(polish) = picker_keyboard_note_layout("pl", "") else {
            return;
        };
        assert!(polish.altgr, "Polish keeps its own letters behind AltGr");
        // <AC01>, which ANSI prints A.
        assert_eq!(polish.rows[2][0].printed(PickerKeyboardLevel::AltGr), "ą");
        let Some(american) = picker_keyboard_note_layout("us", "") else {
            return;
        };
        assert!(!american.altgr, "an American board has no AltGr");
    }

    /// A dead key shows the accent a real keycap shows, and typing it into the
    /// field does nothing rather than putting a stray mark in the query.
    #[test]
    fn picker_keyboard_dead_keys_show_their_accent_and_type_nothing() {
        let Some(french) = picker_keyboard_note_layout("fr", "") else {
            return;
        };
        // <AD11>, which ANSI prints [ and AZERTY prints the circumflex.
        let dead = french.rows[1][10];
        assert_eq!(dead.printed(PickerKeyboardLevel::Plain), "^");
        assert!(matches!(
            dead.at(PickerKeyboardLevel::Plain),
            Some(PickerKeyboardStroke::Dead(_))
        ));
    }

    /// A layout that will not compile is no arrangement at all, which leaves
    /// the board on the ANSI/US rows it is built with.
    #[test]
    fn picker_keyboard_an_impossible_layout_leaves_the_ansi_board() {
        assert!(picker_keyboard_note_layout("no-such-layout-anywhere", "").is_none());
    }

    /// Where the board looks for the keyboard it is a picture of, and in what
    /// order.
    ///
    /// Machine-independent on purpose: which of these places answers here is
    /// this machine's business, but the list must always end in something that
    /// compiles, and no step of it may offer a layout with no name — a keymap
    /// nothing can build, standing in front of one that could.
    #[test]
    fn picker_keyboard_reads_the_setting_before_the_session_and_the_machine() {
        let named = picker_keyboard_layouts();
        assert_eq!(
            named.last(),
            Some(&("us".to_string(), String::new())),
            "the list has to end somewhere xkbcommon will follow"
        );
        assert!(named.iter().all(|(layout, _)| !layout.trim().is_empty()));
        // The shell's own setting, where this machine has one, is asked for
        // first and asked for by reading the file rather than the environment.
        if let Some(chosen) = lxb_toolkit::settings::keyboard_layout() {
            assert_eq!(named.first(), Some(&chosen));
        }
    }

    /// A key with nothing on the face the board is showing types nothing, and
    /// does not spend the modifier armed for the key next to it.
    #[test]
    fn picker_keyboard_a_blank_face_types_nothing_and_keeps_the_modifier() {
        // The cap's own answer first, which is true on every machine: a key
        // built with two characters has nothing on the far two faces.
        let letter = PickerKeyboardCap::letter('a', 'A');
        assert!(letter.at(PickerKeyboardLevel::AltGr).is_none());
        assert!(letter.printed(PickerKeyboardLevel::AltGr).is_empty());
        assert!(!letter.has_altgr());

        // And then the press, on whichever key this machine's layout leaves
        // blank there. Which key that is belongs to xkeyboard-config and
        // changes with the package — Polish, the obvious candidate, includes
        // `latin` and so has something on AltGr for every key — so it is found
        // rather than named, and a layout with none skips this half.
        let mut board = PickerKeyboard::default();
        board.open();
        let blank = (1..5).find_map(|row| {
            picker_keyboard_row_keys(row)
                .into_iter()
                .enumerate()
                .find(|(_, key)| {
                    matches!(key, PickerKeyboardKey::Character(cap)
                        if cap.at(PickerKeyboardLevel::AltGr).is_none())
                })
                .map(|(column, _)| (row, column))
        });
        let Some((row, column)) = blank else {
            return;
        };
        board.select(row, column);
        board.altgr = PickerKeyboardLatch::Once;
        assert_eq!(board.level(), PickerKeyboardLevel::AltGr);
        assert_eq!(board.press(), PickerKeyboardPress::Nothing);
        assert_eq!(
            board.level(),
            PickerKeyboardLevel::AltGr,
            "a key that did nothing takes nothing away"
        );
    }

    #[test]
    fn picker_keyboard_navigation_wraps_and_tracks_key_midpoints() {
        let mut board = PickerKeyboard::default();
        board.open();
        assert_eq!(board.selected_position(), (3, 1));
        // A character key, not a particular letter: which letter the home row's
        // first key prints belongs to the layout this machine is configured
        // for, and a test that named one would be a test about xkeyboard-config.
        assert!(matches!(board.selected(), PickerKeyboardKey::Character(_)));

        assert!(board.move_by(-1, 0));
        assert_eq!(board.selected(), PickerKeyboardKey::Caps);
        assert!(board.move_by(-1, 0), "Left wraps within the home row");
        assert_eq!(
            board.selected(),
            PickerKeyboardKey::Named("Enter", PickerKeyboardStroke::Enter)
        );

        assert!(board.select(2, 10), "put the cursor on the eleventh key");
        assert!(board.move_by(0, 1));
        assert_eq!(
            board.selected_position(),
            (3, 10),
            "Down follows the nearest key midpoint, not the row index"
        );
        assert!(board.select(0, 0));
        assert!(board.move_by(0, -1), "Up wraps from the function row");
        assert_eq!(board.selected_position().0, 5);
    }

    #[test]
    fn picker_keyboard_modifiers_are_one_shot_then_locked() {
        let mut board = PickerKeyboard::default();
        board.open();
        assert!(board.select(4, 0));
        assert_eq!(board.press(), PickerKeyboardPress::Shifted);
        assert_eq!(board.shift, PickerKeyboardLatch::Once);
        assert!(board.select(3, 1));
        assert_eq!(
            board.press(),
            PickerKeyboardPress::Type(PickerKeyboardStroke::Character('A'))
        );
        assert_eq!(board.shift, PickerKeyboardLatch::Off);

        assert!(board.select(4, 0));
        assert_eq!(board.press(), PickerKeyboardPress::Shifted);
        assert_eq!(board.press(), PickerKeyboardPress::Shifted);
        assert_eq!(board.shift, PickerKeyboardLatch::Locked);
        assert!(board.select(3, 1));
        assert_eq!(
            board.press(),
            PickerKeyboardPress::Type(PickerKeyboardStroke::Character('A'))
        );
        assert_eq!(board.shift, PickerKeyboardLatch::Locked);
        assert!(board.select(3, 0));
        assert_eq!(board.press(), PickerKeyboardPress::Shifted);
        assert_eq!(board.shift, PickerKeyboardLatch::Off);

        for (column, modifier) in [(0, PickerKeyboardKey::Control), (1, PickerKeyboardKey::Alt)] {
            let mut board = PickerKeyboard::default();
            board.open();
            assert!(board.select(5, column));
            assert_eq!(board.selected(), modifier);
            assert_eq!(board.press(), PickerKeyboardPress::Shifted);
            assert_eq!(board.latched(modifier), PickerKeyboardLatch::Once);
            assert_eq!(board.press(), PickerKeyboardPress::Shifted);
            assert_eq!(board.latched(modifier), PickerKeyboardLatch::Locked);
            assert_eq!(board.press(), PickerKeyboardPress::Shifted);
            assert_eq!(board.latched(modifier), PickerKeyboardLatch::Off);
        }
    }

    #[test]
    fn picker_keyboard_uses_the_shells_key_units_and_bottom_slide() {
        let picker = [120.0, 80.0, 1380.0, 840.0];
        let layout = picker_keyboard_layout(picker, 840.0 / 1080.0);
        let expected_width = (PICKER_KEYBOARD_KEY_WIDTH * PICKER_KEYBOARD_COLUMNS
            + PICKER_KEYBOARD_PAD * 2.0)
            * layout.scale;
        assert!((layout.panel[2] - expected_width).abs() < 0.01);
        let tab = layout.key_rect(2, 0, 1.0);
        assert!(
            (tab[2] - (1.5 * PICKER_KEYBOARD_KEY_WIDTH - PICKER_KEYBOARD_GAP) * layout.scale).abs()
                < 0.01
        );
        assert!(
            (layout.panel[1] + layout.panel[3] + PICKER_KEYBOARD_BOTTOM * layout.scale
                - (picker[1] + picker[3]))
                .abs()
                < 0.01
        );
        assert!(
            (layout.panel_rect(0.0)[1] - (picker[1] + picker[3])).abs() < 0.01,
            "the closed board begins immediately below the picker window"
        );
    }

    #[test]
    fn picker_keycaps_share_the_boards_pointer_selection() {
        let directory = PickerDirectory::new("keyboard-pointer");
        std::fs::write(directory.path.join("alpha.txt"), b"tour").expect("a visible file");
        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        assert!(picker.move_selection(-1));
        assert!(picker.activate());
        assert!(picker.point_at(Spot::PickerKey { row: 2, column: 2 }));
        assert_eq!(picker.keyboard.selected_position(), (2, 2));
        assert!(!picker.point_at(Spot::PickerRow(0)));
        assert!(picker.close_search_keyboard());
        assert!(
            picker.point_at(Spot::PickerRow(1)),
            "a closed lattice row is selected only when the host aims it on click"
        );
    }

    #[test]
    fn a_picker_search_field_opens_a_controller_board_and_keeps_its_query() {
        let directory = PickerDirectory::new("keyboard");
        std::fs::write(directory.path.join("alpha.txt"), b"tour").expect("a visible file");

        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        assert!(picker.move_selection(-1), "Up reaches the Search head row");
        assert!(picker.activate(), "Accept opens the local search board");
        assert!(picker.search_keyboard_open());
        // Whatever the home row's first key prints on this machine's layout,
        // which is what pressing it has to put in the field.
        let typed = picker.keyboard.selected().label(PickerKeyboardLevel::Plain);
        assert!(!typed.is_empty(), "the board opens on a character key");
        assert!(picker.press_search_keyboard(), "A presses the selected key");
        assert_eq!(
            picker
                .active_level()
                .expect("an active column")
                .picker
                .query(),
            typed
        );
        assert!(picker.close_search_keyboard(), "B closes the board");
        assert!(!picker.search_keyboard_open());
        assert_eq!(
            picker
                .active_level()
                .expect("the column remains")
                .picker
                .query(),
            typed,
            "closing the board must not clear its query"
        );
    }

    #[test]
    fn picker_dates_follow_the_shells_written_calendar_style() {
        use std::time::{Duration, SystemTime};

        // Written in the session's language, so the date is asked of the
        // catalog the same way the row asks for it. What the two languages
        // make of it is below.
        let written = |day: &str, month: usize, year: &str| {
            lxb_toolkit::message!(
                "file-date",
                "day" => day.to_string(),
                "month" => lxb_toolkit::i18n::month(month),
                "year" => year.to_string()
            )
        };
        assert_eq!(
            picker_time_note(SystemTime::UNIX_EPOCH),
            written("1", 1, "1970")
        );
        assert_eq!(picker_civil_date(0), (1970, 1, 1));
        assert_eq!(
            picker_time_note(SystemTime::UNIX_EPOCH + Duration::from_secs(86_400 * 59)),
            written("1", 3, "1970")
        );
    }

    /// The same date in both shipped languages, which is the half the test
    /// above cannot state: Polish writes the month in the genitive.
    #[test]
    fn a_date_is_written_the_way_each_language_writes_one() {
        let catalog = lxb_toolkit::i18n::Catalog::new(lxb_toolkit::i18n::RESOURCES);
        let written = |locale: &str, month: &str| {
            let mut args = lxb_toolkit::i18n::FluentArgs::new();
            args.set("day", "1");
            args.set("month", catalog.text_for(locale, month).to_string());
            args.set("year", "1970");
            catalog.format_for(locale, "file-date", &args)
        };
        assert_eq!(written("en-GB", "month-march"), "1 March 1970");
        assert_eq!(written("pl", "month-march"), "1 marca 1970");
        assert_eq!(written("fr", "month-march"), "1 mars 1970");
        // Spanish fences the month with *de* on both sides, which is why the
        // whole date is one message rather than a separator and three values.
        assert_eq!(written("es", "month-march"), "1 de marzo de 1970");
        // America puts the month first and fences the year off with a comma,
        // which is the whole of why `en-US.ftl` exists.
        assert_eq!(written("en-US", "month-march"), "March 1, 1970");
        // And an English that is neither reads out of the British catalog.
        assert_eq!(written("en_AU.UTF-8", "month-march"), "1 March 1970");
    }

    /// Every message this crate asks for is one the toolkit's catalogs have.
    ///
    /// A label reached by an identifier nothing translates is drawn as the
    /// identifier, in every language including English, and no amount of
    /// checking the catalogs against each other would see it.
    #[test]
    fn every_message_the_components_ask_for_is_one_the_catalogs_have() {
        lxb_toolkit::i18n::Catalog::check_references(
            env!("CARGO_MANIFEST_DIR"),
            lxb_toolkit::i18n::RESOURCES,
        );
    }

    /// Turning the shell's hints off takes this panel's legend with them, and
    /// gives the line beside it the whole foot.
    ///
    /// The setting is one answer for the session rather than a rule about the
    /// shell's own screens — see `lxb_toolkit::settings::button_hints` — so a
    /// file question raised by an application has to obey it too.
    #[test]
    fn the_hints_setting_takes_the_pickers_legend_with_it() {
        let directory = PickerDirectory::new("hints");
        std::fs::write(directory.path.join("only.txt"), b"x").expect("a file");

        let Ok(mut ui) = Ui::headless(1280, 800) else {
            eprintln!("no adapter: the picker's legend was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        let written = |ui: &mut Ui, hints: bool| {
            let mut picker = FilePicker::default();
            assert!(picker.open(PickerSelection::File, &directory.path));
            picker.say_what_the_buttons_do(hints);
            for _ in 0..90 {
                picker.advance(1.0 / 60.0);
            }
            ui.begin(
                1280.0,
                800.0,
                10.0,
                &accent,
                lxb_toolkit::settings::WallpaperStyle::Default,
                false,
                IconStyle::Default,
            );
            ui.file_picker(&mut picker);
            let said: Vec<String> = [PANE, CONTROL_LAYER, OVER]
                .into_iter()
                .flat_map(|layer| ui.scene.layers[layer].runs.iter())
                .map(|run| run.text.clone())
                .collect();
            (said, picker.legend_left)
        };

        let (loud, narrowed) = written(&mut ui, true);
        for word in ["Select", "Options", "Cancel"] {
            assert!(
                loud.iter().any(|said| said == word),
                "the legend says {word}: {loud:?}"
            );
        }

        let (quiet, whole_foot) = written(&mut ui, false);
        for word in ["Select", "Options", "Cancel"] {
            assert!(
                !quiet.iter().any(|said| said == word),
                "{word} was written with the hints off: {quiet:?}"
            );
        }
        // And the panel is otherwise the panel it was: the file is still listed,
        // and the line that says what is showing now has the room the buttons
        // were taking.
        assert!(quiet.iter().any(|said| said == "only.txt"), "{quiet:?}");
        assert!(
            whole_foot > narrowed,
            "the foot kept the room the legend gave back: {whole_foot} against {narrowed}"
        );
    }

    #[test]
    fn round_previews_crop_the_long_axis_through_the_middle() {
        let cell = [0.1, 0.2, 0.9, 0.8];
        let wide = picker_round_preview_cell(crate::renderer::Thumbnail { cell, aspect: 2.0 });
        for (actual, expected) in wide.into_iter().zip([0.3, 0.2, 0.7, 0.8]) {
            assert!((actual - expected).abs() < 0.0001);
        }

        let tall = picker_round_preview_cell(crate::renderer::Thumbnail { cell, aspect: 0.5 });
        assert!((tall[0] - 0.1).abs() < 0.0001);
        assert!((tall[1] - 0.35).abs() < 0.0001);
        assert!((tall[2] - 0.9).abs() < 0.0001);
        assert!((tall[3] - 0.65).abs() < 0.0001);
    }

    #[test]
    fn an_image_file_replaces_its_glyph_with_a_round_preview() {
        let directory = PickerDirectory::new("image-preview");
        std::fs::create_dir(directory.path.join("Scenes")).expect("a child folder");
        let picture = image::RgbaImage::from_fn(80, 40, |x, y| {
            image::Rgba([(x * 3) as u8, (y * 6) as u8, 180, 255])
        });
        picture
            .save(directory.path.join("Scenes/wide.png"))
            .expect("a synthetic picker photograph");

        let Ok(mut ui) = Ui::headless(1280, 800) else {
            eprintln!("no adapter: picker thumbnail staging was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::Image, &directory.path));
        for _ in 0..90 {
            picker.advance(1.0 / 60.0);
        }
        assert!(picker.enter());

        picker.depth = picker.open as f32 - 0.4;
        picker.depth_speed = 0.0;

        ui.begin(
            1280.0,
            800.0,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );
        ui.file_picker(&mut picker);

        let preview = ui.scene.layers[OVER]
            .quads
            .iter()
            .find(|quad| quad.shape[1] == crate::renderer::KIND_IMAGE)
            .expect("the image row should wear its picture");
        assert!((preview.shape[0] - preview.rect[3] * 0.5).abs() < 0.01);
        let panel = picker_window(1280.0, 800.0);
        let content = picker_body(panel, lxb_toolkit::metrics::scale_for(panel[3]));
        assert_eq!(
            preview.cut,
            [
                content[0],
                content[1],
                content[0] + content[2],
                content[1] + content[3]
            ]
        );
        let preview_middle = [
            preview.rect[0] + preview.rect[2] * 0.5,
            preview.rect[1] + preview.rect[3] * 0.5,
        ];
        assert!(
            ui.scene.layers[OVER].quads.iter().any(|quad| {
                quad.shape[1] == crate::renderer::KIND_GLASS
                    && quad.cut == preview.cut
                    && quad.rect[2] > preview.rect[2]
                    && (quad.shape[0] - quad.rect[3] * 0.5).abs() < 0.01
                    && (quad.rect[0] + quad.rect[2] * 0.5 - preview_middle[0]).abs() < 0.01
                    && (quad.rect[1] + quad.rect[3] * 0.5 - preview_middle[1]).abs() < 0.01
            }),
            "the moving photograph left its selection disc"
        );
        let generic = ui.cell(
            ui.mark_index("category-images")
                .expect("the fallback image mark"),
        );
        assert!(ui.scene.layers[OVER]
            .quads
            .iter()
            .all(|quad| { quad.shape[1] != crate::renderer::KIND_GLYPH || quad.cell != generic }));
        ui.end_to_image()
            .expect("the colour preview shader should draw");
    }

    #[test]
    fn picker_content_keeps_its_geometry_and_is_cut_to_the_window() {
        const WIDTH: u32 = 400;
        const HEIGHT: u32 = 800;
        let directory = PickerDirectory::new("visual-clip");
        std::fs::create_dir(directory.path.join("Scenes")).expect("a child folder");
        std::fs::write(
            directory.path.join("Scenes/dawn.svg"),
            b"not decoded in this test",
        )
        .expect("a visible child file");
        let Ok(mut ui) = Ui::headless(WIDTH, HEIGHT) else {
            eprintln!("no adapter: picker clipping was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        for _ in 0..90 {
            picker.advance(1.0 / 60.0);
        }
        assert!(picker.enter());
        picker.depth = picker.open as f32 - 0.51;
        picker.depth_speed = 0.0;

        ui.begin(
            WIDTH as f32,
            HEIGHT as f32,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );
        ui.file_picker(&mut picker);

        let panel = picker_window(WIDTH as f32, HEIGHT as f32);
        let content = picker_body(panel, lxb_toolkit::metrics::scale_for(panel[3]));
        let cut = [
            content[0],
            content[1],
            content[0] + content[2],
            content[1] + content[3],
        ];
        let clipped: Vec<_> = ui.scene.layers[OVER]
            .quads
            .iter()
            .filter(|quad| quad.cut != crate::renderer::NO_CUT)
            .collect();
        assert!(!clipped.is_empty(), "the picker emitted no clipped content");
        assert!(clipped.iter().all(|quad| quad.cut == cut));
        assert!(
            clipped.iter().any(|quad| {
                quad.rect[0] < content[0]
                    || quad.rect[1] < content[1]
                    || quad.rect[0] + quad.rect[2] > content[0] + content[2]
                    || quad.rect[1] + quad.rect[3] > content[1] + content[3]
            }),
            "the regression must include full geometry crossing the viewport"
        );
        ui.end_to_image()
            .expect("the clipped picker shader should draw");
    }

    #[test]
    fn a_path_too_long_for_its_box_gives_up_its_beginning() {
        let Ok(mut ui) = Ui::headless(1280, 800) else {
            eprintln!("no adapter: the path cut was not checked");
            return;
        };
        let size = 20.0;
        let whole = "/home/somebody/Pictures/Holidays/2019/Norway/fjords";
        let width = ui.shaped_width(whole, size, false);

        assert_eq!(
            ui.cut_from_the_front(whole, size, width + 1.0),
            whole,
            "a path that fits is left alone"
        );

        let cut = ui.cut_from_the_front(whole, size, width * 0.5);
        assert!(cut.starts_with('…'), "{cut}");
        assert!(
            whole.ends_with(cut.trim_start_matches('…')),
            "the end of the path is what survives: {cut}"
        );
        assert!(ui.shaped_width(&cut, size, false) <= width * 0.5);
        assert_eq!(ui.cut_from_the_front(whole, size, 0.0), whole);
    }

    #[test]
    fn a_picker_is_a_contained_lattice_window_that_recedes_its_page() {
        let directory = PickerDirectory::new("overlay");
        std::fs::write(directory.path.join("notes.txt"), b"tour").expect("a visible file");
        let Ok(mut ui) = Ui::headless(1280, 800) else {
            eprintln!("no adapter: picker lattice staging was not checked");
            return;
        };
        let accent = lxb_toolkit::accent::Accent::default_accent();
        let mut picker = FilePicker::default();
        assert!(picker.open(PickerSelection::File, &directory.path));
        for _ in 0..90 {
            picker.advance(1.0 / 60.0);
        }

        ui.begin(
            1280.0,
            800.0,
            10.0,
            &accent,
            lxb_toolkit::settings::WallpaperStyle::Default,
            false,
            IconStyle::Default,
        );
        ui.pane([20.0, 20.0, 1240.0, 760.0], Overlay::Dialog);
        ui.spot(7, [100.0, 100.0, 200.0, 60.0]);
        ui.label(
            [100.0, 100.0, 200.0, 60.0],
            Text::Body,
            "the page behind",
            Role::Text,
            Align::Left,
        );
        ui.file_picker(&mut picker);

        let panel = picker_window(1280.0, 800.0);
        let share = PICKER_WINDOW_WIDTH * PICKER_WINDOW_HEIGHT;
        assert!((panel[2] * panel[3] / (1280.0 * 800.0) - share).abs() < 0.0001);
        assert_eq!(ui.at(110.0, 110.0), Spot::OutsidePicker);
        assert!(
            ui.scene.layers[OVER].quads.iter().any(|quad| {
                quad.rect == panel
                    && quad.shape[1] == crate::renderer::KIND_GLASS
                    && quad.material[0] > 0.0
                    && quad.material[1] >= PICKER_WINDOW_FROST
            }),
            "the lattice did not put dense glass on its contained window"
        );
        assert!(
            ui.scene.layers[OVER].quads.iter().all(|quad| {
                quad.rect != [0.0, 0.0, 1280.0, 800.0]
                    || quad.shape[1] != crate::renderer::KIND_GLASS
                    || quad.shape[3] < 1.0
            }),
            "the ambient page frost became another opaque full-screen slab"
        );
        let page_ink: Vec<_> = [PANE, CONTROL_LAYER]
            .into_iter()
            .flat_map(|layer| ui.scene.layers[layer].runs.iter())
            .filter(|run| run.text == "the page behind")
            .collect();
        assert!(
            page_ink.iter().all(|run| run.tint[3] <= 0.13),
            "the page text remained readable behind the picker: {page_ink:?}"
        );
        for (spot, rect) in &ui.spots {
            if matches!(spot, Spot::PickerRow(_) | Spot::PickerTrail(_)) {
                assert!(
                    rect[0] >= panel[0]
                        && rect[1] >= panel[1]
                        && rect[0] + rect[2] <= panel[0] + panel[2]
                        && rect[1] + rect[3] <= panel[1] + panel[3],
                    "a picker target escaped the window: {spot:?} at {rect:?}"
                );
            }
        }
    }

    fn thumbnail_of(aspect: f32) -> crate::renderer::Thumbnail {
        crate::renderer::Thumbnail {
            cell: [0.25, 0.5, 0.5, 0.75],
            aspect,
        }
    }

    #[test]
    fn a_contained_picture_keeps_its_own_shape_inside_the_room_given() {
        let (rect, cell) = fitted(thumbnail_of(2.0), [10.0, 20.0, 200.0, 200.0], Fit::Contain)
            .expect("a picture with room to be drawn in");
        assert_eq!(
            cell,
            [0.25, 0.5, 0.5, 0.75],
            "a contained picture was cropped, when the whole of it is what Contain means"
        );
        assert_eq!(
            rect,
            [10.0, 70.0, 200.0, 100.0],
            "a wide picture in a square did not sit centred at the full width"
        );

        let (rect, _) = fitted(thumbnail_of(0.5), [10.0, 20.0, 200.0, 200.0], Fit::Contain)
            .expect("a picture with room to be drawn in");
        assert_eq!(
            rect,
            [60.0, 20.0, 100.0, 200.0],
            "a tall picture in a square did not sit centred at the full height"
        );
    }

    #[test]
    fn a_covering_picture_fills_the_room_and_loses_the_overhang_from_both_edges() {
        let (rect, cell) = fitted(thumbnail_of(2.0), [0.0, 0.0, 100.0, 100.0], Fit::Cover)
            .expect("a picture with room to be drawn in");
        assert_eq!(
            rect,
            [0.0, 0.0, 100.0, 100.0],
            "a covering picture left part of its rectangle unpainted"
        );
        let [u0, v0, u1, v1] = cell;
        assert!(
            (u1 - u0 - 0.125).abs() < 1e-6,
            "half a twice-as-wide picture should survive a square: {cell:?}"
        );
        assert!(
            (v1 - v0 - 0.25).abs() < 1e-6,
            "a covering crop took height it did not need: {cell:?}"
        );
        assert!(
            (u0 - 0.3125).abs() < 1e-6 && (u1 - 0.4375).abs() < 1e-6,
            "a covering crop was taken from one edge rather than both: {cell:?}"
        );
    }

    #[test]
    fn a_picture_of_no_known_shape_is_treated_as_a_square() {
        let (rect, _) = fitted(
            thumbnail_of(f32::NAN),
            [0.0, 0.0, 200.0, 100.0],
            Fit::Contain,
        )
        .expect("a picture with room to be drawn in");
        assert_eq!(
            rect,
            [50.0, 0.0, 100.0, 100.0],
            "a picture whose shape is not a number was not squared"
        );
    }

    #[test]
    fn a_picture_asked_for_in_no_room_at_all_is_not_drawn() {
        assert!(
            fitted(thumbnail_of(1.0), [0.0, 0.0, 0.0, 40.0], Fit::Contain).is_none(),
            "a picture was fitted into a rectangle of no width"
        );
        assert!(
            fitted(thumbnail_of(1.0), [0.0, 0.0, 40.0, -1.0], Fit::Cover).is_none(),
            "a picture was fitted into a rectangle of negative height"
        );
    }
}
