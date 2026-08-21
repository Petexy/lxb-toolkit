/*
 * Hello, world — the whole of an application in this language.
 *
 * A Wayland window carrying the shell's wallpaper and glass, driven by a
 * keyboard, a pointer and every controller plugged into the machine, and
 * answering with the shell's own sounds. Nothing below names a colour, a
 * radius, a duration or a device.
 *
 *     make hello && ./hello              # the window
 *     ./hello --shot hello.png               # one frame, with no display
 */

#include <stdio.h>
#include <string.h>

#include <lxb_app.h>

struct hello {
    int clicked;
};

static void page(lxb_page *page, void *data)
{
    struct hello *hello = data;

    lxb_page_head(page, "launch", "Hello, world");
    lxb_page_text(page, hello->clicked == 0
                            ? "Press the button. Move with the arrow keys, a "
                              "stick or a D-pad."
                            : hello->clicked == 1 ? "You clicked me!"
                                                  : "You clicked me again.");
    lxb_page_gap(page);

    if (lxb_page_button(page, "Hello World!")) {
        hello->clicked++;
    }
    if (lxb_page_button(page, "Leave")) {
        lxb_page_quit(page);
    }
}

int main(int argc, char **argv)
{
    struct hello hello = { 0 };
    lxb_app *app = lxb_app_new("com.example.HelloWorld", "Hello World");
    int shot = argc > 2 && strcmp(argv[1], "--shot") == 0;
    int code = shot ? lxb_app_shot(app, argv[2], 1280, 800, 10.0f, page, &hello)
                    : lxb_app_run(app, page, &hello);
    if (code != 0) {
        fprintf(stderr, "%s\n", lxb_app_trouble(app));
    }
    lxb_app_free(app);
    return code;
}
