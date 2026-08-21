"""The window, the loop, the controls and the sounds.

Everything an application in this language has in common with every other one,
so that a program contains only what is on its own page::

    import lxb_toolkit as lxb

    app = lxb.App("com.example.HelloWorld", "Hello World")

    @app.page
    def _(page):
        page.head("launch", "Hello, world")
        if page.button("Hello World!"):
            print("clicked")

    app.run()

That is a complete application: a Wayland window carrying the shell's wallpaper
and glass, driven by a keyboard, a pointer and every controller plugged into
the machine, answering with the shell's own sounds. Nothing in it names a
colour, a radius, a duration or a device.

This module opens a *second* library, ``liblxb_app``. The first one — what
:mod:`lxb_toolkit` itself talks to — answers what the language is and has no
dependencies at all, which is the point of it. This one carries a GPU stack, a
window system, the controllers and an audio device, so a program that only
wants the answers never has to have it installed. Importing this module is what
asks for it, and it is imported lazily by the package for exactly that reason.

Controls are numbered by the order they are drawn in, which is the order they
are read in — so the light moves between them in that order too, and no program
has to name, register or lay out its own focus. A control answers ``True`` on
the frame a press lands on it, whether that press came from a key, a
controller's bottom face button or a click.
"""

from __future__ import annotations

import ctypes
from typing import Callable, Sequence

from . import SOUNDS, Action, Metric, Role, Sound, Text, _load

__all__ = ["App", "Page", "Align", "Press", "AppNotFound"]


class AppNotFound(Exception):
    """Raised when ``liblxb_app`` could not be opened.

    Its own name rather than the toolkit's, because the two libraries are
    installed separately on purpose and a program told "the toolkit is missing"
    would go looking in the wrong place.
    """


try:
    _lib, _library_path = _load("lxb_app")
except Exception as trouble:  # pragma: no cover - depends on what is installed
    raise AppNotFound(str(trouble)) from trouble


def library_path():
    """Which shared library this module is talking to."""
    return _library_path


class Align:
    """Which end of its box a line of writing is set at."""

    LEFT = 0
    CENTRE = 1
    RIGHT = 2


class Press:
    """How something that can be pressed is currently standing."""

    RESTING = 0
    FOCUSED = 1
    PRESSED = 2

    @staticmethod
    def going(through: float) -> int:
        """Part-way through a press: 0 at the top of its travel, 1 settled."""
        return 3 + int(max(0.0, min(1.0, through)) * 100)


_PAGE_FN = ctypes.CFUNCTYPE(None, ctypes.c_void_p, ctypes.c_void_p)

_SIGNATURES = [
    ("lxb_app_new", [ctypes.c_char_p, ctypes.c_char_p], ctypes.c_void_p),
    ("lxb_app_size", [ctypes.c_void_p, ctypes.c_double, ctypes.c_double], None),
    ("lxb_app_plain", [ctypes.c_void_p], None),
    ("lxb_app_driven", [ctypes.c_void_p], None),
    ("lxb_app_run", [ctypes.c_void_p, _PAGE_FN, ctypes.c_void_p], ctypes.c_int),
    ("lxb_app_shot",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint, ctypes.c_uint,
      ctypes.c_float, _PAGE_FN, ctypes.c_void_p], ctypes.c_int),
    ("lxb_app_trouble", [ctypes.c_void_p], ctypes.c_char_p),
    ("lxb_app_free", [ctypes.c_void_p], None),
    ("lxb_page_width", [ctypes.c_void_p], ctypes.c_float),
    ("lxb_page_height", [ctypes.c_void_p], ctypes.c_float),
    ("lxb_page_seconds", [ctypes.c_void_p], ctypes.c_float),
    ("lxb_page_action", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_page_focus", [ctypes.c_void_p, ctypes.c_ulong], None),
    ("lxb_page_focused", [ctypes.c_void_p], ctypes.c_ulong),
    ("lxb_page_glide",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_place",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_press", [ctypes.c_void_p, ctypes.c_int], ctypes.c_ulong),
    ("lxb_page_pressed", [ctypes.c_void_p, ctypes.c_uint], ctypes.c_int),
    ("lxb_page_quit", [ctypes.c_void_p], None),
    ("lxb_page_play", [ctypes.c_void_p, ctypes.c_ulong], None),
    ("lxb_page_volume", [ctypes.c_void_p, ctypes.c_float, ctypes.c_int], None),
    ("lxb_page_cursor", [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_set_cursor", [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_title", [ctypes.c_void_p, ctypes.c_char_p], None),
    ("lxb_page_heading", [ctypes.c_void_p, ctypes.c_char_p], None),
    ("lxb_page_text", [ctypes.c_void_p, ctypes.c_char_p], None),
    ("lxb_page_note", [ctypes.c_void_p, ctypes.c_char_p], None),
    ("lxb_page_icon", [ctypes.c_void_p, ctypes.c_char_p], None),
    ("lxb_page_head", [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p], None),
    ("lxb_page_rule", [ctypes.c_void_p], None),
    ("lxb_page_gap", [ctypes.c_void_p], None),
    ("lxb_page_button", [ctypes.c_void_p, ctypes.c_char_p], ctypes.c_int),
    ("lxb_page_row", [ctypes.c_void_p, ctypes.c_char_p], ctypes.c_int),
    ("lxb_page_item", [ctypes.c_void_p, ctypes.c_char_p], ctypes.c_int),
    ("lxb_page_row_value",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p], ctypes.c_int),
    ("lxb_page_menu",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.POINTER(ctypes.c_char_p),
      ctypes.c_ulong], None),
    ("lxb_page_chose", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_page_ask",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p,
      ctypes.POINTER(ctypes.c_char_p), ctypes.c_ulong], None),
    ("lxb_page_answered", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_draw_pane",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_ulong], None),
    ("lxb_draw_card",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_ulong,
      ctypes.c_ulong, ctypes.c_float], None),
    ("lxb_draw_label",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_ulong,
      ctypes.c_char_p, ctypes.c_ulong, ctypes.c_ulong], None),
    ("lxb_draw_paragraph",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_char_p,
      ctypes.c_ulong], ctypes.c_float),
    ("lxb_draw_icon",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_char_p], None),
    ("lxb_draw_rule",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_ulong], None),
    ("lxb_draw_button",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_char_p,
      ctypes.c_ulong], None),
    ("lxb_draw_row",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_char_p,
      ctypes.c_char_p, ctypes.c_ulong], None),
    ("lxb_draw_selection",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float], None),
    ("lxb_draw_spot",
     [ctypes.c_void_p, ctypes.c_uint, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_at",
     [ctypes.c_void_p, ctypes.c_float, ctypes.c_float], ctypes.c_int),
    ("lxb_page_metric", [ctypes.c_void_p, ctypes.c_ulong], ctypes.c_float),
    ("lxb_page_scaled", [ctypes.c_void_p, ctypes.c_float], ctypes.c_float),
    ("lxb_page_line", [ctypes.c_void_p, ctypes.c_ulong], ctypes.c_float),
    ("lxb_page_measure",
     [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_char_p], ctypes.c_float),
]

for _name, _argtypes, _restype in _SIGNATURES:
    _function = getattr(_lib, _name)
    _function.argtypes = _argtypes
    _function.restype = _restype

_FOUR = ctypes.c_float * 4


def _utf8(text):
    """A string on its way across, or NULL for one that was not given."""
    return None if text is None else str(text).encode("utf-8")


def _sound_index(sound) -> int:
    """A recording as the library numbers them, whichever way it was named."""
    if isinstance(sound, str):
        try:
            return SOUNDS.index(sound)
        except ValueError:
            raise ValueError(f"no such recording: {sound!r}") from None
    return int(sound)


def _list(strings: Sequence[str]):
    """A row of string pointers, kept alive for the length of the call."""
    encoded = [_utf8(text) for text in strings]
    array = (ctypes.c_char_p * len(encoded))(*encoded)
    return array, len(encoded)


class Page:
    """One frame, and everything a program does with it.

    The flow methods — :meth:`title`, :meth:`text`, :meth:`button` and the rest
    — lay themselves out down the page at the language's own sizes and
    spacings, so a program that is a list of things never computes a rectangle.
    The ``draw_`` methods are the renderer itself, for one that is not.

    One of these is made per application rather than per frame: it is a handle
    to the frame that is currently being drawn, and a new Python object sixty
    times a second is a cost a program should not pay to say hello.
    """

    __slots__ = ("_page",)

    def __init__(self) -> None:
        self._page = None

    # -- what the frame is -------------------------------------------------

    @property
    def width(self) -> float:
        return _lib.lxb_page_width(self._page)

    @property
    def height(self) -> float:
        return _lib.lxb_page_height(self._page)

    @property
    def seconds(self) -> float:
        """Seconds since the window opened: the one clock everything with a
        movement of its own is measured against."""
        return _lib.lxb_page_seconds(self._page)

    def actions(self):
        """Everything the keyboard and the controllers said since the last
        frame, in the order they said it, with nothing in it saying which sent
        what.

        Empty unless the application asked for ``driven=True`` — a page whose
        light is walked for it must not also walk it, because two things moving
        one selection is one selection too many.
        """
        while True:
            action = _lib.lxb_page_action(self._page)
            if action < 0:
                return
            # The number itself. :class:`~lxb_toolkit.Action` is a set of
            # constants rather than an enumeration, so calling it raises
            # TypeError — and raising inside the draw callback abandons the
            # frame half-drawn, which reads as the window blinking black.
            yield action

    @property
    def focus(self) -> int:
        """Which control the light is on, by the order it is drawn in."""
        return int(_lib.lxb_page_focused(self._page))

    @focus.setter
    def focus(self, index: int) -> None:
        """Say where it is. What a page that moves its own selection does
        before it draws — see ``driven=True``. A page that does not is walked
        between its controls for it, and setting this would be two things
        moving one selection."""
        _lib.lxb_page_focus(self._page, index)

    def glide(self, rect, strength: float = 1.0):
        """The lit capsule that says where the light is, drawn travelling.

        The light is one object crossing the page rather than a property each
        control has, so it glides onto what is chosen instead of appearing on
        it. Answers where it actually is this frame, which is not where it was
        asked to be until it arrives. Draw it *before* the controls it is
        behind, so their faces sit over it.
        """
        at = (ctypes.c_float * 4)()
        _lib.lxb_page_glide(self._page, _FOUR(*rect), strength, at)
        return (at[0], at[1], at[2], at[3])

    def place(self, rect, strength: float = 1.0):
        """The same, but the light appears where it is asked for rather than
        flying to it: what a page does when the row it was on is gone."""
        at = (ctypes.c_float * 4)()
        _lib.lxb_page_place(self._page, _FOUR(*rect), strength, at)
        return (at[0], at[1], at[2], at[3])

    def press(self, lit: bool) -> int:
        """How to draw one of your own controls: resting, lit, or part-way
        through a press.

        The page keeps one press, because the light is in one place — a
        control the light is not on is resting, however many are in flight
        elsewhere. The flow's own controls do this for themselves; this is for
        a page that draws its own, which is a page that asked for
        ``driven=True``.
        """
        return int(_lib.lxb_page_press(self._page, 1 if lit else 0))

    def pressed(self, id: int) -> bool:
        """Whether a press landed on the spot you wrote down under ``id``.

        The pointer's half of a page that draws its own controls: say where
        each one went with :meth:`draw_spot`, then ask here whether one was
        pressed. The numbers are the page's own, so keep them clear of the
        numbered controls the flow draws, which are counted from zero in the
        order they are drawn. Answered once, by the spot the press names and
        by no other. Only a page that asked for ``driven=True`` gets these.
        """
        return bool(_lib.lxb_page_pressed(self._page, id))

    def quit(self) -> None:
        """Close the window at the end of this frame."""
        _lib.lxb_page_quit(self._page)

    def play(self, sound) -> None:
        """Play one of the language's recordings. The interface's own sounds
        are played for you; this is for a program with something of its own to
        say.

        By its name — ``lxb.Sound.MOVE``, or the key an asset is stored under —
        or by its index in :data:`lxb_toolkit.SOUNDS`. A name is what a program
        will have: :class:`lxb_toolkit.Sound` is a namespace of plain strings
        precisely so that a member is the key, and turning it into a number is
        this side's job rather than the caller's.
        """
        _lib.lxb_page_play(self._page, _sound_index(sound))

    def volume(self, value: float, muted: bool = False) -> None:
        """Turn the interface's sounds down, or off. A level rather than a
        switch, because that is what a person has."""
        _lib.lxb_page_volume(self._page, value, 1 if muted else 0)

    @property
    def cursor(self) -> tuple[float, float, float, float]:
        """Where the flow has got to: what the rest of the page has left."""
        out = _FOUR()
        _lib.lxb_page_cursor(self._page, out)
        return (out[0], out[1], out[2], out[3])

    @cursor.setter
    def cursor(self, rect: Sequence[float]) -> None:
        _lib.lxb_page_set_cursor(self._page, _FOUR(*rect))

    # -- the flow ----------------------------------------------------------

    def title(self, text: str) -> None:
        """The page's own name, at the top of it."""
        _lib.lxb_page_title(self._page, _utf8(text))

    def heading(self, text: str) -> None:
        """A name for the group of things under it."""
        _lib.lxb_page_heading(self._page, _utf8(text))

    def text(self, text: str) -> None:
        """A sentence, wrapped to the page's width."""
        _lib.lxb_page_text(self._page, _utf8(text))

    def note(self, text: str) -> None:
        """The same, quieter: a note about the thing above it."""
        _lib.lxb_page_note(self._page, _utf8(text))

    def icon(self, name: str) -> None:
        """One of the shell's own marks, at the size a list uses."""
        _lib.lxb_page_icon(self._page, _utf8(name))

    def head(self, icon: str, title: str) -> None:
        """A mark and the page's name on one line, which is how a page in this
        language introduces itself."""
        _lib.lxb_page_head(self._page, _utf8(icon), _utf8(title))

    def rule(self) -> None:
        """The barely-there hairline the shell groups with."""
        _lib.lxb_page_rule(self._page)

    def gap(self) -> None:
        """One gap's worth of air."""
        _lib.lxb_page_gap(self._page)

    def button(self, label: str) -> bool:
        """A button, as wide as its own label. True on the frame a press lands
        on it, whether that press came from a key, a controller or a click."""
        return bool(_lib.lxb_page_button(self._page, _utf8(label)))

    def row(self, label: str, value: str | None = None) -> bool:
        """A row: the whole width of the page, its label on the left, and what
        it is set to on the right if it is set to anything."""
        if value is None:
            return bool(_lib.lxb_page_row(self._page, _utf8(label)))
        return bool(
            _lib.lxb_page_row_value(self._page, _utf8(label), _utf8(value))
        )

    def item(self, label: str) -> bool:
        """One entry of a list: its label, and the lit capsule behind it when
        the light is on it — and nothing at all when it is not.

        The other half of the pair with :meth:`row`, and the difference is what
        the list is for. A row is a control, so it wears a chip whether or not
        anything is on it: it says *this can be pressed*. An entry is one of
        many, and a column of chips says nothing except that there are a lot of
        them.
        """
        return bool(_lib.lxb_page_item(self._page, _utf8(label)))

    # -- the panels --------------------------------------------------------

    def menu(self, commands: Sequence[str], title: str | None = None) -> None:
        """Raise a context menu over the control the light is on. Nothing
        happens if one is already up; read it back with :meth:`chose`."""
        array, count = _list(commands)
        _lib.lxb_page_menu(self._page, _utf8(title), array, count)

    def chose(self) -> int | None:
        """Which command was pressed, on the frame it was pressed."""
        index = _lib.lxb_page_chose(self._page)
        return None if index < 0 else index

    def ask(self, title: str, body: str, answers: Sequence[str]) -> None:
        """Ask a question. Modal: while it is up it owns every control, which
        is what makes it a question."""
        array, count = _list(answers)
        _lib.lxb_page_ask(self._page, _utf8(title), _utf8(body), array, count)

    def answered(self) -> int | None:
        """Which answer was given, on the frame it was given."""
        index = _lib.lxb_page_answered(self._page)
        return None if index < 0 else index

    # -- the renderer itself -----------------------------------------------

    def draw_pane(self, rect, overlay: int = 1) -> None:
        """A sheet of the shell's glass."""
        _lib.lxb_draw_pane(self._page, _FOUR(*rect), overlay)

    def draw_card(self, rect, surface: int, role: Role = Role.GLASS,
                  alpha: float = 1.0) -> None:
        """One of the three cuts of glass, tinted by a role."""
        _lib.lxb_draw_card(self._page, _FOUR(*rect), surface, int(role), alpha)

    def draw_label(self, rect, text: Text, string: str,
                   role: Role = Role.TEXT, align: int = Align.LEFT) -> None:
        """One line of writing."""
        _lib.lxb_draw_label(self._page, _FOUR(*rect), int(text), _utf8(string),
                            int(role), align)

    def draw_paragraph(self, rect, string: str, role: Role = Role.TEXT) -> float:
        """A sentence, wrapped to the rectangle's width. Answers how tall it
        came out, so whatever is under it can be placed."""
        return _lib.lxb_draw_paragraph(self._page, _FOUR(*rect), _utf8(string),
                                       int(role))

    def draw_icon(self, rect, name: str) -> None:
        """One of the shell's own marks, in the shell's current style."""
        _lib.lxb_draw_icon(self._page, _FOUR(*rect), _utf8(name))

    def draw_rule(self, rect, role: Role = Role.ACCENT_SOFT) -> None:
        """The hairline this language groups with."""
        _lib.lxb_draw_rule(self._page, _FOUR(*rect), int(role))

    def draw_button(self, rect, label: str, press: int = Press.RESTING) -> None:
        """A button at a rectangle of your own choosing. For anything a person
        can move onto, prefer :meth:`button`, which numbers it and answers the
        pointer."""
        _lib.lxb_draw_button(self._page, _FOUR(*rect), _utf8(label), press)

    def draw_row(self, rect, name: str, value: str | None = None,
                 press: int = Press.RESTING) -> None:
        """A row at a rectangle of your own choosing."""
        _lib.lxb_draw_row(self._page, _FOUR(*rect), _utf8(name), _utf8(value),
                          press)

    def draw_selection(self, rect, strength: float = 1.0) -> None:
        """The lit capsule that says where the light is."""
        _lib.lxb_draw_selection(self._page, _FOUR(*rect), strength)

    def draw_spot(self, ident: int, rect) -> None:
        """Write down where something you drew yourself went, so a pointer over
        it can be answered."""
        _lib.lxb_draw_spot(self._page, ident, _FOUR(*rect))

    def at(self, x: float, y: float) -> int | None:
        """What is at a point of the frame that is on screen."""
        found = _lib.lxb_page_at(self._page, x, y)
        return None if found < 0 else found

    # -- the measurements a page needs -------------------------------------

    def m(self, metric: Metric) -> float:
        """One of the language's named lengths, at this window's height."""
        return _lib.lxb_page_metric(self._page, int(metric))

    def s(self, reference: float) -> float:
        """A length written against the 1080p reference, scaled to here."""
        return _lib.lxb_page_scaled(self._page, reference)

    def line(self, text: Text) -> float:
        """How tall one line of a size is, air included."""
        return _lib.lxb_page_line(self._page, int(text))

    def measure(self, text: Text, string: str) -> float:
        """How wide a string is at a size, shaped by the engine that will draw
        it — so it is the width it will actually take."""
        return _lib.lxb_page_measure(self._page, int(text), _utf8(string))


class App:
    """An application: a window, and one function that says what is on its page.

    Built and then run. Nothing here is a handle to anything until :meth:`run`
    is called, so an application can be described before there is a display.
    """

    def __init__(self, app_id: str, title: str, size=None, plain: bool = False,
                 driven: bool = False):
        self._app = _lib.lxb_app_new(_utf8(app_id), _utf8(title))
        if not self._app:
            raise AppNotFound("the application could not be created")
        self._draw: Callable[[Page], None] | None = None
        # Kept on the instance: ctypes will collect a trampoline that only the
        # library still holds, and the crash is in C with no Python in sight.
        self._trampoline = None
        if size is not None:
            _lib.lxb_app_size(self._app, float(size[0]), float(size[1]))
        if plain:
            _lib.lxb_app_plain(self._app)
        if driven:
            _lib.lxb_app_driven(self._app)

    def page(self, draw: Callable[[Page], None]) -> Callable[[Page], None]:
        """Say what is on the page. Usable as a decorator."""
        self._draw = draw
        return draw

    def run(self, draw: Callable[[Page], None] | None = None) -> None:
        """Open the window and run until it is closed.

        Raises :class:`RuntimeError` if the window could not be opened at all.
        A machine with no controller and a machine with no sound are not that:
        both are working states, and neither is worth stopping for.
        """
        if draw is not None:
            self._draw = draw
        if self._draw is None:
            raise RuntimeError("an application with no page to draw")

        frame = Page()
        draw_page = self._draw

        def trampoline(page, _data):
            frame._page = page
            draw_page(frame)

        self._trampoline = _PAGE_FN(trampoline)
        code = _lib.lxb_app_run(self._app, self._trampoline, None)
        if code != 0:
            trouble = _lib.lxb_app_trouble(self._app)
            raise RuntimeError(
                trouble.decode("utf-8", "replace") if trouble else "it did not run"
            )

    def shot(self, path, width: int = 1280, height: int = 800,
             seconds: float = 10.0,
             draw: Callable[[Page], None] | None = None) -> None:
        """One settled frame, written to a PNG, with no display at all.

        The same page function, the same renderer and the same material as
        :meth:`run` — there is no second drawing path here, which is the only
        way a picture is worth anything as a check. ``seconds`` is the clock it
        is drawn at, so a moving wallpaper can be caught at a chosen moment.
        """
        if draw is not None:
            self._draw = draw
        if self._draw is None:
            raise RuntimeError("a picture with no page to draw")

        frame = Page()
        draw_page = self._draw

        def trampoline(page, _data):
            frame._page = page
            draw_page(frame)

        self._trampoline = _PAGE_FN(trampoline)
        code = _lib.lxb_app_shot(self._app, _utf8(str(path)), width, height,
                                 seconds, self._trampoline, None)
        if code != 0:
            trouble = _lib.lxb_app_trouble(self._app)
            raise RuntimeError(
                trouble.decode("utf-8", "replace") if trouble else "no picture"
            )

    def __del__(self):  # pragma: no cover - interpreter teardown
        app, self._app = getattr(self, "_app", None), None
        if app:
            _lib.lxb_app_free(app)
