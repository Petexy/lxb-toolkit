# Examples

## Hello, world

The whole of an application in this language, in each of the three:

```sh
cargo build --release                      # from the repository root, first

(cd rust-hello && cargo run --release)                 # Rust    25 lines
(cd c && make hello && ./hello)                        # C       50 lines
PYTHONPATH=../python python3 python/hello.py           # Python  40 lines
```

Each opens a real Wayland window carrying the shell's wallpaper and glass,
driven by a keyboard, a pointer and every controller plugged into the machine,
and answering with the shell's own sounds. None of them names a colour, a
radius, a duration or a device; none of them opens a window, a surface, a
device or an event loop. All three are one page function against `lxb-app`.

## The tour

Eight pages of the language itself — what it answers, its colour, its material,
its marks, its type, its motion, its sounds, and its built-in file and folder
picker:

```sh
(cd rust && cargo run --release)                       # Rust
(cd c && make tour && ./tour)                          # C
PYTHONPATH=../python python3 python/tour.py            # Python
```

    Up / Down     the page         Left / Right  within it
    Enter         act on it        Menu / F10    a context menu
    Escape        close, or leave  wasd / hjkl   the directions, and y the menu

The C and Python tours ask to drive themselves — `lxb_app_driven` — because a
tour is a screen rather than a list: it has two axes, and only the application
knows which is which. The Rust one goes a step lower still and drives
`lxb-render` directly, with its own window loop, which is what an application
that has outgrown the page function looks like.

Every one of them draws the shell's own material on the GPU: the analytic water
behind everything, panes of real glass that bend what is behind them — so the
sidebar bends the water and the context menu bends the sidebar, the rows and
the words — and every mark shaded into a bead of water out of its own
distance fields. None of them contains a shader, a pipeline or an atlas.

All three raise the same context menu, and it is the shell's rather than a
lookalike: it stands *beside* the row it is about, its list is measured by
`menu::Layout`, it is laid out once and flown out of its anchor as one shape,
and the page behind it steps back, dims and gives up the words the panel is
covering. Their Picker page also opens the same contained Lattice file and
folder window: it covers roughly seventy percent of the app, path columns
recede, the focused row owns the glass, and the surrounding application falls
back under strong frost and depth.

## They are the same frames

Every greeting takes `--shot FILE [PAGE] [OVERLAY]`: one settled frame to a
PNG, with no display, compositor or window involved, through the same page
function and the same renderer as the window. So they can be compared, byte for
byte:

```sh
scripts/check-greetings.sh
```

Seventeen views — eight pages of the tour, its menu, its question, the five
file questions it can put, and the short greeting. A PNG of the same pixels is the same
file, so anything at all that differs fails there. It is the only check that
catches a binding which crosses a value correctly and then uses it slightly
differently.

**Sixteen of those are C against Python.** The seventeenth is the short
greeting, which is the one view that exists in all three languages. The Rust
tour is deliberately not a third copy of the C one — it carries prose of its
own, drives its own window, and reaches the renderer directly rather than
through `lxb-app` — so nothing compares its pixels to anything. What covers it
is its own test suite, and **`cargo test --workspace` does not reach it**,
because this directory is outside the workspace:

```sh
(cd rust && cargo fmt --check \
    && cargo clippy --all-targets --locked -- -D warnings \
    && cargo test --release --locked)
```

Run that, or the tour drifts and every other check stays green while it does.
It has: the accent palette grew to twelve and the Rust tour went on drawing
them in one row, off the edge of the page, for as long as nobody looked.

One trap, and it costs a whole afternoon: **do the arithmetic in double and
narrow once.** Python has no float32, so every length it works out is a double
and is rounded exactly once, where it crosses. A C greeting that computes in
`float` rounds at every step and lands one part in ten million away — invisible
on screen, and fatal here. `tour.c`'s `rect_of` is where that narrowing
happens, and it is the only place it does.

## Without a GPU at all

`paint` draws the three shipped shaders over a buffer of pixels — the same
constants, the same terms, the same order — so a program with a painter and no
renderer gets glass that bends what is behind it rather than a rectangle with a
bright edge painted on:

```sh
(cd c && make && ./swatch)                 # the whole language, as a picture
python3 python/contact_sheet.py            # every mark, as a page
```

**CSS** is the fourth way in and the one with no library at run time, so its
palette is chosen when the stylesheet is written rather than read when the page
is opened:

```sh
(cd css && ./generate.sh && xdg-open hello.html)   # css/hello.html
```

`generate.sh` takes a palette name and otherwise uses this machine's own
accent.

## The whole language, without a window

```sh
cargo run -p lxb-toolkit --example tokens -- 2160  # every token, in the terminal
cargo run -p lxb-render --example one-page         # a whole application, to a PNG
(cd c && make && ./swatch Blue 2160)               # the palette and marks, to swatch.svg
python3 python/contact_sheet.py Green sheet.html   # all of it, as one page
```

## Building something that is not an example

`lxb-new` generates a runnable Wayland application rather than a demonstration
of the library. See the repository README and
[`../docs/application-development.md`](../docs/application-development.md).
