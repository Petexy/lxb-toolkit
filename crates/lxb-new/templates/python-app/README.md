# {{TITLE}}

A small standard Wayland application in Python, built on `lxb-app` — the whole
of the LineXinBar design language with a window around it.

{{BODY}}

## Run it

```sh
python3 main.py
```

`import lxb_toolkit` opens `liblxb_toolkit`, which answers what the language is
and carries no GPU stack with it. The first use of `lxb.App` opens the second
library, `liblxb_app`, which carries the renderer, the window system, the
controllers and the audio device. A program that only wants the answers never
loads it.

## One frame, drawn to a file

```python
app.shot("page.png")
```

Drawn with no display at all, through the same page function and the same
renderer as the window. It is how an interface in this language is checked
without a screen.
