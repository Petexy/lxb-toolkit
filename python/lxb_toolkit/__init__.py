"""The LineXinBar design language, for Python.

Colour roles, glass, motion, sizes, type, marks and sounds — the same values
the Rust and C bindings answer with, because this is those bindings: everything
here goes through the C ABI of ``liblxb_toolkit``, so there is one
implementation and three ways in rather than three implementations.

    >>> from lxb_toolkit import Accent, Role, ease, PALETTES
    >>> accent = Accent("Purple")
    >>> accent.color(Role.ACCENT).hex
    '#8b5cf6'
    >>> accent.preview("Blue")
    True
    >>> while accent.advance(1 / 60): pass
    >>> accent.color(Role.ACCENT).hex
    '#3b82f6'
    >>> ease(0.5)
    0.5

Nothing here is a widget. It answers what to colour something, how large to
draw it, how long to take and what noise to make — you draw it, with whatever
you already draw with. There are three things worth knowing before you start:

* **Colour comes in two forms.** ``Color.hex`` is what was authored, and what a
  stylesheet or a swatch wants. ``Color.linear`` is light, and is what a
  renderer must have — hand a hex value to an sRGB surface and it comes out
  about twice as bright as it was picked.
* **Sizes are written against a 1080-pixel-tall screen.** Pass the height you
  actually have; height decides the scale, never width.
* **Changing palette is a movement.** Use :class:`Accent` rather than reading a
  palette directly, so that everything you draw reads one blend and the whole
  interface arrives together.

The library is found next to this package, then in the usual places
``ctypes.util.find_library`` looks; ``LXB_TOOLKIT_LIBRARY`` overrides both.
"""

from __future__ import annotations

import ctypes
import ctypes.util
import os
import sys
from dataclasses import dataclass
from enum import IntEnum
from pathlib import Path
from typing import Iterator, Sequence, cast

__all__ = [
    "Accent",
    "Action",
    "action_named",
    "action_of_button",
    "action_of_key",
    "action_of_letter",
    "action_repeats",
    "ACTIONS",
    "Align",
    "App",
    "AppNotFound",
    "Button",
    "capsule_radius",
    "Color",
    "Control",
    "control",
    "control_arrival",
    "control_glow_rect",
    "DURATIONS",
    "ease",
    "EntryKind",
    "Face",
    "fill_arrival",
    "font",
    "Glass",
    "glass",
    "GLASS_IOR",
    "glass_wgsl",
    "glide",
    "glyph",
    "glyph_box",
    "GLYPH_CELL",
    "GLYPH_DEPTH_SHARE",
    "GLYPH_LAMP",
    "glyph_sdf",
    "GLYPH_SDF_RANGE",
    "GLYPH_SDF_SUPERSAMPLE",
    "GLYPH_SHADOW",
    "GLYPH_SIMPLE_ALPHA",
    "GLYPH_SIMPLE_STAIN",
    "GLYPH_SIMPLE_TINT",
    "glyph_wgsl",
    "GLYPHS",
    "IconStyle",
    "INPUT_INITIAL_REPEAT",
    "INPUT_POLL_INTERVAL",
    "INPUT_REPEAT_INTERVAL",
    "INPUT_SCROLL_STEP",
    "INPUT_STICK_ENGAGE",
    "INPUT_STICK_RELEASE",
    "INPUT_TAP_SLOP",
    "is_share",
    "Key",
    "key_light",
    "library_path",
    "LINE_HEIGHT",
    "Menu",
    "menu",
    "menu_aside_radius",
    "menu_chip_radius",
    "menu_content_shown",
    "MENU_DETAIL_LINE",
    "menu_growing",
    "MENU_LABEL_LINE",
    "MENU_LABEL_PADDING",
    "menu_lines_in",
    "MENU_MARGIN",
    "MENU_ROW_PADDING",
    "menu_rows_of_that_fit",
    "menu_rows_that_fit",
    "menu_shown",
    "MENU_STAMP_ROOM",
    "menu_title_growth",
    "menu_title_height",
    "MenuLayout",
    "MenuRow",
    "Metric",
    "Overlay",
    "overlay_material",
    "OverlayMaterial",
    "Page",
    "paint",
    "Palette",
    "palette",
    "PALETTES",
    "Picker",
    "PickerEntry",
    "Press",
    "press_scale",
    "pressed",
    "pulse",
    "REFERENCE_HEIGHT",
    "Repeat",
    "Role",
    "roles",
    "scale_for",
    "Selection",
    "SHELL_SOUNDS",
    "ShellTheme",
    "size",
    "smoothstep",
    "Sound",
    "sound",
    "sound_amplitude",
    "sound_fade",
    "SOUND_MUSIC_FADE_IN",
    "SOUND_MUSIC_FADE_OUT",
    "SOUND_REST",
    "SOUND_RETRY_AFTER",
    "SOUNDS",
    "spring",
    "stylesheet",
    "Surface",
    "Text",
    "text_is_bold",
    "text_size",
    "ToolkitNotFound",
    "version",
    "wallpaper_wgsl",
    "WallpaperStyle",
    "Wheel",
]


# --- finding the library ----------------------------------------------------

class ToolkitNotFound(RuntimeError):
    """The shared library could not be found or could not be loaded.

    Raised at import, with the places that were looked in, because a binding
    that imports and then fails on its first call has moved the error away from
    the thing that caused it.
    """


def _candidates(stem: str) -> list[Path]:
    """Where to look for one of the libraries, nearest first."""
    override = os.environ.get(f"{stem.upper()}_LIBRARY")
    if override:
        return [Path(override)]

    name = {
        "darwin": f"lib{stem}.dylib",
        "win32": f"{stem}.dll",
    }.get(sys.platform, f"lib{stem}.so")

    here = Path(__file__).resolve().parent
    found = [
        # Beside the package, which is where a wheel puts it.
        here / name,
        # And where cargo leaves it, so the repository works without installing
        # anything at all.
        here.parent.parent / "target" / "release" / name,
        here.parent.parent / "target" / "debug" / name,
    ]
    system = ctypes.util.find_library(stem)
    if system:
        found.append(Path(system))
    return found


def _load(stem: str) -> tuple[ctypes.CDLL, Path]:
    """Open one of the libraries, or say where it was looked for.

    Two of them, and the second is optional: `lxb_toolkit` is what the language
    answers and has no dependencies at all, while `lxb_app` carries a GPU
    stack, a window system, the controllers and an audio device. A program that
    only wants the answers must not have to have the second one installed —
    see :mod:`lxb_toolkit.app`, which is the only thing that opens it.
    """
    tried = _candidates(stem)
    for path in tried:
        if path.exists() or not path.is_absolute():
            try:
                return ctypes.CDLL(str(path)), path
            except OSError:
                continue
    raise ToolkitNotFound(
        f"lib{stem} could not be loaded. Build it with `cargo build "
        "--release` in the repository, install it, or point "
        f"{stem.upper()}_LIBRARY at it. Looked in:\n  "
        + "\n  ".join(str(path) for path in tried)
    )


_lib, _library_path = _load("lxb_toolkit")


def library_path() -> Path:
    """Which shared library this package is talking to."""
    return _library_path


def __getattr__(name: str):
    """``App``, ``Page``, ``Align`` and ``Press`` live in :mod:`lxb_toolkit.app`.

    Reached lazily, and that is the whole point of the arrangement: this
    package answers what the language *is* and talks to a library with no
    dependencies at all, while the application layer carries a GPU stack, a
    window system, the controllers and an audio device. A program that only
    wants the answers must not be made to install any of that to get them, so
    the second library is opened by the first program that asks for it and
    never by the import.
    """
    if name in ("App", "Page", "Align", "Press", "AppNotFound"):
        from . import app

        return getattr(app, name)
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


# --- the C types ------------------------------------------------------------

class _Rgba(ctypes.Structure):
    _fields_ = [("r", ctypes.c_float), ("g", ctypes.c_float),
                ("b", ctypes.c_float), ("a", ctypes.c_float)]


class _Glass(ctypes.Structure):
    _fields_ = [("depth", ctypes.c_float), ("frost", ctypes.c_float),
                ("gloss", ctypes.c_float), ("curve", ctypes.c_float)]


class _OverlayMaterial(ctypes.Structure):
    _fields_ = [
        ("radius", ctypes.c_float),
        ("glass", _Glass),
        ("stain", ctypes.c_float),
        ("light_inset", ctypes.c_float),
        ("header_x", ctypes.c_float),
        ("header_width", ctypes.c_float),
        ("header_height", ctypes.c_float),
        ("header_height_share", ctypes.c_float),
        ("header_light", ctypes.c_float),
        ("foot_x", ctypes.c_float),
        ("foot_width", ctypes.c_float),
        ("foot_height", ctypes.c_float),
        ("foot_height_share", ctypes.c_float),
        ("foot_light", ctypes.c_float),
        ("rim", ctypes.c_float),
        ("rim_width", ctypes.c_float),
        ("stain_role", ctypes.c_ulong),
        ("header_role", ctypes.c_ulong),
        ("foot_role", ctypes.c_ulong),
        ("rim_role", ctypes.c_ulong),
    ]


class _Menu(ctypes.Structure):
    _fields_ = [
        ("width", ctypes.c_float),
        ("extra_width", ctypes.c_float),
        ("row", ctypes.c_float),
        ("stacked_row", ctypes.c_float),
        ("title", ctypes.c_float),
        ("title_size", ctypes.c_float),
        ("label_size", ctypes.c_float),
        ("detail_size", ctypes.c_float),
        ("stamp_size", ctypes.c_float),
        ("group_gap", ctypes.c_float),
        ("gap", ctypes.c_float),
        ("glow_reach", ctypes.c_float),
        ("scroll_strip", ctypes.c_float),
        ("scroll_arrow", ctypes.c_float),
        ("dim", ctypes.c_float),
        ("scrim", ctypes.c_float),
        ("depth", ctypes.c_float),
        ("content_in", ctypes.c_float),
        ("panel_in", ctypes.c_float),
        ("icon", ctypes.c_float),
        ("glyph", ctypes.c_float),
        ("aside", ctypes.c_float),
        ("aside_gap", ctypes.c_float),
        ("aside_glyph", ctypes.c_float),
        ("max_lines", ctypes.c_uint),
    ]


class _Control(ctypes.Structure):
    _fields_ = [
        ("chip_role", ctypes.c_uint),
        ("chip_tint", ctypes.c_float),
        ("chip_gloss", ctypes.c_float),
        ("aside_tint", ctypes.c_float),
        ("padding", ctypes.c_float),
        ("lit_role", ctypes.c_uint),
        ("lit", ctypes.c_float),
        ("lit_answer", ctypes.c_float),
        ("lit_pulse", ctypes.c_float),
        ("glow", ctypes.c_float),
        ("glow_pulse", ctypes.c_float),
        ("glow_width", ctypes.c_float),
        ("glow_height", ctypes.c_float),
        ("out_role", ctypes.c_uint),
        ("out", ctypes.c_float),
        ("out_width", ctypes.c_float),
        ("ink", ctypes.c_float),
        ("ink_quiet", ctypes.c_float),
        ("mark", ctypes.c_float),
        ("mark_quiet", ctypes.c_float),
    ]


class _Canvas(ctypes.Structure):
    _fields_ = [("pixels", ctypes.POINTER(ctypes.c_ubyte)),
                ("width", ctypes.c_uint),
                ("height", ctypes.c_uint),
                ("stride", ctypes.c_ulong)]


class _Scene(ctypes.Structure):
    _fields_ = [
        ("time", ctypes.c_float),
        ("soften", ctypes.c_float),
        ("style", ctypes.c_ulong),
        ("sky", _Rgba * 4),
        ("accent", _Rgba * 3),
        ("glow", _Rgba),
    ]


class _Pane(ctypes.Structure):
    _fields_ = [
        ("x", ctypes.c_float),
        ("y", ctypes.c_float),
        ("width", ctypes.c_float),
        ("height", ctypes.c_float),
        ("radius", ctypes.c_float),
        ("power", ctypes.c_float),
        ("glass", _Glass),
        ("tint", _Rgba),
        ("opacity", ctypes.c_float),
    ]


class _Mark(ctypes.Structure):
    _fields_ = [
        ("x", ctypes.c_float),
        ("y", ctypes.c_float),
        ("width", ctypes.c_float),
        ("height", ctypes.c_float),
        ("color", _Rgba),
        ("accent_soft", _Rgba),
        ("gloss", ctypes.c_float),
        ("simple", ctypes.c_int),
    ]


class _MenuRow(ctypes.Structure):
    _fields_ = [
        ("stacked", ctypes.c_int),
        ("stamp", ctypes.c_int),
        ("aside", ctypes.c_int),
        ("reading", ctypes.c_int),
        ("group", ctypes.c_uint),
        ("lines", ctypes.c_uint),
        ("detail_lines", ctypes.c_uint),
    ]


class _Bytes(ctypes.Structure):
    _fields_ = [("data", ctypes.POINTER(ctypes.c_ubyte)),
                ("len", ctypes.c_ulong)]


class _ShellTheme(ctypes.Structure):
    _fields_ = [("accent", ctypes.c_ulong),
                ("wallpaper", ctypes.c_int),
                ("icons", ctypes.c_int)]


_SIZE = ctypes.c_ulong
_STR = ctypes.c_char_p
_ACCENT = ctypes.c_void_p

# Every signature, declared up front. ctypes defaults an unknown return type to
# int, which on this ABI truncates a pointer — so a binding that forgets one
# does not fail, it corrupts.
for _name, _argtypes, _restype in [
    ("lxb_palette_count", [], _SIZE),
    ("lxb_palette_name", [_SIZE], _STR),
    ("lxb_palette_index", [_STR], ctypes.c_int),
    ("lxb_role_count", [], _SIZE),
    ("lxb_role_name", [_SIZE], _STR),
    ("lxb_palette_color", [_SIZE, _SIZE], ctypes.c_uint),
    ("lxb_palette_rgba", [_SIZE, _SIZE, ctypes.c_float], _Rgba),
    ("lxb_srgb_to_linear", [ctypes.c_uint, ctypes.c_float], _Rgba),
    ("lxb_linear_to_srgb", [_Rgba], ctypes.c_uint),
    ("lxb_accent_new", [_STR], _ACCENT),
    ("lxb_accent_free", [_ACCENT], None),
    ("lxb_accent_preview", [_ACCENT, _STR], ctypes.c_int),
    ("lxb_accent_commit", [_ACCENT, _STR], ctypes.c_int),
    ("lxb_accent_set", [_ACCENT, _STR], ctypes.c_int),
    ("lxb_accent_restore", [_ACCENT], None),
    ("lxb_accent_advance", [_ACCENT, ctypes.c_float], ctypes.c_int),
    ("lxb_accent_color", [_ACCENT, _SIZE, ctypes.c_float], _Rgba),
    ("lxb_accent_applied", [_ACCENT], _STR),
    ("lxb_icon_style_count", [], _SIZE),
    ("lxb_icon_style_name", [_SIZE], _STR),
    ("lxb_wallpaper_style_count", [], _SIZE),
    ("lxb_wallpaper_style_name", [_SIZE], _STR),
    ("lxb_shell_theme_load", [], _ShellTheme),
    ("lxb_ease", [ctypes.c_float], ctypes.c_float),
    ("lxb_smoothstep", [ctypes.c_float], ctypes.c_float),
    ("lxb_spring", [ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double),
                    ctypes.c_double, ctypes.c_double, ctypes.c_double], None),
    ("lxb_card_spring", [], ctypes.c_double),
    ("lxb_duration_count", [], _SIZE),
    ("lxb_duration_name", [_SIZE], _STR),
    ("lxb_duration", [_SIZE], ctypes.c_float),
    ("lxb_duration_named", [_STR], ctypes.c_float),
    ("lxb_reference_height", [], ctypes.c_float),
    ("lxb_scale_for", [ctypes.c_float], ctypes.c_float),
    ("lxb_capsule_radius", [ctypes.c_float], ctypes.c_float),
    ("lxb_metric_count", [], _SIZE),
    ("lxb_metric_name", [_SIZE], _STR),
    ("lxb_metric", [_SIZE, ctypes.c_float], ctypes.c_float),
    ("lxb_metric_is_share", [_SIZE], ctypes.c_int),
    ("lxb_text_count", [], _SIZE),
    ("lxb_text_name", [_SIZE], _STR),
    ("lxb_text_size", [_SIZE, ctypes.c_float], ctypes.c_float),
    ("lxb_text_is_bold", [_SIZE], ctypes.c_int),
    ("lxb_text_line_height", [], ctypes.c_float),
    ("lxb_surface_count", [], _SIZE),
    ("lxb_surface_name", [_SIZE], _STR),
    ("lxb_surface_glass", [_SIZE], _Glass),
    ("lxb_overlay_count", [], _SIZE),
    ("lxb_overlay_name", [_SIZE], _STR),
    ("lxb_overlay_material_for", [_SIZE], _OverlayMaterial),
    ("lxb_context_menu", [], _Menu),
    ("lxb_menu_rows_that_fit", [ctypes.c_float, ctypes.c_float], _SIZE),
    ("lxb_menu_growing",
     [ctypes.POINTER(ctypes.c_float), ctypes.POINTER(ctypes.c_float),
      ctypes.c_float, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_menu_shown", [ctypes.c_float, ctypes.c_int], ctypes.c_float),
    ("lxb_menu_content_shown", [ctypes.c_float, ctypes.c_int], ctypes.c_float),
    ("lxb_control", [], _Control),
    ("lxb_control_glow_rect",
     [ctypes.POINTER(ctypes.c_float), ctypes.c_float, ctypes.c_float,
      ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_control_arrival",
     [ctypes.POINTER(ctypes.c_float), ctypes.POINTER(ctypes.c_float)],
     ctypes.c_float),
    ("lxb_press_scale", [ctypes.c_float], ctypes.c_float),
    ("lxb_fill_arrival", [ctypes.c_float], ctypes.c_float),
    ("lxb_pulse", [ctypes.c_float], ctypes.c_float),
    ("lxb_pressed",
     [ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_glide",
     [ctypes.POINTER(ctypes.c_float), ctypes.POINTER(ctypes.c_float),
      ctypes.c_float, ctypes.c_float], None),
    ("lxb_key_light", [ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_glass_ior", [], ctypes.c_float),
    ("lxb_glass_wgsl", [], _Bytes),
    ("lxb_wallpaper_wgsl", [], _Bytes),
    ("lxb_glyph_count", [], _SIZE),
    ("lxb_glyph_name", [_SIZE], _STR),
    ("lxb_glyph", [_STR], _Bytes),
    ("lxb_glyph_box", [_STR], ctypes.c_uint),
    ("lxb_glyph_sdf", [ctypes.POINTER(ctypes.c_ubyte), _SIZE, ctypes.c_uint,
                       ctypes.POINTER(ctypes.c_ubyte), _SIZE], ctypes.c_int),
    ("lxb_glyph_wgsl", [], _Bytes),
    ("lxb_glyph_sdf_range", [], ctypes.c_float),
    ("lxb_glyph_sdf_supersample", [], ctypes.c_uint),
    ("lxb_glyph_cell", [], ctypes.c_uint),
    ("lxb_glyph_depth_share", [], ctypes.c_float),
    ("lxb_glyph_lamp", [ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_glyph_shadow", [], ctypes.c_float),
    ("lxb_glyph_simple_tint", [], ctypes.c_float),
    ("lxb_glyph_simple_alpha", [], ctypes.c_float),
    ("lxb_glyph_simple_stain", [], ctypes.c_float),
    ("lxb_sound_count", [], _SIZE),
    ("lxb_sound_name", [_SIZE], _STR),
    ("lxb_sound", [_STR], _Bytes),
    ("lxb_sound_used", [_STR], ctypes.c_int),
    ("lxb_sound_rest", [], ctypes.c_float),
    ("lxb_sound_amplitude", [ctypes.c_float], ctypes.c_float),
    ("lxb_sound_fade", [ctypes.c_float, ctypes.c_float], ctypes.c_float),
    ("lxb_sound_music_fade_in", [], ctypes.c_float),
    ("lxb_sound_music_fade_out", [], ctypes.c_float),
    ("lxb_sound_retry_after", [], ctypes.c_float),
    ("lxb_action_count", [], _SIZE),
    ("lxb_action_name", [_SIZE], _STR),
    ("lxb_action_named", [_STR], ctypes.c_int),
    ("lxb_action_repeats", [ctypes.c_int], ctypes.c_int),
    ("lxb_action_of_key", [ctypes.c_int], ctypes.c_int),
    ("lxb_action_of_letter", [ctypes.c_uint], ctypes.c_int),
    ("lxb_action_of_button", [ctypes.c_int], ctypes.c_int),
    ("lxb_input_poll_interval", [], ctypes.c_float),
    ("lxb_input_initial_repeat", [], ctypes.c_float),
    ("lxb_input_repeat_interval", [], ctypes.c_float),
    ("lxb_input_stick_engage", [], ctypes.c_float),
    ("lxb_input_stick_release", [], ctypes.c_float),
    ("lxb_input_scroll_step", [], ctypes.c_float),
    ("lxb_input_tap_slop", [], ctypes.c_float),
    ("lxb_repeat_new", [], ctypes.c_void_p),
    ("lxb_repeat_free", [ctypes.c_void_p], None),
    ("lxb_repeat_reset", [ctypes.c_void_p], None),
    ("lxb_repeat_update",
     [ctypes.c_void_p, ctypes.c_float, ctypes.POINTER(ctypes.c_int),
      ctypes.c_float, ctypes.c_float, ctypes.POINTER(ctypes.c_int), _SIZE], _SIZE),
    ("lxb_wheel_notches",
     [ctypes.POINTER(ctypes.c_float), ctypes.c_float], ctypes.c_int),
    ("lxb_picker_new", [_SIZE, _STR], ctypes.c_void_p),
    ("lxb_picker_free", [ctypes.c_void_p], None),
    ("lxb_picker_location", [ctypes.c_void_p], _STR),
    ("lxb_picker_note", [ctypes.c_void_p], _STR),
    ("lxb_picker_query", [ctypes.c_void_p], _STR),
    ("lxb_picker_selection", [ctypes.c_void_p], _SIZE),
    ("lxb_picker_can_search", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_picker_can_choose", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_picker_entry_count", [ctypes.c_void_p], _SIZE),
    ("lxb_picker_entry_name", [ctypes.c_void_p, _SIZE], _STR),
    ("lxb_picker_entry_path", [ctypes.c_void_p, _SIZE], _STR),
    ("lxb_picker_entry_kind", [ctypes.c_void_p, _SIZE], ctypes.c_int),
    ("lxb_picker_selected", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_picker_select", [ctypes.c_void_p, _SIZE], ctypes.c_int),
    ("lxb_picker_move", [ctypes.c_void_p, ctypes.c_int], ctypes.c_int),
    ("lxb_picker_enter", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_picker_leave", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_picker_refresh", [ctypes.c_void_p], None),
    ("lxb_picker_search", [ctypes.c_void_p, _STR], None),
    ("lxb_picker_choose", [ctypes.c_void_p], _STR),
    ("lxb_font", [ctypes.c_int], _Bytes),
    ("lxb_font_for", [ctypes.c_char_p, ctypes.c_int], _Bytes),
    ("lxb_version", [], _STR),
    ("lxb_menu_layout_new",
     [ctypes.POINTER(_MenuRow), _SIZE, ctypes.c_uint,
      ctypes.POINTER(ctypes.c_float), ctypes.POINTER(ctypes.c_float),
      _SIZE, _SIZE, _SIZE, ctypes.c_float, ctypes.c_float], ctypes.c_void_p),
    ("lxb_menu_layout_free", [ctypes.c_void_p], None),
    ("lxb_menu_rows_of_that_fit",
     [ctypes.POINTER(_MenuRow), _SIZE, ctypes.c_uint, ctypes.c_float], _SIZE),
    ("lxb_menu_panel", [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_menu_rows_top", [ctypes.c_void_p], ctypes.c_float),
    ("lxb_menu_row_rect",
     [ctypes.c_void_p, _SIZE, ctypes.POINTER(ctypes.c_float)], ctypes.c_int),
    ("lxb_menu_chip_rect",
     [ctypes.c_void_p, _SIZE, ctypes.POINTER(ctypes.c_float)], ctypes.c_int),
    ("lxb_menu_aside_rect",
     [ctypes.c_void_p, _SIZE, ctypes.POINTER(ctypes.c_float)], ctypes.c_int),
    ("lxb_menu_highlight_rect",
     [ctypes.c_void_p, ctypes.c_int, ctypes.POINTER(ctypes.c_float)], ctypes.c_int),
    ("lxb_menu_separator",
     [ctypes.c_void_p, _SIZE, ctypes.POINTER(ctypes.c_float)], ctypes.c_int),
    ("lxb_menu_opened",
     [ctypes.c_void_p, _SIZE, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_menu_chip_radius",
     [ctypes.POINTER(_MenuRow), ctypes.c_float, ctypes.c_float], ctypes.c_float),
    ("lxb_menu_aside_radius",
     [ctypes.POINTER(_MenuRow), ctypes.c_float], ctypes.c_float),
    ("lxb_menu_lines_in", [ctypes.c_float, ctypes.c_float], ctypes.c_uint),
    ("lxb_menu_title_height", [ctypes.c_uint], ctypes.c_float),
    ("lxb_menu_title_growth", [ctypes.c_uint], ctypes.c_float),
    ("lxb_menu_margin", [], ctypes.c_float),
    ("lxb_menu_label_padding", [], ctypes.c_float),
    ("lxb_menu_row_padding", [], ctypes.c_float),
    ("lxb_menu_label_line", [], ctypes.c_float),
    ("lxb_menu_detail_line", [], ctypes.c_float),
    ("lxb_menu_stamp_room", [], ctypes.c_float),
    ("lxb_scene_for", [_SIZE, ctypes.c_float], _Scene),
    ("lxb_accent_scene", [_ACCENT, ctypes.c_float], _Scene),
    ("lxb_paint_wallpaper",
     [ctypes.POINTER(_Canvas), ctypes.POINTER(_Scene)], ctypes.c_int),
    ("lxb_paint_glass",
     [ctypes.POINTER(_Canvas), ctypes.POINTER(_Pane)], ctypes.c_int),
    ("lxb_paint_light",
     [ctypes.POINTER(_Canvas), ctypes.POINTER(ctypes.c_float), _Rgba],
     ctypes.c_int),
    ("lxb_paint_scrim",
     [ctypes.POINTER(_Canvas), ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      _Rgba], ctypes.c_int),
    ("lxb_paint_glyph",
     [ctypes.POINTER(_Canvas), ctypes.POINTER(_Mark),
      ctypes.POINTER(ctypes.c_ubyte), ctypes.c_ulong], ctypes.c_int),
    ("lxb_wallpaper_soften", [], ctypes.c_float),
    ("lxb_stylesheet", [_STR], ctypes.POINTER(ctypes.c_char)),
    ("lxb_string_free", [ctypes.POINTER(ctypes.c_char)], None),
]:
    _fn = getattr(_lib, _name)
    _fn.argtypes = _argtypes
    _fn.restype = _restype


def _text(pointer) -> str:
    return pointer.decode("utf-8") if pointer else ""


def _bytes(handed: _Bytes) -> bytes | None:
    if not handed.data:
        return None
    return bytes(bytearray(handed.data[: handed.len]))


# --- colour -----------------------------------------------------------------

@dataclass(frozen=True)
class Color:
    """One colour, in both forms.

    ``hex`` is the authored sRGB value; ``linear`` is the light, which is what
    a renderer wants. They are the same colour — see the module docstring for
    why keeping them apart is not pedantry.
    """

    rgb: int
    alpha: float = 1.0

    @property
    def hex(self) -> str:
        return f"#{self.rgb & 0xFFFFFF:06x}"

    @property
    def bytes(self) -> tuple[int, int, int]:
        return (self.rgb >> 16 & 0xFF, self.rgb >> 8 & 0xFF, self.rgb & 0xFF)

    @property
    def linear(self) -> tuple[float, float, float, float]:
        got = _lib.lxb_srgb_to_linear(self.rgb, self.alpha)
        return (got.r, got.g, got.b, got.a)

    def with_alpha(self, alpha: float) -> "Color":
        return Color(self.rgb, alpha)

    def __str__(self) -> str:
        return self.hex


def _color_from(rgba: _Rgba) -> Color:
    return Color(_lib.lxb_linear_to_srgb(rgba), rgba.a)


def _named_enum(name: str, count_fn, name_fn) -> type[IntEnum]:
    """Build an enumeration from the library rather than from a list here.

    The C side is the authority on what exists and in what order. A Python
    binding that kept its own copy would be a second list to forget to update,
    and the first anybody would know of it is a colour role coming back as the
    wrong colour.
    """
    members = {}
    for index in range(count_fn()):
        label = _text(name_fn(index)).upper().replace("-", "_")
        members[label] = index
    return IntEnum(name, members)


# These classes are the editor-facing view of the named C ABI.  The actual
# classes still come from the library below, so a new ABI can never silently
# receive an old numeric mapping.  ``cast`` retains this declared surface for
# static analysers: they cannot infer attributes created by IntEnum's
# functional form.
class Role(IntEnum):
    ACCENT = 0
    ACCENT_SOFT = 1
    ACCENT_DEEP = 2
    GLASS = 3
    GLASS_RAISED = 4
    RIM = 5
    TEXT = 6
    TEXT_SOFT = 7
    DANGER = 8
    GLOW = 9
    SKY_TOP = 10
    SKY_BOTTOM = 11
    SKY_TOP_ALT = 12
    SKY_BOTTOM_ALT = 13


class Metric(IntEnum):
    CARD_RADIUS = 0
    PANEL_RADIUS = 1
    PANEL_INSET = 2
    ROW_HEIGHT = 3
    ROW_PADDING = 4
    PANEL_PADDING = 5
    TILE = 6
    GAP = 7
    TILE_GLYPH = 8
    TILE_RADIUS = 9
    ITEM_SPACING = 10
    COLUMN_SPACING = 11
    ITEM_ICON = 12
    ITEM_ICON_FOCUSED = 13
    COLUMN_ICON = 14
    COLUMN_ICON_FOCUSED = 15
    MENU_WIDTH = 16
    DIALOG_WIDTH = 17
    DIALOG_DIM = 18
    POWER_WIDTH = 19
    POWER_DIM = 20


class Text(IntEnum):
    DISPLAY = 0
    TITLE = 1
    BODY = 2
    LABEL = 3
    CAPTION = 4


class Surface(IntEnum):
    PANEL = 0
    CONTROL = 1
    SIDEBAR = 2


class Overlay(IntEnum):
    CONTEXT_MENU = 0
    DIALOG = 1


class IconStyle(IntEnum):
    DEFAULT = 0
    SIMPLE = 1


class WallpaperStyle(IntEnum):
    DEFAULT = 0
    SIMPLE = 1
    CUSTOM = 2


Role = cast(type[Role], _named_enum("Role", _lib.lxb_role_count, _lib.lxb_role_name))
Metric = cast(
    type[Metric], _named_enum("Metric", _lib.lxb_metric_count, _lib.lxb_metric_name)
)
Text = cast(type[Text], _named_enum("Text", _lib.lxb_text_count, _lib.lxb_text_name))
Surface = cast(
    type[Surface], _named_enum("Surface", _lib.lxb_surface_count, _lib.lxb_surface_name)
)
Overlay = cast(
    type[Overlay], _named_enum("Overlay", _lib.lxb_overlay_count, _lib.lxb_overlay_name)
)
IconStyle = cast(
    type[IconStyle],
    _named_enum("IconStyle", _lib.lxb_icon_style_count, _lib.lxb_icon_style_name),
)
WallpaperStyle = cast(
    type[WallpaperStyle],
    _named_enum(
        "WallpaperStyle",
        _lib.lxb_wallpaper_style_count,
        _lib.lxb_wallpaper_style_name,
    ),
)

Role.__doc__ = """What a colour is *for*.

Nothing in this language is coloured by picking a colour; it is coloured by
naming the role and letting the palette answer. That is what lets a whole
interface change accent in one move.
"""

Metric.__doc__ = "A named size, against a 1080-tall screen. See :func:`size`."

Text.__doc__ = "A step of the type scale."

Surface.__doc__ = "What kind of pane something is: the three cuts of glass."

Overlay.__doc__ = "Context menu or dialog: two uses of one layered material."

IconStyle.__doc__ = "How the shell's own marks are made: Default or Simple."

WallpaperStyle.__doc__ = "What stands behind the shell: Default, Simple or Custom."


class Face(IntEnum):
    """The two faces. Nothing is italic."""

    REGULAR = 0
    BOLD = 1


# --- palettes ---------------------------------------------------------------

@dataclass(frozen=True)
class Palette:
    """One palette, as authored. Index it by :class:`Role`."""

    index: int
    name: str

    def color(self, role: Role) -> Color:
        return Color(_lib.lxb_palette_color(self.index, int(role)))

    def __getitem__(self, role: Role) -> Color:
        return self.color(role)

    def colors(self) -> dict[str, Color]:
        """Every role, under the name the other bindings know it by."""
        return {
            _text(_lib.lxb_role_name(index)): self.color(Role(index))
            for index in range(len(Role))
        }


PALETTES: tuple[Palette, ...] = tuple(
    Palette(index, _text(_lib.lxb_palette_name(index)))
    for index in range(_lib.lxb_palette_count())
)


@dataclass(frozen=True)
class ShellTheme:
    """The current shell accent, wallpaper and icon materials.

    Applications only read this setting. Missing, unreadable and unknown
    configuration is represented by Purple and the Default materials.
    """

    accent: Palette
    wallpaper: WallpaperStyle
    icons: IconStyle

    @classmethod
    def load(cls) -> "ShellTheme":
        """Read ``$XDG_CONFIG_HOME/lxb/shell.toml``, falling back through HOME."""
        got = _lib.lxb_shell_theme_load()
        accent = PALETTES[got.accent] if got.accent < len(PALETTES) else PALETTES[0]
        try:
            wallpaper = WallpaperStyle(got.wallpaper)
        except ValueError:
            wallpaper = WallpaperStyle.DEFAULT
        try:
            icons = IconStyle(got.icons)
        except ValueError:
            icons = IconStyle.DEFAULT
        return cls(accent, wallpaper, icons)


def palette(name: str) -> Palette | None:
    """The palette of that name, however capitalised."""
    index = _lib.lxb_palette_index(name.encode())
    return PALETTES[index] if index >= 0 else None


class Accent:
    """A palette being walked, chosen and left.

    Preview shows a colour on the whole interface without choosing it; commit
    chooses it; restore flows back to what was chosen. Advance once a frame and
    read the colours from here rather than from a :class:`Palette` — two
    surfaces that each looked the palette up separately would be a frame apart
    on a slow frame, which is the one thing this exists to prevent.
    """

    def __init__(self, name: str = "Purple") -> None:
        handle = _lib.lxb_accent_new(name.encode())
        if not handle:
            raise ValueError(f"no palette is called {name!r}")
        self._handle = handle

    def __del__(self) -> None:
        # Guarded: an Accent whose constructor raised has no handle, and
        # interpreter teardown can take the library away before the objects
        # using it.
        handle = getattr(self, "_handle", None)
        if handle and _lib is not None:
            _lib.lxb_accent_free(handle)
            self._handle = None

    def __enter__(self) -> "Accent":
        return self

    def __exit__(self, *_exc: object) -> None:
        self.__del__()

    def preview(self, name: str) -> bool:
        """Travel towards a palette without choosing it."""
        return bool(_lib.lxb_accent_preview(self._handle, name.encode()))

    def commit(self, name: str) -> bool:
        """Choose one, keeping any preview already travelling towards it."""
        return bool(_lib.lxb_accent_commit(self._handle, name.encode()))

    def set(self, name: str) -> bool:
        """Both at once, with no animation: the startup path."""
        return bool(_lib.lxb_accent_set(self._handle, name.encode()))

    def restore(self) -> None:
        """Leave a preview behind and flow back to what was chosen."""
        _lib.lxb_accent_restore(self._handle)

    def advance(self, dt: float) -> bool:
        """One frame. True while another is needed."""
        return bool(_lib.lxb_accent_advance(self._handle, dt))

    def color(self, role: Role, alpha: float = 1.0) -> Color:
        """The colour of a role on screen this frame."""
        return _color_from(_lib.lxb_accent_color(self._handle, int(role), alpha))

    @property
    def applied(self) -> Palette:
        """Which palette owns the setting — not what a preview is showing."""
        name = _text(_lib.lxb_accent_applied(self._handle))
        found = palette(name)
        assert found is not None
        return found


# --- motion -----------------------------------------------------------------

def ease(t: float) -> float:
    """Cubic ease-in-out: the shape for anything on its own clock."""
    return _lib.lxb_ease(t)


def smoothstep(t: float) -> float:
    """The other gentle ramp."""
    return _lib.lxb_smoothstep(t)


def spring(position: float, velocity: float, target: float,
           rate: float | None = None, dt: float = 1 / 60) -> tuple[float, float]:
    """One step of a critically damped spring, for a target that can move.

    Returns where it is and how fast it is going. It accelerates from rest and
    settles without ever crossing.
    """
    at = ctypes.c_double(position)
    speed = ctypes.c_double(velocity)
    if rate is None:
        rate = _lib.lxb_card_spring()
    _lib.lxb_spring(ctypes.byref(at), ctypes.byref(speed), target, rate, dt)
    return at.value, speed.value


DURATIONS: dict[str, float] = {
    _text(_lib.lxb_duration_name(index)): _lib.lxb_duration(index)
    for index in range(_lib.lxb_duration_count())
}
"""Every duration, in seconds, under its name.

Carrying the precision of the C floats they come from, so ``DURATIONS["flight"]``
is 0.30000001192092896 rather than 0.3 — compare with a tolerance, or format
them for display, rather than testing one for equality.
"""


# --- sizes and type ---------------------------------------------------------

REFERENCE_HEIGHT: float = _lib.lxb_reference_height()


def scale_for(height: float) -> float:
    """The height/1080 multiplier, clamped to the shell's 0.6..2.5 range."""
    return _lib.lxb_scale_for(height)


def size(metric: Metric, height: float = REFERENCE_HEIGHT) -> float:
    """A named size on a screen that tall. A share comes back unscaled."""
    return _lib.lxb_metric(int(metric), height)


def is_share(metric: Metric) -> bool:
    """Whether a metric is a fraction rather than a length."""
    return bool(_lib.lxb_metric_is_share(int(metric)))


def capsule_radius(height: float) -> float:
    """The radius of a control that tall: half of it, always."""
    return _lib.lxb_capsule_radius(height)


def text_size(text: Text, height: float = REFERENCE_HEIGHT) -> float:
    """A step of the type scale on a screen that tall."""
    return _lib.lxb_text_size(int(text), height)


def text_is_bold(text: Text) -> bool:
    return bool(_lib.lxb_text_is_bold(int(text)))


LINE_HEIGHT: float = _lib.lxb_text_line_height()


# --- glass ------------------------------------------------------------------

@dataclass(frozen=True)
class Glass:
    """A pane, as four numbers. See :mod:`lxb_toolkit` on the material."""

    depth: float
    frost: float
    gloss: float
    curve: float


@dataclass(frozen=True)
class OverlayMaterial:
    """The complete four-layer context-menu/dialog material.

    Lengths are reference pixels. ``header_x``, ``header_width``, ``foot_x``
    and ``foot_width`` are shares of the pane width; each light height is the
    smaller of its scaled pixel cap and its pane-height share.

    The four roles say which colour each layer is drawn in. They are named
    rather than guessed for the same reason everything else here is.
    """

    radius: float
    glass: Glass
    stain: float
    light_inset: float
    header_x: float
    header_width: float
    header_height: float
    header_height_share: float
    header_light: float
    foot_x: float
    foot_width: float
    foot_height: float
    foot_height_share: float
    foot_light: float
    rim: float
    rim_width: float
    stain_role: Role
    header_role: Role
    foot_role: Role
    rim_role: Role


def glass(surface: Surface) -> Glass:
    """The material a kind of surface is cut from."""
    got = _lib.lxb_surface_glass(int(surface))
    return Glass(got.depth, got.frost, got.gloss, got.curve)


def overlay_material(overlay: Overlay) -> OverlayMaterial:
    """The layered pane recipe for a context menu or general dialog.

    Both semantic uses intentionally return equal values. Their width, scrim
    and content may differ; their pane material must not.
    """
    got = _lib.lxb_overlay_material_for(int(overlay))
    return OverlayMaterial(
        got.radius,
        Glass(got.glass.depth, got.glass.frost, got.glass.gloss, got.glass.curve),
        got.stain,
        got.light_inset,
        got.header_x,
        got.header_width,
        got.header_height,
        got.header_height_share,
        got.header_light,
        got.foot_x,
        got.foot_width,
        got.foot_height,
        got.foot_height_share,
        got.foot_light,
        got.rim,
        got.rim_width,
        Role(got.stain_role),
        Role(got.header_role),
        Role(got.foot_role),
        Role(got.rim_role),
    )


@dataclass(frozen=True)
class Menu:
    """The context menu's shape.

    What :func:`overlay_material` is made of, drawn at the right size. Lengths
    are reference pixels; ``glow_reach``, ``dim``, ``scrim``, ``depth``,
    ``content_in``, ``icon``, ``glyph``, ``aside`` and ``aside_glyph`` are
    shares of something and must not be scaled.
    """

    width: float
    extra_width: float
    row: float
    stacked_row: float
    title: float
    title_size: float
    label_size: float
    detail_size: float
    stamp_size: float
    group_gap: float
    gap: float
    glow_reach: float
    scroll_strip: float
    scroll_arrow: float
    dim: float
    scrim: float
    depth: float
    content_in: float
    panel_in: float
    icon: float
    glyph: float
    aside: float
    aside_gap: float
    aside_glyph: float
    max_lines: int


def menu() -> Menu:
    """The context menu's shape, as one value.

    Handed over together rather than read one at a time: a consumer that took
    half of these and invented the rest would draw something that is nearly
    this menu, and nearly is the one thing a design language cannot be.
    """
    got = _lib.lxb_context_menu()
    return Menu(got.width, got.extra_width, got.row, got.stacked_row, got.title, got.title_size, got.label_size, got.detail_size, got.stamp_size, got.group_gap, got.gap, got.glow_reach, got.scroll_strip, got.scroll_arrow, got.dim, got.scrim, got.depth, got.content_in, got.panel_in, got.icon, got.glyph, got.aside, got.aside_gap, got.aside_glyph, got.max_lines)


# --- the panel's shape ------------------------------------------------------

#: The panel's own lengths, in reference pixels.
MENU_MARGIN: float = _lib.lxb_menu_margin()
MENU_LABEL_PADDING: float = _lib.lxb_menu_label_padding()
MENU_ROW_PADDING: float = _lib.lxb_menu_row_padding()
MENU_LABEL_LINE: float = _lib.lxb_menu_label_line()
MENU_DETAIL_LINE: float = _lib.lxb_menu_detail_line()
MENU_STAMP_ROOM: float = _lib.lxb_menu_stamp_room()


@dataclass(frozen=True)
class MenuRow:
    """What the panel needs to know about one row in order to measure it.

    Not the row — you draw that. These are the things that decide how much of
    the column it takes and where everything on it lands. The default is a
    plain command: one line, nothing on either end, in the first band.
    """

    stacked: bool = False
    stamp: bool = False
    aside: bool = False
    reading: bool = False
    group: int = 0
    lines: int = 1
    detail_lines: int = 1

    def _raw(self) -> _MenuRow:
        return _MenuRow(
            1 if self.stacked else 0,
            1 if self.stamp else 0,
            1 if self.aside else 0,
            1 if self.reading else 0,
            int(self.group),
            int(self.lines),
            int(self.detail_lines),
        )


def _rows(rows) -> tuple:
    array = (_MenuRow * len(rows))(*(row._raw() for row in rows))
    return array, len(rows)


def menu_rows_of_that_fit(rows, title_lines: int, height: float) -> int:
    """How many rows of this list a display that tall has room for.

    Room for the one row that will open out is held back, because only the
    selected row grows: without it the panel, which is sized to its rows, would
    grow past the bottom of the display the moment somebody stopped on a long
    one. ``title_lines`` is 0 for a panel raised without a header.
    """
    array, count = _rows(rows)
    return int(_lib.lxb_menu_rows_of_that_fit(array, count, title_lines, height))


class MenuLayout:
    """A settled panel: where it stands, and every rectangle on it.

    Measured once and asked, rather than recomputed by each of the half-dozen
    callers that need a piece of it. Two places computing one rectangle have to
    agree, and the two that matter are the light and the press: the moment they
    disagree the highlight is on one row and the press lands on another.

    Close it, or use it as a context manager. The panel it describes is the
    *settled* one — where it ends up — and :func:`menu_growing` says where it
    is on the way there.
    """

    __slots__ = ("_handle", "_four", "_two")

    def __init__(self, rows, title_lines: int, anchor, display,
                 first: int = 0, visible: int = 0, selected: int = 0,
                 unfolded: float = 0.0, extra: float = 0.0) -> None:
        array, count = _rows(rows)
        if visible <= 0:
            visible = count
        self._four = (ctypes.c_float * 4)()
        self._two = (ctypes.c_float * 2)()
        self._handle = _lib.lxb_menu_layout_new(
            array, count, title_lines,
            (ctypes.c_float * 4)(*anchor), (ctypes.c_float * 2)(*display),
            first, visible, selected, unfolded, extra,
        )
        if not self._handle:
            raise ValueError("a panel with no rows on it is not a panel")

    def close(self) -> None:
        if getattr(self, "_handle", None):
            _lib.lxb_menu_layout_free(self._handle)
            self._handle = None

    __del__ = close

    def __enter__(self) -> "MenuLayout":
        return self

    def __exit__(self, *_exception) -> None:
        self.close()

    def _rect(self, call, *args):
        got = call(self._handle, *args, self._four)
        return tuple(self._four) if got else None

    @property
    def panel(self) -> tuple[float, float, float, float]:
        """Where it settles: beside the control it is about, on whichever side
        of it there is room for, and never off the display."""
        _lib.lxb_menu_panel(self._handle, self._four)
        return tuple(self._four)

    @property
    def rows_top(self) -> float:
        """How far down the panel the first row starts, in pixels."""
        return float(_lib.lxb_menu_rows_top(self._handle))

    def row(self, index: int):
        """The whole line a row occupies, or None where it is scrolled off."""
        return self._rect(_lib.lxb_menu_row_rect, index)

    def chip(self, index: int):
        """Its face: the line, less the button on its end and the air between
        them where it has one."""
        return self._rect(_lib.lxb_menu_chip_rect, index)

    def aside(self, index: int):
        """The button on its right-hand end, or None where it has none."""
        return self._rect(_lib.lxb_menu_aside_rect, index)

    def highlight(self, on_aside: bool = False):
        """Where the light stands: the selected row's face, or its button when
        the highlight has stepped sideways onto one."""
        return self._rect(_lib.lxb_menu_highlight_rect, 1 if on_aside else 0)

    def separators(self) -> list:
        """The rules between the panel's bands, in order."""
        rules = []
        while True:
            rule = self._rect(_lib.lxb_menu_separator, len(rules))
            if rule is None:
                return rules
            rules.append(rule)

    def opened(self, index: int) -> tuple[float, float]:
        """How much of a row has opened out, in pixels: the label's share and
        the second run's."""
        _lib.lxb_menu_opened(self._handle, index, self._two)
        return tuple(self._two)


def menu_chip_radius(row: MenuRow, drawn: float, height: float) -> float:
    """How round a row's own chip is at the height it is drawn at.

    A capsule while the row is one line, and the same rounded square as its
    button once it is taller than one.
    """
    return float(_lib.lxb_menu_chip_radius(ctypes.byref(row._raw()), drawn, height))


def menu_aside_radius(row: MenuRow, height: float) -> float:
    """How round the button on the end of a row is."""
    return float(_lib.lxb_menu_aside_radius(ctypes.byref(row._raw()), height))


def menu_lines_in(room: float, line: float) -> int:
    """How many lines a run may take, given the room it has been given."""
    return int(_lib.lxb_menu_lines_in(room, line))


def menu_title_height(lines: int) -> float:
    """The header's whole height at that many lines. 0 is no header."""
    return float(_lib.lxb_menu_title_height(lines))


def menu_title_growth(lines: int) -> float:
    """How much of that is the header having opened out."""
    return float(_lib.lxb_menu_title_growth(lines))


def menu_rows_that_fit(height: float, row: float) -> int:
    """How many rows that tall a display this tall has room for. Never zero."""
    return int(_lib.lxb_menu_rows_that_fit(height, row))


def menu_growing(anchor: tuple[float, float, float, float],
                 panel: tuple[float, float, float, float],
                 travelled: float) -> tuple[float, float, float, float]:
    """The panel's rectangle on its way out of its anchor.

    A miniature of itself over ``anchor`` at 0, and ``panel`` at 1. It grows
    out of the control it is about; one that grew out of the middle of the
    screen would be a different component wearing this one's material.

    It is the panel's own shape the whole way — only its scale and its centre
    move. Interpolating the two rectangles instead carries it through the
    anchor's proportions, so a wide flat button reshapes into a tall menu on
    its way, which reads as the button turning into the panel rather than as a
    panel arriving.
    """
    four = ctypes.c_float * 4
    out = four()
    _lib.lxb_menu_growing(four(*anchor), four(*panel), travelled, out)
    return (out[0], out[1], out[2], out[3])


def menu_shown(travelled: float, opening: bool) -> float:
    """How much of the panel is *there*, ``travelled`` of the way through its
    flight: what its glass rides, and what the page gives up its words on.

    ``opening`` is whether it is on its way out of its anchor rather than
    folding back into it. The two are the same journey and not the same curve.
    It arrives faster than it moves — all of the glass is there by ``panel_in``
    of the way out, so that what comes out of the control reads as a pane
    rather than as a rectangle being inflated — and it leaves over the whole of
    the journey, because a panel that held its full colour until the last three
    frames and then went out did not fade, it blinked.
    """
    return float(_lib.lxb_menu_shown(travelled, 1 if opening else 0))


def menu_content_shown(travelled: float, opening: bool) -> float:
    """The same for what is written on the panel.

    It holds back on the way out until there is enough panel to read it on —
    rows do not read at the size of an icon — and leaves with the glass rather
    than ahead of it.
    """
    return float(_lib.lxb_menu_content_shown(travelled, 1 if opening else 0))


@dataclass(frozen=True)
class Control:
    """The control everything you can act on is cut from.

    A button, a list row, a switch and a dialog's answer are one object here,
    and it is not a rectangle filled with a colour that changes when you are on
    it. It is three things drawn in order: a halo, the lit capsule that glides
    onto it, and the control's own chip over the top — which fades out by
    :func:`control_arrival` as the capsule reaches it, so the lit control is
    the one place the capsule shows through.

    ``chip_role``, ``lit_role`` and ``out_role`` are indices into
    :data:`Role`; ``padding`` and ``out_width`` are reference pixels;
    everything else is a share or an alpha and must not be scaled.
    """

    chip_role: int
    chip_tint: float
    chip_gloss: float
    aside_tint: float
    padding: float
    lit_role: int
    lit: float
    lit_answer: float
    lit_pulse: float
    glow: float
    glow_pulse: float
    glow_width: float
    glow_height: float
    out_role: int
    out: float
    out_width: float
    ink: float
    ink_quiet: float
    mark: float
    mark_quiet: float


def control() -> Control:
    """The control's whole recipe, as one value."""
    got = _lib.lxb_control()
    return Control(got.chip_role, got.chip_tint, got.chip_gloss,
                   got.aside_tint, got.padding, got.lit_role, got.lit,
                   got.lit_answer, got.lit_pulse, got.glow, got.glow_pulse,
                   got.glow_width, got.glow_height, got.out_role, got.out,
                   got.out_width, got.ink, got.ink_quiet, got.mark,
                   got.mark_quiet)


def control_glow_rect(rect: tuple[float, float, float, float], over: float,
                      tall: float | None = None
                      ) -> tuple[float, float, float, float]:
    """The halo's rectangle for a control at ``rect``.

    ``over`` is the width of the surface the control sits on: the halo is a
    share of that rather than of the control, so a taller row is not read as
    more selected than a shorter one. ``tall`` defaults to the control's own
    height times ``glow_height``.
    """
    four = ctypes.c_float * 4
    out = four()
    _lib.lxb_control_glow_rect(four(*rect), over,
                               -1.0 if tall is None else tall, out)
    return (out[0], out[1], out[2], out[3])


def control_arrival(light: tuple[float, float, float, float],
                    rect: tuple[float, float, float, float]) -> float:
    """How much of the lit capsule has arrived over the control at ``rect``.

    1 when it is sitting on it, 0 while it is still a control away. Measured
    in the control's own widths and heights, in all four numbers: a capsule
    that merely shares a line with a control is not light on it.
    """
    four = ctypes.c_float * 4
    return float(_lib.lxb_control_arrival(four(*light), four(*rect)))


def press_scale(t: float) -> float:
    """How big a control is drawn ``t`` of the way through a press.

    Down, back past its own size, and settled — as a multiple of its own size,
    and 1 outside 0 to 1. A press runs over the ``guide-press`` duration.
    Tinting a control instead says only that it has been *selected*, which the
    light already said.
    """
    return float(_lib.lxb_press_scale(t))


def fill_arrival(t: float) -> float:
    """How much of a state change has arrived ``t`` of the way through a press.

    Held back until the control is at the bottom of its travel, so the colour
    changing and the control going down are one movement: a switch is thrown on
    the way up, which is where a real one latches.
    """
    return float(_lib.lxb_fill_arrival(t))


def pulse(seconds: float) -> float:
    """The breath under whatever is being aimed at: 0 to 1 and back.

    Over the ``pulse`` duration. The one thing in the interface that moves
    without having been asked to, and what separates the control the next press
    will reach from one that merely has a light on it in a screenshot.
    """
    return float(_lib.lxb_pulse(seconds))


def pressed(rect: tuple[float, float, float, float],
            t: float | None) -> tuple[float, float, float, float]:
    """Where a control at ``rect`` is drawn ``t`` of the way through a press.

    Its own rectangle for ``None``, and :func:`press_scale` about its own
    centre otherwise. Everything the control is made of moves together — a
    label that stayed put while its chip sank would read as a hole opening
    behind it.
    """
    four = ctypes.c_float * 4
    out = four()
    _lib.lxb_pressed(four(*rect), -1.0 if t is None else t, out)
    return (out[0], out[1], out[2], out[3])


def glide(position: float, velocity: float, target: float,
          dt: float) -> tuple[float, float]:
    """One step of the spring a lit capsule glides on.

    Returns where it is and how fast it is going. Critically damped at a
    highlight's stiffness: it leans into a move rather than leaving at full
    speed, and a second move part-way carries the first one's momentum on.
    """
    at = ctypes.c_float(position)
    moving = ctypes.c_float(velocity)
    _lib.lxb_glide(ctypes.byref(at), ctypes.byref(moving), target, dt)
    return (at.value, moving.value)


def key_light() -> tuple[float, float, float]:
    """The shared key light for glass panes and the background."""
    out = (ctypes.c_float * 3)()
    _lib.lxb_key_light(out)
    return (out[0], out[1], out[2])


GLASS_IOR: float = _lib.lxb_glass_ior()


def glass_wgsl() -> str:
    """The shading itself, in WGSL, ready to concatenate into your shader."""
    got = _bytes(_lib.lxb_glass_wgsl())
    assert got is not None
    return got.decode("utf-8")


def wallpaper_wgsl() -> str:
    """The standalone current Default-water and Simple-silk wallpaper WGSL."""
    got = _bytes(_lib.lxb_wallpaper_wgsl())
    assert got is not None
    return got.decode("utf-8")


# --- file and folder selection ---------------------------------------------

class Selection(IntEnum):
    """What a :class:`Picker` is allowed to choose.

    ``FILE`` accepts every visible file, ``IMAGE`` the still-image extensions
    LineXinBar uses, and ``SCENERY`` those plus video. ``FOLDER`` lists only
    directories and answers with the directory currently being shown.
    """

    FILE = 0
    IMAGE = 1
    SCENERY = 2
    FOLDER = 3


class EntryKind(IntEnum):
    """Whether a visible picker entry is a folder or a file."""

    FOLDER = 0
    FILE = 1


@dataclass(frozen=True)
class PickerEntry:
    """One visible filesystem entry in a :class:`Picker`."""

    name: str
    path: Path
    kind: EntryKind

    @property
    def is_folder(self) -> bool:
        return self.kind is EntryKind.FOLDER

    @property
    def is_file(self) -> bool:
        return self.kind is EntryKind.FILE


def _picker_string(value, name: str) -> bytes:
    value = os.fspath(value)
    if isinstance(value, bytes):
        value = value.decode("utf-8")
    if not isinstance(value, str):  # pragma: no cover - os.fspath guarantees it
        raise TypeError(f"{name} must be a path or string")
    if "\0" in value:
        raise ValueError(f"{name} must not contain a NUL character")
    return value.encode("utf-8")


class Picker:
    """A lazy, renderer-neutral file or folder chooser.

    It is the selection model, not a native dialog: draw :attr:`entries` with
    the renderer you already use, call :meth:`enter` for a focused folder, and
    call :meth:`choose` only from an explicit action. Folder mode deliberately
    leaves its ``Select folder`` row to the caller, so opening a folder cannot
    select it by accident.

    Visible names exclude dotfiles and non-UTF-8 filenames. Folders precede
    files and both groups are case-insensitively alphabetical. The current
    directory is reread only when it is reached, searched, or refreshed.
    """

    __slots__ = ("_picker",)

    def __init__(self, selection: Selection, directory) -> None:
        try:
            selection = Selection(selection)
        except (TypeError, ValueError) as error:
            raise ValueError(f"unknown picker selection: {selection!r}") from error
        handle = _lib.lxb_picker_new(int(selection), _picker_string(directory, "directory"))
        if not handle:
            raise ValueError("the picker could not be created")
        self._picker = handle

    def close(self) -> None:
        """Release the model. Safe to call more than once."""
        picker, self._picker = getattr(self, "_picker", None), None
        if picker:
            _lib.lxb_picker_free(picker)

    def __enter__(self) -> "Picker":
        return self

    def __exit__(self, *_unused) -> None:
        self.close()

    def __del__(self):  # pragma: no cover - interpreter teardown
        self.close()

    def _handle(self):
        picker = self._picker
        if not picker:
            raise RuntimeError("the picker is closed")
        return picker

    @property
    def selection(self) -> Selection:
        return Selection(int(_lib.lxb_picker_selection(self._handle())))

    @property
    def can_search(self) -> bool:
        """Whether this picker has a search field.

        Folder pickers deliberately put their explicit ``Select folder``
        control there instead, so their query always stays empty. Empty and
        unreadable file listings have no search field either.
        """
        return bool(_lib.lxb_picker_can_search(self._handle()))

    @property
    def can_choose(self) -> bool:
        """Whether an explicit choose action currently has an answer.

        Folder mode becomes choosable only after its current directory opens;
        file mode only when focus rests on a file.
        """
        return bool(_lib.lxb_picker_can_choose(self._handle()))

    @property
    def location(self) -> Path:
        """The directory currently being listed."""
        return Path(_text(_lib.lxb_picker_location(self._handle())))

    @property
    def note(self) -> str:
        """The listing's count, ``Empty``, or ``This cannot be opened``."""
        return _text(_lib.lxb_picker_note(self._handle()))

    @property
    def query(self) -> str:
        """The case-insensitive substring that currently narrows this folder."""
        return _text(_lib.lxb_picker_query(self._handle()))

    @property
    def entries(self) -> tuple[PickerEntry, ...]:
        """Visible entries, folders first. There is no synthetic folder-choice row."""
        picker = self._handle()
        return tuple(
            PickerEntry(
                _text(_lib.lxb_picker_entry_name(picker, index)),
                Path(_text(_lib.lxb_picker_entry_path(picker, index))),
                EntryKind(int(_lib.lxb_picker_entry_kind(picker, index))),
            )
            for index in range(int(_lib.lxb_picker_entry_count(picker)))
        )

    @property
    def selected(self) -> int | None:
        """The focused visible entry, or ``None`` when there is none."""
        index = int(_lib.lxb_picker_selected(self._handle()))
        return None if index < 0 else index

    def select(self, index: int) -> bool:
        """Focus one visible entry. ``False`` means it was invalid or already focused."""
        if not isinstance(index, int) or isinstance(index, bool) or index < 0:
            return False
        return bool(_lib.lxb_picker_select(self._handle(), index))

    def move(self, delta: int) -> bool:
        """Move focus without wrapping at either end of the listing."""
        if not isinstance(delta, int) or isinstance(delta, bool):
            raise TypeError("delta must be an integer")
        return bool(_lib.lxb_picker_move(self._handle(), delta))

    def enter(self) -> bool:
        """Open the focused folder, clearing this folder's search."""
        return bool(_lib.lxb_picker_enter(self._handle()))

    def leave(self) -> bool:
        """Return to the parent directory, stopping at the filesystem root."""
        return bool(_lib.lxb_picker_leave(self._handle()))

    def refresh(self) -> None:
        """Reread the current directory as it is now."""
        _lib.lxb_picker_refresh(self._handle())

    def search(self, query: str) -> None:
        """Replace the current case-insensitive substring search and reread.

        A listing without a search field, including a folder picker, ignores
        this call.
        """
        _lib.lxb_picker_search(self._handle(), _picker_string(query, "query"))

    def choose(self) -> Path | None:
        """Return the deliberate file/folder answer, or ``None`` when none is valid.

        In folder mode it is the current directory after it opens. In file
        mode it is a file only; focus a folder and call :meth:`enter` instead.
        """
        picked = _lib.lxb_picker_choose(self._handle())
        return Path(_text(picked)) if picked else None


# --- assets -----------------------------------------------------------------

GLYPHS: tuple[str, ...] = tuple(
    _text(_lib.lxb_glyph_name(index)) for index in range(_lib.lxb_glyph_count())
)
"""Every mark's name."""

SOUNDS: tuple[str, ...] = tuple(
    _text(_lib.lxb_sound_name(index)) for index in range(_lib.lxb_sound_count())
)
"""Every bundled recording, including two unused compatibility assets."""

SHELL_SOUNDS: tuple[str, ...] = tuple(
    name for name in SOUNDS if _lib.lxb_sound_used(name.encode())
)
"""The eleven effects and background loop the current shell actually plays."""


def glyph(name: str) -> bytes | None:
    """A mark, as SVG source. None if there is no mark of that name.

    Each is a standalone document, and 53 of the 98 build their outline in a
    ``<defs>`` block under the same internal names — ``bead`` and ``pierced``
    among them. Inlining two into one page makes those names collide: they are
    global to the document, a nested ``<svg>`` is not a new scope, and a
    reference resolves to the first matching definition. Prefix each mark's
    names with something of its own, or every masked mark after the first draws
    as that first one. ``examples/python/contact_sheet.py`` does exactly that.
    """
    return _bytes(_lib.lxb_glyph(name.encode()))


def glyph_box(name: str) -> int | None:
    """The square SVG viewBox edge for a mark, or None if it is unknown."""
    edge = _lib.lxb_glyph_box(name.encode())
    return edge or None


def glyph_wgsl() -> str:
    """The standalone Default/Simple glyph-material shader module."""
    got = _bytes(_lib.lxb_glyph_wgsl())
    assert got is not None
    return got.decode("utf-8")


GLYPH_SDF_RANGE: float = _lib.lxb_glyph_sdf_range()
GLYPH_SDF_SUPERSAMPLE: int = _lib.lxb_glyph_sdf_supersample()
GLYPH_CELL: int = _lib.lxb_glyph_cell()
GLYPH_DEPTH_SHARE: float = _lib.lxb_glyph_depth_share()
_glyph_lamp = (ctypes.c_float * 3)()
_lib.lxb_glyph_lamp(_glyph_lamp)
GLYPH_LAMP: tuple[float, float, float] = tuple(_glyph_lamp)  # type: ignore[assignment]
GLYPH_SHADOW: float = _lib.lxb_glyph_shadow()
GLYPH_SIMPLE_TINT: float = _lib.lxb_glyph_simple_tint()
GLYPH_SIMPLE_ALPHA: float = _lib.lxb_glyph_simple_alpha()
GLYPH_SIMPLE_STAIN: float = _lib.lxb_glyph_simple_stain()


def glyph_sdf(coverage: bytes, size: int = GLYPH_CELL) -> bytes:
    """Measure supersampled SVG coverage into the current glyph SDF alpha.

    ``coverage`` must be a square plane whose edge is ``size * 4``. Values at
    least 128 are inside the glyph. The returned plane is ``size`` square and
    is ready for the alpha channel of an atlas cell.
    """
    if not isinstance(coverage, bytes):
        raise TypeError("coverage must be bytes")
    if not isinstance(size, int) or isinstance(size, bool):
        raise TypeError("size must be an integer")
    if size <= 0 or size > 0xFFFFFFFF:
        raise ValueError("size must be between 1 and 2^32 - 1")
    fine = size * GLYPH_SDF_SUPERSAMPLE
    expected = fine * fine
    if len(coverage) != expected:
        raise ValueError(
            f"coverage must contain {expected} bytes for a {size}-texel field"
        )
    source = (ctypes.c_ubyte * expected).from_buffer_copy(coverage)
    output_len = size * size
    output = (ctypes.c_ubyte * output_len)()
    if not _lib.lxb_glyph_sdf(source, expected, size, output, output_len):
        raise ValueError("coverage dimensions could not be converted")
    return bytes(output)


def sound(name: str) -> bytes | None:
    """A recording, as Ogg Vorbis."""
    return _bytes(_lib.lxb_sound(name.encode()))


SOUND_REST: float = _lib.lxb_sound_rest()
"""The shortest gap between two plays of one recording, in seconds.

Nearer than this, drop the request: identical samples add in phase, and one
wheel event can be worth several rows.
"""


class Sound:
    """Which current-shell recording answers what.

    A namespace of plain strings rather than an enumeration, so that a member is
    the key the asset is stored under and can be handed straight to
    :func:`sound` — and so that a caller who has a name from somewhere else can
    use that instead without converting anything.

    An interface driven from a controller answers a press twice: something
    moves, and it clicks. Two rules are worth reading before wiring these up. A
    screen with a voice of its own does not borrow another's — each full-screen
    surface answers a move and a press in its own pair, so somebody who has
    looked away can hear which screen they are driving. And a move sounds by
    where it *landed*, not by what moved it: a click that puts the selection on
    a row is the same move as the direction that would have walked there, while
    hovering is not a move at all.
    """

    MOVE = "press"
    PRESS = "press-selected"
    GUIDE_MOVE = "press-guide"
    GUIDE_PRESS = "press-guide-selected"
    BACK = "press-back"
    KEY = "keyboard-click"
    LAUNCH = "app-launch"
    GUIDE_OPEN = "guide-open"
    SHUTTER = "screenshot"
    AUTHENTICATE = "polkit"
    NOTIFY = "notification"
    MUSIC = "start-bg-music"

    # Still retrievable through sound(), but explicitly not current shell
    # actions. Kept so an application built against toolkit 0.1 does not lose
    # an asset it may have chosen for itself.
    COMPAT_TRASH = "trash"
    COMPAT_ERROR = "error"


SOUND_MUSIC_FADE_IN: float = _lib.lxb_sound_music_fade_in()
"""How long the one looping recording takes to rise out of silence."""

SOUND_MUSIC_FADE_OUT: float = _lib.lxb_sound_music_fade_out()
"""And how long it takes to go when something else has taken the screen."""

SOUND_RETRY_AFTER: float = _lib.lxb_sound_retry_after()
"""How long to leave a sound device alone after failing to open it."""


def sound_amplitude(value: float) -> float:
    """How loud a level of 0..1 actually is, as an amplitude.

    A level is read as loudness, and amplitude is not loudness: a control
    dragged to the middle should sound half as loud rather than measure half as
    tall. This is the curve every volume control on a machine has.
    """
    return _lib.lxb_sound_amplitude(value)


def sound_fade(elapsed: float, over: float) -> float:
    """How far through a fade of ``over`` seconds ``elapsed`` is, eased.

    Not :func:`ease`. A fade is heard rather than watched, and what an ear
    wants at both ends is a tangent of zero.
    """
    return _lib.lxb_sound_fade(elapsed, over)


# --- what the user pressed, and what it means -------------------------------


class Action:
    """One thing the user asked for, whichever control asked for it.

    An interface in this language is driven from a keyboard, a controller and a
    pointer at once, and the whole point is that they are one interface rather
    than three: a direction is a direction whether it came from an arrow key, a
    D-pad or a thumbstick, and the thing under the pointer is selected exactly
    as a direction would have selected it.

    The numbers are the library's own order, which is what :func:`action_of_key`
    and its two companions answer with.
    """

    LEFT = 0
    RIGHT = 1
    UP = 2
    DOWN = 3
    #: Act on what is selected. The shell calls this one "launch", because
    #: there the thing being accepted is an application.
    ACCEPT = 4
    BACK = 5
    MENU = 6
    #: Start. Fold it into ACCEPT unless something is being typed into.
    SUBMIT = 7
    PREVIOUS = 8
    NEXT = 9


class Key:
    """The keys that mean something here, and no others.

    Deliberately small: this is not a keyboard abstraction. Map your toolkit's
    own key events onto these — about fifteen lines — and hand them to
    :func:`action_of_key`.
    """

    LEFT = 0
    RIGHT = 1
    UP = 2
    DOWN = 3
    ENTER = 4
    SPACE = 5
    ESCAPE = 6
    BACKSPACE = 7
    TAB = 8
    BACKTAB = 9
    MENU = 10
    F10 = 11


class Button:
    """A control on a game controller, by where the thumb finds it.

    Named positionally — SOUTH is the bottom face button — because that is the
    only naming that survives the pad being an Xbox one, a DualSense, a Switch
    Pro or a Steam Deck.
    """

    SOUTH = 0
    EAST = 1
    NORTH = 2
    #: Left alone: it belongs to whatever is running.
    WEST = 3
    START = 4
    #: The shell's chord modifier, and nothing on its own.
    SELECT = 5
    LEFT_BUMPER = 6
    RIGHT_BUMPER = 7
    #: The shell's, always. It is the way out of this application.
    GUIDE = 8
    DPAD_LEFT = 9
    DPAD_RIGHT = 10
    DPAD_UP = 11
    DPAD_DOWN = 12


ACTIONS: tuple[str, ...] = tuple(
    _text(_lib.lxb_action_name(index)) for index in range(_lib.lxb_action_count())
)
"""Every action's name, in :class:`Action` order."""


def action_named(name: str) -> int | None:
    """The action written under this name, or None."""
    found = _lib.lxb_action_named(name.encode())
    return None if found < 0 else found


def action_repeats(action: int) -> bool:
    """Whether a held control keeps sending it: the four directions, no more.

    A held Accept is one press somebody is leaning on, not a run of them.
    """
    return bool(_lib.lxb_action_repeats(action))


def action_of_key(key: int) -> int | None:
    """What one :class:`Key` means, or None.

    The letters are deliberately absent; see :func:`action_of_letter`.
    """
    found = _lib.lxb_action_of_key(key)
    return None if found < 0 else found


def action_of_letter(letter: str) -> int | None:
    """The shell's letter shorthands: ``wasd`` and ``hjkl``, and ``y`` for the
    context menu.

    Asked for separately from :func:`action_of_key` because an application with
    a field in it must not bind them — somebody typing "yesterday" into a
    search box would otherwise raise four context menus.
    """
    if len(letter) != 1:
        return None
    found = _lib.lxb_action_of_letter(ord(letter))
    return None if found < 0 else found


def action_of_button(button: int) -> int | None:
    """What one :class:`Button` means, or None.

    Three answer None, each for its own reason: GUIDE is the shell's, WEST is
    left free for the application, and SELECT is the shell's chord modifier.
    """
    found = _lib.lxb_action_of_button(button)
    return None if found < 0 else found


INPUT_POLL_INTERVAL: float = _lib.lxb_input_poll_interval()
"""How often a controller should be read, in seconds."""

INPUT_INITIAL_REPEAT: float = _lib.lxb_input_initial_repeat()
"""How long a direction is held before it starts stepping on its own."""

INPUT_REPEAT_INTERVAL: float = _lib.lxb_input_repeat_interval()
"""And how long between steps after that."""

INPUT_STICK_ENGAGE: float = _lib.lxb_input_stick_engage()
"""How far a stick is pushed before it counts as a direction."""

INPUT_STICK_RELEASE: float = _lib.lxb_input_stick_release()
"""And how far back towards the middle before it counts as let go."""

INPUT_SCROLL_STEP: float = _lib.lxb_input_scroll_step()
"""How far a device with no notches travels to be worth one."""

INPUT_TAP_SLOP: float = _lib.lxb_input_tap_slop()
"""How far a finger may wander and still have been a tap."""


class Repeat:
    """The middle of a held direction, which no platform supplies.

    Wayland hands a client one press and one release; a pad has no repeat at
    all. Keep one of these for the whole application, put every direction into
    it — arrow keys, D-pad, both sticks — and step it once a poll.

    One of these rather than one per device, and that is the point of it: the
    pace of a held control is a property of the interface, not of the thing
    being held.

    >>> repeat = Repeat()
    >>> repeat.update(0.0, (False, False, False, True))
    [3]
    >>> repeat.update(0.2, (False, False, False, True))
    []
    """

    __slots__ = ("_handle",)

    def __init__(self) -> None:
        self._handle = _lib.lxb_repeat_new()
        if not self._handle:
            raise MemoryError("could not create a repeat")

    def __del__(self) -> None:
        handle = getattr(self, "_handle", None)
        if handle:
            _lib.lxb_repeat_free(handle)
            self._handle = None

    def reset(self) -> None:
        """Forget every held control.

        For the moment this application stops being the one driven. Without it
        a direction held across that moment is still held when control comes
        back, and the list walks under nobody's thumb.
        """
        _lib.lxb_repeat_reset(self._handle)

    def update(
        self,
        now: float,
        pressed: Sequence[bool],
        stick: tuple[float, float] = (0.0, 0.0),
    ) -> list[int]:
        """Step to ``now`` seconds and answer every action that is due.

        ``pressed`` is four booleans in left, right, up, down order — arrow keys
        and the D-pad already merged, because to this they are one control.
        ``stick`` is one stick, with y positive **up**, which is how every
        gamepad API normalises it.
        """
        if len(pressed) != 4:
            raise ValueError("pressed must be four booleans: left, right, up, down")
        held = (ctypes.c_int * 4)(*(1 if one else 0 for one in pressed))
        out = (ctypes.c_int * 4)()
        count = _lib.lxb_repeat_update(
            self._handle, now, held, stick[0], stick[1], out, 4
        )
        return [out[index] for index in range(min(int(count), 4))]


class Wheel:
    """What is left of a wheel that has not turned far enough to move anything.

    The remainder is the entire point of it. A touchpad reports a stream of
    fractions of a notch: rounding each of them to nothing means a slow
    two-finger drag never moves at all, and rounding each up means the same
    drag crosses the whole list.

    >>> wheel = Wheel()
    >>> [wheel.notches(0.34) for _ in range(3)]
    [0, 0, 1]
    """

    __slots__ = ("_carried",)

    def __init__(self) -> None:
        self._carried = ctypes.c_float(0.0)

    def notches(self, notches: float) -> int:
        """How many whole steps this much turning is worth, keeping the rest."""
        return _lib.lxb_wheel_notches(ctypes.byref(self._carried), notches)

    def distance(self, pixels: float) -> int:
        """The same, for a device that reports a distance rather than notches."""
        return self.notches(pixels / INPUT_SCROLL_STEP)

    def reset(self) -> None:
        """Forget the part-turn, for when the pointer leaves."""
        self._carried = ctypes.c_float(0.0)


def font(bold: bool = False, script: str = "latin") -> bytes:
    """Roboto, as one of the two faces, or the face for a script it has not
    got: ``"devanagari"`` for Hindi, ``"han"`` for Chinese. TTF."""
    if script == "latin":
        got = _bytes(_lib.lxb_font(1 if bold else 0))
    else:
        got = _bytes(_lib.lxb_font_for(script.encode(), 1 if bold else 0))
    if got is None:
        raise ValueError(f"no face is shipped for the script {script!r}")
    return got


def stylesheet(palette_name: str = "Purple") -> str:
    """The whole language as CSS custom properties."""
    text = _lib.lxb_stylesheet(palette_name.encode())
    if not text:
        raise ValueError(f"no palette is called {palette_name!r}")
    try:
        return ctypes.cast(text, ctypes.c_char_p).value.decode("utf-8")  # type: ignore[union-attr]
    finally:
        _lib.lxb_string_free(text)


def version() -> str:
    """The library's version."""
    return _text(_lib.lxb_version())


def roles() -> Iterator[tuple[str, Role]]:
    """Every role, under the name the other bindings know it by."""
    for index in range(len(Role)):
        yield _text(_lib.lxb_role_name(index)), Role(index)


# --- drawing the material ---------------------------------------------------
#
# Last, because it reaches back into this module for the structures and the
# library handle: everything above has to exist before it is imported.

from . import paint  # noqa: E402
