use crate::color::{Linear, Srgb};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Accent,

    AccentSoft,

    AccentDeep,

    Glass,

    GlassRaised,

    Rim,
    Text,
    TextSoft,

    Danger,

    Glow,

    SkyTop,
    SkyBottom,
    SkyTopAlt,
    SkyBottomAlt,
}

impl Role {
    pub const ALL: [Role; 14] = [
        Role::Accent,
        Role::AccentSoft,
        Role::AccentDeep,
        Role::Glass,
        Role::GlassRaised,
        Role::Rim,
        Role::Text,
        Role::TextSoft,
        Role::Danger,
        Role::Glow,
        Role::SkyTop,
        Role::SkyBottom,
        Role::SkyTopAlt,
        Role::SkyBottomAlt,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Role::Accent => "accent",
            Role::AccentSoft => "accent-soft",
            Role::AccentDeep => "accent-deep",
            Role::Glass => "glass",
            Role::GlassRaised => "glass-raised",
            Role::Rim => "rim",
            Role::Text => "text",
            Role::TextSoft => "text-soft",
            Role::Danger => "danger",
            Role::Glow => "glow",
            Role::SkyTop => "sky-top",
            Role::SkyBottom => "sky-bottom",
            Role::SkyTopAlt => "sky-top-alt",
            Role::SkyBottomAlt => "sky-bottom-alt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub name: &'static str,
    pub accent: Srgb,
    pub accent_soft: Srgb,
    pub accent_deep: Srgb,
    pub glass: Srgb,
    pub glass_raised: Srgb,
    pub rim: Srgb,
    pub text: Srgb,
    pub text_soft: Srgb,
    pub danger: Srgb,
    pub glow: Srgb,

    pub sky: [Srgb; 4],
}

impl Palette {
    pub const fn color(&self, role: Role) -> Srgb {
        match role {
            Role::Accent => self.accent,
            Role::AccentSoft => self.accent_soft,
            Role::AccentDeep => self.accent_deep,
            Role::Glass => self.glass,
            Role::GlassRaised => self.glass_raised,
            Role::Rim => self.rim,
            Role::Text => self.text,
            Role::TextSoft => self.text_soft,
            Role::Danger => self.danger,
            Role::Glow => self.glow,
            Role::SkyTop => self.sky[0],
            Role::SkyBottom => self.sky[1],
            Role::SkyTopAlt => self.sky[2],
            Role::SkyBottomAlt => self.sky[3],
        }
    }

    pub fn rendered(&self) -> Rendered {
        let mut colors = [Linear([0.0; 3]); Role::ALL.len()];
        let mut index = 0;
        while index < Role::ALL.len() {
            colors[index] = self.color(Role::ALL[index]).linear();
            index += 1;
        }
        Rendered { colors }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rendered {
    colors: [Linear; Role::ALL.len()],
}

impl Rendered {
    pub fn color(&self, role: Role) -> Linear {
        self.colors[Role::ALL.iter().position(|&r| r == role).unwrap_or(0)]
    }

    pub fn a(&self, role: Role, alpha: f32) -> [f32; 4] {
        self.color(role).a(alpha)
    }

    pub fn mix(&self, other: &Self, amount: f32) -> Self {
        let mut colors = self.colors;
        for (mixed, (from, to)) in colors
            .iter_mut()
            .zip(self.colors.iter().zip(other.colors.iter()))
        {
            *mixed = from.mix(*to, amount);
        }
        Self { colors }
    }
}

pub const PURPLE: Palette = Palette {
    name: "Purple",
    accent: Srgb(0x8B5CF6),
    accent_soft: Srgb(0xC4B5FD),
    accent_deep: Srgb(0x4C1D95),
    glass: Srgb(0x140B26),
    glass_raised: Srgb(0xB9A7E8),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xD6CBEF),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x5B21B6),
    sky: [
        Srgb(0x1B1140),
        Srgb(0x060314),
        Srgb(0x2A1252),
        Srgb(0x0A0620),
    ],
};

pub const BLUE: Palette = Palette {
    name: "Blue",
    accent: Srgb(0x3B82F6),
    accent_soft: Srgb(0x93C5FD),
    accent_deep: Srgb(0x1E3A8A),
    glass: Srgb(0x081026),
    glass_raised: Srgb(0xA7C6E8),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xCBDDEF),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x1E40AF),
    sky: [
        Srgb(0x0F1B40),
        Srgb(0x030614),
        Srgb(0x122A52),
        Srgb(0x060A20),
    ],
};

pub const GREEN: Palette = Palette {
    name: "Green",
    accent: Srgb(0x16A34A),
    accent_soft: Srgb(0x91D39F),
    accent_deep: Srgb(0x14532D),
    glass: Srgb(0x06170D),
    glass_raised: Srgb(0xA7C8B1),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xC7DFCE),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x14532D),
    sky: [
        Srgb(0x082114),
        Srgb(0x020A05),
        Srgb(0x0A3018),
        Srgb(0x030F08),
    ],
};

pub const YELLOW: Palette = Palette {
    name: "Yellow",
    accent: Srgb(0xCA8A04),
    accent_soft: Srgb(0xDFBC73),
    accent_deep: Srgb(0x713F12),
    glass: Srgb(0x151002),
    glass_raised: Srgb(0xD8C389),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xE9DFC3),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x713F12),
    sky: [
        Srgb(0x292008),
        Srgb(0x0D0A02),
        Srgb(0x382B0A),
        Srgb(0x151003),
    ],
};

pub const RED: Palette = Palette {
    name: "Red",
    accent: Srgb(0xEF4444),
    accent_soft: Srgb(0xFCA5A5),
    accent_deep: Srgb(0x7F1D1D),
    glass: Srgb(0x230707),
    glass_raised: Srgb(0xD8A7A7),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xE9CCCC),
    danger: Srgb(0xF59E0B),
    glow: Srgb(0x991B1B),
    sky: [
        Srgb(0x3A0D0D),
        Srgb(0x100202),
        Srgb(0x501010),
        Srgb(0x1E0505),
    ],
};

pub const TEAL: Palette = Palette {
    name: "Teal",
    accent: Srgb(0x14B8A6),
    accent_soft: Srgb(0x5EEAD4),
    accent_deep: Srgb(0x134E4A),
    glass: Srgb(0x051A19),
    glass_raised: Srgb(0xA7CFC9),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xC7E0DB),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x115E59),
    sky: [
        Srgb(0x08302B),
        Srgb(0x020F0D),
        Srgb(0x0B4038),
        Srgb(0x041614),
    ],
};

pub const INDIGO: Palette = Palette {
    name: "Indigo",
    accent: Srgb(0xA5B4FC),
    accent_soft: Srgb(0xC7D2FE),
    accent_deep: Srgb(0x312E81),
    glass: Srgb(0x0D0E22),
    glass_raised: Srgb(0xB3B9DE),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xD5D9F2),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x252270),
    sky: [
        Srgb(0x101128),
        Srgb(0x030409),
        Srgb(0x15163A),
        Srgb(0x070813),
    ],
};

pub const PINK: Palette = Palette {
    name: "Pink",
    accent: Srgb(0xEC4899),
    accent_soft: Srgb(0xF9A8D4),
    accent_deep: Srgb(0x831843),
    glass: Srgb(0x200714),
    glass_raised: Srgb(0xE0A7C4),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xF0CCE0),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x861042),
    sky: [
        Srgb(0x2C0A1E),
        Srgb(0x0C0207),
        Srgb(0x3E0D28),
        Srgb(0x160410),
    ],
};

pub const ORANGE: Palette = Palette {
    name: "Orange",
    accent: Srgb(0xF97316),
    accent_soft: Srgb(0xFDBA74),
    accent_deep: Srgb(0x7C2D12),
    glass: Srgb(0x1D0B02),
    glass_raised: Srgb(0xE8C3A7),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xF0DAC4),
    danger: Srgb(0xDC2626),
    glow: Srgb(0x842C0F),
    sky: [
        Srgb(0x301206),
        Srgb(0x0D0402),
        Srgb(0x421A07),
        Srgb(0x160702),
    ],
};

pub const WHITE: Palette = Palette {
    name: "White",
    accent: Srgb(0xE4E4E4),
    accent_soft: Srgb(0xF5F5F5),
    accent_deep: Srgb(0x3D3D3D),
    glass: Srgb(0x0C0C0C),
    glass_raised: Srgb(0xC4C4C4),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xC9C9C9),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x444444),
    sky: [
        Srgb(0x151515),
        Srgb(0x040404),
        Srgb(0x1E1E1E),
        Srgb(0x090909),
    ],
};

pub const SILVER: Palette = Palette {
    name: "Silver",
    accent: Srgb(0xA6A6A6),
    accent_soft: Srgb(0xD6D6D6),
    accent_deep: Srgb(0x4A4A4A),
    glass: Srgb(0x121212),
    glass_raised: Srgb(0xC0C0C0),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xCFCFCF),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x6B6B6B),
    sky: [
        Srgb(0x2A2A2A),
        Srgb(0x0B0B0B),
        Srgb(0x383838),
        Srgb(0x131313),
    ],
};

pub const BLACK: Palette = Palette {
    name: "Black",
    accent: Srgb(0x525252),
    accent_soft: Srgb(0x8A8A8A),
    accent_deep: Srgb(0x1F1F1F),
    glass: Srgb(0x0A0A0A),
    glass_raised: Srgb(0x969696),
    rim: Srgb(0xFFFFFF),
    text: Srgb(0xFFFFFF),
    text_soft: Srgb(0xB0B0B0),
    danger: Srgb(0xE0533A),
    glow: Srgb(0x2E2E2E),
    sky: [
        Srgb(0x141414),
        Srgb(0x030303),
        Srgb(0x1C1C1C),
        Srgb(0x070707),
    ],
};

pub const PALETTES: [Palette; 12] = [
    PURPLE, BLUE, GREEN, YELLOW, RED, TEAL, INDIGO, PINK, ORANGE, WHITE, SILVER, BLACK,
];

pub fn palette(name: &str) -> Option<&'static Palette> {
    PALETTES
        .iter()
        .find(|palette| palette.name.eq_ignore_ascii_case(name))
}

pub fn palette_index(name: &str) -> Option<usize> {
    PALETTES
        .iter()
        .position(|palette| palette.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_palette_answers_every_role() {
        for palette in &PALETTES {
            for role in Role::ALL {
                let color = palette.color(role);
                assert_eq!(
                    color.linear().srgb(),
                    color,
                    "{} {}",
                    palette.name,
                    role.name()
                );
            }
        }
    }

    #[test]
    fn role_names_are_unique() {
        let mut names: Vec<&str> = Role::ALL.iter().map(|role| role.name()).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    #[test]
    fn a_palette_is_found_by_the_name_it_is_set_by() {
        assert_eq!(palette("Purple").map(|p| p.name), Some("Purple"));
        assert_eq!(palette("blue").map(|p| p.name), Some("Blue"));
        assert_eq!(palette_index("red"), Some(4));
        assert_eq!(palette_index("black"), Some(PALETTES.len() - 1));
        assert!(palette("mauve").is_none());
        assert_eq!(PALETTES[0].name, "Purple", "the default is the first");
    }

    #[test]
    fn a_rendered_palette_is_the_authored_one_in_light() {
        let purple = PURPLE.rendered();
        assert_eq!(purple.color(Role::Accent), PURPLE.accent.linear());
        assert_eq!(purple.mix(&purple, 0.5), purple);
        assert_eq!(purple.a(Role::Glass, 0.5)[3], 0.5);
    }

    #[test]
    fn a_blend_moves_the_whole_palette_together() {
        let half = PURPLE.rendered().mix(&BLUE.rendered(), 0.5);
        for role in Role::ALL {
            let from = PURPLE.color(role).linear().rgb();
            let to = BLUE.color(role).linear().rgb();
            let mixed = half.color(role).rgb();
            for channel in 0..3 {
                let want = from[channel] + (to[channel] - from[channel]) * 0.5;
                assert!(
                    (mixed[channel] - want).abs() < 1e-6,
                    "{} channel {channel}",
                    role.name()
                );
            }
        }
    }
}
