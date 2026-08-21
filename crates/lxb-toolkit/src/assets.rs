macro_rules! glyphs {
    ($($name:literal),+ $(,)?) => {
        &[
            $(
                (
                    $name,
                    include_bytes!(concat!("../assets/glyphs/", $name, ".svg")) as &[u8],
                ),
            )+
        ]
    };
}

pub const GLYPHS: &[(&str, &[u8])] = glyphs![
    "volume",
    "volume-muted",
    "brightness",
    "media-volume",
    "media-previous",
    "media-play",
    "media-pause",
    "media-next",
    "pointer-stick",
    "volume-mixer",
    "do-not-disturb",
    "notifications",
    "pad-select",
    "pad-west",
    "arrow-left",
    "arrow-down",
    "arrow-up",
    "arrow-right",
    "keyboard-hide",
    "shutdown",
    "category-settings",
    "category-system",
    "category-multimedia",
    "category-graphics",
    "category-internet",
    "category-office",
    "category-games",
    "category-development",
    "category-education",
    "category-utilities",
    "category-other",
    "category-music",
    "category-video",
    "category-images",
    "category-files",
    "file-folder",
    "file-page",
    "file-drive",
    "file-home",
    "setting-appearance",
    "setting-accent",
    "setting-theme",
    "setting-wallpaper",
    "setting-icons",
    "setting-display",
    "setting-resolution",
    "setting-refresh",
    "setting-orientation",
    "setting-rotation-0",
    "setting-rotation-90",
    "setting-rotation-180",
    "setting-rotation-270",
    "setting-order",
    "setting-night-light",
    "setting-schedule",
    "setting-hdr",
    "setting-screen-rest",
    "setting-microphone",
    "setting-network",
    "setting-address",
    "setting-name-server",
    "setting-typed",
    "setting-connect",
    "setting-disconnect",
    "setting-wifi",
    "setting-ethernet",
    "signal-weak",
    "signal-fair",
    "signal-strong",
    "battery-empty",
    "battery-low",
    "battery-half",
    "battery-high",
    "battery-full",
    "battery-charging",
    "setting-bluetooth",
    "setting-system",
    "setting-scale",
    "setting-info",
    "swatch",
    "chosen",
    "uninstall",
    "launch",
    "open-with",
    "sort",
    "copy",
    "move",
    "paste",
    "rename",
    "screenshot",
    "search",
    "search-clear",
    "authenticate",
    "category-steam",
    "steam",
    "refresh",
    "sign-out",
    "logo",
];

pub const SOUNDS: &[(&str, &[u8])] = &[
    (
        "app-launch",
        include_bytes!("../assets/sounds/app-launch.ogg"),
    ),
    ("error", include_bytes!("../assets/sounds/error.ogg")),
    (
        "guide-open",
        include_bytes!("../assets/sounds/guide-open.ogg"),
    ),
    (
        "keyboard-click",
        include_bytes!("../assets/sounds/keyboard-click.ogg"),
    ),
    (
        "notification",
        include_bytes!("../assets/sounds/notification.ogg"),
    ),
    ("polkit", include_bytes!("../assets/sounds/polkit.ogg")),
    (
        "press-back",
        include_bytes!("../assets/sounds/press-back.ogg"),
    ),
    (
        "press-guide",
        include_bytes!("../assets/sounds/press-guide.ogg"),
    ),
    (
        "press-guide-selected",
        include_bytes!("../assets/sounds/press-guide-selected.ogg"),
    ),
    ("press", include_bytes!("../assets/sounds/press.ogg")),
    (
        "press-selected",
        include_bytes!("../assets/sounds/press-selected.ogg"),
    ),
    (
        "screenshot",
        include_bytes!("../assets/sounds/screenshot.ogg"),
    ),
    (
        "start-bg-music",
        include_bytes!("../assets/sounds/start-bg-music.ogg"),
    ),
    ("trash", include_bytes!("../assets/sounds/trash.ogg")),
];

pub const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/Roboto-Regular.ttf");
pub const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/Roboto-Bold.ttf");
pub const FONT_LICENSE: &[u8] = include_bytes!("../assets/fonts/LICENSE-Roboto.txt");

pub const GLASS_WGSL: &[u8] = include_bytes!("../assets/shaders/lxb_glass.wgsl");

pub const GLYPH_WGSL: &[u8] = include_bytes!("../assets/shaders/lxb_glyph.wgsl");

pub const WALLPAPER_WGSL: &[u8] = include_bytes!("../assets/shaders/lxb_wallpaper.wgsl");

pub fn glyph(name: &str) -> Option<&'static [u8]> {
    GLYPHS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, bytes)| *bytes)
}

pub fn sound(name: &str) -> Option<&'static [u8]> {
    SOUNDS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, bytes)| *bytes)
}

pub fn glyph_box(name: &str) -> Option<u32> {
    let bytes = glyph(name)?;
    let source = std::str::from_utf8(bytes).ok()?;
    if source.contains(r#"viewBox="0 0 24 24""#) {
        Some(24)
    } else if source.contains(r#"viewBox="0 0 32 32""#) {
        Some(32)
    } else {
        None
    }
}

pub fn glyph_names() -> impl Iterator<Item = &'static str> {
    GLYPHS.iter().map(|(name, _)| *name)
}

pub fn sound_names() -> impl Iterator<Item = &'static str> {
    SOUNDS.iter().map(|(name, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_asset_is_really_here() {
        assert_eq!(GLYPHS.len(), 98, "the mark set changed size");
        for (name, bytes) in GLYPHS {
            assert!(bytes.len() > 200, "{name} is {} bytes", bytes.len());
            let head = std::str::from_utf8(&bytes[..bytes.len().min(400)]).unwrap_or("");
            assert!(head.contains("<svg") || head.contains("<!--"), "{name}");
        }
        for (name, bytes) in SOUNDS {
            assert!(bytes.len() > 1000, "{name} is {} bytes", bytes.len());
            assert_eq!(&bytes[..4], b"OggS", "{name} is not Ogg");
        }
        assert_eq!(&FONT_REGULAR[..4], &[0x00, 0x01, 0x00, 0x00], "not a TTF");
        assert_eq!(&FONT_BOLD[..4], &[0x00, 0x01, 0x00, 0x00], "not a TTF");
        assert!(FONT_LICENSE.starts_with(b"Apache License"));
        assert!(GLASS_WGSL.len() > 4000);
        assert!(GLYPH_WGSL.len() > 4000);
        assert!(WALLPAPER_WGSL.len() > 8000);
    }

    #[test]
    fn every_mark_is_square_and_says_which_box_it_is_in() {
        for (name, bytes) in GLYPHS {
            let source = std::str::from_utf8(bytes).expect("glyphs are text");
            let box_of = glyph_box(name).expect("every mark is listed");
            assert!(
                source.contains(&format!(r#"viewBox="0 0 {box_of} {box_of}""#)),
                "{name} is not in the {box_of}-unit box it is listed under"
            );
            assert!(
                !source.contains("<text"),
                "{name} needs a font to draw, which a renderer may not have"
            );
        }

        let small = GLYPHS
            .iter()
            .filter(|(name, _)| glyph_box(name) == Some(24))
            .count();
        assert_eq!(small, 10);
    }

    #[test]
    fn every_shell_shape_is_registered_and_neutral() {
        const EXPECTED: &[&str] = &[
            "volume",
            "volume-muted",
            "brightness",
            "media-volume",
            "media-previous",
            "media-play",
            "media-pause",
            "media-next",
            "pointer-stick",
            "volume-mixer",
            "do-not-disturb",
            "notifications",
            "pad-select",
            "pad-west",
            "arrow-left",
            "arrow-down",
            "arrow-up",
            "arrow-right",
            "keyboard-hide",
            "shutdown",
            "category-settings",
            "category-system",
            "category-multimedia",
            "category-graphics",
            "category-internet",
            "category-office",
            "category-games",
            "category-development",
            "category-education",
            "category-utilities",
            "category-other",
            "category-music",
            "category-video",
            "category-images",
            "category-files",
            "file-folder",
            "file-page",
            "file-drive",
            "file-home",
            "setting-appearance",
            "setting-accent",
            "setting-theme",
            "setting-wallpaper",
            "setting-icons",
            "setting-display",
            "setting-resolution",
            "setting-refresh",
            "setting-orientation",
            "setting-rotation-0",
            "setting-rotation-90",
            "setting-rotation-180",
            "setting-rotation-270",
            "setting-order",
            "setting-night-light",
            "setting-schedule",
            "setting-hdr",
            "setting-screen-rest",
            "setting-microphone",
            "setting-network",
            "setting-address",
            "setting-name-server",
            "setting-typed",
            "setting-connect",
            "setting-disconnect",
            "setting-wifi",
            "setting-ethernet",
            "signal-weak",
            "signal-fair",
            "signal-strong",
            "battery-empty",
            "battery-low",
            "battery-half",
            "battery-high",
            "battery-full",
            "battery-charging",
            "setting-bluetooth",
            "setting-system",
            "setting-scale",
            "setting-info",
            "swatch",
            "chosen",
            "uninstall",
            "launch",
            "open-with",
            "sort",
            "copy",
            "move",
            "paste",
            "rename",
            "screenshot",
            "search",
            "search-clear",
            "authenticate",
            "category-steam",
            "steam",
            "refresh",
            "sign-out",
            "logo",
        ];

        assert_eq!(glyph_names().collect::<Vec<_>>(), EXPECTED);

        for (name, bytes) in GLYPHS {
            assert!(
                crate::glyph_material::is_shape(bytes),
                "{name} is missing the lxb:shape marker"
            );
            let source = std::str::from_utf8(bytes).expect("glyphs are text");
            for forbidden in [
                "<linearGradient",
                "<radialGradient",
                "<filter",
                "stop-color",
                "flood-color",
                "rgb(",
                "hsl(",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{name} carries authored material through {forbidden}"
                );
            }
            for attribute in ["fill=\"", "stroke=\""] {
                let mut rest = source;
                while let Some(index) = rest.find(attribute) {
                    rest = &rest[index + attribute.len()..];
                    let value = rest.split_once('"').expect("closed SVG attribute").0;
                    assert!(
                        matches!(value, "none" | "#fff" | "#ffffff" | "#000000"),
                        "{name} has non-neutral {attribute}{value}"
                    );
                }
            }
        }
    }

    #[test]
    fn asset_names_are_spellable_from_any_language() {
        for name in glyph_names().chain(sound_names()) {
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{name}"
            );
        }
        let mut names: Vec<&str> = glyph_names().collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "two marks share a name");
    }

    #[test]
    fn an_unknown_name_is_refused_rather_than_guessed_at() {
        assert!(glyph("launch").is_some());
        assert!(glyph("Launch").is_none());
        assert!(glyph("no-such-mark").is_none());
        assert!(sound("press").is_some());
        assert!(sound("no-such-sound").is_none());
    }
}
