use crate::material::{Glass, Surface, GLOSS_QUIET};
use crate::palette::Role;

pub const CHIP_ROLE: Role = Role::GlassRaised;
pub const CHIP_TINT: f32 = 0.10;

pub const PADDING: f32 = 5.0;

pub const ASIDE_TINT: f32 = 0.16;

pub const LIT_ROLE: Role = Role::Accent;
pub const LIT: f32 = 0.46;
pub const LIT_ANSWER: f32 = 0.50;
pub const LIT_PULSE: f32 = 0.05;

pub const GLOW: f32 = 0.13;
pub const GLOW_PULSE: f32 = 0.05;

pub const GLOW_WIDTH: f32 = 1.24;

pub const GLOW_HEIGHT: f32 = 2.6;

pub const OUT_ROLE: Role = Role::TextSoft;
pub const OUT: f32 = 0.22;
pub const OUT_WIDTH: f32 = 1.5;

pub const INK: f32 = 1.0;
pub const INK_QUIET: f32 = 0.84;

pub const MARK: f32 = 1.0;
pub const MARK_QUIET: f32 = 0.82;

pub const fn chip() -> Glass {
    Glass {
        gloss: GLOSS_QUIET,
        ..Surface::Control.glass()
    }
}

pub const fn lit() -> Glass {
    Surface::Control.glass()
}

pub fn lit_alpha(seconds: f32) -> f32 {
    LIT + LIT_PULSE * crate::motion::pulse(seconds)
}

pub fn answer_alpha(seconds: f32) -> f32 {
    LIT_ANSWER + LIT_PULSE * crate::motion::pulse(seconds)
}

pub fn glow_alpha(seconds: f32) -> f32 {
    GLOW + GLOW_PULSE * crate::motion::pulse(seconds)
}

pub fn glow_rect(rect: [f32; 4], over: f32) -> [f32; 4] {
    glow_rect_tall(rect, over, rect[3] * GLOW_HEIGHT)
}

pub fn glow_rect_tall([x, y, w, h]: [f32; 4], over: f32, tall: f32) -> [f32; 4] {
    let wide = over * GLOW_WIDTH;
    [
        x + w * 0.5 - wide * 0.5,
        y + h * 0.5 - tall * 0.5,
        wide,
        tall,
    ]
}

pub fn arrival(light: [f32; 4], rect: [f32; 4]) -> f32 {
    let (w, h) = (rect[2].max(1.0), rect[3].max(1.0));
    let apart = (light[0] - rect[0]).abs() / w
        + (light[1] - rect[1]).abs() / h
        + (light[2] - rect[2]).abs() / w
        + (light[3] - rect[3]).abs() / h;
    1.0 - apart.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_and_unlit_are_one_slab_at_two_glosses() {
        let (quiet, full) = (chip(), lit());
        assert_eq!(quiet.depth, full.depth);
        assert_eq!(quiet.frost, full.frost);
        assert_eq!(quiet.curve, full.curve);
        assert!(quiet.gloss < full.gloss);
        assert_eq!(full, Surface::Control.glass());
    }

    #[test]
    fn the_light_arrives_only_when_it_is_over_the_control() {
        let rect = [100.0, 200.0, 300.0, 60.0];
        assert_eq!(arrival(rect, rect), 1.0);

        assert_eq!(arrival([100.0, 260.0, 300.0, 60.0], rect), 0.0);
        assert_eq!(arrival([100.0, 140.0, 300.0, 60.0], rect), 0.0);
        assert_eq!(arrival([400.0, 200.0, 300.0, 60.0], rect), 0.0);

        assert!((arrival([100.0, 230.0, 300.0, 60.0], rect) - 0.5).abs() < 1e-5);

        assert_eq!(arrival([100.0, 200.0, 900.0, 60.0], rect), 0.0);
    }

    #[test]
    fn the_halo_is_the_surfaces_width_and_the_controls_centre() {
        let short = glow_rect([40.0, 100.0, 200.0, 40.0], 300.0);
        let tall = glow_rect([40.0, 100.0, 200.0, 80.0], 300.0);
        assert_eq!(short[2], 300.0 * GLOW_WIDTH);
        assert_eq!(tall[2], short[2]);
        assert!((short[0] - tall[0]).abs() < 1e-6);

        for (glow, rect) in [
            (short, [40.0, 100.0, 200.0, 40.0]),
            (tall, [40.0, 100.0, 200.0, 80.0]),
        ] {
            assert!((glow[0] + glow[2] / 2.0 - (rect[0] + rect[2] / 2.0)).abs() < 1e-4);
            assert!((glow[1] + glow[3] / 2.0 - (rect[1] + rect[3] / 2.0)).abs() < 1e-4);
        }
    }

    #[test]
    fn the_pulse_never_takes_a_light_out_of_range() {
        for n in 0..120 {
            let at = n as f32 * crate::motion::duration::PULSE / 120.0;
            for alpha in [lit_alpha(at), answer_alpha(at), glow_alpha(at)] {
                assert!((0.0..=1.0).contains(&alpha), "{alpha}");
            }
            assert!(lit_alpha(at) >= LIT && lit_alpha(at) <= LIT + LIT_PULSE);
            assert!(glow_alpha(at) >= GLOW && glow_alpha(at) <= GLOW + GLOW_PULSE);
        }
    }

    const _: () = {
        assert!(
            LIT > CHIP_TINT,
            "the light is dimmer than the face under it"
        );
        assert!(LIT_ANSWER > LIT, "an answer is quieter than a menu row");
        assert!(INK > INK_QUIET, "the lit label is the dimmer of the two");
        assert!(MARK > MARK_QUIET, "the lit mark is the dimmer of the two");
        assert!(
            ASIDE_TINT > CHIP_TINT,
            "a row's button is quieter than its row"
        );
        assert!(GLOW < LIT, "the halo outshines the capsule over it");
    };
}
