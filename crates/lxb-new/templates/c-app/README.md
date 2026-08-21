# {{TITLE}}

A small standard Wayland application in C, built on `lxb-app` — the whole of
the LineXinBar design language with a window around it.

{{BODY}}

## Build and run

```sh
make && ./{{SLUG}}
```

One library, found through pkg-config: `pkg-config --cflags --libs lxb-app`. It
brings the renderer, the window system, the controllers and the audio device
with it, so there is nothing else to link. `liblxb_toolkit` is the other half
of the pair and carries no dependencies at all; link that one alone if all you
want is what the language answers.

## One frame, drawn to a file

`lxb_app_shot` draws a settled frame to a PNG with no display at all, through
the same page function and the same renderer as the window. It is how an
interface in this language is checked without a screen.
