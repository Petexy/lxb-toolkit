"""The three shipped shaders, on the processor.

:func:`lxb_toolkit.glass_wgsl`, :func:`lxb_toolkit.wallpaper_wgsl` and
:func:`lxb_toolkit.glyph_wgsl` hand over the real drawing, and running WGSL
needs a renderer. This module draws the same three over a buffer of pixels
instead — the same constants, the same terms, in the same order — so that a
program with a painter under it gets the material rather than a flat rectangle
with a bright edge painted on.

    >>> import cairo, lxb_toolkit as lxb
    >>> from lxb_toolkit import paint
    >>> image = cairo.ImageSurface(cairo.FORMAT_ARGB32, 320, 200)
    >>> canvas = paint.Canvas.of(image)
    >>> paint.wallpaper(canvas, paint.scene(lxb.PALETTES[0], 10.0))
    >>> paint.glass(canvas, (20, 20, 280, 160), 22.0,
    ...             lxb.glass(lxb.Surface.SIDEBAR), (0.2, 0.15, 0.4, 0.38))

The canvas is both what a pane refracts and where it lands, so draw back to
front: the background, then the panes standing on it, then the panes standing
on those.

Every one of these draws on every core the machine has. The scene is thirty-odd
transcendental functions per pixel, so a program that wants it cheaply should
fill a smaller canvas and scale that up with its own painter; every term in it
is broad enough to survive that.
"""

from __future__ import annotations

import ctypes
from dataclasses import dataclass, field

from . import (
    Accent,
    Color,
    Glass,
    Palette,
    WallpaperStyle,
    _Canvas,
    _lib,
    _Mark,
    _Pane,
    _Rgba,
    _Scene,
)

__all__ = [
    "Canvas",
    "glass",
    "glyph",
    "light",
    "Scene",
    "scene",
    "scrim",
    "wallpaper",
    "WALLPAPER_SOFTEN",
]

#: How much the scene is softened when an interface is standing on it.
WALLPAPER_SOFTEN: float = _lib.lxb_wallpaper_soften()


def _rgba(color) -> _Rgba:
    """A colour, however it was given, as the four floats the ABI takes."""
    if isinstance(color, Color):
        return _Rgba(*color.linear)
    values = tuple(float(value) for value in color)
    if len(values) == 3:
        values += (1.0,)
    if len(values) != 4:
        raise ValueError("a colour is three linear channels, or four with an alpha")
    return _Rgba(*values)


def _rect(rect) -> tuple[float, float, float, float]:
    values = tuple(float(value) for value in rect)
    if len(values) != 4:
        raise ValueError("a rectangle is x, y, width and height")
    return values


class Canvas:
    """Somebody else's pixels, kept alive for as long as this is.

    32-bit words in the machine's own order, alpha in the top byte,
    premultiplied, sRGB-encoded — which is what cairo's ``FORMAT_ARGB32``,
    Qt's ``Format_ARGB32_Premultiplied`` and an SDL ``ARGB8888`` surface all
    are. ``pixels`` is any writable buffer: a cairo surface's ``get_data()``,
    a ``bytearray``, a ``memoryview``.
    """

    __slots__ = ("_buffer", "_canvas", "_image", "width", "height", "stride")

    def __init__(self, pixels, width: int, height: int, stride: int) -> None:
        if stride < width * 4:
            raise ValueError("the stride does not reach across a row")
        if len(memoryview(pixels)) < (height - 1) * stride + width * 4:
            raise ValueError("the buffer does not reach the last row")
        # Held, not copied: the caller draws on this and then reads it back.
        self._buffer = (ctypes.c_ubyte * len(memoryview(pixels))).from_buffer(pixels)
        self._image = None
        self.width = width
        self.height = height
        self.stride = stride
        self._canvas = _Canvas(self._buffer, width, height, stride)

    @classmethod
    def of(cls, image) -> "Canvas":
        """A cairo ``ImageSurface``, flushed and ready to be drawn on.

        Duck-typed rather than imported: this library has no dependencies, and
        anything answering the same five calls works. Use it with
        :meth:`done`, or as a context manager, so the surface is told its
        pixels changed.
        """
        image.flush()
        canvas = cls(
            image.get_data(), image.get_width(), image.get_height(), image.get_stride()
        )
        canvas._image = image
        return canvas

    def done(self) -> None:
        """Tell a cairo surface its pixels were changed behind its back."""
        if self._image is not None:
            self._image.mark_dirty()

    def __enter__(self) -> "Canvas":
        return self

    def __exit__(self, *_exception) -> None:
        self.done()


@dataclass
class Scene:
    """The analytic scene, and everything constant across one picture of it.

    ``time`` is the wallpaper clock in seconds: the same value here and in the
    shader is the same frame. ``soften`` is 0 for the picture itself and up to
    1 for the blurred, dimmed variant — :data:`WALLPAPER_SOFTEN` is the amount
    to use when an interface is standing on it. A custom wallpaper is not
    something arithmetic can produce, so it draws the Default water.
    """

    time: float = 0.0
    soften: float = 0.0
    style: WallpaperStyle = WallpaperStyle.DEFAULT
    sky: tuple = ()
    accent: tuple = ()
    glow: tuple = field(default=(0.0, 0.0, 0.0, 1.0))

    def _raw(self) -> _Scene:
        raw = _Scene()
        raw.time = float(self.time)
        raw.soften = float(self.soften)
        raw.style = int(self.style)
        if len(self.sky) != 4 or len(self.accent) != 3:
            raise ValueError("a scene is four sky colours and three accents")
        for index, colour in enumerate(self.sky):
            raw.sky[index] = _rgba(colour)
        for index, colour in enumerate(self.accent):
            raw.accent[index] = _rgba(colour)
        raw.glow = _rgba(self.glow)
        return raw


def scene(palette, time: float, soften: float = 0.0,
          style: WallpaperStyle = WallpaperStyle.DEFAULT) -> Scene:
    """The scene a palette draws, at a moment.

    ``palette`` is a :class:`~lxb_toolkit.Palette` or a travelling
    :class:`~lxb_toolkit.Accent` — the accent, so that the scene follows a
    palette change rather than jumping when it lands.
    """
    if isinstance(palette, Accent):
        got = _lib.lxb_accent_scene(palette._handle, time)
    elif isinstance(palette, Palette):
        got = _lib.lxb_scene_for(palette.index, time)
    else:
        raise TypeError("a scene is drawn by a Palette or a travelling Accent")
    return Scene(
        time=got.time,
        soften=soften,
        style=style,
        sky=tuple((c.r, c.g, c.b, c.a) for c in got.sky),
        accent=tuple((c.r, c.g, c.b, c.a) for c in got.accent),
        glow=(got.glow.r, got.glow.g, got.glow.b, got.glow.a),
    )


def wallpaper(canvas: Canvas, scene: Scene) -> None:
    """Fill a canvas with the analytic scene.

    ``uv`` runs over the whole canvas, so this is the background rather than
    something to tile: a smaller canvas is the same picture read more coarsely.
    """
    if not _lib.lxb_paint_wallpaper(ctypes.byref(canvas._canvas),
                                    ctypes.byref(scene._raw())):
        raise ValueError("the canvas cannot be drawn on")


def glass(canvas: Canvas, rect, radius: float, material: Glass, tint,
          power: float = 2.0, opacity: float = 1.0) -> None:
    """Lay one pane of glass over what the canvas already holds.

    ``material.depth`` is the one length here that is not in reference pixels:
    scale it by :func:`~lxb_toolkit.scale_for` first, exactly as you scale a
    row height. ``power`` is 2 for a circle-cornered rectangle and 4 for a
    squircle.
    """
    x, y, width, height = _rect(rect)
    pane = _Pane()
    pane.x, pane.y, pane.width, pane.height = x, y, width, height
    pane.radius = float(radius)
    pane.power = float(power)
    pane.glass.depth = float(material.depth)
    pane.glass.frost = float(material.frost)
    pane.glass.gloss = float(material.gloss)
    pane.glass.curve = float(material.curve)
    pane.tint = _rgba(tint)
    pane.opacity = float(opacity)
    if not _lib.lxb_paint_glass(ctypes.byref(canvas._canvas), ctypes.byref(pane)):
        raise ValueError("the canvas cannot be drawn on")


def light(canvas: Canvas, rect, tint) -> None:
    """The soft ellipse of light under a pane, and behind what is aimed at.

    A gaussian sealed at its own edge, so it never draws a visible disc —
    which a painter's own radial gradient, running linearly to nothing at a
    hard radius, does. Draw it before the pane, so the pane bends it.
    """
    values = (ctypes.c_float * 4)(*_rect(rect))
    if not _lib.lxb_paint_light(ctypes.byref(canvas._canvas), values, _rgba(tint)):
        raise ValueError("the canvas cannot be drawn on")


def scrim(canvas: Canvas, rect, blur: float, tint) -> None:
    """Push what is already on the canvas back: blurred, and dimmed.

    ``blur`` is a rung of the same chain a frosted pane reaches into, and
    nought for no blur at all; ``tint`` is the dim. The region comes back
    opaque.
    """
    values = (ctypes.c_float * 4)(*_rect(rect))
    if not _lib.lxb_paint_scrim(ctypes.byref(canvas._canvas), values,
                                float(blur), _rgba(tint)):
        raise ValueError("the canvas cannot be drawn on")


def glyph(canvas: Canvas, rect, field: bytes, color, accent_soft,
          gloss: float, simple: bool = False) -> None:
    """Shade one mark out of the distance field made of it.

    ``field`` is what :func:`~lxb_toolkit.glyph_sdf` hands back, and must be
    :data:`~lxb_toolkit.GLYPH_CELL` square. The alpha of ``color`` is the
    mark's opacity. Not the SVG: drawing that directly gets the silhouette and
    none of this.
    """
    x, y, width, height = _rect(rect)
    mark = _Mark()
    mark.x, mark.y, mark.width, mark.height = x, y, width, height
    mark.color = _rgba(color)
    mark.accent_soft = _rgba(accent_soft)
    mark.gloss = float(gloss)
    mark.simple = 1 if simple else 0
    plane = (ctypes.c_ubyte * len(field)).from_buffer_copy(field)
    if not _lib.lxb_paint_glyph(ctypes.byref(canvas._canvas), ctypes.byref(mark),
                                plane, len(field)):
        raise ValueError("the field is not one glyph cell, or the canvas cannot"
                         " be drawn on")
