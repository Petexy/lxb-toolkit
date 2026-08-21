pub const HALO_REACH: f32 = 0.24;

pub const HALO_SPACING: f32 = 1.3;

pub const HALO_MIN_COPIES: usize = 8;

pub const HALO_RINGS: [(f32, f32); 3] = [(0.34, 0.20), (0.67, 0.13), (1.0, 0.075)];

pub fn halo_copies(size: f32, strength: f32) -> Vec<(f32, f32, f32)> {
    if !size.is_finite() || size <= 0.0 || !strength.is_finite() || strength <= 0.0 {
        return Vec::new();
    }
    let reach = size * HALO_REACH;
    let mut copies = Vec::new();
    for (ring, (spread, weight)) in HALO_RINGS.iter().enumerate() {
        let radius = reach * spread;
        let steps =
            ((std::f32::consts::TAU * radius / HALO_SPACING).ceil() as usize).max(HALO_MIN_COPIES);
        let turn = std::f32::consts::TAU / steps as f32;
        let lead = turn * ring as f32 / HALO_RINGS.len() as f32;
        for step in 0..steps {
            let angle = turn * step as f32 + lead;
            copies.push((
                radius * angle.cos(),
                radius * angle.sin(),
                (strength * weight).clamp(0.0, 1.0),
            ));
        }
    }
    copies
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

impl Alignment {
    pub const fn name(self) -> &'static str {
        match self {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Overflow {
    Wrap,
    #[default]
    Ellipsis,
    Clip,
}

impl Overflow {
    pub const fn name(self) -> &'static str {
        match self {
            Overflow::Wrap => "wrap",
            Overflow::Ellipsis => "ellipsis",
            Overflow::Clip => "clip",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ClipRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Regular,
    Bold,
}

impl Face {
    pub const fn name(self) -> &'static str {
        match self {
            Face::Regular => "regular",
            Face::Bold => "bold",
        }
    }

    pub const fn bytes(self) -> &'static [u8] {
        match self {
            Face::Regular => crate::assets::FONT_REGULAR,
            Face::Bold => crate::assets::FONT_BOLD,
        }
    }

    pub const fn family(self) -> &'static str {
        "Roboto"
    }

    pub const fn weight(self) -> u16 {
        match self {
            Face::Regular => 400,
            Face::Bold => 700,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Text {
    Display,

    Title,

    Body,

    Label,

    Caption,
}

impl Text {
    pub const ALL: [Text; 5] = [
        Text::Display,
        Text::Title,
        Text::Body,
        Text::Label,
        Text::Caption,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Text::Display => "display",
            Text::Title => "title",
            Text::Body => "body",
            Text::Label => "label",
            Text::Caption => "caption",
        }
    }

    pub const fn size(self) -> f32 {
        match self {
            Text::Display => 62.0,
            Text::Title => 32.0,
            Text::Body => 26.0,
            Text::Label => 24.0,
            Text::Caption => 22.0,
        }
    }

    pub const fn face(self) -> Face {
        match self {
            Text::Display | Text::Title => Face::Bold,
            _ => Face::Regular,
        }
    }

    pub const LINE: f32 = 1.25;

    pub fn on(self, height: f32) -> f32 {
        self.size() * crate::metrics::scale_for(height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_descends_in_visible_steps() {
        let sizes: Vec<f32> = Text::ALL.iter().map(|t| t.size()).collect();
        for pair in sizes.windows(2) {
            assert!(pair[0] > pair[1], "{pair:?}");
        }
        assert!(
            Text::Display.size() / Text::Body.size() > 2.0,
            "the largest type is read first and from furthest away"
        );
    }

    #[test]
    fn headings_are_bold_and_both_faces_are_here() {
        assert_eq!(Text::Title.face(), Face::Bold);
        assert_eq!(Text::Body.face(), Face::Regular);
        for face in [Face::Regular, Face::Bold] {
            assert!(face.bytes().len() > 1000, "{}", face.name());
        }
        assert!(Face::Bold.weight() > Face::Regular.weight());
    }

    #[test]
    fn type_scales_with_the_screen() {
        assert_eq!(Text::Body.on(1080.0), 26.0);
        assert_eq!(Text::Body.on(2160.0), 52.0);
    }

    #[test]
    fn the_halo_is_the_shells_three_soft_rings() {
        assert_eq!(HALO_REACH, 0.24);
        assert_eq!(HALO_SPACING, 1.3);
        assert_eq!(HALO_MIN_COPIES, 8);
        assert_eq!(HALO_RINGS, [(0.34, 0.20), (0.67, 0.13), (1.0, 0.075)]);
        assert!(HALO_RINGS
            .windows(2)
            .all(|pair| { pair[0].0 < pair[1].0 && pair[0].1 > pair[1].1 }));

        let copies = halo_copies(26.0, 1.0);
        assert!(copies.len() > HALO_MIN_COPIES * HALO_RINGS.len());
        assert!(copies.iter().all(|(x, y, opacity)| x.is_finite()
            && y.is_finite()
            && (0.0..=1.0).contains(opacity)));
        assert!(halo_copies(26.0, 0.0).is_empty());
        assert!(halo_copies(f32::NAN, 1.0).is_empty());
    }

    #[test]
    fn layout_contracts_have_stable_renderer_neutral_names() {
        assert_eq!(Alignment::default(), Alignment::Left);
        assert_eq!(Overflow::default(), Overflow::Ellipsis);
        assert_eq!(Alignment::Center.name(), "center");
        assert_eq!(Overflow::Wrap.name(), "wrap");
        assert_eq!(Overflow::Clip.name(), "clip");
    }
}
