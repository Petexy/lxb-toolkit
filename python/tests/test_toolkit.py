"""The Python binding, tested against what the other two answer.

Run with `python3 -m pytest python/tests` from the repository root, or with
`python3 python/tests/test_toolkit.py` if pytest is not installed — the file
runs its own tests when it is executed.

What is worth testing here is not the design language; that is tested in Rust,
where it lives. What is worth testing is the *seam*: that a value survives the
journey through the C ABI unchanged, that nothing is truncated by a missing
signature, that an object with a `free` beside it does not leak or double-free,
and that the enumerations Python builds from the library match what the library
actually has.
"""

from __future__ import annotations

import ctypes
import os
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import lxb_toolkit as lxb


def test_the_palettes_are_the_authored_ones():
    """A colour survives the journey out of the library unchanged."""
    assert [p.name for p in lxb.PALETTES] == [
        "Purple", "Blue", "Green", "Yellow", "Red", "Teal",
        "Indigo", "Pink", "Orange", "White", "Silver", "Black",
    ]
    assert lxb.PALETTES[0].color(lxb.Role.ACCENT).hex == "#8b5cf6"
    assert lxb.PALETTES[0][lxb.Role.DANGER].hex == "#e0533a"
    assert lxb.palette("blue") is lxb.PALETTES[1] or (
        lxb.palette("blue").name == "Blue"
    )
    assert lxb.palette("mauve") is None


def test_colour_comes_back_in_both_forms():
    """And the two forms are the same colour, which is the whole point of
    keeping them apart."""
    white = lxb.Color(0xFFFFFF)
    assert white.linear == (1.0, 1.0, 1.0, 1.0)
    assert white.bytes == (255, 255, 255)

    # A middle grey is a fifth of the light, not half of it. A binding that
    # skipped the conversion would draw everything about twice as bright.
    grey = lxb.Color(0x808080).linear
    assert abs(grey[0] - 0.2158) < 0.001

    accent = lxb.PALETTES[0].color(lxb.Role.ACCENT)
    assert accent.with_alpha(0.5).linear[3] == 0.5
    assert str(accent) == "#8b5cf6"


def test_the_enumerations_come_from_the_library():
    """Python builds them by asking rather than by keeping a copy, so a role
    added in Rust is a role Python has."""
    assert len(lxb.Role) == 14
    assert lxb.Role.ACCENT == 0
    assert lxb.Role.SKY_BOTTOM_ALT == 13
    assert len(lxb.Metric) == 21
    assert len(lxb.Text) == 5
    assert len(lxb.Surface) == 3
    assert list(lxb.Overlay) == [lxb.Overlay.CONTEXT_MENU, lxb.Overlay.DIALOG]
    assert list(lxb.IconStyle) == [lxb.IconStyle.DEFAULT, lxb.IconStyle.SIMPLE]
    assert list(lxb.WallpaperStyle) == [
        lxb.WallpaperStyle.DEFAULT,
        lxb.WallpaperStyle.SIMPLE,
        lxb.WallpaperStyle.CUSTOM,
    ]
    names = dict(lxb.roles())
    assert names["accent"] == lxb.Role.ACCENT
    assert names["glass-raised"] == lxb.Role.GLASS_RAISED


def test_a_picker_walks_and_answers_across_the_abi():
    """The owned filesystem model survives Python's borrowed C strings."""
    with TemporaryDirectory() as raw:
        root = Path(raw)
        (root / "inside").mkdir()
        (root / "note.txt").write_text("x", encoding="utf-8")
        (root / "inside" / "picked.txt").write_text("x", encoding="utf-8")

        with lxb.Picker(lxb.Selection.FILE, root) as picker:
            assert picker.selection is lxb.Selection.FILE
            assert picker.can_search
            assert picker.location == root
            assert picker.note == "1 folder, 1 file"
            assert [(entry.name, entry.kind) for entry in picker.entries] == [
                ("inside", lxb.EntryKind.FOLDER),
                ("note.txt", lxb.EntryKind.FILE),
            ]
            assert picker.selected == 0
            assert picker.choose() is None
            assert not picker.can_choose
            assert picker.select(1)
            assert picker.can_choose
            assert picker.choose() == root / "note.txt"
            assert picker.select(0)
            assert picker.enter()
            assert picker.location == root / "inside"
            picker.search("PICK")
            assert picker.query == "PICK"
            assert picker.choose() == root / "inside" / "picked.txt"
            assert picker.leave()
            assert picker.query == ""

        with lxb.Picker(lxb.Selection.FOLDER, root) as picker:
            assert not picker.can_search
            assert picker.can_choose
            assert picker.choose() == root
            picker.search("inside")
            assert picker.query == ""
            assert [entry.name for entry in picker.entries] == ["inside"]


def test_the_current_shell_theme_crosses_all_three_languages():
    with TemporaryDirectory() as config_home:
        settings = Path(config_home) / "lxb" / "shell.toml"
        settings.parent.mkdir()
        settings.write_text(
            'accent = "Yellow"\n'
            'theme-wallpaper = "Custom wallpaper"\n'
            'theme-icons = "Simple"\n'
            'theme-particles = false\n',
            encoding="utf-8",
        )
        previous = os.environ.get("XDG_CONFIG_HOME")
        os.environ["XDG_CONFIG_HOME"] = config_home
        try:
            theme = lxb.ShellTheme.load()
            assert theme.accent.name == "Yellow"
            assert theme.wallpaper is lxb.WallpaperStyle.CUSTOM
            assert theme.icons is lxb.IconStyle.SIMPLE
            assert theme.particles is False

            settings.write_text('accent = "Green"\ntheme = "Simple"\n', encoding="utf-8")
            legacy = lxb.ShellTheme.load()
            assert legacy.accent.name == "Green"
            assert legacy.wallpaper is lxb.WallpaperStyle.SIMPLE
            assert legacy.icons is lxb.IconStyle.SIMPLE
            assert legacy.particles is True
        finally:
            if previous is None:
                os.environ.pop("XDG_CONFIG_HOME", None)
            else:
                os.environ["XDG_CONFIG_HOME"] = previous


def test_walking_a_palette_shows_it_without_choosing_it():
    """The setting's whole behaviour, across the ABI."""
    with lxb.Accent("Purple") as accent:
        assert accent.applied.name == "Purple"
        assert accent.preview("Blue") is True

        frames = 0
        while accent.advance(1 / 60):
            frames += 1
            assert frames < 1000
        assert frames > 1, "it arrived without animating"

        assert accent.color(lxb.Role.ACCENT).hex == "#3b82f6"
        assert accent.applied.name == "Purple", "a preview chose it"

        accent.restore()
        while accent.advance(1 / 60):
            pass
        assert accent.color(lxb.Role.ACCENT).hex == "#8b5cf6"

        assert accent.commit("Green") is True
        while accent.advance(1 / 60):
            pass
        assert accent.applied.name == "Green"
        assert accent.color(lxb.Role.ACCENT).hex == "#16a34a"


def test_everything_arrives_together():
    """Halfway through a change, every role is halfway. This is the reason the
    transition exists rather than each surface reading a palette itself."""
    accent = lxb.Accent("Purple")
    accent.preview("Blue")
    accent.advance(lxb.DURATIONS["accent-change"] / 2)
    mid = {role: accent.color(role) for role in lxb.Role}
    # Not at either end, and not equal to the palette it came from.
    assert mid[lxb.Role.ACCENT].hex not in ("#8b5cf6", "#3b82f6")
    assert mid[lxb.Role.SKY_TOP].hex != lxb.PALETTES[0].color(lxb.Role.SKY_TOP).hex


def test_an_unknown_palette_is_refused_rather_than_guessed_at():
    try:
        lxb.Accent("mauve")
    except ValueError as err:
        assert "mauve" in str(err)
    else:  # pragma: no cover
        raise AssertionError("an unknown palette was accepted")


def test_motion_is_the_same_curve_it_is_in_rust():
    assert lxb.ease(0.0) == 0.0
    assert lxb.ease(1.0) == 1.0
    assert abs(lxb.ease(0.5) - 0.5) < 1e-6
    # Never linear: the curve is nowhere near the straight line.
    assert abs(lxb.ease(0.1) - 0.1) > 0.01
    assert lxb.smoothstep(0.5) == 0.5

    # A spring accelerates from rest, settles, and never crosses.
    at, speed = 0.0, 0.0
    for _ in range(200):
        at, speed = lxb.spring(at, speed, 1.0)
        assert at <= 1.0
    assert at > 0.999

    # A C float widened to a Python float is not exactly the decimal it was
    # written as — 0.3f is 0.30000001192092896 here — so durations are compared
    # the way anything crossing that seam has to be.
    assert abs(lxb.DURATIONS["flight"] - 0.3) < 1e-6
    assert len(lxb.DURATIONS) == 34
    for name, seconds in {
        "menu-flight": 0.20,
        "menu-unfold": 0.32,
        "menu-press": 0.16,
        "guide-press": 0.34,
        "guide-power-flight": 0.22,
        "guide-media-flight": 0.34,
        "guide-transport-flight": 0.12,
        "notification-hold": 4.0,
        "notification-enter": 0.34,
        "notification-leave": 0.22,
        "notification-badge": 0.20,
        "volume-fill": 0.14,
        "volume-hold": 1.0,
        "volume-leave": 0.28,
        "keyboard-slide": 0.24,
    }.items():
        assert abs(lxb.DURATIONS[name] - seconds) < 1e-6


def test_sizes_scale_by_height():
    assert lxb.REFERENCE_HEIGHT == 1080.0
    assert lxb.scale_for(2160.0) == 2.0
    assert abs(lxb.scale_for(100.0) - 0.6) < 1e-6
    assert lxb.scale_for(10_000.0) == 2.5
    assert lxb.size(lxb.Metric.ROW_HEIGHT) == 76.0
    assert lxb.size(lxb.Metric.ROW_HEIGHT, 2160.0) == 152.0
    # A share is a share on every screen.
    assert lxb.is_share(lxb.Metric.TILE_RADIUS)
    assert lxb.size(lxb.Metric.TILE_RADIUS, 2160.0) == lxb.size(lxb.Metric.TILE_RADIUS)
    # Controls are capsules.
    assert lxb.capsule_radius(76.0) == 38.0
    assert lxb.text_size(lxb.Text.BODY) == 26.0
    assert lxb.text_is_bold(lxb.Text.TITLE)
    assert not lxb.text_is_bold(lxb.Text.BODY)
    assert lxb.size(lxb.Metric.MENU_WIDTH) == 440.0
    assert lxb.size(lxb.Metric.DIALOG_WIDTH) == 680.0
    assert abs(lxb.size(lxb.Metric.DIALOG_DIM) - 0.30) < 1e-6
    assert lxb.size(lxb.Metric.POWER_WIDTH) == 460.0
    assert abs(lxb.size(lxb.Metric.POWER_DIM) - 0.28) < 1e-6


def test_the_glass_is_the_glass():
    panel = lxb.glass(lxb.Surface.PANEL)
    control = lxb.glass(lxb.Surface.CONTROL)
    assert panel.depth > control.depth
    assert panel.frost > control.frost
    assert lxb.glass(lxb.Surface.SIDEBAR).curve > 0

    x, y, z = lxb.key_light()
    assert x < 0 and y < 0 and z > 0, "the glass lamp is up, left and in front"
    assert 1.4 < lxb.GLASS_IOR < 1.6

    wgsl = lxb.glass_wgsl()
    assert "fn lxb_glass_shade" in wgsl
    assert "@group" not in wgsl, "it would take a binding slot from its caller"

    wallpaper = lxb.wallpaper_wgsl()
    assert "fn lxb_wallpaper(" in wallpaper
    assert "fn lxb_wallpaper_water(" in wallpaper
    assert "fn lxb_wallpaper_silk(" in wallpaper
    assert "@group" not in wallpaper


def test_dialogue_and_context_menu_are_the_same_layered_material():
    menu = lxb.overlay_material(lxb.Overlay.CONTEXT_MENU)
    dialog = lxb.overlay_material(lxb.Overlay.DIALOG)
    assert dialog == menu
    assert dialog.glass == lxb.glass(lxb.Surface.SIDEBAR)
    assert dialog.radius == 30.0
    assert abs(dialog.stain - 0.38) < 1e-6
    assert dialog.light_inset == 2.0
    assert abs(dialog.header_x - 0.06) < 1e-6
    assert abs(dialog.header_width - 0.88) < 1e-6
    assert dialog.header_height == 300.0
    assert abs(dialog.header_height_share - 0.36) < 1e-6
    assert abs(dialog.header_light - 0.075) < 1e-6
    assert abs(dialog.foot_x - 0.10) < 1e-6
    assert abs(dialog.foot_width - 0.80) < 1e-6
    assert dialog.foot_height == 360.0
    assert abs(dialog.foot_height_share - 0.38) < 1e-6
    assert abs(dialog.foot_light - 0.04) < 1e-6
    assert abs(dialog.rim - 0.10) < 1e-6
    assert dialog.rim_width == 1.0

    # Which colour each layer takes, not merely how much of it. A binding that
    # had to guess these could not draw the pane in the palette at all.
    assert dialog.stain_role is lxb.Role.GLASS
    assert dialog.header_role is lxb.Role.ACCENT_SOFT
    assert dialog.foot_role is lxb.Role.ACCENT
    assert dialog.rim_role is lxb.Role.ACCENT_SOFT


def test_the_context_menu_is_a_shape_as_well_as_a_material():
    shape = lxb.menu()
    # The width is the metric, not a second copy of it.
    assert shape.width == lxb.size(lxb.Metric.MENU_WIDTH)
    assert shape.row == 64.0
    assert shape.stacked_row == 96.0
    assert shape.title == 62.0
    assert shape.max_lines == 3
    assert abs(shape.dim - 0.42) < 1e-6
    assert abs(shape.scrim - 0.55) < 1e-6
    assert abs(shape.content_in - 0.45) < 1e-6
    assert abs(shape.panel_in - 0.25) < 1e-6

    # Never zero, however short the display.
    # What is left of the display once the panel's own furniture is out of it
    # — the margins, a header, both scroll strips and a band change — not the
    # display divided by a row.
    assert lxb.menu_rows_that_fit(1080.0, shape.row) == 10
    assert lxb.menu_rows_that_fit(60.0, shape.row) >= 1

    # The panel grows out of the control it is about, and back into it.
    anchor = (100.0, 200.0, 64.0, 64.0)
    panel = (300.0, 400.0, 440.0, 320.0)
    def centre(rect):
        return (rect[0] + rect[2] / 2, rect[1] + rect[3] / 2)

    shut = centre(lxb.menu_growing(anchor, panel, 0.0))
    assert abs(shut[0] - centre(anchor)[0]) < 1e-3
    assert abs(shut[1] - centre(anchor)[1]) < 1e-3
    assert lxb.menu_growing(anchor, panel, 1.0) == panel
    assert lxb.menu_growing(anchor, panel, 2.0) == panel

    # And it is the panel's own shape the whole way. Interpolating the two
    # rectangles instead carries it through the anchor's proportions, so a wide
    # flat button reshapes into a tall menu on its way.
    wide = (40.0, 600.0, 300.0, 48.0)
    tall = (500.0, 200.0, 440.0, 700.0)
    want = tall[2] / tall[3]
    for step in range(21):
        _, _, w, h = lxb.menu_growing(wide, tall, step / 20)
        assert abs(w / h - want) < 1e-3, f"at {step}/20 it is {w}x{h}"
    assert abs(lxb.menu_growing(wide, tall, 0.0)[2] - wide[2]) < 1e-3


def test_a_panel_arrives_faster_than_it_leaves():
    shape = lxb.menu()
    # Out of the control the glass is all there in the first quarter, so that
    # what grows out of it reads as a pane rather than as a rectangle being
    # inflated.
    assert lxb.menu_shown(shape.panel_in, True) == 1.0
    assert lxb.menu_shown(shape.panel_in * 0.5, True) == 0.5

    # Back into it the panel gives up its colour the whole way. Holding full
    # colour for three quarters of the fold and then going out over three
    # frames is a blink rather than a fade — and the frame it blinks in is the
    # frame the page underneath gets back every word the panel had taken.
    assert lxb.menu_shown(0.5, False) == 0.5
    assert lxb.menu_shown(0.75, False) > 0.7

    # The two cross at a sixteenth of the journey; past that the arriving
    # curve is always the fuller of the two.
    crossing = 1 / 16
    assert abs(lxb.menu_shown(crossing, True) - crossing) < 1e-4
    for step in range(33):
        travelled = crossing + (1 - crossing) * step / 32
        assert (lxb.menu_shown(travelled, False)
                <= lxb.menu_shown(travelled, True) + 1e-6), travelled

    # What is written on the panel waits for a panel to be read on, and leaves
    # with the glass rather than ahead of it: rows that emptied out first would
    # leave a blank pane standing there.
    assert lxb.menu_content_shown(shape.content_in, True) == 0.0
    assert lxb.menu_content_shown(1.0, True) == 1.0
    for step in range(21):
        travelled = step / 20
        assert (lxb.menu_content_shown(travelled, False)
                == lxb.menu_shown(travelled, False))

    # Off either end it is answered rather than extrapolated.
    assert lxb.menu_shown(-1.0, False) == 0.0
    assert lxb.menu_shown(2.0, True) == 1.0


def test_the_control_is_a_recipe_rather_than_a_colour():
    shape = lxb.control()
    # The chip is glass over glass, and the light is the accent — two roles
    # rather than one colour that changes.
    assert lxb.Role(shape.chip_role) == lxb.Role.GLASS_RAISED
    assert lxb.Role(shape.lit_role) == lxb.Role.ACCENT
    assert lxb.Role(shape.out_role) == lxb.Role.TEXT_SOFT
    assert abs(shape.chip_tint - 0.10) < 1e-6
    assert abs(shape.lit - 0.46) < 1e-6
    assert abs(shape.lit_answer - 0.50) < 1e-6
    assert shape.padding == 5.0
    # Being lit is brighter in every way the language says it.
    assert shape.lit > shape.chip_tint
    assert shape.lit_answer > shape.lit
    assert shape.ink > shape.ink_quiet
    assert shape.mark > shape.mark_quiet
    # A control takes the light quietly; the capsule over it takes it fully.
    assert shape.chip_gloss < lxb.glass(lxb.Surface.CONTROL).gloss

    # The halo is the surface's width and the control's centre, so two rows of
    # different heights are haloed alike.
    short = lxb.control_glow_rect((40.0, 100.0, 200.0, 40.0), 300.0)
    tall = lxb.control_glow_rect((40.0, 100.0, 200.0, 80.0), 300.0)
    assert short[2] == tall[2]
    assert abs(short[2] - 300.0 * shape.glow_width) < 1e-3
    assert abs(short[0] - tall[0]) < 1e-4
    assert abs(short[1] + short[3] / 2 - 120.0) < 1e-4

    # The light is on a control when it is over it, and off it when it is a
    # control away — in any direction, and at any size.
    rect = (100.0, 200.0, 300.0, 60.0)
    assert lxb.control_arrival(rect, rect) == 1.0
    assert lxb.control_arrival((100.0, 260.0, 300.0, 60.0), rect) == 0.0
    assert lxb.control_arrival((400.0, 200.0, 300.0, 60.0), rect) == 0.0
    assert abs(lxb.control_arrival((100.0, 230.0, 300.0, 60.0), rect) - 0.5) < 1e-5
    # A capsule of another shape sitting exactly on its corner is not light on
    # it, which is the case a y-only test gets wrong.
    assert lxb.control_arrival((100.0, 200.0, 900.0, 60.0), rect) == 0.0


def test_a_press_is_a_journey_rather_than_a_state():
    # Down, back past its own size, and settled on it.
    assert lxb.press_scale(0.0) == 1.0
    assert lxb.press_scale(1.0) == 1.0
    assert lxb.press_scale(-1.0) == 1.0
    assert lxb.press_scale(0.3) < 1.0
    over = max(lxb.press_scale(n / 100) for n in range(30, 100))
    assert over > 1.0, "it never came back past its own size"
    assert abs(lxb.press_scale(0.999) - 1.0) < 1e-3

    # The switch is thrown on the way up, not on the way down.
    assert lxb.fill_arrival(0.0) == 0.0
    assert lxb.fill_arrival(0.15) < 0.05
    assert abs(lxb.fill_arrival(1.0) - 1.0) < 1e-6

    # And it is about the control's own centre, so a label inside stays put.
    rect = (100.0, 40.0, 200.0, 60.0)
    assert lxb.pressed(rect, None) == rect
    sunk = lxb.pressed(rect, 0.3)
    assert sunk[2] < rect[2] and sunk[3] < rect[3]
    assert abs(sunk[0] + sunk[2] / 2 - (rect[0] + rect[2] / 2)) < 1e-4
    assert abs(sunk[1] + sunk[3] / 2 - (rect[1] + rect[3] / 2)) < 1e-4

    # The light breathes over one period and comes back where it started.
    period = lxb.DURATIONS["pulse"]
    assert abs(lxb.pulse(0.0) - 0.5) < 1e-6
    assert abs(lxb.pulse(period) - 0.5) < 1e-4
    breaths = [lxb.pulse(n * period / 180) for n in range(180)]
    assert min(breaths) < 0.01 and max(breaths) > 0.99

    # And the glide arrives without ever crossing its target.
    at, speed = 0.0, 0.0
    for _ in range(200):
        at, speed = lxb.glide(at, speed, 100.0, 1 / 60)
        assert at <= 100.0
    assert at > 99.9


def test_the_assets_are_really_here():
    assert len(lxb.GLYPHS) == 109
    assert len(lxb.SOUNDS) == 14
    assert len(lxb.SHELL_SOUNDS) == 12
    assert set(lxb.SOUNDS) - set(lxb.SHELL_SOUNDS) == {"trash", "error"}
    assert not hasattr(lxb.Sound, "TRASH")
    assert not hasattr(lxb.Sound, "ERROR")
    assert lxb.Sound.COMPAT_TRASH == "trash"
    assert lxb.Sound.COMPAT_ERROR == "error"

    launch = lxb.glyph("launch")
    assert launch is not None and launch.startswith(b"<svg")
    assert launch.rstrip().endswith(b"</svg>")
    assert b'data-lxb-material="lxb:shape"' in launch
    assert lxb.glyph("no-such-mark") is None
    assert lxb.glyph_box("volume") == 24
    assert lxb.glyph_box("launch") == 32
    assert lxb.glyph_box("no-such-mark") is None

    shader = lxb.glyph_wgsl()
    assert "fn lxb_glyph_material" in shader
    assert "@group" not in shader
    assert abs(lxb.GLYPH_SDF_RANGE - 0.125) < 1e-6
    assert lxb.GLYPH_SDF_SUPERSAMPLE == 4
    assert lxb.GLYPH_CELL == 128
    assert abs(lxb.GLYPH_DEPTH_SHARE - 0.075) < 1e-6
    assert lxb.GLYPH_LAMP[0] < 0 and lxb.GLYPH_LAMP[1] < 0 < lxb.GLYPH_LAMP[2]
    assert abs(lxb.GLYPH_SHADOW - 0.30) < 1e-6
    assert abs(lxb.GLYPH_SIMPLE_TINT - 0.22) < 1e-6
    assert abs(lxb.GLYPH_SIMPLE_ALPHA - 0.88) < 1e-6
    assert abs(lxb.GLYPH_SIMPLE_STAIN - 0.15) < 1e-6

    size = 4
    fine = size * lxb.GLYPH_SDF_SUPERSAMPLE
    coverage = bytes(
        128 if 4 <= x < 12 and 4 <= y < 12 else 127
        for y in range(fine)
        for x in range(fine)
    )
    field = lxb.glyph_sdf(coverage, size)
    assert len(field) == size * size
    assert field[1 * size + 1] < 128, "the shape is the negative side"
    assert field[0] > 128, "air is the positive side"
    try:
        lxb.glyph_sdf(coverage[:-1], size)
    except ValueError as err:
        assert "256 bytes" in str(err)
    else:  # pragma: no cover
        raise AssertionError("bad coverage dimensions were accepted")

    press = lxb.sound(lxb.Sound.MOVE)
    assert press is not None and press[:4] == b"OggS"
    assert lxb.sound("no-such-sound") is None
    assert 0 < lxb.SOUND_REST < 0.09

    assert lxb.font()[:4] == b"\x00\x01\x00\x00", "not a TTF"
    assert lxb.font(bold=True) != lxb.font()


def test_the_stylesheet_carries_the_same_names():
    css = lxb.stylesheet("Purple")
    assert ":root {" in css
    assert "--lxb-accent: #8b5cf6;" in css
    for name, _role in lxb.roles():
        assert f"--lxb-{name}:" in css
    for name in lxb.DURATIONS:
        assert f"--lxb-duration-{name}:" in css
    for name in [
        "radius", "stain", "header-light", "foot-light", "rim", "rim-width",
    ]:
        assert f"--lxb-overlay-{name}:" in css
    try:
        lxb.stylesheet("mauve")
    except ValueError:
        pass
    else:  # pragma: no cover
        raise AssertionError("an unknown palette produced a stylesheet")


def test_many_accents_can_come_and_go():
    """The one allocation with a free beside it, exercised hard enough that a
    leak or a double free would show."""
    for _ in range(500):
        accent = lxb.Accent("Red")
        accent.preview("Yellow")
        accent.advance(0.05)
        del accent
    # And an explicit close, twice, which must not fault.
    accent = lxb.Accent("Blue")
    accent.__exit__()
    accent.__exit__()


def test_the_painter_draws_the_material_rather_than_a_picture_of_it():
    """The three shaders, over a buffer of pixels.

    Not a comparison against a renderer — there is none here — but against
    what the model says has to be true: the scene is a picture rather than a
    fill, a pane reproduces what is behind it rather than tinting it, and a
    scrim pushes what is behind it back rather than only darkening it.
    """
    from lxb_toolkit import paint

    width, height = 80, 60
    stride = width * 4
    pixels = bytearray(stride * height)
    canvas = paint.Canvas(pixels, width, height, stride)

    def at(x, y):
        start = y * stride + x * 4
        return tuple(pixels[start + channel] for channel in (2, 1, 0))

    scene = paint.scene(lxb.PALETTES[0], 10.0)
    paint.wallpaper(canvas, scene)
    assert at(2, 2) != at(40, 30), "a wallpaper with one colour in it is a fill"
    assert all(pixels[i] == 0xFF for i in range(3, len(pixels), 4))

    # The sparkles are on unless turned off, and they only add light.
    assert scene.particles is True
    wide, tall = 320, 180
    plain = bytearray(wide * 4 * tall)
    lit = bytearray(wide * 4 * tall)
    paint.wallpaper(paint.Canvas(plain, wide, tall, wide * 4),
                    paint.scene(lxb.PALETTES[0], 10.0, particles=False))
    paint.wallpaper(paint.Canvas(lit, wide, tall, wide * 4), scene)
    assert lit != plain, "no sparkle was drawn"
    assert all(a >= b for a, b in zip(lit, plain)), "the sparkles took light away"
    quiet = bytearray(stride * height)
    scene.soften = paint.WALLPAPER_SOFTEN
    paint.wallpaper(paint.Canvas(quiet, width, height, stride), scene)
    assert sum(quiet) < sum(pixels), "softening did not quieten the scene"

    # A nearly clear pane over something dim comes out brighter than what it
    # is standing on: the bevel concentrates what it bends, and that is the
    # model working rather than a fault in it.
    behind = at(40, 30)
    paint.glass(canvas, (10, 10, 60, 40), 12.0,
                lxb.Glass(9.0, 0.2, 1.0, 0.0), (0.1, 0.08, 0.2, 0.1))
    assert at(40, 30) > behind

    # And a scrim leaves the region opaque, dimmer, and softer.
    edge = abs(at(20, 30)[0] - at(21, 30)[0])
    paint.scrim(canvas, (0, 0, width, height), 2.0, (0, 0, 0, 0.5))
    assert abs(at(20, 30)[0] - at(21, 30)[0]) <= max(edge, 1)
    assert all(pixels[i] == 0xFF for i in range(3, len(pixels), 4))


def test_a_canvas_the_painter_cannot_hold_is_refused():
    from lxb_toolkit import paint

    pixels = bytearray(16 * 16 * 4)
    paint.Canvas(pixels, 16, 16, 16 * 4)
    for width, height, stride in ((16, 16, 16 * 4 - 1), (16, 17, 16 * 4)):
        try:
            paint.Canvas(pixels, width, height, stride)
        except ValueError:
            continue
        raise AssertionError(f"{width}x{height} at {stride} was accepted")


def test_a_mark_needs_one_whole_cell():
    from lxb_toolkit import paint

    pixels = bytearray(32 * 32 * 4)
    canvas = paint.Canvas(pixels, 32, 32, 32 * 4)
    for wrong in (b"", bytes(16), bytes(lxb.GLYPH_CELL * lxb.GLYPH_CELL + 1)):
        try:
            paint.glyph(canvas, (0, 0, 32, 32), wrong, (1, 1, 1, 1),
                        (0.5, 0.5, 0.9), 1.0)
        except ValueError:
            continue
        raise AssertionError(f"a field of {len(wrong)} bytes was accepted")
    assert not any(pixels), "a refused mark drew anyway"


def test_a_panel_is_measured_from_the_rows_on_it():
    """The geometry, not just the tokens.

    A panel settles *beside* the control it is about — never over it, because
    the anchor is the only context the panel has — and flips to the other side
    when it would run off the display. Its rows are measured off the list it is
    carrying, with a gap and a rule wherever two of them disagree about which
    band they are in.
    """
    rows = [lxb.MenuRow(), lxb.MenuRow(), lxb.MenuRow(group=1)]
    display = (1280.0, 800.0)
    fit = lxb.menu_rows_of_that_fit(rows, 1, display[1])
    assert fit == 3

    with lxb.MenuLayout(rows, 1, (150, 200, 120, 120), display, visible=fit) as it:
        panel = it.panel
        # Beside the anchor, to the right of it, clear by the component's gap.
        assert panel[0] > 150 + 120
        first, second, third = (it.row(index) for index in range(3))
        assert first is not None and third is not None
        # Every row is the same height and the same width, inside the margins.
        assert abs(first[3] - second[3]) < 1e-3
        assert panel[0] < first[0] and first[0] + first[2] < panel[0] + panel[2]
        # A change of band opens a gap, and rules a hairline centred in it.
        plain = second[1] - first[1]
        assert third[1] - second[1] > plain
        rules = it.separators()
        assert len(rules) == 1
        assert second[1] + second[3] < rules[0][1] < third[1]
        # And the light stands on the selected row's face.
        assert it.highlight() == it.chip(0)

    # An anchor near the right-hand edge flips it to the other side.
    with lxb.MenuLayout(rows, 1, (1100, 200, 120, 120), display, visible=fit) as it:
        assert it.panel[0] + it.panel[2] < 1100

    # A row that has opened out is taller, and only that one.
    opening = [lxb.MenuRow(lines=3), lxb.MenuRow()]
    with lxb.MenuLayout(opening, 0, (10, 400, 40, 40), display,
                        visible=2, selected=0, unfolded=1.0) as it:
        assert it.opened(0)[0] > 0.0
        assert it.opened(1) == (0.0, 0.0)
        assert it.row(0)[3] > it.row(1)[3]


def test_every_documented_public_name_is_exported():
    assert len(lxb.__all__) == len(set(lxb.__all__))
    for name in lxb.__all__:
        assert hasattr(lxb, name), name
    for name in [
        "palette", "glass", "size", "is_share", "text_size",
        "text_is_bold", "roles", "ShellTheme", "glyph_box", "glyph_wgsl",
        "wallpaper_wgsl",
        "overlay_material", "paint", "MenuLayout", "MenuRow",
        "Action", "Key", "Button", "Repeat", "Wheel",
        "action_of_key", "action_of_letter", "action_of_button",
        "sound_amplitude", "sound_fade",
    ]:
        assert name in lxb.__all__


# --- what the user pressed, and what it means -------------------------------


def test_every_action_has_a_name_and_finds_its_way_back() -> None:
    """One list of actions, and every one of them addressable by name."""
    assert lxb.ACTIONS[lxb.Action.ACCEPT] == "accept"
    assert len(set(lxb.ACTIONS)) == len(lxb.ACTIONS)
    for index, name in enumerate(lxb.ACTIONS):
        assert lxb.action_named(name) == index
    assert lxb.action_named("guide") is None
    # The shell's own words for the two that differ are not this library's.
    assert lxb.action_named("launch") is None
    assert lxb.action_named("next-screen") is None


def test_the_keys_mean_what_the_rust_library_says_they_mean() -> None:
    assert lxb.action_of_key(lxb.Key.LEFT) == lxb.Action.LEFT
    assert lxb.action_of_key(lxb.Key.ENTER) == lxb.Action.ACCEPT
    assert lxb.action_of_key(lxb.Key.SPACE) == lxb.Action.ACCEPT
    assert lxb.action_of_key(lxb.Key.ESCAPE) == lxb.Action.BACK
    assert lxb.action_of_key(lxb.Key.BACKSPACE) == lxb.Action.BACK
    assert lxb.action_of_key(lxb.Key.F10) == lxb.Action.MENU
    # The shell gives this one to the guide and says why; an application has
    # no guide to give it to.
    assert lxb.action_of_key(lxb.Key.MENU) == lxb.Action.MENU
    assert lxb.action_of_key(lxb.Key.TAB) == lxb.Action.NEXT
    assert lxb.action_of_key(lxb.Key.BACKTAB) == lxb.Action.PREVIOUS
    assert lxb.action_of_key(99) is None


def test_the_letters_are_asked_for_separately() -> None:
    """An application with a field in it must not bind them."""
    assert lxb.action_of_letter("w") == lxb.Action.UP
    assert lxb.action_of_letter("W") == lxb.Action.UP
    assert lxb.action_of_letter("h") == lxb.Action.LEFT
    assert lxb.action_of_letter("Y") == lxb.Action.MENU
    assert lxb.action_of_letter("q") is None
    assert lxb.action_of_letter("") is None
    assert lxb.action_of_letter("ab") is None


def test_the_guide_button_is_never_an_applications() -> None:
    """Three buttons answer nothing, each for its own reason."""
    for button in (lxb.Button.GUIDE, lxb.Button.WEST, lxb.Button.SELECT):
        assert lxb.action_of_button(button) is None
    assert lxb.action_of_button(lxb.Button.SOUTH) == lxb.Action.ACCEPT
    assert lxb.action_of_button(lxb.Button.EAST) == lxb.Action.BACK
    assert lxb.action_of_button(lxb.Button.NORTH) == lxb.Action.MENU
    assert lxb.action_of_button(lxb.Button.START) == lxb.Action.SUBMIT
    assert lxb.action_of_button(99) is None


def test_only_a_direction_repeats() -> None:
    """A finger resting on Enter is one press, not a run of them."""
    for action in (lxb.Action.LEFT, lxb.Action.RIGHT, lxb.Action.UP, lxb.Action.DOWN):
        assert lxb.action_repeats(action)
    for action in (lxb.Action.ACCEPT, lxb.Action.BACK, lxb.Action.MENU):
        assert not lxb.action_repeats(action)


def test_a_held_direction_fires_once_and_then_repeats() -> None:
    """The same cadence the pad walks at, because it is the same cadence."""
    repeat = lxb.Repeat()
    down = (False, False, False, True)
    assert repeat.update(0.0, down) == [lxb.Action.DOWN]
    assert repeat.update(lxb.INPUT_INITIAL_REPEAT - 0.01, down) == []
    assert repeat.update(lxb.INPUT_INITIAL_REPEAT + 0.01, down) == [lxb.Action.DOWN]

    # Letting go and pressing again starts the delay over.
    none = (False, False, False, False)
    assert repeat.update(0.5, none) == []
    assert repeat.update(0.51, down) == [lxb.Action.DOWN]

    repeat.reset()
    try:
        repeat.update(0.0, (True, False))
    except ValueError:
        pass
    else:
        raise AssertionError("four directions or nothing")


def test_a_stick_engages_and_releases_at_different_distances() -> None:
    """One threshold would make a stick resting near it chatter."""
    repeat = lxb.Repeat()
    none = (False, False, False, False)
    assert lxb.INPUT_STICK_RELEASE < lxb.INPUT_STICK_ENGAGE
    assert repeat.update(0.0, none, (lxb.INPUT_STICK_ENGAGE - 0.01, 0.0)) == []
    assert repeat.update(0.01, none, (lxb.INPUT_STICK_ENGAGE + 0.01, 0.0)) == [
        lxb.Action.RIGHT
    ]
    # Below engage is not far enough back to be let go.
    assert repeat.update(0.02, none, (lxb.INPUT_STICK_RELEASE + 0.05, 0.0)) == []
    # A corner is two directions, and answers as two.
    fresh = lxb.Repeat()
    assert fresh.update(0.0, none, (-0.9, 0.9)) == [lxb.Action.LEFT, lxb.Action.UP]


def test_a_part_turned_wheel_is_carried_rather_than_lost() -> None:
    wheel = lxb.Wheel()
    assert [wheel.notches(0.34) for _ in range(3)] == [0, 0, 1]

    # A wheel says how many notches it turned and is taken at its word.
    real = lxb.Wheel()
    assert real.notches(1.0) == 1
    assert real.notches(-2.0) == -2

    touchpad = lxb.Wheel()
    assert touchpad.distance(lxb.INPUT_SCROLL_STEP) == 1
    assert touchpad.distance(lxb.INPUT_SCROLL_STEP / 2) == 0
    assert touchpad.distance(lxb.INPUT_SCROLL_STEP / 2) == 1
    touchpad.reset()
    assert touchpad.distance(lxb.INPUT_SCROLL_STEP / 2) == 0


def test_a_level_is_loudness_rather_than_amplitude() -> None:
    """Half a control is not half the scale, which is the whole curve."""
    assert lxb.sound_amplitude(0.0) == 0.0
    assert abs(lxb.sound_amplitude(1.0) - 1.0) < 1e-5
    assert lxb.sound_amplitude(0.5) < 0.25
    assert lxb.sound_amplitude(-1.0) == 0.0
    assert lxb.sound_amplitude(2.0) == lxb.sound_amplitude(1.0)


def test_a_fade_starts_and_ends_flat() -> None:
    """An ear hears a corner as a click, which is what a fade avoids."""
    over = lxb.SOUND_MUSIC_FADE_IN
    assert lxb.sound_fade(0.0, over) == 0.0
    assert lxb.sound_fade(over, over) == 1.0
    assert lxb.sound_fade(over * 99, over) == 1.0
    assert abs(lxb.sound_fade(over / 2, over) - 0.5) < 1e-6
    assert lxb.SOUND_MUSIC_FADE_OUT > lxb.SOUND_MUSIC_FADE_IN


def test_a_wheel_says_which_list_it_was_over_without_taking_its_directions() -> None:
    """The pointer half of a gesture, asked the way C asks for it.

    Every step is already in `actions()`; this only says which of a page's own
    lists the pointer was over, so a page can move that one instead of
    whichever the light was left on.
    """
    from lxb_toolkit import app as app_module

    asked = []

    class Stub:
        @staticmethod
        def lxb_page_scrolled(_page, ident):
            asked.append(ident)
            return 3 if ident == 0x7001 else 0

        @staticmethod
        def lxb_draw_soft_edges(_page, rect, band, top, bottom):
            asked.append((rect[0], rect[1], rect[2], rect[3], band, top, bottom))

    page = app_module.Page.__new__(app_module.Page)
    page._page = None
    real, app_module._lib = app_module._lib, Stub()
    try:
        over_list = page.scrolled(0x7001)
        over_panel = page.scrolled(0x7000)
        page.draw_soft_edges((10.0, 20.0, 300.0, 400.0), 40.0, 1.0, 0.5)
    finally:
        app_module._lib = real

    assert over_list == 3
    assert over_panel == 0
    assert asked[:2] == [0x7001, 0x7000]
    assert asked[2] == (10.0, 20.0, 300.0, 400.0, 40.0, 1.0, 0.5)


def test_what_the_controls_said_comes_back_as_numbers() -> None:
    """Every action a driven page reads, without a window.

    `Action` is a set of constants, not an enumeration, so calling it raises
    TypeError. `actions()` did exactly that, and an exception raised inside the
    draw callback abandons the frame half-drawn — which on screen is the window
    blinking black on every key. No picture could catch it: `shot` draws a
    settled frame and never delivers an action at all.
    """
    from lxb_toolkit import app as app_module

    said = [lxb.Action.DOWN, lxb.Action.ACCEPT, lxb.Action.MENU, -1]
    calls = iter(said)

    class Stub:
        @staticmethod
        def lxb_page_action(_page):
            return next(calls)

    page = app_module.Page.__new__(app_module.Page)
    page._page = None
    real, app_module._lib = app_module._lib, Stub()
    try:
        got = list(page.actions())
    finally:
        app_module._lib = real

    assert got == said[:-1], got
    assert all(isinstance(action, int) for action in got)
    assert got[2] == lxb.Action.MENU, "it has to compare against the constants"


def test_every_file_of_an_answer_is_read_one_call_at_a_time() -> None:
    """picked_files walks lxb_page_picked_next until it answers null.

    The C side hands one owned string back per call and null when the answer is
    spent, so a binding that read it once would silently drop every file but
    the first of a many-files question.
    """
    from lxb_toolkit import app as app_module

    answer = [b"/tmp/one.png", b"/tmp/two.png"]
    freed: list[int] = []
    kept: list[ctypes.Array] = []

    class Stub:
        @staticmethod
        def lxb_page_picked_next(_page):
            if not answer:
                return None
            buffer = ctypes.create_string_buffer(answer.pop(0))
            kept.append(buffer)
            return ctypes.cast(buffer, ctypes.c_void_p).value

        @staticmethod
        def lxb_app_string_free(raw):
            freed.append(raw)

    page = app_module.Page.__new__(app_module.Page)
    page._page = None
    real, app_module._lib = app_module._lib, Stub()
    try:
        got = page.picked_files()
    finally:
        app_module._lib = real

    assert got == [Path("/tmp/one.png"), Path("/tmp/two.png")], got
    assert len(freed) == 2, "every path handed over has to be released"


def test_a_question_names_the_purpose_it_puts() -> None:
    """pick, pick_many and save reach three different calls."""
    from lxb_toolkit import app as app_module

    asked: list[tuple[str, tuple]] = []

    class Stub:
        @staticmethod
        def lxb_page_pick(page, selection, directory):
            asked.append(("pick", (selection, directory)))
            return 1

        @staticmethod
        def lxb_page_pick_many(page, selection, directory):
            asked.append(("pick_many", (selection, directory)))
            return 1

        @staticmethod
        def lxb_page_save(page, name, directory):
            asked.append(("save", (name, directory)))
            return 1

    page = app_module.Page.__new__(app_module.Page)
    page._page = None
    real, app_module._lib = app_module._lib, Stub()
    try:
        assert page.pick(lxb.Selection.IMAGE, "/tmp")
        assert page.pick_many(lxb.Selection.FILE, "/tmp")
        assert page.save("notes.txt", "/tmp")
        try:
            page.pick("not a selection", "/tmp")
        except ValueError:
            pass
        else:  # pragma: no cover - the guard is the point
            raise AssertionError("an unknown selection was accepted")
    finally:
        app_module._lib = real

    assert [name for name, _ in asked] == ["pick", "pick_many", "save"]
    assert asked[0][1] == (int(lxb.Selection.IMAGE), b"/tmp")
    assert asked[2][1] == (b"notes.txt", b"/tmp")


if __name__ == "__main__":
    failures = 0
    for name, test in sorted(globals().items()):
        if name.startswith("test_") and callable(test):
            try:
                test()
                print(f"ok   {name}")
            except Exception as err:  # noqa: BLE001
                failures += 1
                print(f"FAIL {name}: {err}")
    print(f"\n{'FAILED' if failures else 'ok'} — {failures} failed")
    sys.exit(1 if failures else 0)
