use naga::valid::{Capabilities, ValidationFlags, Validator};

fn source(asset: &'static [u8]) -> &'static str {
    std::str::from_utf8(asset).expect("the shader is text")
}

fn assert_shader(name: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|err| {
        panic!(
            "the shipped {name} does not parse:\n{}",
            err.emit_to_string(source)
        )
    });
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator
        .validate(&module)
        .unwrap_or_else(|err| panic!("the shipped {name} does not validate: {err:?}"));
}

#[test]
fn the_glass_is_a_shader() {
    assert_shader("glass", source(lxb_toolkit::assets::GLASS_WGSL));
}

#[test]
fn the_glyph_material_is_a_shader() {
    assert_shader("glyph material", source(lxb_toolkit::assets::GLYPH_WGSL));
}

#[test]
fn the_wallpaper_is_a_shader() {
    assert_shader("wallpaper", source(lxb_toolkit::assets::WALLPAPER_WGSL));
}

#[test]
fn it_brings_nothing_of_its_own_to_collide_with() {
    for (name, source) in [
        ("glass", source(lxb_toolkit::assets::GLASS_WGSL)),
        ("glyph material", source(lxb_toolkit::assets::GLYPH_WGSL)),
        ("wallpaper", source(lxb_toolkit::assets::WALLPAPER_WGSL)),
    ] {
        for forbidden in [
            "@group",
            "@binding",
            "@vertex",
            "@fragment",
            "@compute",
            "var<uniform>",
        ] {
            assert!(
                !source.contains(forbidden),
                "the {name} declares {forbidden}, which is a consumer's to declare"
            );
        }

        for line in source.lines() {
            let line = line.trim_start();
            for keyword in ["fn ", "const ", "struct "] {
                if let Some(rest) = line.strip_prefix(keyword) {
                    let declaration = rest.split(['(', ':', ' ', '{']).next().unwrap_or("");
                    assert!(
                        declaration.starts_with("lxb_")
                            || declaration.starts_with("Lxb")
                            || declaration.starts_with("LXB_"),
                        "{declaration} in {name} lacks the prefix that keeps it out of the way"
                    );
                }
            }
        }
    }
}

#[test]
fn the_shader_and_the_library_agree_about_the_glass() {
    use lxb_toolkit::material::optics;
    let source = source(lxb_toolkit::assets::GLASS_WGSL);
    let declares = |name: &str, value: f32| {
        let needle = format!("{name}: f32 = {value}");
        assert!(
            source.contains(&needle),
            "the shader does not say {needle}; the library does"
        );
    };
    declares("LXB_GLASS_IOR", optics::IOR);
    declares("LXB_GLASS_DISPERSION", optics::DISPERSION);
    declares("LXB_GLASS_FLOAT", optics::FLOAT);
    declares("LXB_BLUR_LEVELS", optics::BLUR_LEVELS);
    declares("LXB_MAX_SLAB_SHARE", optics::MAX_SLAB_SHARE);

    let [x, y, z] = optics::KEY_LIGHT;
    assert!(
        source.contains(&format!("vec3<f32>({x}, {y}, {z})")),
        "the lamp in the shader is not the lamp in the library"
    );
    let [r, g, b] = optics::FROST_SCATTER;
    assert!(
        source.contains(&format!("vec3<f32>({r:.3}, {g:.3}, {b:.3})")),
        "the frost in the shader is not the frost in the library"
    );
}

#[test]
fn the_shader_and_the_library_agree_about_glyphs() {
    use lxb_toolkit::{glyph_material as glyph, material::optics};

    let source = source(lxb_toolkit::assets::GLYPH_WGSL);
    let declares = |name: &str, value: f32| {
        let needle = format!("{name}: f32 = {value}");
        assert!(
            source.contains(&needle),
            "the shader does not say {needle}; the library does"
        );
    };
    declares("LXB_GLYPH_SDF_RANGE", glyph::SDF_RANGE);
    declares("LXB_GLYPH_DEPTH_SHARE", glyph::DEPTH_SHARE);
    declares("LXB_GLYPH_SHADOW", glyph::SHADOW);
    declares("LXB_GLYPH_SIMPLE_TINT", glyph::SIMPLE_TINT);
    declares("LXB_GLYPH_SIMPLE_ALPHA", glyph::SIMPLE_ALPHA);
    declares("LXB_GLYPH_SIMPLE_STAIN", glyph::SIMPLE_STAIN);
    declares("LXB_GLYPH_GRADIENT_ARM", glyph::GRADIENT_ARM);
    declares("LXB_GLYPH_COVERAGE_FEATHER", glyph::COVERAGE_FEATHER);
    declares("LXB_GLYPH_MIN_DEPTH", glyph::MIN_DEPTH);
    declares("LXB_GLYPH_RIDGE_BEGIN", glyph::RIDGE_BEGIN);
    declares("LXB_GLYPH_RIDGE_END", glyph::RIDGE_END);
    declares("LXB_GLYPH_SHADOW_OFFSET", glyph::SHADOW_OFFSET);
    declares("LXB_GLYPH_DISPERSION", optics::DISPERSION);

    assert!(source.contains(&format!("LXB_GLYPH_CELL: f32 = {}.0", glyph::CELL)));
    let [x, y, z] = glyph::LAMP;
    assert!(
        source.contains(&format!("vec3<f32>({x}, {y}, {z})")),
        "the glyph lamp in the shader is not the lamp in the library"
    );
    assert!(source.contains("fn lxb_glyph_simple("));
    assert!(source.contains("fn lxb_glyph_default("));
    assert!(source.contains("fn lxb_glyph_material("));
}

#[test]
fn the_shader_and_the_library_agree_about_the_wallpaper() {
    use lxb_toolkit::{material::optics, wallpaper};

    let source = source(lxb_toolkit::assets::WALLPAPER_WGSL);
    let declares = |name: &str, value: f32| {
        let needle = format!("{name}: f32 = {value}");
        assert!(
            source.contains(&needle),
            "the shader does not say {needle}; the library does"
        );
    };
    declares("LXB_WALLPAPER_BAND_FOLD", wallpaper::BAND_FOLD);
    declares("LXB_WALLPAPER_BAND_BOW", wallpaper::BAND_BOW);
    declares("LXB_WALLPAPER_BAND_SHARE", wallpaper::BAND_SHARE);
    declares(
        "LXB_WALLPAPER_BAND_EDGE_SAMPLES",
        wallpaper::BAND_EDGE_SAMPLES,
    );
    declares("LXB_WALLPAPER_DISPERSION", optics::DISPERSION);

    let [x, y, z] = wallpaper::KEY_LIGHT;
    assert!(source.contains(&format!("vec3<f32>({x}, {y}, {z})")));
    assert!(source.contains("fn lxb_wallpaper("));
    assert!(source.contains("style > 0.5 && style < 1.5"));
}
