use std::path::{Path, PathBuf};

use crate::palette::{palette, Palette, PURPLE};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum IconStyle {
    #[default]
    Default = 0,

    Simple = 1,
}

impl IconStyle {
    pub const ALL: [Self; 2] = [Self::Default, Self::Simple];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Simple => "Simple",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        match name {
            "Default" => Some(Self::Default),
            "Simple" => Some(Self::Simple),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum WallpaperStyle {
    #[default]
    Default = 0,

    Simple = 1,

    Custom = 2,
}

impl WallpaperStyle {
    pub const ALL: [Self; 3] = [Self::Default, Self::Simple, Self::Custom];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Simple => "Simple",
            Self::Custom => "Custom",
        }
    }

    pub fn configured(name: &str) -> Option<Self> {
        match name {
            "Default" => Some(Self::Default),
            "Simple" => Some(Self::Simple),
            "Custom wallpaper" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellTheme {
    pub accent: &'static Palette,
    pub wallpaper: WallpaperStyle,
    pub icons: IconStyle,
}

impl Default for ShellTheme {
    fn default() -> Self {
        Self {
            accent: &PURPLE,
            wallpaper: WallpaperStyle::Default,
            icons: IconStyle::Default,
        }
    }
}

impl ShellTheme {
    pub fn load() -> Self {
        settings_path().map(Self::load_from).unwrap_or_default()
    }

    pub fn load_from(path: impl AsRef<Path>) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .map(|raw| Self::from_toml(&raw))
            .unwrap_or_default()
    }

    pub fn from_toml(raw: &str) -> Self {
        let accent = top_level_string(raw, "accent")
            .as_deref()
            .and_then(palette)
            .unwrap_or(&PURPLE);
        let modern_wallpaper = top_level_string(raw, "theme-wallpaper");
        let modern_icons = top_level_string(raw, "theme-icons");
        let legacy = top_level_string(raw, "theme");
        let wallpaper = modern_wallpaper
            .as_deref()
            .or(legacy.as_deref())
            .and_then(WallpaperStyle::configured)
            .unwrap_or_default();
        let icons = modern_icons
            .as_deref()
            .or(legacy.as_deref())
            .and_then(IconStyle::named)
            .unwrap_or_default();
        Self {
            accent,
            wallpaper,
            icons,
        }
    }
}

pub fn settings_path() -> Option<PathBuf> {
    settings_path_from(
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

fn settings_path_from(xdg: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    let config = xdg
        .filter(|path| path.is_absolute())
        .or_else(|| home.map(|home| home.join(".config")))?;
    Some(config.join("lxb").join("shell.toml"))
}

/// Which control the user last reached for, as the shell last wrote it down.
///
/// The shell watches for this — a stick or a button on the pad, any key on a
/// keyboard — and keeps it in the same file the accent and the icon style come
/// out of. An application built on this toolkit reads it so that what it says
/// about its own buttons agrees with what the shell says about the shell's.
///
/// `None` where there is no such file, which is what running under GNOME or
/// Plasma looks like: nothing has been written down, so nothing is claimed.
pub fn controller_in_hand() -> Option<bool> {
    controller_in_hand_from(&std::fs::read_to_string(settings_path()?).ok()?)
}

fn controller_in_hand_from(raw: &str) -> Option<bool> {
    match top_level_word(raw, "controller-in-hand")?.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// The keyboard arrangement the shell is set to, as an xkb layout and variant.
///
/// The answer somebody gave on Settings > Input > Keyboard > Keyboard layout,
/// out of the same file the accent and the icon style come from. One key and
/// not two — `keyboard-layout = "pl (qwertz)"` — because a layout and a variant
/// of it are two halves of one answer, spelled the way xkeyboard-config's own
/// registry spells the pair.
///
/// Read so that an application's on-screen board prints the letters the person
/// in front of it actually types. A session exports `XKB_DEFAULT_LAYOUT` for
/// the same purpose, and this is the better of the two where there is a choice:
/// a variable is copied into an application as it starts, so one left running
/// across a change to the setting holds the answer from before it, and a
/// desktop that is not this shell's compositor exports nothing at all.
///
/// `None` where there is no such file, or the key is not in it — a machine that
/// has never had this shell as much as one whose owner has never been asked.
pub fn keyboard_layout() -> Option<(String, String)> {
    keyboard_layout_from(&std::fs::read_to_string(settings_path()?).ok()?)
}

/// `layout (variant)`, or a bare layout where there is no variant.
///
/// Tolerant on both counts, because this key can be typed by hand: the bracket
/// may never be closed, and the spacing is not part of the answer. A key with
/// nothing in it is nothing rather than a layout named the empty string, which
/// is a keymap nothing can compile.
fn keyboard_layout_from(raw: &str) -> Option<(String, String)> {
    let key = top_level_string(raw, "keyboard-layout")?;
    let (layout, variant) = match key.trim().split_once('(') {
        Some((layout, rest)) => (layout.trim(), rest.trim_end().trim_end_matches(')').trim()),
        None => (key.trim(), ""),
    };
    (!layout.is_empty()).then(|| (layout.to_string(), variant.to_string()))
}

/// A bare top-level value: a number, a boolean, anything not in quotes.
fn top_level_word(raw: &str, wanted: &str) -> Option<String> {
    top_level_value(raw, wanted).map(|value| value.to_string())
}

fn top_level_string(raw: &str, wanted: &str) -> Option<String> {
    top_level_value(raw, wanted).and_then(toml_string)
}

/// The text after `wanted =`, at the top level of the file only.
///
/// TOML puts every key after a table header inside that table, so a key of the
/// same name under `[media-sort]` is a different key and is not this one.
fn top_level_value<'a>(raw: &'a str, wanted: &str) -> Option<&'a str> {
    let mut top_level = true;
    for line in raw.lines() {
        let line = without_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            top_level = false;
            continue;
        }
        if !top_level {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == wanted {
            return Some(value.trim());
        }
    }
    None
}

fn without_comment(line: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote == Some('"') && ch == '\\' {
            escaped = true;
            continue;
        }
        match (quote, ch) {
            (None, '"' | '\'') => quote = Some(ch),
            (Some(open), close) if open == close => quote = None,
            (None, '#') => return &line[..index],
            _ => {}
        }
    }
    line
}

fn toml_string(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.chars().next()?;
    if !matches!(quote, '"' | '\'') {
        return None;
    }
    let body = value.strip_prefix(quote)?.strip_suffix(quote)?;
    if quote == '"' && body.contains('\\') {
        return None;
    }
    Some(body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_control_is_in_hand_is_read_off_the_shell_s_own_file() {
        assert_eq!(
            controller_in_hand_from("accent = \"purple\"\ncontroller-in-hand = true\n"),
            Some(true)
        );
        assert_eq!(
            controller_in_hand_from("controller-in-hand = false  # watched, not chosen\n"),
            Some(false)
        );
        assert_eq!(controller_in_hand_from("accent = 'jade'\n"), None);
        // TOML puts everything after a header inside that table, so this one
        // is `media-sort.controller-in-hand` and says nothing about hands.
        assert_eq!(
            controller_in_hand_from("[media-sort]\ncontroller-in-hand = true\n"),
            None
        );
    }

    #[test]
    fn the_keyboard_the_shell_was_set_to_is_read_off_the_same_file() {
        assert_eq!(
            keyboard_layout_from("accent = \"purple\"\nkeyboard-layout = \"pl (qwertz)\"\n"),
            Some(("pl".to_string(), "qwertz".to_string()))
        );
        // A layout with no variant is written as the bare layout.
        assert_eq!(
            keyboard_layout_from("keyboard-layout = 'us'\n"),
            Some(("us".to_string(), String::new()))
        );
        // Typed by hand, and still an answer.
        assert_eq!(
            keyboard_layout_from("keyboard-layout = \"  de  ( neo  \"\n"),
            Some(("de".to_string(), "neo".to_string()))
        );
        // Not an answer: nothing to compile, and a table's key is not this one.
        assert_eq!(keyboard_layout_from("keyboard-layout = \"\"\n"), None);
        assert_eq!(
            keyboard_layout_from("[display.TEST]\nkeyboard-layout = \"pl\"\n"),
            None
        );
        assert_eq!(keyboard_layout_from("accent = 'jade'\n"), None);
    }

    #[test]
    fn defaults_are_safe_without_a_file() {
        assert_eq!(ShellTheme::from_toml(""), ShellTheme::default());
        assert_eq!(
            ShellTheme::load_from("/no/such/lxb/shell.toml"),
            ShellTheme::default()
        );
    }

    #[test]
    fn current_keys_choose_the_application_theme() {
        let theme = ShellTheme::from_toml(
            r#"
                accent = "Blue"
                theme-wallpaper = "Custom wallpaper"
                theme-icons = "Simple" # the mark material
                theme = "Default"
            "#,
        );
        assert_eq!(theme.accent.name, "Blue");
        assert_eq!(theme.wallpaper, WallpaperStyle::Custom);
        assert_eq!(theme.icons, IconStyle::Simple);
    }

    #[test]
    fn the_old_whole_shell_key_is_both_halves_fallback() {
        let theme = ShellTheme::from_toml("accent = 'Green'\ntheme = 'Simple'\n");
        assert_eq!(theme.accent.name, "Green");
        assert_eq!(theme.wallpaper, WallpaperStyle::Simple);
        assert_eq!(theme.icons, IconStyle::Simple);
    }

    #[test]
    fn an_unknown_current_value_is_default_not_legacy() {
        let theme = ShellTheme::from_toml(
            "accent = \"Chartreuse\"\ntheme-wallpaper = \"Future\"\n\
             theme-icons = \"Future\"\ntheme = \"Simple\"\n",
        );
        assert_eq!(theme, ShellTheme::default());
    }

    #[test]
    fn values_in_a_table_cannot_replace_the_shell_wide_values() {
        let theme = ShellTheme::from_toml(
            "accent = \"Red\"\n[display.HDMI]\naccent = \"Blue\"\ntheme-icons = \"Simple\"\n",
        );
        assert_eq!(theme.accent.name, "Red");
        assert_eq!(theme.wallpaper, WallpaperStyle::Default);
        assert_eq!(theme.icons, IconStyle::Default);
    }

    #[test]
    fn the_shell_path_obeys_xdg_and_falls_back_to_home() {
        assert_eq!(
            settings_path_from(
                Some(PathBuf::from("/tmp/config")),
                Some(PathBuf::from("/home/a"))
            ),
            Some(PathBuf::from("/tmp/config/lxb/shell.toml"))
        );
        assert_eq!(
            settings_path_from(
                Some(PathBuf::from("relative")),
                Some(PathBuf::from("/home/a"))
            ),
            Some(PathBuf::from("/home/a/.config/lxb/shell.toml"))
        );
        assert_eq!(settings_path_from(None, None), None);
    }
}
