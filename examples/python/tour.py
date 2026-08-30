#!/usr/bin/env python3
"""A tour of the LineXinBar design language, in Python.

Eight pages: what the language answers, its colour, its material, its marks,
its type, its motion, its sounds and its built-in picker. Every one is drawn
by the shell's own renderer on the GPU — the same wallpaper shader, the same
glass shader and the same glyph shader the shell itself runs — through one
library and one page
function.

    python3 tour.py                    # the window
    python3 tour.py --shot tour.png    # one frame of it, with no display
    python3 tour.py --shot tour.png Colour
    python3 tour.py --shot picker.png Picker picker-file

Up and Down walk the pages; Left and Right walk within one. Enter presses;
Escape leaves; the Menu key or the right mouse button raises a context menu.
A controller does all four, and so does a pointer.

This application asks to drive itself — `driven=True` — because it is a screen
rather than a list: it has two axes, and only the application knows which is
which. An application that is one list of things has none of this and is a
dozen lines; see hello-app.py.
"""

import os
import sys
from pathlib import Path
from typing import Optional

# A source-checkout tour must use the Python binding and shared libraries it
# sits beside.  Without this, running `python3 tour.py` from this directory can
# silently load an older installed binding: its callback then lacks new page
# methods and the native window has only wallpaper left to draw.
_SOURCE_PYTHON = Path(__file__).resolve().parents[2] / "python"
if _SOURCE_PYTHON.is_dir():
    sys.path.insert(0, str(_SOURCE_PYTHON))

import lxb_toolkit as lxb

SECTIONS = ("Hello", "Colour", "Material", "Marks", "Type", "Motion", "Sound",
            "Picker")

# Where this page's own items are, so a pointer over one can be answered. The
# numbers are this program's, and are kept clear of the sidebar's, which
# page.item numbers from zero in the order it draws them.
ITEM_SPOT = 0x200

GREETING = "Hello, world"

# Six of the thirty-four durations, chosen because each is a different kind of
# arrival.
SHOWN_DURATIONS = (
    ("panel", "a panel arrives"),
    ("flight", "a card flies"),
    ("accent-change", "the accent travels"),
    ("guide-press", "a control is pressed"),
    ("scenery-fade", "a scene crossfades"),
    ("launch-open", "an application opens"),
)

MENU_ROWS = ("Simple marks", "Ask a question", "Copy this page's values", "Close")
PANEL_SHOTS = ("menu", "dialog", "picker-file", "picker-many", "picker-folder",
               "picker-save", "picker-image")

# The four questions the Picker page can put, in the order it offers them, and
# the name each is raised by for a headless shot. A save arrives named because
# that is the case worth a picture: it is the one purpose whose column opens on
# the row that answers.
PICKER_LABELS = ("Choose a file", "Choose some files", "Choose a folder",
                 "Choose somewhere to save", "Choose an image")
PICKER_PANELS = ("picker-file", "picker-many", "picker-folder", "picker-save",
                 "picker-image")
PICKER_SAVE_NAME = "untitled.txt"


def tour_files() -> Path:
    """The checked-in picker contents, or the one a screenshot test supplied.

    C and Python read the same override rather than each creating a temporary
    directory of its own, which keeps the visible location and listing
    identical in their headless frames.
    """
    default = Path(__file__).resolve().parents[1] / "tour-files"
    return Path(os.environ.get("LXB_TOUR_FILES", default))


class Tour:
    """The whole of the application: which page, and where in it."""

    raise_panel: Optional[str]

    def __init__(self):
        self.section = 0
        self.cursor = [0] * len(SECTIONS)
        # Which page the light is on, so it appears on a new one rather than
        # flying across from the last.
        self.lit_section = 0
        self.theme = lxb.ShellTheme.load()
        self.palette = next(
            (index for index, palette in enumerate(lxb.PALETTES)
             if palette.name == self.theme.accent.name), 0)
        self.cursor[1] = self.palette
        self.asked = False
        # A panel to raise on the first frame, for a picture of one.
        self.raise_panel = None
        self.picker_root = tour_files()
        self.picked_path: Optional[Path] = None

    # -- what is on each page ---------------------------------------------

    def count(self) -> int:
        """How many things the current page has to walk between."""
        return (1, len(lxb.PALETTES), len(lxb.Surface), len(lxb.GLYPHS),
                len(lxb.Text), len(SHOWN_DURATIONS), len(lxb.SOUNDS),
                len(PICKER_LABELS))[self.section]

    def at(self) -> int:
        return self.cursor[self.section]

    # -- what the controls mean -------------------------------------------

    def act(self, page, action):
        """One action, whichever control sent it.

        The single place it happens, so that the same move made two ways
        cannot end up doing two different things.
        """
        if action in (lxb.Action.UP, lxb.Action.PREVIOUS):
            self.walk_page(page, -1)
        elif action in (lxb.Action.DOWN, lxb.Action.NEXT):
            self.walk_page(page, 1)
        elif action == lxb.Action.LEFT:
            self.walk(page, -1)
        elif action == lxb.Action.RIGHT:
            self.walk(page, 1)
        elif action in (lxb.Action.ACCEPT, lxb.Action.SUBMIT):
            self.press(page)
        elif action == lxb.Action.MENU:
            page.menu(MENU_ROWS, SECTIONS[self.section])
        elif action == lxb.Action.BACK:
            # Nothing left to step out of, so this is the way out — asked
            # rather than taken.
            page.ask("Leave the tour?",
                     "The window closes. Nothing here is saved, because "
                     "nothing here is yours: every value on these pages is the "
                     "toolkit's answer, not this program's.",
                     ("Leave", "Stay"))
            self.asked = True

    def walk_page(self, page, delta):
        want = min(max(self.section + delta, 0), len(SECTIONS) - 1)
        if want != self.section:
            self.section = want
            page.play(lxb.Sound.MOVE)

    def walk(self, page, delta):
        total = self.count()
        want = min(max(self.at() + delta, 0), total - 1)
        if want != self.at():
            self.cursor[self.section] = want
            page.play(lxb.Sound.MOVE)
            # The palette page shows what it is on rather than what has been
            # chosen: walking previews, and only Enter applies.
            if self.section == 6:
                page.play(want)

    def apply(self, page):
        """What pressing this page's selection does, without the click it is
        answered with: a pointer's press has already been sounded by the time
        the page hears about it, and a control that said it twice would be
        saying the pointer is a different kind of thing."""
        if self.section == 1:
            self.palette = self.at()
        elif self.section == 6:
            page.play(self.at())
        elif self.section == 7:
            self.ask_for_files(page, self.at())

    def ask_for_files(self, page, index):
        """Put the question the Picker page's row at `index` asks."""
        if index == 1:
            page.pick_many(lxb.Selection.FILE, self.picker_root)
        elif index == 2:
            page.pick(lxb.Selection.FOLDER, self.picker_root)
        elif index == 3:
            page.save(PICKER_SAVE_NAME, self.picker_root)
        # The one question with a kind of file in force, which is what the
        # panel's Types row exists for.
        elif index == 4:
            page.pick(lxb.Selection.IMAGE, self.picker_root)
        else:
            page.pick(lxb.Selection.FILE, self.picker_root)

    def light(self, page, rect):
        """The lit capsule, travelling.

        A light that has never been on this page appears where it is asked for
        rather than flying in from the last page's row: the two are different
        lists, and a light crossing between them would be saying they are one.
        """
        if self.lit_section == self.section:
            page.glide(rect)
        else:
            page.place(rect)
            self.lit_section = self.section

    def press(self, page):
        page.play(lxb.Sound.PRESS)
        self.apply(page)

    # -- the screen --------------------------------------------------------

    def draw(self, page):
        for action in page.actions():
            self.act(page, action)
        if self.raise_panel is not None:
            if self.raise_panel == "menu":
                self.act(page, lxb.Action.MENU)
            elif self.raise_panel == "dialog":
                self.act(page, lxb.Action.BACK)
            elif self.raise_panel in PICKER_PANELS:
                self.ask_for_files(page, PICKER_PANELS.index(self.raise_panel))
            self.raise_panel = None

        picked = page.picked()
        if picked is not None:
            self.picked_path = picked

        # A press on one of this page's own items. The pointer's half of Left
        # and Right: it carries the selection there and acts on it, which is
        # the two steps a walk and an Enter are, in one.
        for index in range(self.count()):
            if page.pressed(ITEM_SPOT + index):
                self.cursor[self.section] = index
                self.apply(page)
                break

        chosen = page.chose()
        if chosen == 1:
            page.ask("A question", "This is the shell's own panel: the same "
                     "glass, the same arrival, the same answer capsules.",
                     ("Good", "Close"))
        elif chosen == 3:
            page.quit()
        if self.asked and page.answered() == 0:
            page.quit()

        inset = page.m(lxb.Metric.PANEL_INSET)
        gap = page.m(lxb.Metric.GAP)
        sidebar = min(page.m(lxb.Metric.MENU_WIDTH), page.width * 0.34)
        tall = page.height - 2 * inset

        self.draw_sidebar(page, (inset, inset, sidebar, tall))

        x = inset + sidebar + gap
        content = (x, inset, page.width - x - inset, tall)
        page.draw_pane(content)
        pad = page.m(lxb.Metric.PANEL_PADDING)
        page.cursor = (content[0] + pad, content[1] + pad,
                       content[2] - 2 * pad, content[3] - 2 * pad)
        (self.hello, self.colour, self.material, self.marks,
         self.type_, self.motion, self.sound, self.picker)[self.section](page)

    def draw_sidebar(self, page, rect):
        """The pages, as a column beside the page they are about.

        These are the application's only numbered controls, so the light is
        told where it is — the tour moves its own selection — and a click on
        one of them comes back as a press on that row.
        """
        x, y, width, height = rect
        page.draw_pane(rect)
        pad = page.m(lxb.Metric.PANEL_PADDING)
        row = page.m(lxb.Metric.ROW_HEIGHT)
        page.focus = self.section
        page.cursor = (x + pad, y + pad, width - 2 * pad, height - 2 * pad)

        page.note(f"lxb-toolkit {lxb.version()}")
        for index, name in enumerate(SECTIONS):
            if page.item(name):
                self.section = index

        # The keys, at the foot of it. Written down rather than discovered,
        # because an interface driven three ways has to say so.
        left = page.measure(lxb.Text.LABEL, "Left / Right") + page.m(lxb.Metric.GAP)
        line = page.line(lxb.Text.LABEL)
        foot = y + height - pad - 5 * line
        for name, what in (("Up / Down", "the page"), ("Left / Right", "within it"),
                           ("Enter", "act on it"), ("Menu", "a context menu"),
                           ("Escape", "close, or leave")):
            page.draw_label((x + pad, foot, width, line), lxb.Text.LABEL, name)
            page.draw_label((x + pad + left, foot, width, line), lxb.Text.LABEL,
                            what, lxb.Role.TEXT_SOFT)
            foot += line

    # -- the eight pages ---------------------------------------------------

    def head(self, page, title, mark=None):
        if mark is None:
            page.heading(title)
        else:
            page.head(mark, title)
        x, y, width, _ = page.cursor
        page.draw_rule((x, y - page.m(lxb.Metric.GAP) * 0.5, width, max(page.s(2.0), 1.0)))

    def hello(self, page):
        self.head(page, GREETING, "launch")
        column = page.m(lxb.Metric.COLUMN_SPACING)
        line = page.line(lxb.Text.BODY)
        palette = lxb.PALETTES[self.palette]
        for name, value in (
            ("theme", f"{palette.name}, {self.theme.wallpaper.name.capitalize()}"
                      f" wallpaper, {self.theme.icons.name.capitalize()} marks"),
            ("colour", f"text {palette.color(lxb.Role.TEXT)} over "
                       f"sky-top {palette.color(lxb.Role.SKY_TOP)}"),
            ("size", f"title {page.line(lxb.Text.TITLE):.0f}px in a "
                     f"{page.m(lxb.Metric.ROW_HEIGHT):.0f}px row"),
            ("motion", f"a panel arrives over {lxb.DURATIONS['panel'] * 1000:.0f}ms, "
                       "and never linearly"),
            ("mark", "launch, drawn in a 32-unit square"),
            ("sound", f"press-selected, {len(lxb.SOUNDS)} recordings in all"),
        ):
            x, y, width, _ = page.cursor
            page.draw_label((x, y, column, line), lxb.Text.BODY, name)
            page.draw_label((x + column, y, width - column, line), lxb.Text.CAPTION,
                            value, lxb.Role.TEXT_SOFT)
            page.cursor = (x, y + line, width, 0)
        page.gap()
        page.note("Every value on this page was named rather than picked, and "
                  "every one of them depends on the height it was asked at — "
                  "resize the window and read them again.")
        page.text("Everything here is the shell's own material: the water "
                  "behind it is the wallpaper shader, every pane bends what is "
                  "behind it through the glass shader, and the mark above is a "
                  "distance field shaded into a bead of water by the glyph "
                  "shader. This page asks for them by name and never touches "
                  "one.")

    def colour(self, page):
        self.head(page, "Colour")
        x, y, width, _ = page.cursor
        gap = page.m(lxb.Metric.GAP)
        step = page.line(lxb.Text.BODY) * 1.35
        chip = page.m(lxb.Metric.TILE)
        column = (width - gap) / 2
        per = (len(lxb.Role) + 1) // 2
        palette = lxb.PALETTES[self.cursor[1]]

        for index, role in enumerate(lxb.Role):
            left = x + (index // per) * (column + gap)
            at = y + (index % per) * step
            height = chip * 0.42
            page.draw_card((left, at + (step - height) / 2, chip, height),
                           lxb.Surface.CONTROL, role)
            page.draw_label((left + chip + gap * 0.5, at, column, step),
                            lxb.Text.BODY, role.name.lower().replace("_", "-"))
            page.draw_label((left, at, column, step), lxb.Text.CAPTION,
                            palette.color(role), lxb.Role.TEXT_SOFT,
                            lxb.Align.RIGHT)

        page.cursor = (x, y + per * step + gap, width, 0)
        self.chips(page, [p.name for p in lxb.PALETTES], self.cursor[1])
        page.gap()
        page.text(f"{lxb.PALETTES[self.palette].name} is what the shell has. "
                  "Left and Right choose, Enter applies — and the "
                  f"{len(lxb.Role)} colours above travel to their new values "
                  f"over {lxb.DURATIONS['accent-change'] * 1000:.0f}ms, in "
                  "linear light.")

    def material(self, page):
        self.head(page, "Material")
        x, y, width, _ = page.cursor
        gap = page.m(lxb.Metric.GAP)
        card = (width - 2 * gap) / 3
        line = page.line(lxb.Text.CAPTION)
        height = page.line(lxb.Text.TITLE) + 4 * line + 2 * gap

        for index, surface in enumerate(lxb.Surface):
            glass = lxb.glass(surface)
            left = x + index * (card + gap)
            lit = index == self.cursor[2]
            page.draw_spot(ITEM_SPOT + index, (left, y, card, height))
            page.draw_card((left, y, card, height), surface,
                           lxb.Role.ACCENT_SOFT if lit else lxb.Role.GLASS,
                           1.0 if lit else 0.8)
            page.draw_label((left + gap, y + gap * 0.5, card - 2 * gap,
                             page.line(lxb.Text.TITLE)), lxb.Text.TITLE,
                            surface.name.capitalize())
            top = y + gap * 0.5 + page.line(lxb.Text.TITLE)
            for name, value in (("depth", f"{glass.depth:.0f}px"),
                                ("frost", f"{glass.frost:.2f}"),
                                ("gloss", f"{glass.gloss:.2f}"),
                                ("curve", f"{glass.curve:.0f}")):
                page.draw_label((left + gap, top, card - 2 * gap, line),
                                lxb.Text.CAPTION, name, lxb.Role.TEXT_SOFT)
                page.draw_label((left + gap, top, card - 2 * gap, line),
                                lxb.Text.CAPTION, value, lxb.Role.TEXT,
                                lxb.Align.RIGHT)
                top += line

        page.cursor = (x, y + height + gap, width, 0)
        page.text("Three cuts, and everything is made of one of them: a compact "
                  "slab for a panel, a clear lozenge for what you can act on, "
                  "and a broad shallow sheet for a sidebar. How much each one "
                  "shows of the page it is standing on is the material rather "
                  "than an alpha.")
        page.note("The two panes on this screen are the fourth thing: the "
                  "layered menu-and-dialog material — a stain, two lights under "
                  "it and a hairline over it. Every one of them refracts what "
                  "is behind it.")

    def marks(self, page):
        self.head(page, "Marks")
        x, y, width, _ = page.cursor
        gap = page.m(lxb.Metric.GAP) * 0.5
        cell = page.m(lxb.Metric.ITEM_ICON) + gap
        across = max(int(width // cell), 1)
        chosen = self.cursor[3]

        for index, name in enumerate(lxb.GLYPHS):
            left = x + (index % across) * cell
            at = y + (index // across) * cell
            if index == chosen:
                self.light(page, (left - gap * 0.5, at - gap * 0.5, cell, cell))
            page.draw_spot(ITEM_SPOT + index,
                           (left - gap * 0.5, at - gap * 0.5, cell, cell))
            page.draw_icon((left, at, cell - gap, cell - gap), name)

        rows = (len(lxb.GLYPHS) + across - 1) // across
        page.cursor = (x, y + rows * cell + gap, width, 0)
        page.heading(lxb.GLYPHS[chosen])
        page.note(f"All {len(lxb.GLYPHS)} of the shell's marks, shipped in the "
                  "library. Each is a shape, and the shader makes it a bead of "
                  "water: the same one the shell draws, at whatever size it is "
                  "asked for.")

    def type_(self, page):
        self.head(page, "Type")
        x, y, width, _ = page.cursor
        gap = page.m(lxb.Metric.GAP)
        column = page.m(lxb.Metric.COLUMN_SPACING)
        for index, text in enumerate(lxb.Text):
            line = page.line(text)
            lit = index == self.cursor[4]
            if lit:
                self.light(page, (x - gap * 0.5, y, width + gap, line))
            page.draw_spot(ITEM_SPOT + index, (x - gap * 0.5, y, width + gap, line))
            page.draw_label((x, y, width, line), text, text.name.capitalize())
            # Against the far edge rather than in a second column: Display is
            # three times the height of Caption, so no one column start clears
            # every name on this page.
            page.draw_label((x, y, width - page.m(lxb.Metric.GAP), line),
                            lxb.Text.CAPTION,
                            f"{lxb.text_size(text, page.height):.0f}px, line "
                            f"{line:.0f}px, "
                            f"{'bold' if lxb.text_is_bold(text) else 'regular'}",
                            lxb.Role.TEXT_SOFT, lxb.Align.RIGHT)
            y += line + gap * 0.5
        page.cursor = (x, y + gap * 0.5, width, 0)
        page.text("Five sizes and no others, each a share of the display's "
                  "height. The face is shipped with the library, so a machine "
                  "with no fonts installed draws exactly this.")

    def motion(self, page):
        self.head(page, "Motion")
        x, y, width, _ = page.cursor
        gap = page.m(lxb.Metric.GAP)
        line = page.line(lxb.Text.BODY)
        column = page.m(lxb.Metric.COLUMN_SPACING)
        # The bar and what it says are two columns, not one: a caption laid
        # over the track it describes is a caption with a bead running through
        # it twice a second.
        caption = page.m(lxb.Metric.COLUMN_SPACING) * 1.4
        track = width - column - caption - gap
        bead = page.s(14.0)

        for index, (name, what) in enumerate(SHOWN_DURATIONS):
            over = lxb.DURATIONS[name]
            # Each runs on its own clock, over and back, so the whole page is
            # one loop and every bar is the same journey at its own length.
            through = (page.seconds % (over * 2)) / over
            travelled = lxb.ease(through if through <= 1 else 2 - through)
            lit = index == self.cursor[5]
            page.draw_spot(ITEM_SPOT + index, (x, y, width, line))
            page.draw_label((x, y, column, line), lxb.Text.BODY, name)
            page.draw_rule((x + column, y + line * 0.5, track, max(page.s(2.0), 1.0)))
            page.draw_card((x + column + travelled * (track - bead),
                            y + (line - bead) * 0.5, bead, bead),
                           lxb.Surface.CONTROL,
                           lxb.Role.ACCENT if lit else lxb.Role.ACCENT_SOFT)
            page.draw_label((x + column + track + gap, y, caption, line),
                            lxb.Text.CAPTION, f"{over * 1000:.0f}ms, {what}",
                            lxb.Role.TEXT_SOFT, lxb.Align.RIGHT)
            y += line + gap * 0.5

        page.cursor = (x, y + gap * 0.5, width, 0)
        page.text(f"{len(lxb.DURATIONS)} named durations, and not one of them "
                  "is linear: everything in this language leaves and arrives "
                  "gently, because nothing physical starts at full speed.")

    def sound(self, page):
        self.head(page, "Sound")
        x, y, width, _ = page.cursor
        row = page.m(lxb.Metric.ROW_HEIGHT) * 0.7
        chosen = self.cursor[6]
        pad = page.m(lxb.Metric.ROW_PADDING)
        for index, name in enumerate(lxb.SOUNDS):
            lit = index == chosen
            # The light and nothing else: a column of fourteen chips says only
            # that there are fourteen of them. This is what `page.item` draws,
            # laid out here because the page moves its own selection.
            if lit:
                self.light(page, (x, y, width, row))
            page.draw_spot(ITEM_SPOT + index, (x, y, width, row))
            page.draw_label((x + pad, y, width - 2 * pad, row), lxb.Text.BODY,
                            name, lxb.Role.TEXT if lit else lxb.Role.TEXT_SOFT)
            y += row
        page.cursor = (x, y + page.m(lxb.Metric.GAP), width, 0)
        page.text("Fourteen recordings, shipped with the library. Left and "
                  "Right walk them and each one plays as it is reached; Enter "
                  "plays it again. No clip is ever laid over a copy of itself.")

    def picker(self, page):
        self.head(page, "File and folder picker", "file-folder")
        self.chips(page, PICKER_LABELS, self.cursor[7])
        page.gap()
        page.note("No file or folder chosen yet." if self.picked_path is None
                  else f"Last choice: {self.picked_path.name}")
        page.text("One call opens a centred Lattice window covering roughly seventy percent "
                  "of this application. Folder columns recede along the trail while strong "
                  "frost and depth put this page behind it. A on Search opens its controller "
                  "keyboard; Start finishes; B or its hide key returns without losing the "
                  "query.")

    def chips(self, page, names, chosen):
        """A row of capsules, one of them lit. What choosing between a handful
        of things looks like in this language."""
        x, y, width, _ = page.cursor
        gap = page.m(lxb.Metric.GAP) * 0.5
        height = page.s(44.0)
        left = x
        top = y
        rows = 1
        rects = []
        for name in names:
            room = page.measure(lxb.Text.LABEL, name) + 2 * page.m(lxb.Metric.ROW_PADDING)
            # A capsule that would hang off the page starts the next row
            # instead. One that is wider than the whole column stays where it
            # is, because there is nowhere better for it to go.
            if left > x and left + room > x + width:
                left = x
                top += height + gap
                rows += 1
            rects.append((left, top, room, height))
            left += room + gap

        # The light first, and the faces over it: one object crossing the row
        # rather than a property each capsule has, so walking the row is the
        # light travelling and not five capsules taking turns.
        self.light(page, rects[chosen])
        for index, name in enumerate(names):
            page.draw_spot(ITEM_SPOT + index, rects[index])
            # Asked of the page rather than decided here: a control the light
            # is on is part-way through a press whenever one is in flight, and
            # this program has no way of its own to know that.
            page.draw_button(rects[index], name, page.press(index == chosen))
        page.cursor = (x, y + rows * (height + gap), width, 0)


def main():
    tour = Tour()
    app = lxb.App("com.example.LxbTour", "lxb-toolkit", plain=True, driven=True)
    app.page(tour.draw)
    if sys.argv[1:2] == ["--shot"]:
        if len(sys.argv) > 3:
            tour.section = [name.lower() for name in SECTIONS].index(sys.argv[3].lower())
        # A panel, if one was asked for: raised on the first frame, settled by
        # the second, which is the one that is kept.
        tour.raise_panel = sys.argv[4] if len(sys.argv) > 4 else None
        if tour.raise_panel is not None and tour.raise_panel not in PANEL_SHOTS:
            raise SystemExit("panel must be one of: " + ", ".join(PANEL_SHOTS))
        app.shot(sys.argv[2])
    else:
        app.run()


main()
