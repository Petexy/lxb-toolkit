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
import os
from pathlib import Path
from typing import Callable, Sequence

from . import SOUNDS, Action, Metric, Role, Selection, Sound, Text, _load, _picker_string

__all__ = ["App", "Page", "Align", "Fit", "Press", "AppNotFound"]


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


class Fit:
    """How a picture fills the rectangle it is drawn in."""

    #: The whole picture, as large as it fits and centred.
    CONTAIN = 0
    #: The rectangle filled, and what overflows it cut away.
    COVER = 1


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
    ("lxb_app_low_end", [ctypes.c_void_p], None),
    ("lxb_app_mark",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_ulong],
     ctypes.c_int),
    ("lxb_app_run", [ctypes.c_void_p, _PAGE_FN, ctypes.c_void_p], ctypes.c_int),
    ("lxb_app_shot",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint, ctypes.c_uint,
      ctypes.c_float, _PAGE_FN, ctypes.c_void_p], ctypes.c_int),
    ("lxb_app_shot_after",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint, ctypes.c_uint,
      ctypes.c_float, ctypes.c_uint, _PAGE_FN, ctypes.c_void_p], ctypes.c_int),
    ("lxb_app_trouble", [ctypes.c_void_p], ctypes.c_char_p),
    ("lxb_app_free", [ctypes.c_void_p], None),
    ("lxb_page_width", [ctypes.c_void_p], ctypes.c_float),
    ("lxb_page_height", [ctypes.c_void_p], ctypes.c_float),
    ("lxb_page_seconds", [ctypes.c_void_p], ctypes.c_float),
    ("lxb_page_action", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_page_act", [ctypes.c_void_p, ctypes.c_ulong], None),
    ("lxb_page_scrolled", [ctypes.c_void_p, ctypes.c_uint], ctypes.c_int),
    ("lxb_page_focus", [ctypes.c_void_p, ctypes.c_ulong], None),
    ("lxb_page_focused", [ctypes.c_void_p], ctypes.c_ulong),
    ("lxb_page_glide",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_place",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_light_shaped",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.c_float, ctypes.c_int, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_press", [ctypes.c_void_p, ctypes.c_int], ctypes.c_ulong),
    ("lxb_page_pressed", [ctypes.c_void_p, ctypes.c_uint], ctypes.c_int),
    ("lxb_page_dragging",
     [ctypes.c_void_p, ctypes.c_uint, ctypes.POINTER(ctypes.c_float)],
     ctypes.c_int),
    ("lxb_page_pointer",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float)], ctypes.c_int),
    ("lxb_page_pointer_moved", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_page_hovered", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_page_taking_text",
     [ctypes.c_void_p, ctypes.c_int, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_typed", [ctypes.c_void_p], ctypes.c_void_p),
    ("lxb_page_board",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float)], ctypes.c_int),
    ("lxb_page_quit", [ctypes.c_void_p], None),
    ("lxb_page_keep_awake", [ctypes.c_void_p, ctypes.c_int, ctypes.c_int], None),
    ("lxb_page_redraw_within", [ctypes.c_void_p, ctypes.c_float], None),
    ("lxb_page_low_end", [ctypes.c_void_p], ctypes.c_int),
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
    ("lxb_page_light_at",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float)], None),
    ("lxb_page_menu",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.POINTER(ctypes.c_char_p),
      ctypes.c_ulong], None),
    ("lxb_page_menu_marked",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.POINTER(ctypes.c_char_p),
      ctypes.c_ulong, ctypes.c_ulong], None),
    ("lxb_page_menu_on",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.POINTER(ctypes.c_char_p),
      ctypes.c_ulong, ctypes.c_ulong], None),
    ("lxb_page_chose", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_page_ask",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p,
      ctypes.POINTER(ctypes.c_char_p), ctypes.c_ulong], None),
    ("lxb_page_answered", [ctypes.c_void_p], ctypes.c_int),
    ("lxb_page_pick",
     [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_char_p], ctypes.c_int),
    ("lxb_page_pick_many",
     [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_char_p], ctypes.c_int),
    ("lxb_page_save",
     [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p], ctypes.c_int),
    ("lxb_page_picked", [ctypes.c_void_p], ctypes.c_void_p),
    ("lxb_page_picked_next", [ctypes.c_void_p], ctypes.c_void_p),
    ("lxb_app_string_free", [ctypes.c_void_p], None),
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
    ("lxb_draw_picture",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.c_char_p, ctypes.c_ulong, ctypes.c_float], ctypes.c_int),
    ("lxb_draw_chosen",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float], None),
    ("lxb_draw_switch",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_int,
      ctypes.c_float], None),
    ("lxb_switch_width", [ctypes.c_float], ctypes.c_float),
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
    ("lxb_draw_soft_edges",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.c_float, ctypes.c_float], None),
    ("lxb_draw_soft_side_edges",
     [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.c_float,
      ctypes.c_float, ctypes.c_float], None),
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

_TWO = ctypes.c_float * 2
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

    def act(self, action: int) -> None:
        """Hand the window an action (one of :class:`~lxb_toolkit.Action`'s
        numbers) as a key or a controller would.

        A question, a menu or the file chooser standing over the page takes
        it, a driven page finds it in :meth:`actions`, and otherwise it walks
        or presses the light. What a picture of something being answered
        presses with; call it before reading the frame's actions.
        """
        _lib.lxb_page_act(self._page, int(action))

    def scrolled(self, ident: int) -> int:
        """How far a wheel or touchpad moved over one of the spots this page
        drew, counted in directions and signed downwards.

        The directions themselves are already in :meth:`actions`, so a page
        that never asks this still scrolls. Ask it to decide *which* of the
        page's lists this frame's Up and Down are moving — the one question a
        pointer raises and an action cannot answer.
        """
        return int(_lib.lxb_page_scrolled(self._page, ident))

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
        behind, so their faces sit over it. Where the session is counted
        low-end (:meth:`low_end`) it stands where it is asked for at once.
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

    def light_shaped(self, rect, radius: float, strength: float = 1.0,
                     appears: bool = False):
        """The same light drawn in the rectangle's own shape, its corners
        rounded by ``radius`` rather than a capsule's: a card or a picture lit
        as itself.

        Carried there as :meth:`glide` carries the capsule, or put there at
        once as :meth:`place` puts it where ``appears`` is true. It is the one
        light the capsule is, so a page that lights its rows with one and its
        cards with the other has it travel between them. Answers where it is
        drawn this frame.
        """
        at = _FOUR(*rect)
        _lib.lxb_page_light_shaped(self._page, _FOUR(*rect), radius, strength,
                                   1 if appears else 0, at)
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

    def dragging(self, id: int) -> tuple[float, float] | None:
        """Where the pointer is while a press that took hold of ``id`` is held.

        The one gesture a press and a release cannot describe between them: a
        bar taken hold of and moved. ``None`` while nothing is being dragged
        from that spot.

        Asking is what makes ``id`` a bar: a press on a control the page asked
        about on the frame it landed on is held, and is a drag rather than a
        hover until the button comes up. A press held on any other control
        goes on hovering, as in LineXinBar. Ask on every frame the bar is
        drawn.

        Only a pointer drags. A finger on the same control moves the list
        instead, which is what a finger does everywhere else on the page.
        """
        at = _TWO()
        if not _lib.lxb_page_dragging(self._page, id, at):
            return None
        return (at[0], at[1])

    def pointer(self) -> tuple[float, float] | None:
        """Where the pointer rests over the page.

        ``None`` once it has left the window, while a menu, a dialog or the
        file chooser stands over the page, from the moment a key, a pad or a
        finger is reached for until the pointer moves again, and while a press
        that took hold of a bar is held down: that is a drag, which
        ``dragging`` follows, and not a hover. A press held on any other
        control leaves it answering.
        """
        at = _TWO()
        if not _lib.lxb_page_pointer(self._page, at):
            return None
        return (at[0], at[1])

    def pointer_moved(self) -> bool:
        """Whether the pointer moved over the page since the last frame.

        The only time hovering may move a light: one the pad moved stays where
        the pad put it under a pointer that is resting.
        """
        return bool(_lib.lxb_page_pointer_moved(self._page))

    def hovered(self) -> int | None:
        """The control the pointer moved onto since the last frame, by the
        number it was drawn with — ``None`` on a frame it did not move, and
        over nothing of the page's own.

        Read off the frame the pointer was moving over, so it can be asked
        before anything is drawn. Light what stands still with it; a list that
        scrolls to its selection moves under the pointer, and there a first
        click carries the light instead.
        """
        found = _lib.lxb_page_hovered(self._page)
        return None if found < 0 else found

    def taking_text(self, rect=None, taking: bool = True) -> None:
        """A field takes text this frame, standing at ``rect`` (or where it
        was said last); ``taking=False`` takes back what was said earlier in
        the same frame.

        The letters typed, and Backspace, then arrive through :meth:`typed`
        instead of as actions; directions, Accept and Escape still arrive as
        actions. The desktop's text input is told of the field, and LineXinBar
        raises its on-screen keyboard over it while a controller is in hand
        (:meth:`board` says where). Say it on every frame the field takes
        text: a frame that does not say it has no field taking text.
        """
        _lib.lxb_page_taking_text(
            self._page, 1 if taking else 0, None if rect is None else _FOUR(*rect))

    def typed(self) -> str:
        """What was typed since the last frame, in order: its letters, and
        ``"\\b"`` (U+0008) for each Backspace. Empty for nothing.

        Read it on every frame a field takes text, because what a frame does
        not read is gone with it.
        """
        raw = _lib.lxb_page_typed(self._page)
        if not raw:
            return ""
        try:
            return ctypes.string_at(raw).decode("utf-8")
        finally:
            _lib.lxb_app_string_free(raw)

    def board(self) -> tuple[float, float, float, float] | None:
        """Where LineXinBar's on-screen keyboard stands over the window while
        it is up, and ``None`` where it is not.

        The shell never moves it out of the way, so a field being typed into
        belongs above its top for as long as this answers: lift it, or scroll
        the form it is in, and give the room back when it goes. ``None``
        always outside LineXinBar.
        """
        at = _FOUR()
        if not _lib.lxb_page_board(self._page, at):
            return None
        return (at[0], at[1], at[2], at[3])

    def quit(self) -> None:
        """Close the window at the end of this frame."""
        _lib.lxb_page_quit(self._page)

    def keep_awake(self, screen: bool = True, sleep: bool = False) -> None:
        """Keep the screen lit, the machine awake, or both, for as long as it
        is said; say ``keep_awake(False)`` to let go. A film playing holds both,
        music holds only sleep. The desktop is asked once, when it changes."""
        _lib.lxb_page_keep_awake(self._page, int(screen), int(sleep))

    def redraw_within(self, seconds: float) -> None:
        """Draw the next frame no later than this many seconds from now. Only
        low-end hardware mode waits between frames at all; a page with
        something running that the frame itself notices says so here."""
        _lib.lxb_page_redraw_within(self._page, float(seconds))

    def low_end(self) -> bool:
        """Whether the toolkit counts this session as low-end hardware: the
        shell's low-end mode where Settings has said, and otherwise whether
        the window is drawn in software.

        A page that moves things itself — a page giving way to the next, a
        column stepping in — puts them where they are going at once where
        this is true. Decided as the window opens, and true for an
        application made with ``low_end=True``; a picture (``shot``) answers
        false unless it was taken of such an application.
        """
        return bool(_lib.lxb_page_low_end(self._page))

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

    def light_at(self, rect: Sequence[float]) -> None:
        """Say where the light is standing, for a page that draws its own
        controls. A menu grows out of it; without it one raised over a card a
        page laid out itself grows out of the corner of the window."""
        _lib.lxb_page_light_at(self._page, _FOUR(*rect))

    def menu(self, commands: Sequence[str], title: str | None = None,
             marked: int | None = None, on: int | None = None) -> None:
        """Raise a context menu over the control the light is on. Nothing
        happens if one is already up; read it back with :meth:`chose`. It is
        as wide as its widest command needs to stand on one line, within the
        window.

        `marked` is which command is the answer already in force: it wears the
        language's own chosen mark, so a menu of alternatives says which one
        you are on without anybody having to press to find out, and the menu
        opens with the light on it, so Accept at once keeps it.

        `on`, for a menu of acts with nothing marked, is the command the light
        opens on: what a menu a person comes back to does, the light on the
        row they went in by. A menu with a row marked opens on that row, so
        the two are never given together: that raises ValueError, as Rust and
        C, which have a function for each, cannot be asked for both at all.
        """
        if marked is not None and on is not None:
            raise ValueError(
                "a menu opens on its marked row: give marked or on, not both")
        array, count = _list(commands)
        if on is not None:
            _lib.lxb_page_menu_on(self._page, _utf8(title), array, count, on)
            return
        if marked is None:
            _lib.lxb_page_menu(self._page, _utf8(title), array, count)
            return
        _lib.lxb_page_menu_marked(
            self._page, _utf8(title), array, count, marked)

    def chose(self) -> int | None:
        """Which command was pressed, on the frame it was pressed."""
        index = _lib.lxb_page_chose(self._page)
        return None if index < 0 else index

    def ask(self, title: str, body: str, answers: Sequence[str]) -> None:
        """Ask a question. Modal: while it is up it owns every control, which
        is what makes it a question.

        The body keeps its lines: a line break is a line break on the pane, and
        a line that fits is written as it came. Only a line too wide is broken
        again, at a space, never at a no-break space or inside quotation marks,
        and never before a lone "·" or "/".
        """
        array, count = _list(answers)
        _lib.lxb_page_ask(self._page, _utf8(title), _utf8(body), array, count)

    def answered(self) -> int | None:
        """Which answer was given, on the frame it was given."""
        index = _lib.lxb_page_answered(self._page)
        return None if index < 0 else index

    def pick(self, selection: Selection, directory) -> bool:
        """Ask for one file, or for a folder.

        The desktop is asked first: where a session runs an
        ``org.freedesktop.portal.FileChooser`` — every desktop does — the
        question is put through it, so the answer comes from the chooser the
        rest of the machine uses and the application is handed a path it was
        given permission to. Where there is no portal, the toolkit draws the
        question itself: a centred Lattice glass window covering roughly
        seventy percent of the application, whose directory columns recede
        while strong frost and depth keep the page behind it, and which owns
        input until it is answered. Set ``LXB_FILE_PORTAL=0`` to use that one
        always.

        ``selection`` is one of :class:`lxb.Selection`, and decides both what
        is listed and — where the desktop is asked — which kinds of file the
        chooser offers; ``directory`` is any UTF-8 :class:`os.PathLike` or
        string to open in. Read an accepted answer once with :meth:`picked`.
        In the toolkit's own window Escape and Back close it without an
        answer, and controller Accept on Search opens its local keyboard:
        D-pad/left stick moves, A enters, Start finishes, and its hide key
        returns with the query intact. B does the same from anywhere on the
        board.
        """
        return self._ask(_lib.lxb_page_pick, selection, directory)

    def pick_many(self, selection: Selection, directory) -> bool:
        """Ask for any number of files at once.

        As :meth:`pick`, with an answer that may be several paths — read it
        with :meth:`picked_files`. In the toolkit's own window each file is
        ticked with Accept and the head row at the top of the column ends the
        question.
        """
        return self._ask(_lib.lxb_page_pick_many, selection, directory)

    def save(self, name: str, directory) -> bool:
        """Ask where to write a file, and what to call it.

        ``name`` is what it is called to begin with and may be empty. The
        answer is a path that need not exist yet; read it with :meth:`picked`.
        """
        return bool(_lib.lxb_page_save(
            self._page, _utf8(name), _picker_string(directory, "directory")
        ))

    def _ask(self, call, selection: Selection, directory) -> bool:
        try:
            selection = Selection(selection)
        except (TypeError, ValueError) as error:
            raise ValueError(f"unknown picker selection: {selection!r}") from error
        return bool(call(
            self._page, int(selection), _picker_string(directory, "directory")
        ))

    def picked(self) -> Path | None:
        """The answer given since the last frame, once.

        The whole answer is taken: where several files were chosen this is the
        first of them and the rest are dropped. Use :meth:`picked_files` for a
        question that may be answered with more than one.
        """
        return self._path(_lib.lxb_page_picked(self._page))

    def picked_files(self) -> list[Path]:
        """Every file of the answer given since the last frame, once."""
        found: list[Path] = []
        while True:
            path = self._path(_lib.lxb_page_picked_next(self._page))
            if path is None:
                return found
            found.append(path)

    def _path(self, raw) -> Path | None:
        if not raw:
            return None
        try:
            return Path(ctypes.string_at(raw).decode("utf-8"))
        finally:
            _lib.lxb_app_string_free(raw)

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
        """One of the shell's own marks, or one the application added with
        :meth:`App.mark`, in the shell's current style."""
        _lib.lxb_draw_icon(self._page, _FOUR(*rect), _utf8(name))

    def draw_picture(self, rect, radius: float, path, fit: int = Fit.CONTAIN,
                     alpha: float = 1.0) -> bool:
        """A picture from a file on this disk — PNG, JPEG, GIF, WebP, BMP,
        TIFF or SVG — in ``rect``, its corners rounded by ``radius``, fitted
        as ``fit`` (one of :class:`Fit`) says, at ``alpha``.

        ``path`` is any :class:`os.PathLike`, string or bytes. True once it is
        drawn; False while it is still being read, or when it cannot be. The
        file is read on a worker thread the first time it is asked for, so ask
        on every frame and draw what stands in for it until this is True. A
        picture of the window (:meth:`App.shot`) reads it at once.
        """
        encoded = os.fsencode(path)
        if b"\0" in encoded:
            raise ValueError("a picture's path must not contain a NUL character")
        return bool(_lib.lxb_draw_picture(
            self._page, _FOUR(*rect), radius, encoded, int(fit), alpha))

    def draw_chosen(self, rect, alpha: float = 1.0) -> None:
        """The chosen mark, the tick that says a value is in force, in the
        square ``rect``, ringed in dark glass so a lit control shows it."""
        _lib.lxb_draw_chosen(self._page, _FOUR(*rect), alpha)

    def draw_switch(self, rect, on: bool, ink: float = 1.0) -> None:
        """A switch standing in ``rect``, on or off, in the marks' material.

        ``ink`` is how strongly it is drawn: 1 on a control that can be used,
        less on one that cannot. :meth:`switch_width` says how long one is.
        """
        _lib.lxb_draw_switch(self._page, _FOUR(*rect), 1 if on else 0, ink)

    @staticmethod
    def switch_width(height: float) -> float:
        """How long a switch ``height`` tall is drawn."""
        return _lib.lxb_switch_width(height)

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

    def draw_soft_edges(self, rect, band: float, top: float = 1.0,
                        bottom: float = 1.0) -> None:
        """Blur and fade the top and bottom edges of a scrolling area, after
        everything inside it has been drawn.

        ``band`` is the feather in points; ``top`` and ``bottom`` are how
        strongly each edge is there, which is how a list says whether anything
        really continues past it.
        """
        _lib.lxb_draw_soft_edges(self._page, _FOUR(*rect), band, top, bottom)

    def draw_soft_side_edges(self, rect, band: float, left: float = 1.0,
                             right: float = 1.0) -> None:
        """Blur and fade the left and right edges of an area, after
        everything inside it has been drawn: what is carried sideways across
        a pane dissolves into it there.

        ``band`` is the feather in points; ``left`` and ``right`` are how
        strongly each edge is there.
        """
        _lib.lxb_draw_soft_side_edges(self._page, _FOUR(*rect), band, left,
                                      right)

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
                 driven: bool = False, low_end: bool = False):
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
        if low_end:
            # Counted low-end hardware whatever the shell's Settings say: the
            # window drawn as it is there, every light standing on what is
            # chosen at once, and a picture of it drawn at one instant.
            _lib.lxb_app_low_end(self._app)

    def mark(self, name: str, svg) -> None:
        """A mark of the application's own, drawn by ``name`` wherever one of
        the toolkit's is (:meth:`Page.draw_icon`, :meth:`Page.icon`,
        :meth:`Page.head`) and in the same material.

        ``svg`` is the drawing, as bytes or as its text: a shape and nothing
        else, white where the mark is and cut through where it is open,
        saying ``lxb:shape`` as every mark of the toolkit does. Raises
        :class:`ValueError` with the reason when it is refused — a drawing
        that is not a shape, does not parse or has nothing in it, a name the
        toolkit already draws a mark by, or more marks than there is room
        for. The same name given again takes the new drawing.
        """
        drawing = svg.encode("utf-8") if isinstance(svg, str) else bytes(svg)
        if not _lib.lxb_app_mark(self._app, _utf8(name), drawing, len(drawing)):
            trouble = _lib.lxb_app_trouble(self._app)
            raise ValueError(
                trouble.decode("utf-8", "replace") if trouble else "the mark was refused"
            )

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
        failure = None

        def trampoline(page, _data):
            nonlocal failure
            frame._page = page
            if failure is not None:
                return
            try:
                draw_page(frame)
            except BaseException as error:
                # ctypes cannot carry an exception through the native frame
                # callback.  Without retaining it, a live window would carry
                # on drawing only its wallpaper after Python stopped drawing
                # its page.  Ask the runtime to leave, then raise normally
                # once control is back in Python.
                failure = error
                _lib.lxb_page_quit(page)

        self._trampoline = _PAGE_FN(trampoline)
        code = _lib.lxb_app_run(self._app, self._trampoline, None)
        if failure is not None:
            raise failure
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
        self._picture(path, width, height, seconds, draw, None)

    def shot_after(self, path, width: int = 1280, height: int = 800,
                   seconds: float = 10.0, frames: int = 1,
                   draw: Callable[[Page], None] | None = None) -> None:
        """A picture of the window ``frames`` frames after its first, each
        drawn a sixtieth of a second after the one before.

        Whatever is on its way somewhere — a menu unfolding, a question coming
        up, the page's own motion — is caught where it has got to, where
        :meth:`shot` puts it where it is going first. The page is drawn once a
        frame, ``page.seconds()`` moving on by a sixtieth each time, and the
        last frame is the one written.
        """
        if frames < 0:
            raise ValueError("a picture cannot be taken before its first frame")
        self._picture(path, width, height, seconds, draw, frames)

    def _picture(self, path, width, height, seconds, draw, frames) -> None:
        if draw is not None:
            self._draw = draw
        if self._draw is None:
            raise RuntimeError("a picture with no page to draw")

        frame = Page()
        draw_page = self._draw
        failure = None

        def trampoline(page, _data):
            nonlocal failure
            frame._page = page
            if failure is not None:
                return
            try:
                draw_page(frame)
            except BaseException as error:
                failure = error
                _lib.lxb_page_quit(page)

        self._trampoline = _PAGE_FN(trampoline)
        if frames is None:
            code = _lib.lxb_app_shot(self._app, _utf8(str(path)), width,
                                     height, seconds, self._trampoline, None)
        else:
            code = _lib.lxb_app_shot_after(self._app, _utf8(str(path)), width,
                                           height, seconds, frames,
                                           self._trampoline, None)
        if failure is not None:
            raise failure
        if code != 0:
            trouble = _lib.lxb_app_trouble(self._app)
            raise RuntimeError(
                trouble.decode("utf-8", "replace") if trouble else "no picture"
            )

    def __del__(self):  # pragma: no cover - interpreter teardown
        app, self._app = getattr(self, "_app", None), None
        if app:
            _lib.lxb_app_free(app)
