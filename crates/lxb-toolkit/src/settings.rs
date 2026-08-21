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

fn top_level_string(raw: &str, wanted: &str) -> Option<String> {
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
            return toml_string(value.trim());
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
