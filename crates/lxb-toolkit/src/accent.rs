use crate::motion::{duration::ACCENT_CHANGE, smoothstep};
use crate::palette::{palette_index, Palette, Rendered, Role, PALETTES};

#[derive(Debug, Clone, Copy)]
pub struct Accent {
    applied: usize,

    target: usize,
    from: Rendered,
    to: Rendered,
    shown: Rendered,
    progress: f32,
}

impl Accent {
    pub fn new(name: &str) -> Option<Self> {
        Some(Self::settled(palette_index(name)?))
    }

    pub fn default_accent() -> Self {
        Self::settled(0)
    }

    fn settled(index: usize) -> Self {
        let shown = PALETTES[index].rendered();
        Self {
            applied: index,
            target: index,
            from: shown,
            to: shown,
            shown,
            progress: 1.0,
        }
    }

    pub fn applied(&self) -> &'static Palette {
        &PALETTES[self.applied.min(PALETTES.len() - 1)]
    }

    pub fn shown(&self) -> Rendered {
        self.shown
    }

    pub fn color(&self, role: Role) -> crate::color::Linear {
        self.shown.color(role)
    }

    pub fn preview(&mut self, name: &str) -> bool {
        match palette_index(name) {
            Some(index) => self.travel_to(index),
            None => false,
        }
    }

    pub fn commit(&mut self, name: &str) -> bool {
        let Some(index) = palette_index(name) else {
            return false;
        };
        self.applied = index;
        self.travel_to(index);
        true
    }

    pub fn restore(&mut self) {
        self.travel_to(self.applied.min(PALETTES.len() - 1));
    }

    pub fn set(&mut self, name: &str) -> bool {
        match palette_index(name) {
            Some(index) => {
                *self = Self::settled(index);
                true
            }
            None => false,
        }
    }

    fn travel_to(&mut self, index: usize) -> bool {
        if self.target == index {
            return false;
        }
        let to = PALETTES[index].rendered();
        if self.shown == to {
            let applied = self.applied;
            *self = Self::settled(index);
            self.applied = applied;
            return false;
        }
        self.target = index;
        self.from = self.shown;
        self.to = to;
        self.progress = 0.0;
        true
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        if self.progress >= 1.0 {
            return false;
        }
        if !dt.is_finite() || dt <= 0.0 {
            return true;
        }
        self.progress = (self.progress + dt / ACCENT_CHANGE).min(1.0);
        self.shown = self.from.mix(&self.to, smoothstep(self.progress));
        if self.progress >= 1.0 {
            self.shown = self.to;
            false
        } else {
            true
        }
    }

    pub fn moving(&self) -> bool {
        self.progress < 1.0
    }
}

impl Default for Accent {
    fn default() -> Self {
        Self::default_accent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(accent: &mut Accent) {
        for _ in 0..1000 {
            if !accent.advance(1.0 / 60.0) {
                return;
            }
        }
        panic!("a transition that never finished");
    }

    #[test]
    fn walking_a_list_shows_colours_without_choosing_them() {
        let mut accent = Accent::new("Purple").expect("Purple exists");
        assert!(accent.preview("Blue"));
        settle(&mut accent);
        assert_eq!(accent.shown(), crate::palette::BLUE.rendered(), "not shown");
        assert_eq!(accent.applied().name, "Purple", "a preview chose it");

        accent.restore();
        settle(&mut accent);
        assert_eq!(accent.shown(), crate::palette::PURPLE.rendered());
    }

    #[test]
    fn a_press_keeps_the_colour_that_is_already_on_screen() {
        let mut accent = Accent::new("Purple").unwrap();
        accent.preview("Green");
        settle(&mut accent);
        assert!(accent.commit("Green"));
        assert!(!accent.moving(), "the press restarted a finished journey");
        assert_eq!(accent.applied().name, "Green");
        accent.restore();
        assert!(!accent.moving());
    }

    #[test]
    fn a_change_of_mind_carries_on_from_where_it_is() {
        let mut accent = Accent::new("Purple").unwrap();
        accent.preview("Blue");
        accent.advance(ACCENT_CHANGE / 2.0);
        let halfway = accent.shown();
        assert_ne!(halfway, crate::palette::PURPLE.rendered());
        assert_ne!(halfway, crate::palette::BLUE.rendered());

        accent.preview("Red");
        assert_eq!(accent.shown(), halfway, "it jumped on the frame it turned");
        settle(&mut accent);
        assert_eq!(accent.shown(), crate::palette::RED.rendered());
    }

    #[test]
    fn a_settled_palette_is_the_authored_one() {
        let mut accent = Accent::new("Purple").unwrap();
        accent.preview("Yellow");
        settle(&mut accent);
        for role in Role::ALL {
            assert_eq!(
                accent.color(role).srgb(),
                crate::palette::YELLOW.color(role),
                "{}",
                role.name()
            );
        }
    }

    #[test]
    fn everything_on_screen_is_the_same_distance_along() {
        let mut accent = Accent::new("Purple").unwrap();
        accent.preview("Blue");
        accent.advance(ACCENT_CHANGE * 0.4);
        let shown = accent.shown();

        let progress = |role: Role| {
            let from = crate::palette::PURPLE.color(role).linear().rgb();
            let to = crate::palette::BLUE.color(role).linear().rgb();
            let channel = (0..3)
                .max_by(|a, b| {
                    (to[*a] - from[*a])
                        .abs()
                        .total_cmp(&(to[*b] - from[*b]).abs())
                })
                .expect("three channels");
            let travel = to[channel] - from[channel];
            assert!(travel.abs() > 1e-4, "{} does not move at all", role.name());
            (shown.color(role).rgb()[channel] - from[channel]) / travel
        };
        let accent_progress = progress(Role::Accent);
        assert!(
            accent_progress > 0.0 && accent_progress < 1.0,
            "nothing moved"
        );
        for role in [Role::Glass, Role::SkyTop, Role::Glow, Role::TextSoft] {
            assert!(
                (progress(role) - accent_progress).abs() < 1e-3,
                "{} is not travelling with the accent",
                role.name()
            );
        }
    }

    #[test]
    fn a_bad_frame_neither_finishes_nor_freezes_it() {
        let mut accent = Accent::new("Purple").unwrap();
        accent.preview("Blue");
        assert!(accent.advance(0.0), "a zero frame ended it");
        assert!(accent.advance(f32::NAN), "a broken clock ended it");
        assert_eq!(accent.shown(), crate::palette::PURPLE.rendered());
        settle(&mut accent);
    }

    #[test]
    fn an_unknown_name_changes_nothing() {
        assert!(Accent::new("mauve").is_none());
        let mut accent = Accent::new("Purple").unwrap();
        assert!(!accent.preview("mauve"));
        assert!(!accent.commit("mauve"));
        assert!(!accent.set("mauve"));
        assert!(!accent.moving());
        assert_eq!(accent.applied().name, "Purple");
        assert_eq!(Accent::default().applied().name, "Purple");
    }
}
