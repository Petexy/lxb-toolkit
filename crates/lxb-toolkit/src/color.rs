#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Srgb(pub u32);

impl Srgb {
    pub const fn bytes(self) -> [u8; 3] {
        [
            (self.0 >> 16 & 0xff) as u8,
            (self.0 >> 8 & 0xff) as u8,
            (self.0 & 0xff) as u8,
        ]
    }

    pub fn hex(self) -> String {
        format!("#{:06x}", self.0 & 0xff_ffff)
    }

    pub fn linear(self) -> Linear {
        let [r, g, b] = self.bytes();
        Linear([
            to_linear(r as f32 / 255.0),
            to_linear(g as f32 / 255.0),
            to_linear(b as f32 / 255.0),
        ])
    }

    pub fn a(self, alpha: f32) -> [f32; 4] {
        self.linear().a(alpha)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linear(pub [f32; 3]);

impl Linear {
    pub const fn rgb(self) -> [f32; 3] {
        self.0
    }

    pub fn a(self, alpha: f32) -> [f32; 4] {
        let [r, g, b] = self.0;
        [r, g, b, alpha]
    }

    pub fn mix(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        let [ar, ag, ab] = self.0;
        let [br, bg, bb] = other.0;
        Self([
            ar + (br - ar) * amount,
            ag + (bg - ag) * amount,
            ab + (bb - ab) * amount,
        ])
    }

    pub fn srgb(self) -> Srgb {
        let [r, g, b] = self.0;
        let channel = |v: f32| (to_srgb(v).clamp(0.0, 1.0) * 255.0).round() as u32;
        Srgb(channel(r) << 16 | channel(g) << 8 | channel(b))
    }
}

pub fn to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

pub fn to_srgb(light: f32) -> f32 {
    if light <= 0.003_130_8 {
        light * 12.92
    } else {
        1.055 * light.powf(1.0 / 2.4) - 0.055
    }
}

pub fn luminance(color: Linear) -> f32 {
    let [r, g, b] = color.0;
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_survives_the_journey_to_light_and_back() {
        for hex in [0x000000, 0xffffff, 0x8b5cf6, 0x140b26, 0xe0533a] {
            assert_eq!(Srgb(hex).linear().srgb(), Srgb(hex), "{hex:#08x}");
        }
        assert_eq!(Srgb(0x000000).linear().rgb(), [0.0; 3]);
        assert_eq!(Srgb(0xffffff).linear().rgb(), [1.0; 3]);
    }

    #[test]
    fn a_middle_grey_is_a_fifth_of_the_light() {
        let [light, _, _] = Srgb(0x808080).linear().rgb();
        assert!(
            (light - 0.2158).abs() < 0.001,
            "sRGB 0x80 is {light} of the light"
        );
    }

    #[test]
    fn a_blend_runs_through_light_rather_than_through_hex() {
        let half = Srgb(0x000000).linear().mix(Srgb(0xffffff).linear(), 0.5);
        assert_eq!(half.rgb(), [0.5; 3]);
        assert_eq!(half.srgb(), Srgb(0xbcbcbc));
    }

    #[test]
    fn a_colour_prints_as_the_hex_it_was_authored_as() {
        assert_eq!(Srgb(0x8b5cf6).hex(), "#8b5cf6");
        assert_eq!(Srgb(0x000000).hex(), "#000000");
        assert_eq!(Srgb(0x8b5cf6).bytes(), [0x8b, 0x5c, 0xf6]);
    }
}
