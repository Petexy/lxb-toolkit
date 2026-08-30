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
