use lxb_toolkit::{
    control,
    material::{Overlay, Surface},
    menu,
    metrics::{capsule_radius, Metric},
    motion,
    palette::Role,
    settings::IconStyle,
    typography::{Face, Text},
};

use crate::renderer::{Quad, Run, Ui, OVER, PANE};
use crate::{Align, Spot};

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

        let tint = self.tinted(role, control::lit_alpha(seconds) * strength);
        let scale = self.scale;
        self.quad(
            layer,
            Quad::glass(rect, radius, tint, control::lit(), scale),
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
        if string.is_empty() {
            return;
        }
        let [x, y, width, height] = rect;
        let size = self.size(text);
        let left = match align {
            Align::Left => x,
            Align::Centre => x + (width - self.measure(text, string)) / 2.0,
            Align::Right => x + width - self.measure(text, string),
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
                clip: None,
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
            let quiet = self.tinted(Role::TextSoft, out);
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

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROL_LAYER: usize = crate::renderer::CONTROL;

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
}
