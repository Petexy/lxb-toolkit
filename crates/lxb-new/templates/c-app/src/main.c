/*
 * {{TITLE}}.
 *
 * A Wayland window carrying the shell's wallpaper and glass, driven by a
 * keyboard, a pointer and every controller plugged into the machine, and
 * answering with the shell's own sounds. Nothing below names a colour, a
 * radius, a duration or a device: every one of those is the language's answer
 * at this window's height, and asking for it is the whole of the work.
 *
 * Controls are numbered by the order they are drawn in, which is the order
 * they are read in — so the light moves between them in that order, and this
 * file never registers, names or lays out a focus. A control answers non-zero
 * on the frame a press lands on it, whether that press came from a key, a
 * controller's bottom face button or a click.
 *
 * The lxb_draw_* calls are the renderer itself, for anything the flow here
 * does not cover: panes, cards, marks and writing at rectangles of your own
 * choosing.
 */

#include <stdio.h>

#include <lxb_app.h>

struct state {
    int activated;
};

static void page(lxb_page *page, void *data)
{
    struct state *state = data;

    lxb_page_head(page, "launch", "{{TITLE}}");
    lxb_page_text(page, "A standard Wayland window, drawn with the LineXinBar "
                        "design language. Move with the arrow keys, a stick or "
                        "a D-pad.");
    lxb_page_gap(page);

    if (lxb_page_button(page, "Activate")) {
        printf("Activated %d times\n", ++state->activated);
    }
    if (lxb_page_button(page, "Leave")) {
        lxb_page_quit(page);
    }
}

int main(void)
{
    struct state state = { 0 };
    lxb_app *app = lxb_app_new("{{APP_ID}}", "{{TITLE}}");
    int code = lxb_app_run(app, page, &state);
    if (code != 0) {
        fprintf(stderr, "%s\n", lxb_app_trouble(app));
    }
    lxb_app_free(app);
    return code;
}
