#!/usr/bin/env python3

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
