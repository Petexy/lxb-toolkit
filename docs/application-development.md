# Building an application for LineXinBar

An application for LineXinBar is a normal desktop application. It opens a
standard Wayland window, is installed through a standard desktop entry, and can
also run under other Wayland desktops. There is no LXB-only application
protocol to opt into.

The toolkit has six layers, and most applications meet only the last two:

- `lxb-toolkit` is the renderer-neutral source of truth for colour, material,
  motion, type, metrics, marks, sound and what a control means. Rust, C and
  Python all read the same values. It links no GPU stack, no window system and
  no audio device; its one dependency is the Fluent catalogs the words in its
  own controls come from — see [localization](localization.md).
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

**A wheel is directions, like every other control.** It arrives as Up and Down
in the same `Page::actions()`, in the order it happened, so a page that has
never heard of a wheel still scrolls — in any of the three languages — and a
gesture cannot overtake a press that came before it. Open menus, dialogs and
file pickers keep intercepting it first, as they do a key.

What a wheel has that a key has not is somewhere it was pointed, and that is
said beside those directions rather than instead of them. `Page::scrolls()`
gives each gesture's `Spot`, its signed `steps` — positive is down — and
`from`, where the first of its directions sits in this frame's actions, so the
two accounts can be walked together. `Scroll::covers` answers whether one
action came from one gesture. C and Python ask the same thing the way they ask
about a press: `lxb_page_scrolled(page, id)` and `page.scrolled(id)` answer how
far the wheel moved over one spot the page drew. That distinction matters for a
shelf panel beside a product list — the wheel moves the pane under the hand,
not whichever pane a keyboard or controller happened to leave selected.

`App::shot` draws a settled frame to a PNG with no display at all, through the
same page function and the same renderer as the window. It is how an interface
in this language is checked without a screen.

## The same thing, elsewhere

C and Python get all of it, through a second shared library: `liblxb_app`,
which carries the GPU stack, the window system, the controllers and the audio
device. `liblxb_toolkit` goes on carrying none of that, so a program that only
wants the language's answers still links a library with no GPU stack in it.

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

## The wallpaper's clock

The wallpaper is a function, not a film: everything about the frame on screen
is the palette and a number of seconds. So a window opening in front of one
does not need the picture — it needs the second, and then it draws the same
frame the screen was already showing.

`App::run` asks for that second at startup and nothing has to call anything.
It reads `LXB_BACKGROUND_HANDOFF` — a `key=value` record, `;` separated, ASCII,
at most 1024 bytes:

```text
v=1;visual=lxb-wallpaper-v2;clock=linux-monotonic;boot=<boot id>;sample-ns=<n>;scene-ns=<n>;accent=<palette>[;theme=<Default|Simple>]
```

`sample-ns` is `CLOCK_MONOTONIC` when the record was written and `scene-ns` is
the wallpaper's clock at that instant, so the reader advances one by the
difference to recover the other. `boot` is `/proc/sys/kernel/random/boot_id`,
which is what makes one process's monotonic sample mean anything to another.

A record is used only if it names this wallpaper, was sampled on this boot, is
not from the future, and is under thirty seconds old. Anything else is refused
with a line on stderr and the window comes up at the beginning of the
animation, which is what a window with nothing to continue from does anyway.
The accent in the record is **not** applied: `shell.toml` is the setting and a
record that disagrees with it only says so. And the record is consumed — taken
out of the environment before the window opens — so no child of the application
inherits a one-shot record meant for it.

`App::shot` never reads it. A picture is taken at the second you name.

Who writes one is the other half of this. A display manager writes one for the
session it hands over to, and `lxb-compositor` passes that record to the
session shell — and to nothing else. An ordinary application launched from
inside a running session is deliberately not handed one and starts its
wallpaper at zero. So this matters when your program **is** the session's shell
or its login screen, which is the case the toolkit shares with LineXinBar's own
greeter.

To hand the phase on to something you start yourself, or to run the clock in a
window you drew without `App::run`:

```rust
use lxb_render::WallpaperClock;

// Once, before this process starts a thread of its own: reading the record
// takes it out of the environment.
let clock = WallpaperClock::from_environment(theme.accent.name)
    .unwrap_or_else(WallpaperClock::local);

// The wallpaper's second, which is the only one that came from elsewhere.
ui.begin(width, height, clock.elapsed_secs(), &accent, theme.wallpaper, theme.icons);

// And handing it on to something you start yourself.
if let Some(record) = clock.capture(theme.accent.name, Some(theme.wallpaper.name())) {
    command.env(lxb_toolkit::handoff::ENV, record.encode());
}
```

`WallpaperClock` lives in `lxb-render`, which every program drawing the
wallpaper already has, and `lxb-app` re-exports it. `lxb_toolkit::handoff` is
the record on its own — parsing, encoding and every refusal — with no clock and
no window behind it, for a program that has its own of both.

`examples/rust` is the worked version of all of this: it draws its own window,
so it does the two lines above itself.

Keep this clock separate from the one your animations run on, exactly as
LineXinBar does. Transitions, key repeat and controller motion are all this
process's own time and must not inherit a second from another process; only
the wallpaper does.

## Dialogs and context menus

A normal application dialog uses `Overlay::Dialog.material()`, which is exactly
the material returned by `Overlay::ContextMenu.material()`. It is one broad
Sidebar pane with two quiet accent lights underneath and a hairline over it;
only the dialog's width, position, scrim, and staging differ from a context
menu. Do not use `Surface::Panel` for a normal dialog. That dense, deeply
frosted compact cut belongs to LineXinBar's Power question and opaque wells.

## File and folder selection

One call asks for a file, for several, for a folder, or for somewhere to save.
Where it is answered depends on the machine, and an application does not have
to care which.

**The desktop is asked first.** Every desktop runs an
`org.freedesktop.portal.FileChooser` — that is what a portal is for — and a
question put through it is answered by the chooser the rest of the machine
uses, with the places, the recent folders and the permissions that come with
it. `lxb-app` speaks that portal itself, over the session bus, with no D-Bus
library and no runtime behind it. The question is put on a thread of its own,
so the frame loop never stops: the answer arrives at a later frame and `picked`
reports it then.

**Where there is no portal, the toolkit puts the question itself.** This is the
built-in chooser: a centred Lattice glass window covering roughly seventy
percent of the window already running. The current directory stays at the
active column, each parent path column recedes to its left, and the focused row
owns the glass and light. The application remains visibly behind the window,
but strong frost, depth, and suppressed text keep it out of the choice's way.
It owns keyboard, controller, and pointer input until it closes.

The fallback answers the same four questions as the portal, in the same shapes:
`New folder` and the row that ends the question sit at the head of the column,
a many-files question ticks each file with Accept — the tick is drawn out at the
end of the row, so a photograph keeps the picture of itself that is its mark —
and a save carries a `Name` row above `Save here`.

A legend at the foot says what the buttons do: **Select** or **Choose**,
**Approve** where a head row ends the question, **Options**, **Cancel**. Each is
drawn as the control it names — a pad's face buttons where one is plugged in, a
key or the right mouse button otherwise — so nothing has to be lettered for a
device somebody may not be holding.

**Options** is the panel's own menu, raised with the right mouse button, the
`Menu` key or the pad's north button. Three answers live there, and none of them
chooses a file, which is why none of them is on the column:

* **Types** — which kinds of file are being shown, with **Everything** always
  under them. A filter is the application's guess and is sometimes wrong; this
  is the only way past it, and without the row nothing on the panel says the
  rest of the disk is one press away. Absent where the application named no
  kind.
* **Sort** — nine orders, folders first in all of them.
* **Show hidden files** — the names beginning with a dot.

Whatever is answered there holds for every column of the walk, including ones
opened afterwards. A click past the panel, with either button, cancels the
question — which is how every panel on every desktop dismisses.

A session whose portal is broken, or an application that would rather always
draw its own, has two ways to say so: `LXB_FILE_PORTAL=0` in the environment,
and `App::own_file_questions()` in the builder. `App::shot` never asks the
desktop — a picture of a window has to contain the chooser.

```rust
use lxb_app::{App, PickerSelection};

App::new("com.example.Gallery", "Gallery").run(|page| {
    if page.button("Choose image") {
        page.pick(PickerSelection::Image, "/home/me/Pictures");
    }
    if page.button("Choose several") {
        page.pick_many(PickerSelection::Image, "/home/me/Pictures");
    }
    if page.button("Save a copy") {
        page.save("untitled.png", "/home/me/Pictures");
    }
    if let Some(path) = page.picked() {
        println!("selected {}", path.display());
    }
})
```

```c
if (lxb_page_button(page, "Choose a folder")) {
    lxb_page_pick(page, LXB_PICKER_FOLDER, "/home/me/Documents");
}
char *path = lxb_page_picked(page);
if (path != NULL) {
    printf("selected %s\n", path);
    lxb_app_string_free(path);
}
```

```python
if page.button("Choose a file"):
    page.pick(lxb.Selection.FILE, "/home/me/Documents")
if path := page.picked():
    print("selected", path)
```

`Page::pick` / `lxb_page_pick` / `Page.pick` returns false when another panel,
or a question already put to the desktop, owns the answer. `picked` is a
one-shot accepted path; a question answered with several files is read with
`picked_files` in Rust and Python, and with `lxb_page_picked_next` in C, called
until it answers null. Cancelling has no result either way — an application is
told nothing was chosen and carries on, which is the whole point of the portal
being between them.

In the toolkit's own chooser, Up and Down walk a column, Right opens the
focused folder, and Left follows the path trail back. **Use this folder**,
**Open** and **Save here** are head rows of the active column rather than rows
of the listing: activating a listed directory always opens it instead, so a
press out of habit can never hand over something nobody chose. A column never
opens on the row that answers — except a save that was already given a name,
which cannot lose anything that way. The `selection` also decides which kinds
of file a portal chooser offers: `Image` asks for images by every spelling of
their extension, `Scenery` for images and films. Pointer hover is inert: the first click moves focus
and the second activates the row. Controller Accept on **Search** opens the
picker's local keyboard; D-pad or left stick moves its focus, A enters a key,
Start finishes the query, and its bottom-right hide key returns to the lattice
without discarding it. B does the same from anywhere on the board.

For a custom renderer or a page that needs a different picker layout, the
renderer-neutral `lxb_toolkit::picker::Picker` remains available. Give it the
directory your application wants to start in, draw its entries yourself, and
keep the resulting path in your own state:

```rust
use lxb_toolkit::picker::{Picker, Selection};

let mut picker = Picker::new(Selection::Image, "/home/me/Pictures");

// Draw picker.entries(); folders come first and lead further in.
// On an explicit accept action:
if picker.selected_entry().is_some_and(|entry| entry.is_folder()) {
    picker.enter();
} else if let Some(path) = picker.choose() {
    println!("selected {}", path.display());
}
```

`Selection::File` accepts every visible file, `Image` accepts still images,
`Scenery` accepts still images and films, and `Folder` shows directories only.
Folders are always kept in the three file modes, so the user can walk further
in. The model reads a directory only when it reaches it, hides dotfiles and
non-UTF-8 names, follows directory symlinks, orders folders before files, and
caps one read at 10,000 rows. `search()` is a case-insensitive substring walk;
entering or leaving clears it. `can_search()` says whether the current listing
has a field at all: empty and unreadable file listings do not, while a search
with zero matches retains its field so it can be cleared.

Folder mode deliberately has no search field. Draw an explicit **Use this
folder** action at the head of the list only while `picker.accessible()` is
true, then call `choose()` from that action. This is what prevents activating a
folder row from selecting it instead of opening it. An unreadable directory
has no such answer and says `This cannot be opened` through `note()`.

`Purpose` is the other half of the model, and it is what the four questions
are: `OneFile`, `ManyFiles`, `AFolder`, `ANewFile`. It carries no state — it
answers what a column should offer, so a renderer of your own reaches the same
shape the built-in one does:

```rust
use lxb_toolkit::picker::{make_folder, writable, Purpose};

let purpose = Purpose::ANewFile;
purpose.lists_files();              // false only for a folder
purpose.makes_folders();            // a New folder row, where something is written
purpose.takes_several();            // rows are ticked rather than pressed
purpose.answers_with_a_head_row();  // false only for one file
purpose.accept();                   // "Open", "Use this folder", "Save here"

if writable(picker.location()) {
    let made = make_folder(picker.location(), "Holiday");
}
```

`Selection::kind()` turns the filter into the `Kind` and `Pattern` a portal
understands — case-insensitive globs written as bracket expressions, which is
the spelling every backend matches. `lxb-app` uses it to describe the question
to the desktop; a renderer of your own can put the same names in front of the
user.

### A program that draws its own window

`lxb-app` puts the whole question behind `page.pick(…)` — one call, one answer.
A program that draws its own window instead gets the same thing from
**`lxb_render::Files`**, which owns all of it: whether to ask the desktop or
draw the chooser, the thread the portal question waits on, and every action,
key and click that reaches the panel.

```rust
use lxb_render::Files;

let mut files = Files::default();

// Ask. Where it is answered is not this program's business.
files.one_file(Selection::Image, "/home/me/Pictures");
files.many_files(Selection::File, "/home/me/Documents");
files.a_folder("/home/me");
files.somewhere_to_save("untitled.png", "/home/me/Pictures");

// Every frame.
files.advance(dt);
files.hand(controls.pads() > 0);
files.draw(&mut ui);
if let Some(chosen) = files.answered() {
    println!("{chosen:?}");
}

// Every way in. Each answers the sound to play, or nothing.
files.act(action);
files.key(key, text);
files.press_at(spot, right_button);
files.point_at(spot);

// While one is open or in flight, it owns input.
if files.busy() { /* the page below is not driven */ }
```

That is the whole of it. `files.act` already knows that the search board owns
the actions while it is up, that the menu owns them while *it* is, that Accept
on a file ticks it where several were asked for and answers where one was, and
that Back closes one thing at a time. None of that has to be written twice —
which it was, in two places that had already drifted apart.

`Files::own_questions()` makes it never ask the desktop, which is what a
headless picture needs and what `App::shot` sets for itself.

The drawn chooser under it is `FilePicker`, and it takes all four:
`open_for(purpose, selection, directory, name)` is what `open` is a shorthand
for, `chose()` answers with every path (`choose()` with the first of them),
and `named()` and `ticked()` are the save's name and the set that has been
ticked. Its menu is `open_menu()` / `menu_step()` / `menu_press()` /
`close_menu()`, and `hand(pad)` says which control the legend should draw.
Reach for it only where `Files` will not do — it is the component, and `Files`
is the thing you want.

C has the same owned `lxb_picker` through `lxb_picker_*`; Python has
`lxb_toolkit.Picker`, used as a context manager. That lower-level model keeps
the same filters, sort order, search, and deliberate folder choice as the
built-in Lattice chooser. The generated [API reference](api-reference.md)
gives each binding's exact ownership and borrowed-string rules.

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

Six rules are worth knowing before wiring any of it up.

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
- **A wheel belongs to the pane under it.** It arrives as ordinary Up and Down
  actions, so a page scrolls without knowing about wheels at all; what a page
  reads `Page::scrolls()` — or `lxb_page_scrolled` — for is *which* of its
  lists the pointer was over. One fast event may carry several steps, and slow
  touchpad fractions are kept until they make one.
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

The frame is drawn in five layers — the lights under a pane, the panes, what is
on them, post-composite page effects, and what stands over the page — and each
bends what the one before it left. That is not a convenience: glass has to have
something to refract, so a menu that did not have the page underneath it already
drawn would be a tinted rectangle. Choosing a layer is not something a caller
does; each component knows which one it belongs in.

`ui.soft_edges(viewport, band, top, bottom)` — `lxb_draw_soft_edges`
in C, `page.draw_soft_edges` in Python — is the post-composite effect for a
scrolling viewport. It blurs and dissolves both surfaces and words together
near the requested ends, leaves the middle untouched, and remains behind a menu
or dialog. Blur and translucency grow together, and the last of the fade is
exactly what is behind the page's own content there — its panes, or the ground
— so a list stops without anything to stop at. `top` and `bottom` are how much
really continues past each end, from zero to one: an end with nothing beyond it
is nought and stays crisp, and an end part of the way there fades over a
*narrower* band rather than a fainter one, because a fade that stopped short of
the ground would stop at a line.

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
