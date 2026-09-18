#![allow(clippy::missing_safety_doc)]
use std::ffi::{c_char, c_float, c_int, c_uint, c_void, CStr, CString};
use std::os::raw::c_ulong;
use std::sync::OnceLock;

use lxb_toolkit::accent::Accent;
use lxb_toolkit::input::{Action, Button, Key};
use lxb_toolkit::material::{Overlay, OverlayMaterial, Surface};
use lxb_toolkit::menu::Menu;
use lxb_toolkit::metrics::Metric;
use lxb_toolkit::palette::{Role, PALETTES};
use lxb_toolkit::picker::{EntryKind, Picker, Selection};
use lxb_toolkit::settings::{IconStyle, ShellTheme, WallpaperStyle};
use lxb_toolkit::sound::Sound;
use lxb_toolkit::typography::{Face, Text};

type Size = c_ulong;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbRgba {
    pub r: c_float,
    pub g: c_float,
    pub b: c_float,
    pub a: c_float,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbGlass {
    pub depth: c_float,
    pub frost: c_float,
    pub gloss: c_float,
    pub curve: c_float,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbOverlayMaterial {
    pub radius: c_float,
    pub glass: LxbGlass,
    pub stain: c_float,
    pub light_inset: c_float,
    pub header_x: c_float,
    pub header_width: c_float,
    pub header_height: c_float,
    pub header_height_share: c_float,
    pub header_light: c_float,
    pub foot_x: c_float,
    pub foot_width: c_float,
    pub foot_height: c_float,
    pub foot_height_share: c_float,
    pub foot_light: c_float,
    pub rim: c_float,
    pub rim_width: c_float,

    pub stain_role: Size,
    pub header_role: Size,
    pub foot_role: Size,
    pub rim_role: Size,
}

fn role_index(role: Role) -> Size {
    Role::ALL
        .iter()
        .position(|candidate| *candidate == role)
        .unwrap_or(0) as Size
}

impl From<OverlayMaterial> for LxbOverlayMaterial {
    fn from(material: OverlayMaterial) -> Self {
        Self {
            radius: material.radius,
            glass: LxbGlass {
                depth: material.glass.depth,
                frost: material.glass.frost,
                gloss: material.glass.gloss,
                curve: material.glass.curve,
            },
            stain: material.stain,
            light_inset: material.light_inset,
            header_x: material.header_x,
            header_width: material.header_width,
            header_height: material.header_height,
            header_height_share: material.header_height_share,
            header_light: material.header_light,
            foot_x: material.foot_x,
            foot_width: material.foot_width,
            foot_height: material.foot_height,
            foot_height_share: material.foot_height_share,
            foot_light: material.foot_light,
            rim: material.rim,
            rim_width: material.rim_width,
            stain_role: role_index(material.stain_role),
            header_role: role_index(material.header_role),
            foot_role: role_index(material.foot_role),
            rim_role: role_index(material.rim_role),
        }
    }
}

impl LxbOverlayMaterial {
    const NONE: Self = Self {
        radius: 0.0,
        glass: LxbGlass {
            depth: 0.0,
            frost: 0.0,
            gloss: 0.0,
            curve: 0.0,
        },
        stain: 0.0,
        light_inset: 0.0,
        header_x: 0.0,
        header_width: 0.0,
        header_height: 0.0,
        header_height_share: 0.0,
        header_light: 0.0,
        foot_x: 0.0,
        foot_width: 0.0,
        foot_height: 0.0,
        foot_height_share: 0.0,
        foot_light: 0.0,
        rim: 0.0,
        rim_width: 0.0,
        stain_role: 0,
        header_role: 0,
        foot_role: 0,
        rim_role: 0,
    };
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbControl {
    pub chip_role: c_uint,
    pub chip_tint: c_float,
    pub chip_gloss: c_float,
    pub aside_tint: c_float,
    pub padding: c_float,
    pub lit_role: c_uint,
    pub lit: c_float,
    pub lit_answer: c_float,
    pub lit_pulse: c_float,
    pub glow: c_float,
    pub glow_pulse: c_float,
    pub glow_width: c_float,
    pub glow_height: c_float,
    pub out_role: c_uint,
    pub out: c_float,
    pub out_width: c_float,
    pub ink: c_float,
    pub ink_quiet: c_float,
    pub mark: c_float,
    pub mark_quiet: c_float,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbMenu {
    pub width: c_float,
    pub extra_width: c_float,
    pub row: c_float,
    pub stacked_row: c_float,
    pub title: c_float,
    pub title_size: c_float,
    pub label_size: c_float,
    pub detail_size: c_float,
    pub stamp_size: c_float,
    pub group_gap: c_float,
    pub gap: c_float,
    pub glow_reach: c_float,
    pub scroll_strip: c_float,
    pub scroll_arrow: c_float,
    pub dim: c_float,
    pub scrim: c_float,
    pub depth: c_float,
    pub content_in: c_float,
    pub panel_in: c_float,
    pub icon: c_float,
    pub glyph: c_float,
    pub aside: c_float,
    pub aside_gap: c_float,
    pub aside_glyph: c_float,
    pub max_lines: c_uint,
}

impl From<Menu> for LxbMenu {
    fn from(shape: Menu) -> Self {
        Self {
            width: shape.width,
            extra_width: shape.extra_width,
            row: shape.row,
            stacked_row: shape.stacked_row,
            title: shape.title,
            title_size: shape.title_size,
            label_size: shape.label_size,
            detail_size: shape.detail_size,
            stamp_size: shape.stamp_size,
            group_gap: shape.group_gap,
            gap: shape.gap,
            glow_reach: shape.glow_reach,
            scroll_strip: shape.scroll_strip,
            scroll_arrow: shape.scroll_arrow,
            dim: shape.dim,
            scrim: shape.scrim,
            depth: shape.depth,
            content_in: shape.content_in,
            panel_in: shape.panel_in,
            icon: shape.icon,
            glyph: shape.glyph,
            aside: shape.aside,
            aside_gap: shape.aside_gap,
            aside_glyph: shape.aside_glyph,
            max_lines: shape.max_lines as c_uint,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbBytes {
    pub data: *const u8,
    pub len: Size,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbShellTheme {
    pub accent: Size,
    pub wallpaper: c_int,
    pub icons: c_int,
}

impl From<ShellTheme> for LxbShellTheme {
    fn from(theme: ShellTheme) -> Self {
        Self {
            accent: lxb_toolkit::palette::palette_index(theme.accent.name).unwrap_or(0) as Size,
            wallpaper: theme.wallpaper as c_int,
            icons: theme.icons as c_int,
        }
    }
}

impl LxbBytes {
    const NONE: Self = Self {
        data: std::ptr::null(),
        len: 0,
    };

    fn of(bytes: &'static [u8]) -> Self {
        Self {
            data: bytes.as_ptr(),
            len: bytes.len() as Size,
        }
    }
}

fn borrowed(text: &'static str) -> *const c_char {
    text.as_ptr() as *const c_char
}

unsafe fn as_str<'a>(text: *const c_char) -> Option<&'a str> {
    if text.is_null() {
        return None;
    }
    CStr::from_ptr(text).to_str().ok()
}

macro_rules! names {
    ($vis:vis $table:ident : $kind:ty { $($variant:expr => $name:literal),* $(,)? }) => {
        $vis static $table: &[($kind, &str)] = &[$(($variant, concat!($name, "\0"))),*];
    };
}

names!(ROLE_NAMES: Role {
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
});

static PALETTE_NAMES: &[&str] = &[
    "Purple\0", "Blue\0", "Green\0", "Yellow\0", "Red\0", "Teal\0", "Indigo\0", "Pink\0",
    "Orange\0", "White\0", "Silver\0", "Black\0",
];

static ICON_STYLE_NAMES: &[&str] = &["Default\0", "Simple\0"];
static WALLPAPER_STYLE_NAMES: &[&str] = &["Default\0", "Simple\0", "Custom\0"];
static OVERLAY_NAMES: &[&str] = &["context-menu\0", "dialog\0"];

#[no_mangle]
pub extern "C" fn lxb_palette_count() -> Size {
    PALETTES.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_palette_name(index: Size) -> *const c_char {
    match PALETTE_NAMES.get(index as usize) {
        Some(name) => borrowed(name),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_palette_index(name: *const c_char) -> c_int {
    match as_str(name).and_then(lxb_toolkit::palette::palette_index) {
        Some(index) => index as c_int,
        None => -1,
    }
}

#[no_mangle]
pub extern "C" fn lxb_role_count() -> Size {
    Role::ALL.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_role_name(role: Size) -> *const c_char {
    match ROLE_NAMES.get(role as usize) {
        Some((_, name)) => borrowed(name),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn lxb_palette_color(palette: Size, role: Size) -> c_uint {
    match (
        PALETTES.get(palette as usize),
        ROLE_NAMES.get(role as usize),
    ) {
        (Some(palette), Some((role, _))) => palette.color(*role).0 as c_uint,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn lxb_palette_rgba(palette: Size, role: Size, alpha: c_float) -> LxbRgba {
    match (
        PALETTES.get(palette as usize),
        ROLE_NAMES.get(role as usize),
    ) {
        (Some(palette), Some((role, _))) => {
            let [r, g, b, a] = palette.color(*role).a(alpha);
            LxbRgba { r, g, b, a }
        }
        _ => LxbRgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: alpha,
        },
    }
}

#[no_mangle]
pub extern "C" fn lxb_srgb_to_linear(hex: c_uint, alpha: c_float) -> LxbRgba {
    let [r, g, b, a] = lxb_toolkit::color::Srgb(hex).a(alpha);
    LxbRgba { r, g, b, a }
}

#[no_mangle]
pub extern "C" fn lxb_linear_to_srgb(color: LxbRgba) -> c_uint {
    lxb_toolkit::color::Linear([color.r, color.g, color.b])
        .srgb()
        .0 as c_uint
}

pub struct LxbAccent(Accent);

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_new(name: *const c_char) -> *mut LxbAccent {
    let accent = match as_str(name) {
        Some(name) => match Accent::new(name) {
            Some(accent) => accent,
            None => return std::ptr::null_mut(),
        },
        None => Accent::default_accent(),
    };
    Box::into_raw(Box::new(LxbAccent(accent)))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_free(accent: *mut LxbAccent) {
    if !accent.is_null() {
        drop(Box::from_raw(accent));
    }
}

unsafe fn accent_mut<'a>(accent: *mut LxbAccent) -> Option<&'a mut Accent> {
    accent.as_mut().map(|accent| &mut accent.0)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_preview(accent: *mut LxbAccent, name: *const c_char) -> c_int {
    match (accent_mut(accent), as_str(name)) {
        (Some(accent), Some(name)) => accent.preview(name) as c_int,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_commit(accent: *mut LxbAccent, name: *const c_char) -> c_int {
    match (accent_mut(accent), as_str(name)) {
        (Some(accent), Some(name)) => accent.commit(name) as c_int,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_set(accent: *mut LxbAccent, name: *const c_char) -> c_int {
    match (accent_mut(accent), as_str(name)) {
        (Some(accent), Some(name)) => accent.set(name) as c_int,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_restore(accent: *mut LxbAccent) {
    if let Some(accent) = accent_mut(accent) {
        accent.restore();
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_advance(accent: *mut LxbAccent, dt: c_float) -> c_int {
    match accent_mut(accent) {
        Some(accent) => accent.advance(dt) as c_int,
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_color(
    accent: *mut LxbAccent,
    role: Size,
    alpha: c_float,
) -> LxbRgba {
    match (accent_mut(accent), ROLE_NAMES.get(role as usize)) {
        (Some(accent), Some((role, _))) => {
            let [r, g, b, a] = accent.color(*role).a(alpha);
            LxbRgba { r, g, b, a }
        }
        _ => LxbRgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: alpha,
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_applied(accent: *mut LxbAccent) -> *const c_char {
    match accent_mut(accent) {
        Some(accent) => {
            let name = accent.applied().name;
            PALETTE_NAMES
                .iter()
                .find(|known| known.trim_end_matches('\0') == name)
                .map(|name| borrowed(name))
                .unwrap_or(std::ptr::null())
        }
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn lxb_icon_style_count() -> Size {
    IconStyle::ALL.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_icon_style_name(index: Size) -> *const c_char {
    ICON_STYLE_NAMES
        .get(index as usize)
        .map(|name| borrowed(name))
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub extern "C" fn lxb_wallpaper_style_count() -> Size {
    WallpaperStyle::ALL.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_wallpaper_style_name(index: Size) -> *const c_char {
    WALLPAPER_STYLE_NAMES
        .get(index as usize)
        .map(|name| borrowed(name))
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub extern "C" fn lxb_shell_theme_load() -> LxbShellTheme {
    ShellTheme::load().into()
}

#[no_mangle]
pub extern "C" fn lxb_ease(t: c_float) -> c_float {
    lxb_toolkit::motion::ease(t)
}

#[no_mangle]
pub extern "C" fn lxb_smoothstep(t: c_float) -> c_float {
    lxb_toolkit::motion::smoothstep(t)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_spring(
    position: *mut f64,
    velocity: *mut f64,
    target: f64,
    rate: f64,
    dt: f64,
) {
    let (Some(position), Some(velocity)) = (position.as_mut(), velocity.as_mut()) else {
        return;
    };
    let (at, speed) = lxb_toolkit::motion::spring(*position, *velocity, target, rate, dt);
    *position = at;
    *velocity = speed;
}

#[no_mangle]
pub extern "C" fn lxb_card_spring() -> f64 {
    lxb_toolkit::motion::CARD_SPRING
}

#[no_mangle]
pub extern "C" fn lxb_duration_count() -> Size {
    lxb_toolkit::motion_durations().len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_duration_name(index: Size) -> *const c_char {
    match duration_c_names().get(index as usize) {
        Some(name) => name.as_ptr(),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn lxb_duration(index: Size) -> c_float {
    lxb_toolkit::motion_durations()
        .get(index as usize)
        .map(|(_, seconds)| *seconds)
        .unwrap_or(0.0)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_duration_named(name: *const c_char) -> c_float {
    let Some(name) = as_str(name) else {
        return 0.0;
    };
    lxb_toolkit::motion_durations()
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, seconds)| *seconds)
        .unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn lxb_scale_for(height: c_float) -> c_float {
    lxb_toolkit::metrics::scale_for(height)
}

#[no_mangle]
pub extern "C" fn lxb_reference_height() -> c_float {
    lxb_toolkit::metrics::REFERENCE_HEIGHT
}

#[no_mangle]
pub extern "C" fn lxb_capsule_radius(height: c_float) -> c_float {
    lxb_toolkit::metrics::capsule_radius(height)
}

#[no_mangle]
pub extern "C" fn lxb_metric_count() -> Size {
    Metric::ALL.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_metric_name(index: Size) -> *const c_char {
    match METRIC_NAMES.get(index as usize) {
        Some(name) => borrowed(name),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn lxb_metric(index: Size, height: c_float) -> c_float {
    Metric::ALL
        .get(index as usize)
        .map(|metric| metric.on(height))
        .unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn lxb_metric_is_share(index: Size) -> c_int {
    Metric::ALL
        .get(index as usize)
        .map(|metric| metric.is_share() as c_int)
        .unwrap_or(0)
}

static METRIC_NAMES: &[&str] = &[
    "card-radius\0",
    "panel-radius\0",
    "panel-inset\0",
    "row-height\0",
    "row-padding\0",
    "panel-padding\0",
    "tile\0",
    "gap\0",
    "tile-glyph\0",
    "tile-radius\0",
    "item-spacing\0",
    "column-spacing\0",
    "item-icon\0",
    "item-icon-focused\0",
    "column-icon\0",
    "column-icon-focused\0",
    "menu-width\0",
    "dialog-width\0",
    "dialog-dim\0",
    "power-width\0",
    "power-dim\0",
];

#[no_mangle]
pub extern "C" fn lxb_text_count() -> Size {
    Text::ALL.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_text_name(index: Size) -> *const c_char {
    match TEXT_NAMES.get(index as usize) {
        Some(name) => borrowed(name),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn lxb_text_size(index: Size, height: c_float) -> c_float {
    Text::ALL
        .get(index as usize)
        .map(|text| text.on(height))
        .unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn lxb_text_is_bold(index: Size) -> c_int {
    Text::ALL
        .get(index as usize)
        .map(|text| (text.face() == Face::Bold) as c_int)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn lxb_text_line_height() -> c_float {
    Text::LINE
}

static TEXT_NAMES: &[&str] = &["display\0", "title\0", "body\0", "label\0", "caption\0"];

#[no_mangle]
pub extern "C" fn lxb_surface_count() -> Size {
    Surface::ALL.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_surface_name(index: Size) -> *const c_char {
    match SURFACE_NAMES.get(index as usize) {
        Some(name) => borrowed(name),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn lxb_surface_glass(index: Size) -> LxbGlass {
    match Surface::ALL.get(index as usize) {
        Some(surface) => {
            let glass = surface.glass();
            LxbGlass {
                depth: glass.depth,
                frost: glass.frost,
                gloss: glass.gloss,
                curve: glass.curve,
            }
        }
        None => LxbGlass {
            depth: 0.0,
            frost: 0.0,
            gloss: 0.0,
            curve: 0.0,
        },
    }
}

#[no_mangle]
pub extern "C" fn lxb_overlay_count() -> Size {
    2
}

#[no_mangle]
pub extern "C" fn lxb_overlay_name(index: Size) -> *const c_char {
    match OVERLAY_NAMES.get(index as usize) {
        Some(name) => borrowed(name),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn lxb_control() -> LxbControl {
    use lxb_toolkit::control as c;
    LxbControl {
        chip_role: role_index(c::CHIP_ROLE) as c_uint,
        chip_tint: c::CHIP_TINT,
        chip_gloss: c::chip().gloss,
        aside_tint: c::ASIDE_TINT,
        padding: c::PADDING,
        lit_role: role_index(c::LIT_ROLE) as c_uint,
        lit: c::LIT,
        lit_answer: c::LIT_ANSWER,
        lit_pulse: c::LIT_PULSE,
        glow: c::GLOW,
        glow_pulse: c::GLOW_PULSE,
        glow_width: c::GLOW_WIDTH,
        glow_height: c::GLOW_HEIGHT,
        out_role: role_index(c::OUT_ROLE) as c_uint,
        out: c::OUT,
        out_width: c::OUT_WIDTH,
        ink: c::INK,
        ink_quiet: c::INK_QUIET,
        mark: c::MARK,
        mark_quiet: c::MARK_QUIET,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_control_glow_rect(
    rect: *const c_float,
    over: c_float,
    tall: c_float,
    out: *mut c_float,
) {
    if rect.is_null() || out.is_null() {
        return;
    }
    let r = std::slice::from_raw_parts(rect, 4);
    let rect = [r[0], r[1], r[2], r[3]];
    let glow = if tall < 0.0 {
        lxb_toolkit::control::glow_rect(rect, over)
    } else {
        lxb_toolkit::control::glow_rect_tall(rect, over, tall)
    };
    std::slice::from_raw_parts_mut(out, 4).copy_from_slice(&glow);
}

#[no_mangle]
pub unsafe extern "C" fn lxb_control_arrival(
    light: *const c_float,
    rect: *const c_float,
) -> c_float {
    if light.is_null() || rect.is_null() {
        return 0.0;
    }
    let l = std::slice::from_raw_parts(light, 4);
    let r = std::slice::from_raw_parts(rect, 4);
    lxb_toolkit::control::arrival([l[0], l[1], l[2], l[3]], [r[0], r[1], r[2], r[3]])
}

#[no_mangle]
pub extern "C" fn lxb_press_scale(t: c_float) -> c_float {
    lxb_toolkit::motion::press_scale(t)
}

#[no_mangle]
pub extern "C" fn lxb_fill_arrival(t: c_float) -> c_float {
    lxb_toolkit::motion::fill_arrival(t)
}

#[no_mangle]
pub extern "C" fn lxb_pulse(seconds: c_float) -> c_float {
    lxb_toolkit::motion::pulse(seconds)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_pressed(rect: *const c_float, t: c_float, out: *mut c_float) {
    if rect.is_null() || out.is_null() {
        return;
    }
    let r = std::slice::from_raw_parts(rect, 4);
    let sunk = lxb_toolkit::motion::pressed([r[0], r[1], r[2], r[3]], (t >= 0.0).then_some(t));
    std::slice::from_raw_parts_mut(out, 4).copy_from_slice(&sunk);
}

#[no_mangle]
pub unsafe extern "C" fn lxb_glide(
    position: *mut c_float,
    velocity: *mut c_float,
    target: c_float,
    dt: c_float,
) {
    if position.is_null() || velocity.is_null() {
        return;
    }
    let (at, moving) = lxb_toolkit::motion::spring(
        *position as f64,
        *velocity as f64,
        target as f64,
        lxb_toolkit::motion::HIGHLIGHT_SPRING,
        dt as f64,
    );
    *position = at as c_float;
    *velocity = moving as c_float;
}

#[no_mangle]
pub extern "C" fn lxb_context_menu() -> LxbMenu {
    LxbMenu::from(lxb_toolkit::menu::CONTEXT)
}

#[no_mangle]
pub extern "C" fn lxb_menu_rows_that_fit(height: c_float, row: c_float) -> Size {
    lxb_toolkit::menu::rows_that_fit(height, row) as Size
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_growing(
    anchor: *const c_float,
    panel: *const c_float,
    travelled: c_float,
    out: *mut c_float,
) {
    if anchor.is_null() || panel.is_null() || out.is_null() {
        return;
    }
    let from = std::slice::from_raw_parts(anchor, 4);
    let to = std::slice::from_raw_parts(panel, 4);
    let grown = lxb_toolkit::menu::growing(
        [from[0], from[1], from[2], from[3]],
        [to[0], to[1], to[2], to[3]],
        travelled,
    );
    std::slice::from_raw_parts_mut(out, 4).copy_from_slice(&grown);
}

#[no_mangle]
pub extern "C" fn lxb_menu_shown(travelled: c_float, opening: c_int) -> c_float {
    lxb_toolkit::menu::CONTEXT.shown(travelled, opening != 0)
}

#[no_mangle]
pub extern "C" fn lxb_menu_content_shown(travelled: c_float, opening: c_int) -> c_float {
    lxb_toolkit::menu::CONTEXT.content_shown(travelled, opening != 0)
}

#[no_mangle]
pub extern "C" fn lxb_overlay_material_for(index: Size) -> LxbOverlayMaterial {
    match index {
        0 => Overlay::ContextMenu.material().into(),
        1 => Overlay::Dialog.material().into(),
        _ => LxbOverlayMaterial::NONE,
    }
}

static SURFACE_NAMES: &[&str] = &["panel\0", "control\0", "sidebar\0"];

#[no_mangle]
pub unsafe extern "C" fn lxb_key_light(out: *mut c_float) {
    if out.is_null() {
        return;
    }
    let light = lxb_toolkit::material::optics::KEY_LIGHT;
    std::ptr::copy_nonoverlapping(light.as_ptr(), out, 3);
}

#[no_mangle]
pub extern "C" fn lxb_glass_ior() -> c_float {
    lxb_toolkit::material::optics::IOR
}

#[no_mangle]
pub extern "C" fn lxb_glyph_count() -> Size {
    lxb_toolkit::assets::GLYPHS.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_glyph_name(index: Size) -> *const c_char {
    match glyph_c_names().get(index as usize) {
        Some(name) => name.as_ptr(),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_glyph(name: *const c_char) -> LxbBytes {
    match as_str(name).and_then(lxb_toolkit::assets::glyph) {
        Some(bytes) => LxbBytes::of(bytes),
        None => LxbBytes::NONE,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_glyph_box(name: *const c_char) -> c_uint {
    as_str(name)
        .and_then(lxb_toolkit::assets::glyph_box)
        .unwrap_or(0) as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn lxb_glyph_sdf(
    coverage: *const u8,
    coverage_len: Size,
    size: c_uint,
    out_alpha: *mut u8,
    out_len: Size,
) -> c_int {
    if size == 0 || coverage.is_null() || out_alpha.is_null() {
        return 0;
    }
    let fine = match size.checked_mul(lxb_toolkit::glyph_material::SDF_SUPERSAMPLE) {
        Some(fine) => fine as usize,
        None => return 0,
    };
    let Some(expected_coverage) = fine.checked_mul(fine) else {
        return 0;
    };
    let cell = size as usize;
    let Some(expected_out) = cell.checked_mul(cell) else {
        return 0;
    };
    let (Ok(coverage_len), Ok(out_len)) = (usize::try_from(coverage_len), usize::try_from(out_len))
    else {
        return 0;
    };
    if coverage_len != expected_coverage || out_len != expected_out {
        return 0;
    }
    let coverage = std::slice::from_raw_parts(coverage, expected_coverage);
    let Some(alpha) =
        lxb_toolkit::glyph_material::distance_field_alpha_from_coverage(coverage, size)
    else {
        return 0;
    };
    std::ptr::copy_nonoverlapping(alpha.as_ptr(), out_alpha, expected_out);
    1
}

#[no_mangle]
pub extern "C" fn lxb_glyph_wgsl() -> LxbBytes {
    LxbBytes::of(lxb_toolkit::assets::GLYPH_WGSL)
}

#[no_mangle]
pub extern "C" fn lxb_glyph_sdf_range() -> c_float {
    lxb_toolkit::glyph_material::SDF_RANGE
}

#[no_mangle]
pub extern "C" fn lxb_glyph_sdf_supersample() -> c_uint {
    lxb_toolkit::glyph_material::SDF_SUPERSAMPLE
}

#[no_mangle]
pub extern "C" fn lxb_glyph_cell() -> c_uint {
    lxb_toolkit::glyph_material::CELL
}

#[no_mangle]
pub extern "C" fn lxb_glyph_depth_share() -> c_float {
    lxb_toolkit::glyph_material::DEPTH_SHARE
}

#[no_mangle]
pub unsafe extern "C" fn lxb_glyph_lamp(out: *mut c_float) {
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(lxb_toolkit::glyph_material::LAMP.as_ptr(), out, 3);
    }
}

#[no_mangle]
pub extern "C" fn lxb_glyph_shadow() -> c_float {
    lxb_toolkit::glyph_material::SHADOW
}

#[no_mangle]
pub extern "C" fn lxb_glyph_simple_tint() -> c_float {
    lxb_toolkit::glyph_material::SIMPLE_TINT
}

#[no_mangle]
pub extern "C" fn lxb_glyph_simple_alpha() -> c_float {
    lxb_toolkit::glyph_material::SIMPLE_ALPHA
}

#[no_mangle]
pub extern "C" fn lxb_glyph_simple_stain() -> c_float {
    lxb_toolkit::glyph_material::SIMPLE_STAIN
}

#[no_mangle]
pub extern "C" fn lxb_sound_count() -> Size {
    Sound::ALL.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_sound_name(index: Size) -> *const c_char {
    match SOUND_NAMES.get(index as usize) {
        Some(name) => borrowed(name),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_sound(name: *const c_char) -> LxbBytes {
    match as_str(name).and_then(lxb_toolkit::assets::sound) {
        Some(bytes) => LxbBytes::of(bytes),
        None => LxbBytes::NONE,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_sound_used(name: *const c_char) -> c_int {
    as_str(name)
        .and_then(|name| Sound::ALL.into_iter().find(|sound| sound.name() == name))
        .map(|sound| sound.used_by_shell() as c_int)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn lxb_sound_rest() -> c_float {
    lxb_toolkit::sound::REST
}

#[no_mangle]
pub extern "C" fn lxb_sound_amplitude(value: c_float) -> c_float {
    lxb_toolkit::sound::amplitude(value)
}

#[no_mangle]
pub extern "C" fn lxb_sound_fade(elapsed: c_float, over: c_float) -> c_float {
    lxb_toolkit::sound::fade(elapsed, over)
}

#[no_mangle]
pub extern "C" fn lxb_sound_music_fade_in() -> c_float {
    lxb_toolkit::sound::MUSIC_FADE_IN
}

#[no_mangle]
pub extern "C" fn lxb_sound_music_fade_out() -> c_float {
    lxb_toolkit::sound::MUSIC_FADE_OUT
}

#[no_mangle]
pub extern "C" fn lxb_sound_retry_after() -> c_float {
    lxb_toolkit::sound::RETRY_AFTER
}

const ACTIONS: [Action; Action::ALL.len()] = Action::ALL;

#[no_mangle]
pub extern "C" fn lxb_action_count() -> Size {
    ACTIONS.len() as Size
}

#[no_mangle]
pub extern "C" fn lxb_action_name(index: Size) -> *const c_char {
    match ACTION_NAMES.get(index as usize) {
        Some(name) => borrowed(name),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_action_named(name: *const c_char) -> c_int {
    as_str(name)
        .and_then(Action::named)
        .map(index_of_action)
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn lxb_action_repeats(index: c_int) -> c_int {
    action_at(index).map(|a| a.repeats() as c_int).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn lxb_action_of_key(key: c_int) -> c_int {
    let key = match key {
        0 => Key::Left,
        1 => Key::Right,
        2 => Key::Up,
        3 => Key::Down,
        4 => Key::Enter,
        5 => Key::Space,
        6 => Key::Escape,
        7 => Key::Backspace,
        8 => Key::Tab,
        9 => Key::BackTab,
        10 => Key::Menu,
        11 => Key::F10,
        _ => return -1,
    };
    Action::of_key(key).map(index_of_action).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn lxb_action_of_letter(codepoint: c_uint) -> c_int {
    char::from_u32(codepoint)
        .and_then(Action::of_letter)
        .map(index_of_action)
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn lxb_action_of_button(button: c_int) -> c_int {
    let button = match button {
        0 => Button::South,
        1 => Button::East,
        2 => Button::North,
        3 => Button::West,
        4 => Button::Start,
        5 => Button::Select,
        6 => Button::LeftBumper,
        7 => Button::RightBumper,
        8 => Button::Guide,
        9 => Button::DPadLeft,
        10 => Button::DPadRight,
        11 => Button::DPadUp,
        12 => Button::DPadDown,
        _ => return -1,
    };
    Action::of_button(button).map(index_of_action).unwrap_or(-1)
}

fn index_of_action(action: Action) -> c_int {
    ACTIONS
        .iter()
        .position(|one| *one == action)
        .map(|at| at as c_int)
        .unwrap_or(-1)
}

fn action_at(index: c_int) -> Option<Action> {
    usize::try_from(index)
        .ok()
        .and_then(|at| ACTIONS.get(at))
        .copied()
}

#[no_mangle]
pub extern "C" fn lxb_input_poll_interval() -> c_float {
    lxb_toolkit::input::POLL_INTERVAL.as_secs_f32()
}

#[no_mangle]
pub extern "C" fn lxb_input_initial_repeat() -> c_float {
    lxb_toolkit::input::INITIAL_REPEAT_DELAY.as_secs_f32()
}

#[no_mangle]
pub extern "C" fn lxb_input_repeat_interval() -> c_float {
    lxb_toolkit::input::REPEAT_INTERVAL.as_secs_f32()
}

#[no_mangle]
pub extern "C" fn lxb_input_stick_engage() -> c_float {
    lxb_toolkit::input::STICK_ENGAGE
}

#[no_mangle]
pub extern "C" fn lxb_input_stick_release() -> c_float {
    lxb_toolkit::input::STICK_RELEASE
}

#[no_mangle]
pub extern "C" fn lxb_input_scroll_step() -> c_float {
    lxb_toolkit::input::SCROLL_STEP
}

#[no_mangle]
pub extern "C" fn lxb_input_tap_slop() -> c_float {
    lxb_toolkit::input::TAP_SLOP
}

pub struct LxbRepeat(lxb_toolkit::input::Repeat);

#[no_mangle]
pub extern "C" fn lxb_repeat_new() -> *mut LxbRepeat {
    Box::into_raw(Box::new(LxbRepeat(lxb_toolkit::input::Repeat::default())))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_repeat_free(repeat: *mut LxbRepeat) {
    if !repeat.is_null() {
        drop(Box::from_raw(repeat));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_repeat_reset(repeat: *mut LxbRepeat) {
    if let Some(repeat) = repeat.as_mut() {
        repeat.0.reset();
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_repeat_update(
    repeat: *mut LxbRepeat,
    now: c_float,
    pressed: *const c_int,
    x: c_float,
    y: c_float,
    out: *mut c_int,
    capacity: Size,
) -> Size {
    let (Some(repeat), false) = (repeat.as_mut(), pressed.is_null()) else {
        return 0;
    };
    let held = std::slice::from_raw_parts(pressed, 4);
    let mut down = [false; 4];
    for (slot, value) in down.iter_mut().zip(held) {
        *slot = *value != 0;
    }
    let mut due = Vec::new();
    repeat.0.update(
        std::time::Duration::from_secs_f32(now.max(0.0)),
        down,
        (x, y),
        &mut due,
    );
    let room = due.len().min(capacity as usize);
    if !out.is_null() {
        for (at, action) in due.iter().take(room).enumerate() {
            *out.add(at) = index_of_action(*action);
        }
    }
    due.len() as Size
}

#[no_mangle]
pub unsafe extern "C" fn lxb_wheel_notches(carried: *mut c_float, notches: c_float) -> c_int {
    let Some(carried) = carried.as_mut() else {
        return 0;
    };
    let mut wheel = lxb_toolkit::input::Wheel::default();
    let steps = wheel.notches(*carried + notches);
    *carried = *carried + notches - steps as c_float;
    steps
}

struct LxbPickerEntry {
    name: CString,
    path: CString,
    kind: EntryKind,
}

pub struct LxbPicker {
    picker: Picker,
    location: CString,
    note: CString,
    query: CString,
    entries: Vec<LxbPickerEntry>,
    choice: Option<CString>,
}

impl LxbPicker {
    fn new(selection: Selection, directory: &str) -> Self {
        let picker = Picker::new(selection, directory);
        let mut wrapped = Self {
            picker,
            location: empty_c_string(),
            note: empty_c_string(),
            query: empty_c_string(),
            entries: Vec::new(),
            choice: None,
        };
        wrapped.sync();
        wrapped
    }

    fn sync(&mut self) {
        self.location = path_c_string(self.picker.location());
        self.note = c_string(self.picker.note());
        self.query = c_string(self.picker.query());
        self.entries = self
            .picker
            .entries()
            .iter()
            .map(|entry| LxbPickerEntry {
                name: c_string(&entry.name),
                path: path_c_string(&entry.path),
                kind: entry.kind,
            })
            .collect();
        self.choice = None;
    }
}

fn empty_c_string() -> CString {
    CString::new("").expect("an empty C string is valid")
}

fn c_string(text: &str) -> CString {
    CString::new(text).expect("picker strings never contain a NUL")
}

fn path_c_string(path: &std::path::Path) -> CString {
    c_string(path.to_string_lossy().as_ref())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_new(
    selection: Size,
    directory: *const c_char,
) -> *mut LxbPicker {
    let (Some(selection), Some(directory)) = (
        Selection::ALL.get(selection as usize).copied(),
        as_str(directory),
    ) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(LxbPicker::new(selection, directory)))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_free(picker: *mut LxbPicker) {
    if !picker.is_null() {
        drop(Box::from_raw(picker));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_location(picker: *const LxbPicker) -> *const c_char {
    picker
        .as_ref()
        .map_or(std::ptr::null(), |picker| picker.location.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_note(picker: *const LxbPicker) -> *const c_char {
    picker
        .as_ref()
        .map_or(std::ptr::null(), |picker| picker.note.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_query(picker: *const LxbPicker) -> *const c_char {
    picker
        .as_ref()
        .map_or(std::ptr::null(), |picker| picker.query.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_selection(picker: *const LxbPicker) -> Size {
    picker
        .as_ref()
        .map_or(0, |picker| picker.picker.selection() as Size)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_can_search(picker: *const LxbPicker) -> c_int {
    c_int::from(
        picker
            .as_ref()
            .is_some_and(|picker| picker.picker.can_search()),
    )
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_can_choose(picker: *const LxbPicker) -> c_int {
    c_int::from(
        picker
            .as_ref()
            .is_some_and(|picker| picker.picker.can_choose()),
    )
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_entry_count(picker: *const LxbPicker) -> Size {
    picker
        .as_ref()
        .map_or(0, |picker| picker.entries.len() as Size)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_entry_name(
    picker: *const LxbPicker,
    index: Size,
) -> *const c_char {
    picker
        .as_ref()
        .and_then(|picker| picker.entries.get(index as usize))
        .map_or(std::ptr::null(), |entry| entry.name.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_entry_path(
    picker: *const LxbPicker,
    index: Size,
) -> *const c_char {
    picker
        .as_ref()
        .and_then(|picker| picker.entries.get(index as usize))
        .map_or(std::ptr::null(), |entry| entry.path.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_entry_kind(picker: *const LxbPicker, index: Size) -> c_int {
    picker
        .as_ref()
        .and_then(|picker| picker.entries.get(index as usize))
        .map_or(-1, |entry| entry.kind as c_int)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_selected(picker: *const LxbPicker) -> c_int {
    picker
        .as_ref()
        .and_then(|picker| picker.picker.selected())
        .and_then(|index| c_int::try_from(index).ok())
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_select(picker: *mut LxbPicker, index: Size) -> c_int {
    let Some(picker) = picker.as_mut() else {
        return 0;
    };
    let changed = picker.picker.select(index as usize);
    if changed {
        picker.sync();
    }
    c_int::from(changed)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_move(picker: *mut LxbPicker, delta: c_int) -> c_int {
    let Some(picker) = picker.as_mut() else {
        return 0;
    };
    let changed = picker.picker.move_selection(delta as isize);
    if changed {
        picker.sync();
    }
    c_int::from(changed)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_enter(picker: *mut LxbPicker) -> c_int {
    let Some(picker) = picker.as_mut() else {
        return 0;
    };
    let changed = picker.picker.enter();
    if changed {
        picker.sync();
    }
    c_int::from(changed)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_leave(picker: *mut LxbPicker) -> c_int {
    let Some(picker) = picker.as_mut() else {
        return 0;
    };
    let changed = picker.picker.leave();
    if changed {
        picker.sync();
    }
    c_int::from(changed)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_refresh(picker: *mut LxbPicker) {
    if let Some(picker) = picker.as_mut() {
        picker.picker.refresh();
        picker.sync();
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_search(picker: *mut LxbPicker, query: *const c_char) {
    if let (Some(picker), Some(query)) = (picker.as_mut(), as_str(query)) {
        picker.picker.search(query);
        picker.sync();
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_picker_choose(picker: *mut LxbPicker) -> *const c_char {
    let Some(picker) = picker.as_mut() else {
        return std::ptr::null();
    };
    let choice = picker.picker.choose().map(|path| path_c_string(&path));
    if picker.choice.as_ref().map(CString::as_bytes) != choice.as_ref().map(CString::as_bytes) {
        picker.choice = choice;
    }
    picker
        .choice
        .as_ref()
        .map_or(std::ptr::null(), |choice| choice.as_ptr())
}

#[no_mangle]
pub extern "C" fn lxb_font(bold: c_int) -> LxbBytes {
    LxbBytes::of(if bold != 0 {
        lxb_toolkit::assets::FONT_BOLD
    } else {
        lxb_toolkit::assets::FONT_REGULAR
    })
}

/// The same face for a script Roboto has not got: `"latin"` is Roboto,
/// `"devanagari"` and `"han"` the two Noto faces bundled beside it. Empty for
/// a name that is none of the three.
///
/// # Safety
/// `script` is a NUL-terminated string or null.
#[no_mangle]
pub unsafe extern "C" fn lxb_font_for(script: *const c_char, bold: c_int) -> LxbBytes {
    let Some(script) = as_str(script).and_then(lxb_toolkit::typography::Script::named) else {
        return LxbBytes::NONE;
    };
    let face = if bold != 0 {
        lxb_toolkit::typography::Face::Bold
    } else {
        lxb_toolkit::typography::Face::Regular
    };
    LxbBytes::of(face.bytes_for(script))
}

#[no_mangle]
pub extern "C" fn lxb_glass_wgsl() -> LxbBytes {
    LxbBytes::of(lxb_toolkit::assets::GLASS_WGSL)
}

#[no_mangle]
pub extern "C" fn lxb_wallpaper_wgsl() -> LxbBytes {
    LxbBytes::of(lxb_toolkit::assets::WALLPAPER_WGSL)
}

#[no_mangle]
pub extern "C" fn lxb_version() -> *const c_char {
    borrowed(concat!(env!("CARGO_PKG_VERSION"), "\0"))
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbMenuRow {
    pub stacked: c_int,
    pub stamp: c_int,
    pub aside: c_int,
    pub reading: c_int,
    pub group: c_uint,
    pub lines: c_uint,
    pub detail_lines: c_uint,
}

impl From<&LxbMenuRow> for lxb_toolkit::menu::Row {
    fn from(row: &LxbMenuRow) -> Self {
        Self {
            stacked: row.stacked != 0,
            stamp: row.stamp != 0,
            aside: row.aside != 0,
            reading: row.reading != 0,
            group: row.group.min(u8::MAX as c_uint) as u8,
            lines: row.lines.min(u8::MAX as c_uint) as u8,
            detail_lines: row.detail_lines.min(u8::MAX as c_uint) as u8,
        }
    }
}

pub struct LxbMenuLayout {
    rows: Vec<lxb_toolkit::menu::Row>,
    title_lines: Option<u8>,
    anchor: [c_float; 4],
    display: [c_float; 2],
    first: usize,
    visible: usize,
    selected: usize,
    unfolded: c_float,
    extra: c_float,
}

impl LxbMenuLayout {
    fn measured(&self) -> lxb_toolkit::menu::Layout<'_> {
        lxb_toolkit::menu::Layout::new(
            &self.rows,
            self.title_lines,
            self.anchor,
            self.display,
            self.first,
            self.visible,
            self.selected,
            self.unfolded,
            self.extra,
        )
    }
}

unsafe fn rows_of(rows: *const LxbMenuRow, count: Size) -> Option<Vec<lxb_toolkit::menu::Row>> {
    let count = usize::try_from(count).ok()?;
    if rows.is_null() || count == 0 {
        return None;
    }
    Some(
        std::slice::from_raw_parts(rows, count)
            .iter()
            .map(lxb_toolkit::menu::Row::from)
            .collect(),
    )
}

fn write4(out: *mut c_float, rect: [c_float; 4]) {
    if out.is_null() {
        return;
    }
    unsafe { std::ptr::copy_nonoverlapping(rect.as_ptr(), out, 4) };
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn lxb_menu_layout_new(
    rows: *const LxbMenuRow,
    count: Size,
    title_lines: c_uint,
    anchor: *const c_float,
    display: *const c_float,
    first: Size,
    visible: Size,
    selected: Size,
    unfolded: c_float,
    extra: c_float,
) -> *mut LxbMenuLayout {
    let (Some(rows), false, false) = (rows_of(rows, count), anchor.is_null(), display.is_null())
    else {
        return std::ptr::null_mut();
    };
    let anchor = std::slice::from_raw_parts(anchor, 4);
    let display = std::slice::from_raw_parts(display, 2);
    Box::into_raw(Box::new(LxbMenuLayout {
        rows,
        title_lines: (title_lines > 0).then(|| title_lines.min(u8::MAX as c_uint) as u8),
        anchor: [anchor[0], anchor[1], anchor[2], anchor[3]],
        display: [display[0], display[1]],
        first: first as usize,
        visible: visible as usize,
        selected: selected as usize,
        unfolded,
        extra,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_layout_free(layout: *mut LxbMenuLayout) {
    if !layout.is_null() {
        drop(Box::from_raw(layout));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_panel(layout: *const LxbMenuLayout, out: *mut c_float) {
    let Some(layout) = layout.as_ref() else {
        write4(out, [0.0; 4]);
        return;
    };
    write4(out, layout.measured().rect);
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_rows_top(layout: *const LxbMenuLayout) -> c_float {
    layout.as_ref().map_or(0.0, |it| it.measured().rows_top())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_row_rect(
    layout: *const LxbMenuLayout,
    index: Size,
    out: *mut c_float,
) -> c_int {
    menu_rect(layout, index, out, |it, index| it.row(index))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_chip_rect(
    layout: *const LxbMenuLayout,
    index: Size,
    out: *mut c_float,
) -> c_int {
    menu_rect(layout, index, out, |it, index| it.chip(index))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_aside_rect(
    layout: *const LxbMenuLayout,
    index: Size,
    out: *mut c_float,
) -> c_int {
    menu_rect(layout, index, out, |it, index| it.aside(index))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_highlight_rect(
    layout: *const LxbMenuLayout,
    on_aside: c_int,
    out: *mut c_float,
) -> c_int {
    let Some(layout) = layout.as_ref() else {
        write4(out, [0.0; 4]);
        return 0;
    };
    match layout.measured().highlight(on_aside != 0) {
        Some(rect) => {
            write4(out, rect);
            1
        }
        None => {
            write4(out, [0.0; 4]);
            0
        }
    }
}

unsafe fn menu_rect(
    layout: *const LxbMenuLayout,
    index: Size,
    out: *mut c_float,
    of: impl Fn(&lxb_toolkit::menu::Layout, usize) -> Option<[c_float; 4]>,
) -> c_int {
    let Some(layout) = layout.as_ref() else {
        write4(out, [0.0; 4]);
        return 0;
    };
    match of(&layout.measured(), index as usize) {
        Some(rect) => {
            write4(out, rect);
            1
        }
        None => {
            write4(out, [0.0; 4]);
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_separator(
    layout: *const LxbMenuLayout,
    index: Size,
    out: *mut c_float,
) -> c_int {
    let Some(layout) = layout.as_ref() else {
        write4(out, [0.0; 4]);
        return 0;
    };
    let rules = layout.measured().separators();
    match rules.get(index as usize) {
        Some(rect) => {
            write4(out, *rect);
            1
        }
        None => {
            write4(out, [0.0; 4]);
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_opened(
    layout: *const LxbMenuLayout,
    index: Size,
    out: *mut c_float,
) {
    let opened = layout
        .as_ref()
        .map_or((0.0, 0.0), |it| it.measured().opened(index as usize));
    if !out.is_null() {
        *out = opened.0;
        *out.add(1) = opened.1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_rows_of_that_fit(
    rows: *const LxbMenuRow,
    count: Size,
    title_lines: c_uint,
    height: c_float,
) -> Size {
    let Some(rows) = rows_of(rows, count) else {
        return 1;
    };
    lxb_toolkit::menu::rows_of_that_fit(
        &rows,
        (title_lines > 0).then(|| title_lines.min(u8::MAX as c_uint) as u8),
        height,
    ) as Size
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_chip_radius(
    row: *const LxbMenuRow,
    drawn: c_float,
    height: c_float,
) -> c_float {
    match row.as_ref() {
        Some(row) => lxb_toolkit::menu::chip_radius(&row.into(), drawn, height),
        None => drawn * 0.5,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_menu_aside_radius(row: *const LxbMenuRow, height: c_float) -> c_float {
    match row.as_ref() {
        Some(row) => lxb_toolkit::menu::aside_radius(&row.into(), height),
        None => 0.0,
    }
}

#[no_mangle]
pub extern "C" fn lxb_menu_lines_in(room: c_float, line: c_float) -> c_uint {
    lxb_toolkit::menu::lines_in(room, line) as c_uint
}

#[no_mangle]
pub extern "C" fn lxb_menu_title_height(lines: c_uint) -> c_float {
    lxb_toolkit::menu::title_height((lines > 0).then(|| lines.min(u8::MAX as c_uint) as u8))
}

#[no_mangle]
pub extern "C" fn lxb_menu_title_growth(lines: c_uint) -> c_float {
    lxb_toolkit::menu::title_growth(lines.min(u8::MAX as c_uint) as u8)
}

#[no_mangle]
pub extern "C" fn lxb_menu_margin() -> c_float {
    lxb_toolkit::menu::MARGIN
}

#[no_mangle]
pub extern "C" fn lxb_menu_label_padding() -> c_float {
    lxb_toolkit::menu::LABEL_PADDING
}

#[no_mangle]
pub extern "C" fn lxb_menu_row_padding() -> c_float {
    lxb_toolkit::menu::ROW_PADDING
}

#[no_mangle]
pub extern "C" fn lxb_menu_label_line() -> c_float {
    lxb_toolkit::menu::LABEL_LINE
}

#[no_mangle]
pub extern "C" fn lxb_menu_detail_line() -> c_float {
    lxb_toolkit::menu::DETAIL_LINE
}

#[no_mangle]
pub extern "C" fn lxb_menu_stamp_room() -> c_float {
    lxb_toolkit::menu::STAMP_ROOM
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbCanvas {
    pub pixels: *mut u8,
    pub width: c_uint,
    pub height: c_uint,

    pub stride: Size,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbScene {
    pub time: c_float,
    pub soften: c_float,
    pub style: Size,
    pub sky: [LxbRgba; 4],
    pub accent: [LxbRgba; 3],
    pub glow: LxbRgba,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbPane {
    pub x: c_float,
    pub y: c_float,
    pub width: c_float,
    pub height: c_float,
    pub radius: c_float,

    pub power: c_float,
    pub glass: LxbGlass,

    pub tint: LxbRgba,

    pub opacity: c_float,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LxbMark {
    pub x: c_float,
    pub y: c_float,
    pub width: c_float,
    pub height: c_float,

    pub color: LxbRgba,

    pub accent_soft: LxbRgba,
    pub gloss: c_float,

    pub simple: c_int,
}

unsafe fn borrow(surface: *const LxbCanvas) -> Option<(&'static mut [u8], c_uint, c_uint, usize)> {
    let surface = surface.as_ref()?;
    if surface.pixels.is_null() || surface.width == 0 || surface.height == 0 {
        return None;
    }
    let stride = usize::try_from(surface.stride).ok()?;
    if stride < surface.width as usize * 4 {
        return None;
    }
    let bytes = (surface.height as usize).checked_mul(stride)?;
    let pixels = std::slice::from_raw_parts_mut(surface.pixels, bytes);
    Some((pixels, surface.width, surface.height, stride))
}

fn linear(color: LxbRgba) -> [c_float; 4] {
    [color.r, color.g, color.b, color.a]
}

#[no_mangle]
pub extern "C" fn lxb_scene_for(palette: Size, time: c_float) -> LxbScene {
    let palette = PALETTES.get(palette as usize).unwrap_or(&PALETTES[0]);
    scene_of(&lxb_toolkit::paint::Scene::new(palette, time))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_accent_scene(accent: *mut LxbAccent, time: c_float) -> LxbScene {
    let Some(accent) = accent_mut(accent) else {
        return lxb_scene_for(0, time);
    };
    let shown = accent.shown();
    let mut scene = lxb_toolkit::paint::Scene::new(&PALETTES[0], time);
    let role = |role: Role| lxb_toolkit::color::Linear(shown.color(role).0);
    scene.palette = lxb_toolkit::wallpaper::ShaderPalette {
        sky: [
            role(Role::SkyTop).a(1.0),
            role(Role::SkyBottom).a(1.0),
            role(Role::SkyTopAlt).a(1.0),
            role(Role::SkyBottomAlt).a(1.0),
        ],
        accent: [
            role(Role::Accent).a(1.0),
            role(Role::AccentSoft).a(1.0),
            role(Role::AccentDeep).a(1.0),
        ],
        glow: role(Role::Glow).a(1.0),
    };
    scene_of(&scene)
}

fn scene_of(scene: &lxb_toolkit::paint::Scene) -> LxbScene {
    let rgba = |c: [c_float; 4]| LxbRgba {
        r: c[0],
        g: c[1],
        b: c[2],
        a: c[3],
    };
    LxbScene {
        time: scene.time,
        soften: scene.soften,
        style: WallpaperStyle::ALL
            .iter()
            .position(|&style| style == scene.style)
            .unwrap_or(0) as Size,
        sky: [
            rgba(scene.palette.sky[0]),
            rgba(scene.palette.sky[1]),
            rgba(scene.palette.sky[2]),
            rgba(scene.palette.sky[3]),
        ],
        accent: [
            rgba(scene.palette.accent[0]),
            rgba(scene.palette.accent[1]),
            rgba(scene.palette.accent[2]),
        ],
        glow: rgba(scene.palette.glow),
    }
}

fn scene_from(scene: &LxbScene) -> lxb_toolkit::paint::Scene {
    lxb_toolkit::paint::Scene {
        time: scene.time,
        soften: scene.soften,
        style: *WallpaperStyle::ALL
            .get(scene.style as usize)
            .unwrap_or(&WallpaperStyle::Default),
        palette: lxb_toolkit::wallpaper::ShaderPalette {
            sky: [
                linear(scene.sky[0]),
                linear(scene.sky[1]),
                linear(scene.sky[2]),
                linear(scene.sky[3]),
            ],
            accent: [
                linear(scene.accent[0]),
                linear(scene.accent[1]),
                linear(scene.accent[2]),
            ],
            glow: linear(scene.glow),
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_paint_wallpaper(
    surface: *const LxbCanvas,
    scene: *const LxbScene,
) -> c_int {
    let (Some((pixels, width, height, stride)), Some(scene)) = (borrow(surface), scene.as_ref())
    else {
        return 0;
    };
    let Some(mut canvas) = lxb_toolkit::paint::Canvas::new(pixels, width, height, stride) else {
        return 0;
    };
    lxb_toolkit::paint::wallpaper(&mut canvas, &scene_from(scene));
    1
}

#[no_mangle]
pub unsafe extern "C" fn lxb_paint_glass(surface: *const LxbCanvas, pane: *const LxbPane) -> c_int {
    let (Some((pixels, width, height, stride)), Some(pane)) = (borrow(surface), pane.as_ref())
    else {
        return 0;
    };
    let Some(mut canvas) = lxb_toolkit::paint::Canvas::new(pixels, width, height, stride) else {
        return 0;
    };
    lxb_toolkit::paint::glass(
        &mut canvas,
        &lxb_toolkit::paint::Pane {
            rect: [pane.x, pane.y, pane.width, pane.height],
            radius: pane.radius,
            power: pane.power,
            glass: lxb_toolkit::material::Glass {
                depth: pane.glass.depth,
                frost: pane.glass.frost,
                gloss: pane.glass.gloss,
                curve: pane.glass.curve,
            },
            tint: linear(pane.tint),
            opacity: pane.opacity,
        },
    );
    1
}

#[no_mangle]
pub unsafe extern "C" fn lxb_paint_light(
    surface: *const LxbCanvas,
    rect: *const c_float,
    tint: LxbRgba,
) -> c_int {
    let Some((pixels, width, height, stride)) = borrow(surface) else {
        return 0;
    };
    if rect.is_null() {
        return 0;
    }
    let rect = std::slice::from_raw_parts(rect, 4);
    let Some(mut canvas) = lxb_toolkit::paint::Canvas::new(pixels, width, height, stride) else {
        return 0;
    };
    lxb_toolkit::paint::light(
        &mut canvas,
        [rect[0], rect[1], rect[2], rect[3]],
        linear(tint),
    );
    1
}

#[no_mangle]
pub unsafe extern "C" fn lxb_paint_scrim(
    surface: *const LxbCanvas,
    rect: *const c_float,
    blur: c_float,
    tint: LxbRgba,
) -> c_int {
    let Some((pixels, width, height, stride)) = borrow(surface) else {
        return 0;
    };
    if rect.is_null() {
        return 0;
    }
    let rect = std::slice::from_raw_parts(rect, 4);
    let Some(mut canvas) = lxb_toolkit::paint::Canvas::new(pixels, width, height, stride) else {
        return 0;
    };
    lxb_toolkit::paint::scrim(
        &mut canvas,
        [rect[0], rect[1], rect[2], rect[3]],
        blur,
        linear(tint),
    );
    1
}

#[no_mangle]
pub unsafe extern "C" fn lxb_paint_glyph(
    surface: *const LxbCanvas,
    mark: *const LxbMark,
    field: *const u8,
    field_len: Size,
) -> c_int {
    let (Some((pixels, width, height, stride)), Some(mark)) = (borrow(surface), mark.as_ref())
    else {
        return 0;
    };
    if field.is_null() {
        return 0;
    }
    let Ok(field_len) = usize::try_from(field_len) else {
        return 0;
    };
    let cell = lxb_toolkit::glyph_material::CELL as usize;
    if field_len != cell * cell {
        return 0;
    }
    let field = std::slice::from_raw_parts(field, field_len);
    let Some(mut canvas) = lxb_toolkit::paint::Canvas::new(pixels, width, height, stride) else {
        return 0;
    };
    c_int::from(lxb_toolkit::paint::glyph(
        &mut canvas,
        &lxb_toolkit::paint::Mark {
            rect: [mark.x, mark.y, mark.width, mark.height],
            color: [mark.color.r, mark.color.g, mark.color.b],
            accent_soft: [mark.accent_soft.r, mark.accent_soft.g, mark.accent_soft.b],
            gloss: mark.gloss,
            opacity: mark.color.a,
            simple: mark.simple != 0,
        },
        field,
    ))
}

#[no_mangle]
pub extern "C" fn lxb_wallpaper_soften() -> c_float {
    lxb_toolkit::wallpaper::SOFTEN
}

#[no_mangle]
pub unsafe extern "C" fn lxb_stylesheet(name: *const c_char) -> *mut c_char {
    let Some(css) = as_str(name).and_then(lxb_toolkit::css::stylesheet_named) else {
        return std::ptr::null_mut();
    };
    let mut bytes = css.into_bytes();
    bytes.push(0);
    bytes.shrink_to_fit();
    let text = bytes.as_mut_ptr() as *mut c_char;
    std::mem::forget(bytes);
    text
}

#[no_mangle]
pub unsafe extern "C" fn lxb_string_free(text: *mut c_char) {
    if text.is_null() {
        return;
    }
    let len = CStr::from_ptr(text).to_bytes().len() + 1;
    drop(Vec::from_raw_parts(text as *mut u8, len, len));
}

#[allow(dead_code)]
fn _opaque(_: *mut c_void) {}

static DURATION_C_NAMES: OnceLock<Vec<CString>> = OnceLock::new();

fn duration_c_names() -> &'static [CString] {
    DURATION_C_NAMES.get_or_init(|| {
        lxb_toolkit::motion_durations()
            .iter()
            .map(|(name, _)| CString::new(*name).expect("duration names never contain nul"))
            .collect()
    })
}

static GLYPH_C_NAMES: OnceLock<Vec<CString>> = OnceLock::new();

fn glyph_c_names() -> &'static [CString] {
    GLYPH_C_NAMES.get_or_init(|| {
        lxb_toolkit::assets::glyph_names()
            .map(|name| CString::new(name).expect("glyph names never contain nul"))
            .collect()
    })
}

static SOUND_NAMES: &[&str] = &[
    "press\0",
    "press-selected\0",
    "press-guide\0",
    "press-guide-selected\0",
    "press-back\0",
    "keyboard-click\0",
    "app-launch\0",
    "guide-open\0",
    "screenshot\0",
    "polkit\0",
    "notification\0",
    "trash\0",
    "error\0",
    "start-bg-music\0",
];

static ACTION_NAMES: &[&str] = &[
    "left\0",
    "right\0",
    "up\0",
    "down\0",
    "accept\0",
    "back\0",
    "menu\0",
    "submit\0",
    "previous\0",
    "next\0",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn how_solid_a_panel_is_crosses_the_abi() {
        let shape = lxb_toolkit::menu::CONTEXT;
        for travelled in [0.0, 0.05, 0.25, 0.5, 0.75, 1.0] {
            for opening in [0, 1] {
                assert_eq!(
                    lxb_menu_shown(travelled, opening),
                    shape.shown(travelled, opening != 0)
                );
                assert_eq!(
                    lxb_menu_content_shown(travelled, opening),
                    shape.content_shown(travelled, opening != 0)
                );
            }
        }

        assert_eq!(lxb_menu_shown(0.5, 0), 0.5);
        assert_eq!(lxb_menu_shown(0.5, 1), 1.0);

        assert_eq!(lxb_menu_shown(-3.0, 0), 0.0);
        assert_eq!(lxb_menu_content_shown(4.0, 1), 1.0);
    }

    #[test]
    fn the_input_contract_crosses_the_abi() {
        assert_eq!(lxb_action_count() as usize, Action::ALL.len());
        assert_eq!(lxb_action_of_key(0), 0, "left is left");
        assert_eq!(lxb_action_of_key(4), 4, "Enter accepts");
        assert_eq!(lxb_action_of_key(10), 6, "the Menu key raises the menu");
        assert_eq!(lxb_action_of_key(99), -1);

        assert_eq!(lxb_action_of_letter('w' as c_uint), 2);
        assert_eq!(lxb_action_of_letter('Y' as c_uint), 6);
        assert_eq!(lxb_action_of_letter('q' as c_uint), -1);

        for button in [8, 3, 5] {
            assert_eq!(lxb_action_of_button(button), -1, "button {button}");
        }
        assert_eq!(lxb_action_of_button(0), 4, "the bottom face button accepts");
        assert_eq!(lxb_action_of_button(2), 6, "the top one raises the menu");

        unsafe {
            assert_eq!(lxb_action_named(c"accept".as_ptr()), 4);
            assert_eq!(lxb_action_named(c"launch".as_ptr()), -1);
            assert_eq!(lxb_action_named(std::ptr::null()), -1);
        }
        assert_eq!(lxb_action_repeats(3), 1, "Down repeats");
        assert_eq!(lxb_action_repeats(4), 0, "Accept does not");
        assert_eq!(lxb_action_repeats(-1), 0);
    }

    #[test]
    fn a_held_direction_repeats_across_the_abi() {
        let repeat = lxb_repeat_new();
        assert!(!repeat.is_null());
        let down = [0, 0, 0, 1];
        let mut out = [-1; 4];
        let step = |at: f32, out: &mut [c_int; 4]| unsafe {
            lxb_repeat_update(repeat, at, down.as_ptr(), 0.0, 0.0, out.as_mut_ptr(), 4)
        };
        assert_eq!(step(0.0, &mut out), 1);
        assert_eq!(out[0], 3, "Down");
        assert_eq!(step(0.349, &mut out), 0);

        assert_eq!(step(0.360, &mut out), 1);

        unsafe {
            lxb_repeat_reset(repeat);

            assert_eq!(
                lxb_repeat_update(repeat, 0.4, down.as_ptr(), 0.0, 0.0, out.as_mut_ptr(), 4),
                1
            );
            assert_eq!(
                lxb_repeat_update(repeat, 0.5, down.as_ptr(), 0.0, 0.0, out.as_mut_ptr(), 4),
                0
            );

            assert_eq!(
                lxb_repeat_update(
                    repeat,
                    1.0,
                    std::ptr::null(),
                    0.0,
                    0.0,
                    std::ptr::null_mut(),
                    0
                ),
                0
            );
            lxb_repeat_reset(std::ptr::null_mut());
            lxb_repeat_free(repeat);
            lxb_repeat_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn a_wheel_carries_its_remainder_across_the_abi() {
        let mut carried: c_float = 0.0;
        unsafe {
            assert_eq!(lxb_wheel_notches(&mut carried, 0.34), 0);
            assert_eq!(lxb_wheel_notches(&mut carried, 0.34), 0);
            assert_eq!(lxb_wheel_notches(&mut carried, 0.34), 1);
            assert_eq!(lxb_wheel_notches(&mut carried, -1.5), -1);
            assert_eq!(lxb_wheel_notches(std::ptr::null_mut(), 5.0), 0);
        }
    }

    #[test]
    fn a_picker_walks_and_answers_across_the_abi() {
        struct Scratch(std::path::PathBuf);

        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock has an epoch")
            .as_nanos();
        let scratch = Scratch(
            std::env::temp_dir().join(format!("lxb-picker-ffi-{}-{unique}", std::process::id())),
        );
        std::fs::create_dir_all(scratch.0.join("inside")).unwrap();
        std::fs::write(scratch.0.join("note.txt"), b"x").unwrap();
        std::fs::write(scratch.0.join("inside/picked.txt"), b"x").unwrap();
        let directory = CString::new(scratch.0.to_string_lossy().as_bytes()).unwrap();

        unsafe {
            let picker = lxb_picker_new(Selection::File as Size, directory.as_ptr());
            assert!(!picker.is_null());
            assert_eq!(lxb_picker_selection(picker), Selection::File as Size);
            assert_eq!(lxb_picker_can_search(picker), 1);
            assert_eq!(lxb_picker_can_choose(picker), 0);
            assert_eq!(
                CStr::from_ptr(lxb_picker_location(picker)).to_bytes(),
                scratch.0.to_string_lossy().as_bytes()
            );
            assert_eq!(lxb_picker_entry_count(picker), 2);
            assert_eq!(
                CStr::from_ptr(lxb_picker_entry_name(picker, 0)).to_bytes(),
                b"inside"
            );
            assert_eq!(lxb_picker_entry_kind(picker, 0), EntryKind::Folder as c_int);
            assert_eq!(lxb_picker_selected(picker), 0);
            assert!(lxb_picker_choose(picker).is_null());

            assert_eq!(lxb_picker_select(picker, 1), 1);
            assert_eq!(lxb_picker_can_choose(picker), 1);
            let first_choice = lxb_picker_choose(picker);
            assert_eq!(
                CStr::from_ptr(first_choice).to_bytes(),
                scratch.0.join("note.txt").to_string_lossy().as_bytes()
            );
            assert_eq!(
                lxb_picker_choose(picker),
                first_choice,
                "a read-only repeat keeps a borrowed answer alive"
            );
            assert_eq!(lxb_picker_select(picker, 0), 1);
            assert_eq!(lxb_picker_enter(picker), 1);
            assert_eq!(
                CStr::from_ptr(lxb_picker_entry_name(picker, 0)).to_bytes(),
                b"picked.txt"
            );
            lxb_picker_search(picker, c"PICK".as_ptr());
            assert_eq!(CStr::from_ptr(lxb_picker_query(picker)).to_bytes(), b"PICK");
            assert_eq!(lxb_picker_entry_count(picker), 1);
            assert_eq!(
                CStr::from_ptr(lxb_picker_choose(picker)).to_bytes(),
                scratch
                    .0
                    .join("inside/picked.txt")
                    .to_string_lossy()
                    .as_bytes()
            );
            assert_eq!(lxb_picker_leave(picker), 1);
            assert!(CStr::from_ptr(lxb_picker_query(picker))
                .to_bytes()
                .is_empty());
            lxb_picker_free(picker);

            let folders = lxb_picker_new(Selection::Folder as Size, directory.as_ptr());
            assert!(!folders.is_null());
            assert_eq!(lxb_picker_can_search(folders), 0);
            assert_eq!(lxb_picker_can_choose(folders), 1);
            lxb_picker_search(folders, c"inside".as_ptr());
            assert!(CStr::from_ptr(lxb_picker_query(folders))
                .to_bytes()
                .is_empty());
            assert_eq!(
                CStr::from_ptr(lxb_picker_choose(folders)).to_bytes(),
                scratch.0.to_string_lossy().as_bytes()
            );
            lxb_picker_free(folders);

            assert!(lxb_picker_new(99, directory.as_ptr()).is_null());
            assert!(lxb_picker_new(0, std::ptr::null()).is_null());
        }
    }

    #[test]
    fn the_sound_curves_cross_the_abi() {
        assert_eq!(lxb_sound_amplitude(0.0), 0.0);
        assert!((lxb_sound_amplitude(1.0) - 1.0).abs() < 1e-5);
        assert!(lxb_sound_amplitude(0.5) < 0.25);
        assert_eq!(lxb_sound_fade(0.0, lxb_sound_music_fade_in()), 0.0);
        assert_eq!(lxb_sound_fade(9.0, lxb_sound_music_fade_in()), 1.0);
        assert!(lxb_sound_music_fade_out() > lxb_sound_music_fade_in());
        assert_eq!(lxb_sound_retry_after(), lxb_toolkit::sound::RETRY_AFTER);
    }

    #[test]
    fn the_name_tables_agree_with_the_library() {
        let trimmed = |name: &&str| name.trim_end_matches('\0').to_string();
        assert_eq!(
            ACTION_NAMES.iter().map(trimmed).collect::<Vec<_>>(),
            Action::ALL
                .iter()
                .map(|action| action.name().to_string())
                .collect::<Vec<_>>(),
        );

        let roles: Vec<String> = ROLE_NAMES
            .iter()
            .map(|(_, name)| name.trim_end_matches('\0').to_string())
            .collect();
        let want: Vec<String> = Role::ALL.iter().map(|r| r.name().to_string()).collect();
        assert_eq!(roles, want);
        for (index, (role, _)) in ROLE_NAMES.iter().enumerate() {
            assert_eq!(*role, Role::ALL[index], "role {index} is out of order");
        }

        assert_eq!(
            PALETTE_NAMES.iter().map(trimmed).collect::<Vec<_>>(),
            PALETTES
                .iter()
                .map(|p| p.name.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            ICON_STYLE_NAMES.iter().map(trimmed).collect::<Vec<_>>(),
            IconStyle::ALL
                .iter()
                .map(|style| style.name().to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            WALLPAPER_STYLE_NAMES
                .iter()
                .map(trimmed)
                .collect::<Vec<_>>(),
            WallpaperStyle::ALL
                .iter()
                .map(|style| style.name().to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            METRIC_NAMES.iter().map(trimmed).collect::<Vec<_>>(),
            Metric::ALL
                .iter()
                .map(|m| m.name().to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            TEXT_NAMES.iter().map(trimmed).collect::<Vec<_>>(),
            Text::ALL
                .iter()
                .map(|t| t.name().to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            SURFACE_NAMES.iter().map(trimmed).collect::<Vec<_>>(),
            Surface::ALL
                .iter()
                .map(|s| s.name().to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            duration_c_names()
                .iter()
                .map(|name| name.to_str().unwrap().to_string())
                .collect::<Vec<_>>(),
            lxb_toolkit::motion_durations()
                .iter()
                .map(|(name, _)| name.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            glyph_c_names()
                .iter()
                .map(|name| name.to_str().unwrap().to_string())
                .collect::<Vec<_>>(),
            lxb_toolkit::assets::glyph_names()
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            SOUND_NAMES.iter().map(trimmed).collect::<Vec<_>>(),
            Sound::ALL
                .iter()
                .map(|s| s.name().to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn every_string_handed_to_c_is_terminated() {
        let tables = [
            PALETTE_NAMES,
            METRIC_NAMES,
            TEXT_NAMES,
            SURFACE_NAMES,
            ICON_STYLE_NAMES,
            WALLPAPER_STYLE_NAMES,
            SOUND_NAMES,
        ];
        for table in tables {
            for name in table {
                assert!(name.ends_with('\0'), "{name:?}");
                assert_eq!(name.matches('\0').count(), 1, "{name:?}");
            }
        }
        for (_, name) in ROLE_NAMES {
            assert!(name.ends_with('\0'), "{name:?}");
        }
        for name in duration_c_names().iter().chain(glyph_c_names()) {
            assert_eq!(name.as_bytes_with_nul().last(), Some(&0));
            assert!(!name.as_bytes().contains(&0));
        }
    }

    #[test]
    fn null_is_answered_rather_than_dereferenced() {
        unsafe {
            assert!(lxb_glyph(std::ptr::null()).data.is_null());
            assert_eq!(lxb_glyph_box(std::ptr::null()), 0);
            assert_eq!(
                lxb_glyph_sdf(std::ptr::null(), 0, 4, std::ptr::null_mut(), 0),
                0
            );
            lxb_glyph_lamp(std::ptr::null_mut());
            assert!(lxb_sound(std::ptr::null()).data.is_null());
            assert_eq!(lxb_sound_used(std::ptr::null()), 0);
            assert_eq!(lxb_palette_index(std::ptr::null()), -1);
            assert_eq!(lxb_duration_named(std::ptr::null()), 0.0);
            assert!(lxb_stylesheet(std::ptr::null()).is_null());
            lxb_string_free(std::ptr::null_mut());
            lxb_accent_free(std::ptr::null_mut());
            lxb_accent_restore(std::ptr::null_mut());
            assert_eq!(lxb_accent_advance(std::ptr::null_mut(), 0.1), 0);
            assert!(lxb_accent_applied(std::ptr::null_mut()).is_null());
            lxb_picker_free(std::ptr::null_mut());
            assert!(lxb_picker_location(std::ptr::null()).is_null());
            assert!(lxb_picker_note(std::ptr::null()).is_null());
            assert!(lxb_picker_query(std::ptr::null()).is_null());
            assert_eq!(lxb_picker_can_search(std::ptr::null()), 0);
            assert_eq!(lxb_picker_can_choose(std::ptr::null()), 0);
            assert_eq!(lxb_picker_entry_count(std::ptr::null()), 0);
            assert!(lxb_picker_entry_name(std::ptr::null(), 0).is_null());
            assert!(lxb_picker_entry_path(std::ptr::null(), 0).is_null());
            assert_eq!(lxb_picker_entry_kind(std::ptr::null(), 0), -1);
            assert_eq!(lxb_picker_selected(std::ptr::null()), -1);
            assert_eq!(lxb_picker_select(std::ptr::null_mut(), 0), 0);
            assert_eq!(lxb_picker_move(std::ptr::null_mut(), 1), 0);
            assert_eq!(lxb_picker_enter(std::ptr::null_mut()), 0);
            assert_eq!(lxb_picker_leave(std::ptr::null_mut()), 0);
            lxb_picker_refresh(std::ptr::null_mut());
            lxb_picker_search(std::ptr::null_mut(), std::ptr::null());
            assert!(lxb_picker_choose(std::ptr::null_mut()).is_null());
            lxb_key_light(std::ptr::null_mut());
            lxb_spring(std::ptr::null_mut(), std::ptr::null_mut(), 1.0, 1.0, 1.0);
            lxb_glide(std::ptr::null_mut(), std::ptr::null_mut(), 1.0, 0.1);
            lxb_pressed(std::ptr::null(), 0.5, std::ptr::null_mut());
            lxb_control_glow_rect(std::ptr::null(), 100.0, -1.0, std::ptr::null_mut());
            assert_eq!(lxb_control_arrival(std::ptr::null(), std::ptr::null()), 0.0);
        }

        assert!(lxb_palette_name(99).is_null());
        assert!(lxb_role_name(99).is_null());
        assert!(lxb_icon_style_name(99).is_null());
        assert!(lxb_wallpaper_style_name(99).is_null());
        assert!(lxb_overlay_name(99).is_null());
        assert_eq!(lxb_overlay_material_for(99).radius, 0.0);
        assert!(lxb_glyph_name(999).is_null());
        assert_eq!(lxb_metric(999, 1080.0), 0.0);
        assert_eq!(lxb_palette_color(99, 0), 0);
    }

    #[test]
    fn the_journey_a_binding_makes_works() {
        unsafe {
            let purple = c"Purple".as_ptr();
            assert_eq!(lxb_palette_index(purple), 0);
            assert_eq!(lxb_palette_color(0, 0), 0x8B5CF6);

            let accent = lxb_accent_new(purple);
            assert!(!accent.is_null());
            assert_eq!(lxb_accent_preview(accent, c"Blue".as_ptr()), 1);
            let mut frames = 0;
            while lxb_accent_advance(accent, 1.0 / 60.0) == 1 {
                frames += 1;
                assert!(frames < 1000);
            }
            assert!(frames > 1, "it arrived without animating");
            let blue = lxb_accent_color(accent, 0, 1.0);
            let expected = lxb_srgb_to_linear(0x3B82F6, 1.0);
            assert!((blue.r - expected.r).abs() < 1e-6);
            assert_eq!(lxb_linear_to_srgb(blue), 0x3B82F6);
            lxb_accent_free(accent);

            let css = lxb_stylesheet(c"Green".as_ptr());
            assert!(!css.is_null());
            let text = CStr::from_ptr(css).to_str().expect("utf-8");
            assert!(text.contains("--lxb-accent: #16a34a;"));
            lxb_string_free(css);
        }
    }

    #[test]
    fn the_default_theme_crosses_the_abi() {
        let theme: LxbShellTheme = ShellTheme::default().into();
        assert_eq!(theme.accent, 0);
        assert_eq!(theme.wallpaper, WallpaperStyle::Default as c_int);
        assert_eq!(theme.icons, IconStyle::Default as c_int);
        assert_eq!(lxb_wallpaper_style_count(), 3);
        assert_eq!(lxb_icon_style_count(), 2);
        let shader = lxb_wallpaper_wgsl();
        assert!(!shader.data.is_null());
        assert!(shader.len > 10_000);
    }

    #[test]
    fn the_dialog_and_context_menu_share_every_layer_across_the_abi() {
        assert_eq!(lxb_overlay_count(), 2);
        assert_eq!(
            unsafe { CStr::from_ptr(lxb_overlay_name(0)) }.to_bytes(),
            b"context-menu"
        );
        assert_eq!(
            unsafe { CStr::from_ptr(lxb_overlay_name(1)) }.to_bytes(),
            b"dialog"
        );

        let menu = lxb_overlay_material_for(0);
        let dialog = lxb_overlay_material_for(1);
        assert_eq!(dialog.radius, menu.radius);
        assert_eq!(dialog.glass.depth, menu.glass.depth);
        assert_eq!(dialog.glass.frost, menu.glass.frost);
        assert_eq!(dialog.glass.gloss, menu.glass.gloss);
        assert_eq!(dialog.glass.curve, menu.glass.curve);
        assert_eq!(dialog.stain, menu.stain);
        assert_eq!(dialog.light_inset, menu.light_inset);
        assert_eq!(dialog.header_x, menu.header_x);
        assert_eq!(dialog.header_width, menu.header_width);
        assert_eq!(dialog.header_height, menu.header_height);
        assert_eq!(dialog.header_height_share, menu.header_height_share);
        assert_eq!(dialog.header_light, menu.header_light);
        assert_eq!(dialog.foot_x, menu.foot_x);
        assert_eq!(dialog.foot_width, menu.foot_width);
        assert_eq!(dialog.foot_height, menu.foot_height);
        assert_eq!(dialog.foot_height_share, menu.foot_height_share);
        assert_eq!(dialog.foot_light, menu.foot_light);
        assert_eq!(dialog.rim, menu.rim);
        assert_eq!(dialog.rim_width, menu.rim_width);
        assert_eq!(dialog.stain_role, menu.stain_role);
        assert_eq!(dialog.header_role, menu.header_role);
        assert_eq!(dialog.foot_role, menu.foot_role);
        assert_eq!(dialog.rim_role, menu.rim_role);

        assert_eq!(dialog.radius, 30.0);
        assert_eq!(dialog.glass.depth, 15.0);
        assert_eq!(dialog.glass.frost, 0.46);
        assert_eq!(dialog.glass.gloss, 0.66);
        assert_eq!(dialog.glass.curve, 1.0);
        assert_eq!(dialog.stain, 0.38);
        assert_eq!(dialog.header_light, 0.075);
        assert_eq!(dialog.foot_light, 0.04);
        assert_eq!(dialog.rim, 0.10);
    }

    fn header_enum(kind: &str) -> Vec<(usize, String)> {
        const HEADER: &str = include_str!("../include/lxb_toolkit.h");

        let prefix = format!("LXB_{}_", kind.to_uppercase());
        let marker = format!("\n    {prefix}");
        let introduced = HEADER
            .find(&marker)
            .unwrap_or_else(|| panic!("the header declares no lxb_{kind} values"));
        let open = "enum {";
        let start = HEADER[..introduced]
            .rfind(open)
            .unwrap_or_else(|| panic!("lxb_{kind} is not an enum"))
            + open.len();
        let end = start
            + HEADER[start..]
                .find("};")
                .unwrap_or_else(|| panic!("lxb_{kind} is never closed"));
        HEADER[start..end]
            .lines()
            .filter_map(|line| line.trim().strip_prefix(&prefix))
            .map(|rest| {
                let (name, value) = rest
                    .split_once('=')
                    .unwrap_or_else(|| panic!("no value in LXB_{prefix}{rest}"));
                (
                    value
                        .trim()
                        .trim_end_matches(',')
                        .parse()
                        .expect("an enumerator's value is a number"),
                    name.trim().to_lowercase().replace('_', "-"),
                )
            })
            .collect()
    }

    const BLACK: LxbRgba = LxbRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    fn header_struct(name: &str) -> Vec<String> {
        const HEADER: &str = include_str!("../include/lxb_toolkit.h");
        let closes = format!("}} {name};");
        let end = HEADER
            .find(&closes)
            .unwrap_or_else(|| panic!("the header declares no {name}"));
        let open = "typedef struct {";
        let start = HEADER[..end]
            .rfind(open)
            .unwrap_or_else(|| panic!("{name} is not a struct"))
            + open.len();
        HEADER[start..end]
            .lines()
            .filter_map(|line| {
                let line = line.trim().strip_suffix(';')?;

                line.rsplit_once(' ').map(|(_, field)| field.to_string())
            })
            .collect()
    }

    #[test]
    fn the_controls_fields_are_one_order_on_both_sides() {
        let expected = [
            "chip_role",
            "chip_tint",
            "chip_gloss",
            "aside_tint",
            "padding",
            "lit_role",
            "lit",
            "lit_answer",
            "lit_pulse",
            "glow",
            "glow_pulse",
            "glow_width",
            "glow_height",
            "out_role",
            "out",
            "out_width",
            "ink",
            "ink_quiet",
            "mark",
            "mark_quiet",
        ];
        assert_eq!(header_struct("lxb_control_shape"), expected);
        assert_eq!(
            std::mem::size_of::<LxbControl>(),
            expected.len() * 4,
            "the struct is not the {} four-byte fields the header declares",
            expected.len()
        );
    }

    #[test]
    fn a_panel_is_measured_the_same_on_both_sides() {
        assert_eq!(
            header_struct("lxb_menu_row"),
            [
                "stacked",
                "stamp",
                "aside",
                "reading",
                "group",
                "lines",
                "detail_lines"
            ]
        );
        let rows = [
            LxbMenuRow {
                stacked: 0,
                stamp: 0,
                aside: 0,
                reading: 0,
                group: 0,
                lines: 1,
                detail_lines: 1,
            },
            LxbMenuRow {
                group: 1,
                ..rows_first()
            },
        ];
        let anchor = [150.0_f32, 200.0, 120.0, 120.0];
        let display = [1280.0_f32, 800.0];
        unsafe {
            let fit = lxb_menu_rows_of_that_fit(rows.as_ptr(), 2, 1, display[1]);
            assert_eq!(fit, 2);
            let layout = lxb_menu_layout_new(
                rows.as_ptr(),
                2,
                1,
                anchor.as_ptr(),
                display.as_ptr(),
                0,
                fit,
                0,
                0.0,
                0.0,
            );
            assert!(!layout.is_null());

            let mut panel = [0.0_f32; 4];
            lxb_menu_panel(layout, panel.as_mut_ptr());
            let own = [
                lxb_toolkit::menu::Row::command(0),
                lxb_toolkit::menu::Row::command(1),
            ];
            let shape =
                lxb_toolkit::menu::Layout::new(&own, Some(1), anchor, display, 0, 2, 0, 0.0, 0.0);
            assert_eq!(panel, shape.rect, "the panel is not the library's own");

            let mut row = [0.0_f32; 4];
            assert_eq!(lxb_menu_row_rect(layout, 0, row.as_mut_ptr()), 1);
            assert_eq!(row, shape.row(0).expect("a drawn row"));
            assert_eq!(lxb_menu_row_rect(layout, 9, row.as_mut_ptr()), 0);

            assert_eq!(lxb_menu_separator(layout, 0, row.as_mut_ptr()), 1);
            assert_eq!(lxb_menu_separator(layout, 1, row.as_mut_ptr()), 0);

            assert_eq!(lxb_menu_aside_rect(layout, 0, row.as_mut_ptr()), 0);
            assert_eq!(lxb_menu_highlight_rect(layout, 0, row.as_mut_ptr()), 1);
            assert_eq!(row, shape.chip(0).expect("a face"));

            lxb_menu_layout_free(layout);

            assert!(lxb_menu_layout_new(
                std::ptr::null(),
                0,
                1,
                anchor.as_ptr(),
                display.as_ptr(),
                0,
                1,
                0,
                0.0,
                0.0
            )
            .is_null());
            assert_eq!(lxb_menu_row_rect(std::ptr::null(), 0, row.as_mut_ptr()), 0);
            lxb_menu_layout_free(std::ptr::null_mut());
        }
        assert_eq!(lxb_menu_margin(), lxb_toolkit::menu::MARGIN);
        assert_eq!(lxb_menu_label_line(), lxb_toolkit::menu::LABEL_LINE);
        assert_eq!(lxb_menu_lines_in(0.0, 10.0), 1);
    }

    fn rows_first() -> LxbMenuRow {
        LxbMenuRow {
            stacked: 0,
            stamp: 0,
            aside: 0,
            reading: 0,
            group: 0,
            lines: 1,
            detail_lines: 1,
        }
    }

    #[test]
    fn the_painters_structs_are_one_order_on_both_sides() {
        assert_eq!(
            header_struct("lxb_canvas"),
            ["*pixels", "width", "height", "stride"]
        );
        assert_eq!(
            header_struct("lxb_scene"),
            ["time", "soften", "style", "sky[4]", "accent[3]", "glow"]
        );
        let pane = [
            "x", "y", "width", "height", "radius", "power", "glass", "tint", "opacity",
        ];
        assert_eq!(header_struct("lxb_pane"), pane);

        assert_eq!(std::mem::size_of::<LxbPane>(), 15 * 4);
        let mark = [
            "x",
            "y",
            "width",
            "height",
            "color",
            "accent_soft",
            "gloss",
            "simple",
        ];
        assert_eq!(header_struct("lxb_mark"), mark);
        assert_eq!(std::mem::size_of::<LxbMark>(), 14 * 4);
    }

    #[test]
    fn a_scene_crosses_the_abi_unchanged() {
        let scene = lxb_scene_for(0, 7.5);
        assert_eq!(scene.time, 7.5);
        assert_eq!(scene.soften, 0.0);
        assert_eq!(scene.style, 0);
        let sky = PALETTES[0].rendered().a(Role::SkyTop, 1.0);
        assert_eq!([scene.sky[0].r, scene.sky[0].g, scene.sky[0].b], sky[..3]);
        let back = scene_from(&scene);
        assert_eq!(back.time, 7.5);
        assert_eq!(back.palette.sky[0], sky);
        assert_eq!(lxb_wallpaper_soften(), lxb_toolkit::wallpaper::SOFTEN);
    }

    #[test]
    fn the_painter_refuses_a_canvas_it_cannot_hold() {
        let mut pixels = vec![0_u8; 16 * 16 * 4];
        let scene = lxb_scene_for(0, 0.0);
        let good = LxbCanvas {
            pixels: pixels.as_mut_ptr(),
            width: 16,
            height: 16,
            stride: 16 * 4,
        };
        let narrow = LxbCanvas { stride: 16, ..good };
        let empty = LxbCanvas { width: 0, ..good };
        let nothing = LxbCanvas {
            pixels: std::ptr::null_mut(),
            ..good
        };
        unsafe {
            assert_eq!(lxb_paint_wallpaper(&good, &scene), 1);
            assert_eq!(lxb_paint_wallpaper(&narrow, &scene), 0);
            assert_eq!(lxb_paint_wallpaper(&empty, &scene), 0);
            assert_eq!(lxb_paint_wallpaper(&nothing, &scene), 0);
            assert_eq!(lxb_paint_wallpaper(std::ptr::null(), &scene), 0);
            assert_eq!(lxb_paint_wallpaper(&good, std::ptr::null()), 0);
            assert_eq!(lxb_paint_glass(&good, std::ptr::null()), 0);
            assert_eq!(lxb_paint_light(&good, std::ptr::null(), BLACK), 0);
            assert_eq!(lxb_paint_scrim(&good, std::ptr::null(), 0.0, BLACK), 0);
            let mark = LxbMark {
                x: 0.0,
                y: 0.0,
                width: 8.0,
                height: 8.0,
                color: BLACK,
                accent_soft: BLACK,
                gloss: 1.0,
                simple: 0,
            };

            let field = [128_u8; 4];
            assert_eq!(lxb_paint_glyph(&good, &mark, field.as_ptr(), 4), 0);
            assert_eq!(lxb_paint_glyph(&good, &mark, std::ptr::null(), 0), 0);
        }

        assert!(pixels.iter().any(|&byte| byte != 0));
    }

    #[test]
    fn the_control_crosses_the_abi_unchanged() {
        use lxb_toolkit::control as c;
        let shape = lxb_control();
        assert_eq!(shape.chip_tint, c::CHIP_TINT);
        assert_eq!(shape.chip_gloss, lxb_toolkit::material::GLOSS_QUIET);
        assert_eq!(shape.lit, c::LIT);
        assert_eq!(shape.lit_answer, c::LIT_ANSWER);
        assert_eq!(shape.glow_width, c::GLOW_WIDTH);
        assert_eq!(shape.padding, c::PADDING);

        assert_eq!(
            Role::ALL[shape.chip_role as usize].name(),
            c::CHIP_ROLE.name()
        );
        assert_eq!(
            Role::ALL[shape.lit_role as usize].name(),
            c::LIT_ROLE.name()
        );
        assert_eq!(
            Role::ALL[shape.out_role as usize].name(),
            c::OUT_ROLE.name()
        );

        assert_eq!(lxb_press_scale(0.0), 1.0);
        assert!(lxb_press_scale(0.3) < 1.0, "it never went down");
        let over = (30..100)
            .map(|n| lxb_press_scale(n as c_float / 100.0))
            .fold(0.0f32, f32::max);
        assert!(over > 1.0, "it never came back past its own size");
        assert_eq!(lxb_press_scale(1.0), 1.0);

        let rect = [10.0f32, 20.0, 200.0, 60.0];
        let mut sunk = [0.0f32; 4];
        unsafe { lxb_pressed(rect.as_ptr(), 0.3, sunk.as_mut_ptr()) };
        assert!(sunk[2] < rect[2] && sunk[3] < rect[3]);
        assert!((sunk[0] + sunk[2] / 2.0 - (rect[0] + rect[2] / 2.0)).abs() < 1e-4);
        unsafe { lxb_pressed(rect.as_ptr(), -1.0, sunk.as_mut_ptr()) };
        assert_eq!(sunk, rect);

        let elsewhere = [10.0f32, 80.0, 200.0, 60.0];
        unsafe {
            assert_eq!(lxb_control_arrival(rect.as_ptr(), rect.as_ptr()), 1.0);
            assert_eq!(lxb_control_arrival(elsewhere.as_ptr(), rect.as_ptr()), 0.0);
        }

        let mut halo = [0.0f32; 4];
        unsafe { lxb_control_glow_rect(rect.as_ptr(), 400.0, -1.0, halo.as_mut_ptr()) };
        assert_eq!(halo[2], 400.0 * c::GLOW_WIDTH);
        assert_eq!(halo[3], rect[3] * c::GLOW_HEIGHT);
    }

    #[test]
    fn the_headers_enumerations_are_the_librarys_own_order() {
        let expected: [(&str, Vec<String>); 7] = [
            (
                "action",
                Action::ALL.iter().map(|a| a.name().to_string()).collect(),
            ),
            (
                "key",
                [
                    "left",
                    "right",
                    "up",
                    "down",
                    "enter",
                    "space",
                    "escape",
                    "backspace",
                    "tab",
                    "backtab",
                    "menu",
                    "f10",
                ]
                .iter()
                .map(|name| name.to_string())
                .collect(),
            ),
            (
                "button",
                [
                    "south",
                    "east",
                    "north",
                    "west",
                    "start",
                    "select",
                    "left-bumper",
                    "right-bumper",
                    "guide",
                    "dpad-left",
                    "dpad-right",
                    "dpad-up",
                    "dpad-down",
                ]
                .iter()
                .map(|name| name.to_string())
                .collect(),
            ),
            (
                "role",
                Role::ALL.iter().map(|r| r.name().to_string()).collect(),
            ),
            (
                "metric",
                Metric::ALL.iter().map(|m| m.name().to_string()).collect(),
            ),
            (
                "text",
                Text::ALL.iter().map(|t| t.name().to_string()).collect(),
            ),
            (
                "surface",
                Surface::ALL.iter().map(|s| s.name().to_string()).collect(),
            ),
        ];

        for (kind, names) in expected {
            let declared = header_enum(kind);
            assert_eq!(
                declared.len(),
                names.len(),
                "the header declares {} lxb_{kind} values and the library has {}",
                declared.len(),
                names.len()
            );
            for (index, ((value, declared_name), name)) in
                declared.iter().zip(names.iter()).enumerate()
            {
                assert_eq!(declared_name, name, "lxb_{kind} {index} is out of order");
                assert_eq!(
                    *value,
                    index,
                    "LXB_{}_{} has the wrong value",
                    kind.to_uppercase(),
                    name.to_uppercase().replace('-', "_")
                );
            }
        }
    }

    #[test]
    fn every_overlay_layer_names_the_role_it_is_drawn_in() {
        let material = lxb_overlay_material_for(1);
        let authored = Overlay::Dialog.material();
        for (got, want) in [
            (material.stain_role, authored.stain_role),
            (material.header_role, authored.header_role),
            (material.foot_role, authored.foot_role),
            (material.rim_role, authored.rim_role),
        ] {
            let name = unsafe { CStr::from_ptr(lxb_role_name(got)) };
            assert_eq!(name.to_bytes(), want.name().as_bytes());
        }
        assert_eq!(
            unsafe { CStr::from_ptr(lxb_role_name(material.stain_role)) }.to_bytes(),
            b"glass"
        );
        assert_eq!(
            unsafe { CStr::from_ptr(lxb_role_name(material.rim_role)) }.to_bytes(),
            b"accent-soft"
        );
    }

    #[test]
    fn coverage_crosses_into_a_caller_owned_distance_field() {
        let size = 4usize;
        let fine = size * lxb_toolkit::glyph_material::SDF_SUPERSAMPLE as usize;
        let coverage: Vec<u8> = (0..fine)
            .flat_map(|y| {
                (0..fine).map(move |x| {
                    if (4..12).contains(&x) && (4..12).contains(&y) {
                        128
                    } else {
                        127
                    }
                })
            })
            .collect();
        let mut field = vec![0u8; size * size];
        unsafe {
            assert_eq!(
                lxb_glyph_sdf(
                    coverage.as_ptr(),
                    coverage.len() as Size,
                    size as c_uint,
                    field.as_mut_ptr(),
                    field.len() as Size,
                ),
                1
            );
            assert_eq!(
                lxb_glyph_sdf(
                    coverage.as_ptr(),
                    coverage.len() as Size,
                    size as c_uint,
                    field.as_mut_ptr(),
                    (field.len() - 1) as Size,
                ),
                0
            );
        }
        assert!(field[size + 1] < 128, "the shape is the negative side");
        assert!(field[0] > 128, "air is the positive side");
    }

    #[test]
    fn compatibility_recordings_remain_assets_not_shell_actions() {
        unsafe {
            assert_eq!(lxb_sound_used(c"press".as_ptr()), 1);
            assert_eq!(lxb_sound_used(c"start-bg-music".as_ptr()), 1);
            assert_eq!(lxb_sound_used(c"trash".as_ptr()), 0);
            assert_eq!(lxb_sound_used(c"error".as_ptr()), 0);
            assert_eq!(lxb_sound_used(c"no-such-sound".as_ptr()), 0);
        }
    }
}
