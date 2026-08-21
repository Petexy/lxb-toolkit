# Building an application for LineXinBar

An application for LineXinBar is a normal desktop application. It opens a
standard Wayland window, is installed through a standard desktop entry, and can
also run under other Wayland desktops. There is no LXB-only application
protocol to opt into.

The toolkit has six layers, and most applications meet only the last two:

- `lxb-toolkit` is the renderer-neutral source of truth for colour, material,
  motion, type, metrics, marks, sound and what a control means. Rust, C and
  Python all read the same values, and it carries no dependencies at all.
- `lxb-render` draws them. One call per thing — `ui.pane`, `ui.button`,
  `ui.icon`, `ui.context_menu` — each of which is the real material rather than
  a shape with the right colour in it. It owns the shader, the passes, the mark
  atlas and the text, because otherwise every application would own an
  identical copy of them. It also answers what is under the pointer, because a
  pointer has to know what is on the screen and only the thing that drew it
  does.
- `lxb-input` reads the controls. Every controller on the machine and the
  keyboard, as one list of actions with nothing in it saying which sent what —
  including the middle of a held direction, which no platform supplies.
- `lxb-sound` answers them, at the rests the language keeps.
- `lxb-app` is the window they all meet in. It opens the surface, runs the
  frames, polls every controller, keeps the light on one control at a time and
  plays the click — so that an application contains only what is on its own
  page. It is what most applications use, and the only one of the five they
  have to name.
- `lxb-new` creates that application around it, in Rust, C or Python: a real
  window, a first page, focus and activation, and matching application
  metadata.

## The short form

```rust
lxb_app::App::new("com.example.hello", "Hello World").run(|page| {
    page.head("launch", "Hello, world");
    if page.button("Hello World!") {
        println!("clicked");
    }
})
```

Controls are numbered by the order they are drawn in, which is the order they
are read in — so the light moves between them in that order, and no application
registers, names or lays out its own focus. A control answers `true` on the
frame a press lands on it, whether that press came from a key, a controller's
bottom face button or a click.

The flow — `head`, `text`, `note`, `button`, `row`, `item`, `rule`, `gap` —
lays itself out down the page at the language's own sizes and spacings, so a
page that is a list of things never computes a rectangle. For one that is not,
`Page::ui` is the renderer itself and draws in exactly the same material;
`App::plain` gives you the window's own rectangle rather than the standard
page, and `App::driven` hands over the actions so a screen with more than one
axis can move its own selection.

`App::shot` draws a settled frame to a PNG with no display at all, through the
same page function and the same renderer as the window. It is how an interface
in this language is checked without a screen.

## The same thing, elsewhere

C and Python get all of it, through a second shared library: `liblxb_app`,
which carries the GPU stack, the window system, the controllers and the audio
device. `liblxb_toolkit` goes on carrying nothing at all, so a program that
only wants the language's answers still links a library with no dependencies.

```c
#include <lxb_app.h>

static void page(lxb_page *page, void *data)
{
    lxb_page_head(page, "launch", "Hello, world");
    if (lxb_page_button(page, "Hello World!")) {
        puts("clicked");
    }
}
```

```python
import lxb_toolkit as lxb

app = lxb.App("com.example.hello", "Hello World")

@app.page
def _(page):
    page.head("launch", "Hello, world")
    if page.button("Hello World!"):
        print("clicked")

app.run()
```

Every enumeration that crosses is an index into `liblxb_toolkit`'s own list of
the same values, so the two libraries agree by construction rather than by two
hand-kept tables.

An application that would rather keep its own window and toolkit — GTK, Qt,
cairo, a web view — reads `lxb-toolkit` alone and draws with what it has. It
can have the material too, without a renderer: `paint` draws the three shipped
shaders over a buffer of 32-bit pixels, so a pane
really does bend what is behind it and a mark really is shaded out of its
distance field. `lxb_paint_*` in C, `lxb_toolkit.paint` in Python; the price is
processor time rather than fidelity, and `examples/c` and `examples/python` are
the whole of it. A web view still gets the honest approximation, and the CSS
surface exists to make that trade-off explicit rather than accidental; see
`stylesheet()`.

## Start an application

From this repository:

```sh
cargo run -p lxb-new -- --toolkit-path crates/lxb-toolkit my-player /path/to/my-player
cd /path/to/my-player
cargo run
```

The generated directory is deliberately ordinary. It can be moved into its own
repository, edited without the generator, and packaged like any other Cargo
application. `--toolkit-path` records an explicit local core-crate path for an
unpublished checkout; omit it after the matching release is available from the
package registry.

The generator refuses invalid application ids and refuses to write over any
existing target. That matters because one stable name has to agree in five
places:

```text
my-player.desktop
StartupWMClass=my-player
Icon=my-player
xdg_toplevel.app_id = my-player
the executable name = my-player
```

The visible window title is for people and may contain spaces. The stable id is
for matching a launched process to the window that appeared.

## Desktop entry contract

LineXinBar discovers applications through XDG desktop files. At minimum:

```ini
[Desktop Entry]
Type=Application
Name=My Player
Exec=my-player
Icon=my-player
StartupWMClass=my-player
Categories=AudioVideo;
OnlyShowIn=LineXinBar;
```

`OnlyShowIn` is optional; omit it if the application belongs on other desktops
too. Keeping it costs one thing worth knowing in advance: `desktop-file-validate`
rejects `LineXinBar` as an unregistered value, because the desktop names it
accepts are the ones registered with freedesktop.org and this is not one of
them. The entry is correct — the session declares the same name in
`DesktopNames`, and the shell matches it case-insensitively — but a distribution
packaging a generated application will see that error from `lintian` or
`rpmlint` and should expect it rather than "fix" it by renaming, which would
hide the application from the only desktop it was written for. Use standard category names such as `AudioVideo`, `Graphics`, `Network`,
`Office`, `Game`, `Development`, `Education`, `Science`, `System`, or `Utility`.
The application icon belongs to the installed application and therefore follows
the system icon-theme rules; LXB's bundled marks are for controls inside the
interface.

LineXinBar removes argument-dependent field codes when it launches an entry
without a file. Keep a normal `Exec` line and handle files through the usual XDG
mechanisms rather than relying on shell-private behavior.

## Theme and scale

`ShellTheme::load()` reads the current user's `accent`, `theme-wallpaper`, and
`theme-icons` from `$XDG_CONFIG_HOME/lxb/shell.toml` (or
`~/.config/lxb/shell.toml`), including the older single `theme` key. Missing,
unreadable and unknown values safely produce Purple and the Default materials.

This is a startup snapshot, not a live theme protocol: LineXinBar currently
publishes no accent-change protocol to ordinary applications. Reload it when
your own application is already rebuilding its UI, or at its next launch.

All authored lengths use a 1080-pixel reference height. `scale_for(height)` is
the shared conversion and clamps the result to `0.6..=2.5`. Pass height in the
same coordinate units your renderer uses for layout and drawing, and convert
between logical and physical coordinates exactly once. Fractional output scale
must not be applied a second time to values already expressed in render-target
pixels.

## Dialogs and context menus

A normal application dialog uses `Overlay::Dialog.material()`, which is exactly
the material returned by `Overlay::ContextMenu.material()`. It is one broad
Sidebar pane with two quiet accent lights underneath and a hairline over it;
only the dialog's width, position, scrim, and staging differ from a context
menu. Do not use `Surface::Panel` for a normal dialog. That dense, deeply
frosted compact cut belongs to LineXinBar's Power question and opaque wells.

## Input

An interface in this language is driven from a keyboard, a controller and a
pointer at once, and the whole point is that they are one interface rather than
three. `lxb_toolkit::input` is what decides that — the mapping, the cadence and
the thresholds, with nothing in it that opens a device — and `lxb-input` is the
half that reads real ones:

```rust
let mut controls = Controls::new();          // never fails; no pad is fine
// … on a key event
if let Some(action) = controls.key(key, down, now) { act(action); }
// … once a frame
for action in controls.poll(now) { act(action); }
```

Five rules are worth knowing before wiring any of it up.

- **The middle of a held direction is invented, not taken.** Wayland hands a
  client one press and one release; a pad has no repeat at all. So the pace is
  decided in `input::Repeat`, and it is the *same* pace for an arrow key and a
  D-pad — a held arrow that walked a list faster than a held D-pad would make
  the two controls two interfaces. Drop your platform's own key repeat.
- **The guide button is never an application's.** It is the way out of the
  application, so an application that could act on it could swallow the way out
  of itself. `Action::of_button` answers `None` for it, `lxb-input` does not
  report it at all, and the right bumper is held back while it is down so that
  photographing the application does not also turn its page.
- **A press acts on the way down** — a key, a face button, a mouse button —
  because a row activated on the release fires twice on a chord.
- **Pointing at something inside a panel selects it, silently.** Hovering is not
  a move: a pointer swept across the screen must not be a stream of clicks.
  A list the application draws itself moves in *two* steps instead — the first
  click carries the selection there and sounds like the direction that would
  have walked to it, and the second acts.
- **The letter shorthands are asked for separately.** `Action::of_letter` has
  the shell's `wasd`, `hjkl` and `y`; `Action::of_key` deliberately does not.
  Bind it only where nothing in your application is being typed into.

**The right button raises the context menu**, which is what the Menu key does —
the same `Action::Menu`, so a menu cannot come to mean two things depending on
how it was asked for. The light lands on whatever was pointed at first, so a
menu raised by a click is about the thing that was clicked.

Two things differ from the shell on purpose, and both are the same difference —
the shell has to keep a way out of a game that the game cannot take, and an
application has nothing to keep a way out of. The **Menu key** is the context
menu here rather than the guide, and **Tab** moves within the application rather
than between displays.

A controller is not a Wayland input device and never passes through a
compositor, so every application on the machine reads the same pad at the same
time. That is a property of Linux gamepads rather than a decision here.

What is under the pointer is answered by `lxb-render` rather than by
`lxb-input`, because a pointer has to know what is on the screen: every
component writes down where its rows went as it draws them, and `Ui::at(x, y)`
reads that back from the frame already on the screen. Register your own
controls with `Ui::spot(id, rect)` beside the call that drew them.

## Sound

`lxb_toolkit::sound` says which recording answers which press and — the part
that matters — when nothing should be heard at all. `lxb-sound` opens the
machine's output and puts the clip on it.

Silence is a working state, not a failure: a machine with no sound card, a
session whose server has not started, a clip that will not decode. Every call is
infallible and one that found no device simply made no sound.

- **A move sounds by where it landed, not by what moved it.** A click that puts
  the selection on a row is the same move as the direction that would have
  walked there.
- **A press that starts nothing stays silent**, and so does a move that landed
  nowhere.
- **A component keeps its own voice wherever it is raised.** A menu, a dialog
  and a mixer sound the same whichever screen opened them.
- **No clip is laid on top of a copy of itself.** Identical samples add in
  phase and one wheel event can be worth several rows; nearer than
  `sound::REST` the request is dropped rather than mixed.

## Lifecycle

- Keyboard, pointer, touch, clipboard and text input are standard Wayland.
- Applications are maximised and pinned to the display on which they launched.
  Respond to every configure instead of assuming a particular resolution or
  connector.
- A fully hidden application process tree can be stopped with `SIGSTOP` and
  resumed with `SIGCONT`. Do not use elapsed wall time as an animation delta
  after resume; clamp it, as the toolkit's spring does.
- A media application should publish MPRIS state. LineXinBar uses active,
  audible playback to keep a covered player running.

## The light, and the press

Two animations, and an application should not have to write either.

**The light is one object crossing the page**, not a property each control has.
It glides onto what is chosen rather than appearing on it, and every control
drawn after it hands its own face over by however much of the light has
arrived — which is why a control's own face cannot be what says it is chosen.
**A press** takes the control down by a seventh of itself, past its own size on
the way back, and settles, over `guide-press`.

**Using the flow, both are free.** `page.button`, `page.row` and `page.item`
number themselves, lay the light down under the first of them, glide it, and
take the press. There is nothing to switch on and nothing to keep:

```python
if page.button("Hello World!"):     # lit, glided onto, and pressed, for free
    ...
```

**Drawing your own controls, both are three calls** — what `driven` costs, and
all it costs. Lay the light down *before* the controls it is behind:

```python
page.glide(rects[chosen])                    # or place(), see below
for index, name in enumerate(names):
    page.draw_spot(ITEM_SPOT + index, rects[index])   # so a pointer finds it
    page.draw_button(rects[index], name, page.press(index == chosen))
...
if page.pressed(ITEM_SPOT + index):          # a click landed on that one
```

- `glide(rect)` travels; `place(rect)` appears, which is what a page uses when
  the row the light was on is gone — a new page of a different shape — because
  a light that flew there would be saying the two lists are one.
- `press(lit)` is the page's one press, asked per control. One is enough: the
  light is in one place, so the press is too.
- Keep your own spot numbers clear of the flow's, which count from zero in the
  order they are drawn. Both tours use `0x200`.

The same four calls exist in all three languages under the same names, and
`examples/{c,python}/tour` uses every one of them.

## Drawing

`lxb-render` is the shortest way to get any of the below on screen:

```rust
let mut ui = Ui::new(&instance, Some(&surface), format, width, height).await?;
ui.begin(width, height, seconds, &accent, theme.wallpaper, theme.icons);
ui.pane(rect, Overlay::Dialog);          // real glass over the real wallpaper
ui.icon(rect, "launch", theme.icons);    // a bead of water, not the SVG
ui.spot(PLAY, rect);                     // where a pointer landing here lands
ui.button(rect, "Press", Press::Focused);
ui.context_menu(&menu);                  // the whole component, rows and all
ui.end(&view)?;
```

The frame is drawn in four layers — the lights under a pane, the panes, what is
on them, and what stands over the page — and each bends what the one before it
left. That is not a convenience: glass has to have something to refract, so a
menu that did not have the page underneath it already drawn would be a tinted
rectangle. Choosing a layer is not something a caller does; each component
knows which one it belongs in.

What follows describes the material itself, for a renderer that is not this
one.

## Marks

Every bundled SVG is shape source. Rasterise it at four times the target cell,
convert the coverage with
`glyph_material::distance_field_alpha_from_coverage` (or `glyph_sdf()` in
Python / `lxb_glyph_sdf` in C), and shade the field with
`assets::GLYPH_WGSL`. Its Default branch makes the water bead; its Simple
branch applies the flat tint, alpha, and stain. Direct SVG rendering is only a
neutral source preview. The constants beside the shader are public so an atlas
encoder and shader cannot silently disagree about cell size or range.

Never copy the shell's private `lxb_shell_v1` protocol into an application. If
an application needs a capability not provided by standard Wayland, that is a
protocol-design question for the compositor rather than a hidden toolkit hook.
