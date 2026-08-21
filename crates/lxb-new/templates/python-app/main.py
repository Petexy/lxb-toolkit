#!/usr/bin/env python3
"""{{TITLE}}.

A Wayland window carrying the shell's wallpaper and glass, driven by a
keyboard, a pointer and every controller plugged into the machine, and
answering with the shell's own sounds. Nothing below names a colour, a radius,
a duration or a device: every one of those is the language's answer at this
window's height, and asking for it is the whole of the work.

Controls are numbered by the order they are drawn in, which is the order they
are read in — so the light moves between them in that order, and this file
never registers, names or lays out a focus. A control answers True on the frame
a press lands on it, whether that press came from a key, a controller's bottom
face button or a click.

The page.draw_* methods are the renderer itself, for anything the flow here
does not cover: panes, cards, marks and writing at rectangles of your own
choosing.

    python3 main.py
"""

import lxb_toolkit as lxb

app = lxb.App("{{APP_ID}}", "{{TITLE}}")
activated = 0


@app.page
def _(page):
    global activated
    page.head("launch", "{{TITLE}}")
    page.text("A standard Wayland window, drawn with the LineXinBar design "
              "language. Move with the arrow keys, a stick or a D-pad.")
    page.gap()

    if page.button("Activate"):
        activated += 1
        print(f"Activated {activated} times")
    if page.button("Leave"):
        page.quit()


app.run()
