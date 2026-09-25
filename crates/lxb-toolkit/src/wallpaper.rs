use crate::{
    palette::{Palette, Role},
    settings::WallpaperStyle,
};

pub const VISUAL: &str = "lxb-wallpaper-v6";

pub const STYLE_DEFAULT: f32 = 0.0;

pub const STYLE_SIMPLE: f32 = 1.0;

pub const PARTICLES_OFF: f32 = 0.0;

pub const PARTICLES_ON: f32 = 1.0;

pub const KEY_LIGHT: [f32; 3] = [-0.42, -0.66, 0.62];

pub const BAND_FOLD: f32 = 0.020;

pub const BAND_BOW: f32 = 0.50;

pub const BAND_SHARE: f32 = 0.88;

pub const BAND_EDGE_SAMPLES: f32 = 0.8;

pub const SOFTEN: f32 = 0.34;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaderPalette {
    pub sky: [[f32; 4]; 4],
    pub accent: [[f32; 4]; 3],
    pub glow: [f32; 4],
}

impl ShaderPalette {
    pub fn from_palette(palette: &Palette) -> Self {
        let rendered = palette.rendered();
        Self {
            sky: [
                rendered.a(Role::SkyTop, 1.0),
                rendered.a(Role::SkyBottom, 1.0),
                rendered.a(Role::SkyTopAlt, 1.0),
                rendered.a(Role::SkyBottomAlt, 1.0),
            ],
            accent: [
                rendered.a(Role::Accent, 1.0),
                rendered.a(Role::AccentSoft, 1.0),
                rendered.a(Role::AccentDeep, 1.0),
            ],
            glow: rendered.a(Role::Glow, 1.0),
        }
    }
}

pub const fn shader_style(style: WallpaperStyle) -> f32 {
    match style {
        WallpaperStyle::Simple => STYLE_SIMPLE,
        WallpaperStyle::Default | WallpaperStyle::Custom => STYLE_DEFAULT,
    }
}

pub const fn shader_particles(particles: bool) -> f32 {
    if particles {
        PARTICLES_ON
    } else {
        PARTICLES_OFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::PURPLE;

    #[test]
    fn custom_is_an_analytic_default() {
        assert_eq!(shader_style(WallpaperStyle::Default), STYLE_DEFAULT);
        assert_eq!(shader_style(WallpaperStyle::Simple), STYLE_SIMPLE);
        assert_eq!(shader_style(WallpaperStyle::Custom), STYLE_DEFAULT);
    }

    #[test]
    fn particles_are_one_where_drawn_and_nought_where_not() {
        assert_eq!(shader_particles(true), PARTICLES_ON);
        assert_eq!(shader_particles(false), PARTICLES_OFF);
        assert_eq!(PARTICLES_OFF, 0.0);
    }

    #[test]
    fn shader_palette_is_linear_and_in_semantic_order() {
        let packed = ShaderPalette::from_palette(&PURPLE);
        let rendered = PURPLE.rendered();
        assert_eq!(packed.sky[0], rendered.a(Role::SkyTop, 1.0));
        assert_eq!(packed.sky[3], rendered.a(Role::SkyBottomAlt, 1.0));
        assert_eq!(packed.accent[0], rendered.a(Role::Accent, 1.0));
        assert_eq!(packed.accent[2], rendered.a(Role::AccentDeep, 1.0));
        assert_eq!(packed.glow, rendered.a(Role::Glow, 1.0));
    }
}
