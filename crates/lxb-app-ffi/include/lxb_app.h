/*
 * lxb_app.h — the window, the loop, the controls and the sounds.
 *
 * Everything an application in the LineXinBar design language has in common
 * with every other one, so that a program contains only what is on its own
 * page:
 *
 *     #include <lxb_app.h>
 *
 *     static void page(lxb_page *page, void *data)
 *     {
 *         lxb_page_head(page, "launch", "Hello, world");
 *         if (lxb_page_button(page, "Hello World!")) {
 *             puts("clicked");
 *         }
 *     }
 *
 *     int main(void)
 *     {
 *         lxb_app *app = lxb_app_new("com.example.HelloWorld", "Hello World");
 *         int code = lxb_app_run(app, page, NULL);
 *         lxb_app_free(app);
 *         return code;
 *     }
 *
 * That is a complete application: a Wayland window carrying the shell's
 * wallpaper and glass, driven by a keyboard, a pointer and every controller
 * plugged into the machine, answering with the shell's own sounds. Nothing in
 * it names a colour, a radius, a duration or a device.
 *
 * Link against -llxb_app. It brings the whole stack with it — the renderer, a
 * window system, the controllers and an audio device — which is why it is a
 * separate library from liblxb_toolkit, whose whole point is that it has no
 * dependencies at all. A program that only wants the language's answers should
 * link that one and nothing else.
 *
 * ENUMERATIONS. Every one here is an index into liblxb_toolkit's own list of
 * the same values: lxb_role, lxb_text, lxb_metric, lxb_overlay, lxb_surface
 * and the recordings of lxb_sound_name. That is why this header includes
 * lxb_toolkit.h. An index nobody has is answered by the first value rather
 * than by a crash.
 *
 * RECTANGLES cross as four floats — x, y, width, height — read but never kept.
 * STRINGS cross as UTF-8, are copied out during the call, and are not kept
 * either: the buffer may be freed or reused the moment the call returns.
 *
 * THE PAGE POINTER handed to the draw callback is *that frame's* and no other.
 * It is a borrow of the renderer, not a handle to it. Storing it and using it
 * after the callback returns is the one thing here that is undefined rather
 * than merely wrong.
 */

#ifndef LXB_APP_H
#define LXB_APP_H

#include <lxb_toolkit.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct lxb_app lxb_app;
typedef struct lxb_page lxb_page;

/* How something that can be pressed is currently standing.
 *
 * lxb_press:
 *   0  resting — somewhere to look, not somewhere the user is
 *   1  focused — the light is on it
 *   2  pressed — held at the bottom of its travel
 *   3+ going   — 3 + n is n hundredths of the way through a press
 */
#define LXB_PRESS_RESTING 0ul
#define LXB_PRESS_FOCUSED 1ul
#define LXB_PRESS_PRESSED 2ul
#define LXB_PRESS_GOING(hundredths) (3ul + (unsigned long)(hundredths))

/* Which end of its box a line of writing is set at.
 *
 * lxb_align:
 *   0 left
 *   1 centre
 *   2 right
 */
#define LXB_ALIGN_LEFT 0ul
#define LXB_ALIGN_CENTRE 1ul
#define LXB_ALIGN_RIGHT 2ul

/* One frame, and whatever the program is keeping. */
typedef void (*lxb_page_fn)(lxb_page *page, void *data);

/*
 * The application itself.
 */

/* A new application. `app_id` is the stable name the desktop entry, the
 * executable and StartupWMClass all have to agree on; `title` is what a person
 * sees. Never null; free it with lxb_app_free. */
lxb_app *lxb_app_new(const char *app_id, const char *title);

/* How large the window asks to be, in logical pixels. A request rather than a
 * demand: the compositor decides, and everything drawn is measured from the
 * height it actually gets. */
void lxb_app_size(lxb_app *app, double width, double height);

/* Draw the whole window rather than the standard page. The pane inset from the
 * window's edges, and the flow inside it, are the common case and are drawn for
 * you; this is for a program that is a screen rather than a panel. */
void lxb_app_plain(lxb_app *app);

/* Move your own selection: the light is not walked between controls for you,
 * and every action arrives at the page through lxb_page_action.
 *
 * One list read down the screen is what an application usually is, and for
 * that the default is the whole of the work. An application that is a screen —
 * a column of pages beside the page they are about, a grid, a board — has more
 * than one axis, and the language's own answer is that Up and Down are the
 * outer list while Left and Right are within it. Nothing in the library can
 * know which is which, so this hands the actions over rather than guessing. */
void lxb_app_driven(lxb_app *app);

/* Open the window and run until it is closed. Zero if it ran; non-zero if it
 * could not, and then lxb_app_trouble says why. */
int lxb_app_run(lxb_app *app, lxb_page_fn page, void *data);

/* One settled frame, written to a PNG at `path`, with no display at all. Zero
 * if it was written; non-zero if it was not, and then lxb_app_trouble says why.
 *
 * The same page function, the same renderer and the same material as
 * lxb_app_run — there is no second drawing path here, which is the only way a
 * picture is worth anything as a check. `seconds` is the clock it is drawn at,
 * so a moving wallpaper can be caught at a chosen moment. */
int lxb_app_shot(lxb_app *app, const char *path, unsigned int width,
                 unsigned int height, float seconds, lxb_page_fn page,
                 void *data);

/* Why the window could not open, or null if it did. Owned by the application. */
const char *lxb_app_trouble(const lxb_app *app);

/* Give the application back. Null is answered by doing nothing. */
void lxb_app_free(lxb_app *app);

/*
 * What a frame is.
 */

float lxb_page_width(const lxb_page *page);
float lxb_page_height(const lxb_page *page);

/* Seconds since the window opened: the one clock everything with a movement of
 * its own is measured against. */
float lxb_page_seconds(const lxb_page *page);

/* The next thing the keyboard or a controller said since the last frame, as an
 * index into lxb_action, or -1 when there is nothing left. Read it in a loop;
 * nothing in it says which control sent it. Always -1 unless the application
 * asked for lxb_app_driven. */
int lxb_page_action(lxb_page *page);

/* Say which control the light is on, by the order it is drawn in. What a page
 * that moves its own selection does before it draws — see lxb_app_driven. */
void lxb_page_focus(lxb_page *page, unsigned long index);

/* Which control the light is on. The same number a left click reports; the
 * right button raises the context menu, as the Menu key does. */
unsigned long lxb_page_focused(const lxb_page *page);

/* The lit capsule that says where the light is, drawn travelling. `out` may be
 * null; otherwise it is filled with where the light actually is this frame,
 * which is not where it was asked to be until it arrives. The light is one
 * object crossing the page rather than a property each control has.
 *
 * For a page that draws its own controls. Draw it *before* the controls it is
 * behind, so their faces sit over it. */
void lxb_page_glide(lxb_page *page, const float *rect, float strength,
                    float *out);

/* The same, but the light appears where it is asked for rather than flying to
 * it: what a page does when the row the light was on is gone, because a light
 * that travelled there would be saying the two lists are one. */
void lxb_page_place(lxb_page *page, const float *rect, float strength,
                    float *out);

/* How to draw one of your own controls, as an lxb_press: resting, lit, or
 * part-way through a press. `lit` says whether the light is on this one. The
 * page keeps one press, because the light is in one place — a control the
 * light is not on is resting, however many are in flight elsewhere. The flow's
 * own controls do this for themselves; this is for a page that draws its own,
 * which is a page that asked for lxb_app_driven. */
unsigned long lxb_page_press(const lxb_page *page, int lit);

/* Whether a press landed on the spot written down under `id` with
 * lxb_draw_spot, answered once and by that spot alone. The pointer's half of a
 * page that draws its own controls: the numbers are the page's own, so keep
 * them clear of the numbered controls the flow draws, which are counted from
 * zero in the order they are drawn. Only a page that asked for lxb_app_driven
 * gets these. */
int lxb_page_pressed(lxb_page *page, unsigned int id);

/* Close the window at the end of this frame. */
void lxb_page_quit(lxb_page *page);

/* Play one of the language's recordings, by its index in lxb_sound_name's
 * list. The interface's own sounds are played for you; this is for a program
 * with something of its own to say. */
void lxb_page_play(lxb_page *page, unsigned long sound);

/* Turn the interface's sounds down, or off. A level rather than a switch,
 * because that is what a person has. `value` is 0 to 1 and is read as loudness
 * rather than as amplitude. */
void lxb_page_volume(lxb_page *page, float value, int muted);

/* Where the flow has got to: `out` is filled with x, y, width and height. */
void lxb_page_cursor(const lxb_page *page, float *out);

/* Put the flow somewhere else: it lays out inside `rect` from its top. */
void lxb_page_set_cursor(lxb_page *page, const float *rect);

/*
 * The flow. Each of these lays itself out down the page at the language's own
 * sizes and spacings, so a program that is a list of things never computes a
 * rectangle.
 *
 * Controls are numbered by the order they are drawn in, which is the order
 * they are read in — so the light moves between them in that order too, and no
 * program has to name, register or lay out its own focus.
 */

/* The page's own name, at the top of it. */
void lxb_page_title(lxb_page *page, const char *text);

/* A name for the group of things under it. */
void lxb_page_heading(lxb_page *page, const char *text);

/* A sentence, wrapped to the page's width however many lines that takes. */
void lxb_page_text(lxb_page *page, const char *text);

/* The same, quieter: a note about the thing above it rather than the thing
 * itself. */
void lxb_page_note(lxb_page *page, const char *text);

/* One of the shell's own marks, at the size a list uses. */
void lxb_page_icon(lxb_page *page, const char *name);

/* A mark and the page's name on one line, which is how a page in this language
 * introduces itself. */
void lxb_page_head(lxb_page *page, const char *icon, const char *title);

/* The barely-there hairline the shell groups with. A grouping, not a border. */
void lxb_page_rule(lxb_page *page);

/* One gap's worth of air, for a page that wants a break in it. */
void lxb_page_gap(lxb_page *page);

/* A button, as wide as its own label. Non-zero on the frame a press lands on
 * it, whether that press came from a key, a controller or a click. */
int lxb_page_button(lxb_page *page, const char *label);

/* A row: the whole width of the page, its label on the left. What a list of
 * things to choose between is made of. */
int lxb_page_row(lxb_page *page, const char *label);

/* One entry of a list: its label, and the lit capsule behind it when the light
 * is on it — and nothing at all when it is not.
 *
 * The other half of the pair with lxb_page_row, and the difference is what the
 * list is for. A row is a control, so it wears a chip whether or not anything
 * is on it: it says *this can be pressed*. An entry is one of many, and a
 * column of chips says nothing except that there are a lot of them. */
int lxb_page_item(lxb_page *page, const char *label);

/* The same, with what it is currently set to on the right of it — how this
 * language writes a setting: the name and the answer on one line, and pressing
 * it is how the answer is changed. */
int lxb_page_row_value(lxb_page *page, const char *label, const char *value);

/*
 * The panels.
 */

/* Raise a context menu over the control the light is on. `title` may be null
 * for a menu that is about nothing in particular. Nothing happens if one is
 * already up. */
void lxb_page_menu(lxb_page *page, const char *title,
                   const char *const *commands, unsigned long count);

/* Which command was pressed, on the frame it was pressed, or -1 for none. */
int lxb_page_chose(lxb_page *page);

/* Ask a question. Modal: while it is up it owns every control, which is what
 * makes it a question. Nothing happens if one is already up. */
void lxb_page_ask(lxb_page *page, const char *title, const char *body,
                  const char *const *answers, unsigned long count);

/* Which answer was given, on the frame it was given, or -1 for none. */
int lxb_page_answered(lxb_page *page);

/*
 * The renderer itself. Everything above lays itself out; everything here is
 * given a rectangle, for a page that is not a list.
 */

/* A sheet of the shell's glass. `overlay` indexes lxb_overlay. */
void lxb_draw_pane(lxb_page *page, const float *rect, unsigned long overlay);

/* One of the three cuts of glass, tinted by a role. */
void lxb_draw_card(lxb_page *page, const float *rect, unsigned long surface,
                   unsigned long role, float alpha);

/* One line of writing. */
void lxb_draw_label(lxb_page *page, const float *rect, unsigned long text,
                    const char *string, unsigned long role, unsigned long align);

/* A sentence, wrapped to the rectangle's width. Answers how tall it came out,
 * so whatever is under it can be placed. */
float lxb_draw_paragraph(lxb_page *page, const float *rect, const char *string,
                         unsigned long role);

/* One of the shell's own marks, shaded into a bead of water out of its own
 * shape. The style is the shell's current one. */
void lxb_draw_icon(lxb_page *page, const float *rect, const char *name);

/* The hairline this language groups with. */
void lxb_draw_rule(lxb_page *page, const float *rect, unsigned long role);

/* A button at a rectangle of your own choosing. For anything a person can move
 * onto, prefer lxb_page_button, which numbers it and answers the pointer. */
void lxb_draw_button(lxb_page *page, const float *rect, const char *label,
                     unsigned long press);

/* A row at a rectangle of your own choosing. `value` may be null. */
void lxb_draw_row(lxb_page *page, const float *rect, const char *name,
                  const char *value, unsigned long press);

/* The lit capsule that says where the light is, without a control under it. */
void lxb_draw_selection(lxb_page *page, const float *rect, float strength);

/* Write down where something you drew yourself went, so that a pointer over it
 * can be answered. `id` comes back from lxb_page_at. */
void lxb_draw_spot(lxb_page *page, unsigned int id, const float *rect);

/* What is at a point of the frame that is on screen: the id of the thing
 * there, or -1 for nothing. Read from the last frame drawn, because a pointer
 * event is about the picture the person could see. */
int lxb_page_at(const lxb_page *page, float x, float y);

/*
 * The measurements a page needs. Each is the language's own answer at this
 * window's height, which is the only thing any size here depends on.
 */

/* One of the language's named lengths. `metric` indexes lxb_metric. */
float lxb_page_metric(const lxb_page *page, unsigned long metric);

/* A length written against the 1080p reference, scaled to this window. */
float lxb_page_scaled(const lxb_page *page, float reference);

/* How tall one line of a size is, air included. `text` indexes lxb_text. */
float lxb_page_line(const lxb_page *page, unsigned long text);

/* How wide a string is at a size, in this window's pixels. Shaped by the same
 * engine that will draw it, so it is the width it will actually take. */
float lxb_page_measure(lxb_page *page, unsigned long text, const char *string);

#ifdef __cplusplus
}
#endif

#endif /* LXB_APP_H */
