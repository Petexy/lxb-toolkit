It opens a real Wayland window carrying LineXinBar's current analytic wallpaper
and its glass, is driven from a keyboard, a controller and a pointer at once,
and answers all three with the shell's own click. It does not use LineXinBar's
private session-shell protocol.

## What you write

One function, called once a frame, that says what is on the page. Everything
else — the window, the surface, the frame loop, every controller on the
machine, the audio device, the theme, the accent and the motion — belongs to
the application layer and is not in this project.

Controls are numbered by the order they are drawn in, which is the order they
are read in. So the light moves between them in that order, and nothing here
registers, names or lays out a focus. A control reports one press on the frame
the press lands, whether it came from a key, a controller's bottom face button
or a click.

The flow — `head`, `text`, `note`, `button`, `row`, `rule`, `gap` — lays itself
out down the page at the language's own sizes and spacings, so a page that is a
list of things never computes a rectangle. For a page that is not, the renderer
itself is one call away and draws in exactly the same material.

## How it is driven

Arrow keys, the controller D-pad and the left stick all send the same
directions, at the same pace — the middle of a held one is invented by the
input layer, because Wayland gives a client one press and one release and a pad
has no repeat at all. Activate with Enter, Space, the bottom face button or a
click; step back with Escape or the right face button; raise a context menu
with the Menu key, F10 or the top face button.

The guide button is never yours: it is how somebody leaves your application.

## The theme

The application reads the shell's theme once at startup and follows its accent,
its Default or Simple wallpaper and its Default or Simple glyph material.
LineXinBar publishes no accent-change protocol to ordinary applications, so
that reading is a snapshot: take it again when you are already rebuilding your
interface.

## Installing it

`{{SLUG}}.desktop` and `assets/app.svg` are the desktop entry and the icon. The
entry's `Exec` basename, its `StartupWMClass` and the application id in the
source all have to stay the same string, or the shell will draw your window
without its name.
