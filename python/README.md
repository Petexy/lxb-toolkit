# lxb-toolkit, for Python

The current LineXinBar design language through the same native core as the Rust
and C APIs: colour roles, theme choices, glass, motion, sizes, type, 98 neutral
marks, their SDF/material contract, sound semantics, and what a key, a face
button and a stick mean.

## A whole application

```python
import lxb_toolkit as lxb

app = lxb.App("com.example.HelloWorld", "Hello World")

@app.page
def _(page):
    page.head("launch", "Hello, world")
    if page.button("Hello World!"):
        print("clicked")

app.run()
```

That is a real Wayland window carrying the shell's wallpaper and glass, drawn
by the shell's own renderer on the GPU, driven by a keyboard, a pointer and
every controller plugged into the machine, and answering with the shell's own
sounds. `app.shot("page.png")` draws one settled frame of it with no display at
all.

`lxb.App` opens a *second* library, `liblxb_app`, the first time it is used —
it carries the GPU stack, the window system, the controllers and the audio
device. Importing this package never asks for it, so a program that only wants
what the language answers keeps a library with no GPU stack in it and nothing
else.

```python
from lxb_toolkit import (
    Accent, Metric, Overlay, Role, ShellTheme, ease, glyph,
    overlay_material, size,
)

theme = ShellTheme.load()
accent = Accent(theme.accent.name)

accent.preview("Blue")               # show without applying
while accent.advance(1 / 60):        # one coherent frame at a time
    tint = accent.color(Role.ACCENT).linear
accent.restore()                     # leave without choosing
while accent.advance(1 / 60):
    ...

size(Metric.ROW_HEIGHT, 2160)        # 152.0; scale is height-based and clamped
ease(0.25)                           # 0.0625; motion is never linear
glyph("launch")                      # neutral lxb:shape SVG source

# A normal dialogue and a context menu are cut from this same four-layer pane.
assert overlay_material(Overlay.DIALOG) == overlay_material(Overlay.CONTEXT_MENU)
```

`glyph_sdf()` converts raster coverage into the encoded field consumed by
`glyph_wgsl()`. Both the Default water-bead and Simple flat materials use that
same field; rendering the SVG directly is only a source preview.

`lxb_toolkit.paint` draws the three shipped shaders without a renderer, over
whatever buffer of 32-bit pixels you are painting on — `wallpaper()` for the
scene, `glass()` for a pane that bends what is behind it, `glyph()` for that
same field shaded into a bead of water, plus the `light()` under a pane and the
`scrim()` behind a panel. `examples/python/hello.py` is the whole of it against
cairo.

`Action`, `Key` and `Button` are what a control *means*, which is the half of
input worth agreeing on across languages — reading a real device is your own
toolkit's job, and turning its key events into keys of the language is about
fifteen lines:

```python
action = action_of_key(KEYS.get(keyval, -1))
if action is None and 0 < keyval < 128:
    action = action_of_letter(chr(keyval))   # wasd/hjkl/y, where nothing is typed
```

`Repeat` is the middle of a held direction, which no platform supplies: hand it
the arrow keys, the D-pad and the sticks and step it once a poll, and a held
direction walks a list at the same pace on the keyboard and on the pad. `Wheel`
carries the part-turn a touchpad reports, so a slow drag still moves. The
`INPUT_*` constants are the cadence and the two stick distances.
`action_of_button(Button.GUIDE)` answers `None`, and always will: the guide
button is how somebody gets out of your application.

`overlay_material()` supplies the complete current pane recipe: the two quiet
lights under the glass, the curved Sidebar cut, and its hairline rim. Dialogue
width, scrim, and content may differ from a context menu; the material does not.

For a normal `lxb.App`, use the built-in Lattice picker. It stays in the
running application window as a centred glass window covering roughly seventy
percent of the app: directory columns recede along the trail, the focused row
owns the glass, and the surrounding page falls under strong frost and depth
while the picker owns keyboard, controller, and pointer input:

```python
from pathlib import Path
import lxb_toolkit as lxb

app = lxb.App("com.example.Gallery", "Gallery")

@app.page
def draw(page):
    if page.button("Choose image"):
        page.pick(lxb.Selection.IMAGE, Path.home() / "Pictures")
    if path := page.picked():
        print("selected", path)

app.run()
```

`page.pick` returns `False` when another panel already owns input; `page.picked()`
returns a `Path` once when a file or folder was accepted, and `None` after a
cancellation. Folder mode exposes **Select folder** at the head of its active
column, so a
listed folder is always opened rather than accidentally accepted.

`Picker` remains the lazy lower-level file/folder-selection model for a custom
renderer. Start it where your application wants, draw `picker.entries`, call
`picker.enter()` on a folder, and save `picker.choose()` from your own explicit
action. `Selection.FOLDER` lists directories only and deliberately has no
search field; draw **Select folder** only while `picker.can_choose` is true. It
follows folder symlinks, hides dotfiles, and keeps folders before files. Use it
as a context manager so its native model is released promptly:

```python
from pathlib import Path
import lxb_toolkit as lxb

with lxb.Picker(lxb.Selection.IMAGE, Path.home() / "Pictures") as picker:
    for index, entry in enumerate(picker.entries):
        draw_row(entry.name, focused=index == picker.selected)
    # On an explicit accept action: picker.enter() or picker.choose().
```

The module is `ctypes` over `liblxb_toolkit`. It looks for the native library
beside the package, then in this repository's `target/release` and
`target/debug`, and finally through the platform library search. Set
`LXB_TOOLKIT_LIBRARY` to an explicit build during development. A distributable
wheel must include the built shared library beside `lxb_toolkit` as described in
the repository README.

See the repository's `docs/application-development.md` for the standard Wayland
window, desktop-entry, theme, scaling, input, sound, and lifecycle contract.
