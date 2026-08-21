#!/usr/bin/env python3
"""Hello, world — the whole of an application in this language.

A Wayland window carrying the shell's wallpaper and glass, driven by a
keyboard, a pointer and every controller plugged into the machine, and
answering with the shell's own sounds. Nothing below names a colour, a radius,
a duration or a device.

    python3 hello.py                    # the window
    python3 hello.py --shot hello.png   # one frame, with no display
"""

import sys

import lxb_toolkit as lxb

app = lxb.App("com.example.HelloWorld", "Hello World")
clicked = 0


@app.page
def _(page):
    global clicked
    page.head("launch", "Hello, world")
    page.text({
        0: "Press the button. Move with the arrow keys, a stick or a D-pad.",
        1: "You clicked me!",
    }.get(clicked, "You clicked me again."))
    page.gap()

    if page.button("Hello World!"):
        clicked += 1
    if page.button("Leave"):
        page.quit()


if sys.argv[1:2] == ["--shot"]:
    app.shot(sys.argv[2])
else:
    app.run()
