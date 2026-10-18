#ifndef LXB_APP_H
#define LXB_APP_H

#include <lxb_toolkit.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct lxb_app lxb_app;
typedef struct lxb_page lxb_page;

#define LXB_PRESS_RESTING 0ul
#define LXB_PRESS_FOCUSED 1ul
#define LXB_PRESS_PRESSED 2ul
#define LXB_PRESS_GOING(hundredths) (3ul + (unsigned long)(hundredths))

#define LXB_ALIGN_LEFT 0ul
#define LXB_ALIGN_CENTRE 1ul
#define LXB_ALIGN_RIGHT 2ul

#define LXB_FIT_CONTAIN 0ul
#define LXB_FIT_COVER 1ul

typedef void (*lxb_page_fn)(lxb_page *page, void *data);

lxb_app *lxb_app_new(const char *app_id, const char *title);

void lxb_app_size(lxb_app *app, double width, double height);

void lxb_app_plain(lxb_app *app);

void lxb_app_driven(lxb_app *app);

/* Count the session as low-end hardware whatever the shell's Settings say:
   the window is drawn as it is there, every light standing on what is chosen
   at once, and lxb_page_low_end answers 1. A picture of it is drawn so too,
   every frame at the one instant it is given. */
void lxb_app_low_end(lxb_app *app);

/* A mark of the application's own: `length` bytes of an SVG drawing of a
   shape, white where the mark is and cut through where it is open, saying
   lxb:shape, as every mark of the toolkit is drawn. From then on it is drawn
   by `name` wherever a mark of the toolkit's is (lxb_draw_icon,
   lxb_page_icon, lxb_page_head), in the same material and lit by the same
   lamp. Answers 1, or 0 with the reason in lxb_app_trouble: a drawing that
   does not say it is a shape, does not parse or has nothing in it, a name
   the toolkit already draws a mark by, or more marks than there is room for
   (64). The same name given again takes the new drawing. Before
   lxb_app_run or lxb_app_shot. */
int lxb_app_mark(lxb_app *app, const char *name, const void *svg,
                 unsigned long length);

int lxb_app_run(lxb_app *app, lxb_page_fn page, void *data);

int lxb_app_shot(lxb_app *app, const char *path, unsigned int width,
                 unsigned int height, float seconds, lxb_page_fn page,
                 void *data);

/* A picture of the window `frames` frames after its first, each drawn a
   sixtieth of a second after the one before, with a menu unfolding, a
   question coming up or the page's own motion caught where it has got to.
   The page is called once a frame; the last frame is the one written. */
int lxb_app_shot_after(lxb_app *app, const char *path, unsigned int width,
                       unsigned int height, float seconds, unsigned int frames,
                       lxb_page_fn page, void *data);

const char *lxb_app_trouble(const lxb_app *app);

void lxb_app_free(lxb_app *app);

float lxb_page_width(const lxb_page *page);
float lxb_page_height(const lxb_page *page);

float lxb_page_seconds(const lxb_page *page);

int lxb_page_action(lxb_page *page);

/* Hand the window an action, one of lxb_toolkit.h's LXB_ACTION_ numbers
   (those lxb_page_action answers with and lxb_action_name names), as a key
   or a controller would: a question, a menu or the file chooser over the
   page takes it, a driven page reads it with lxb_page_action, and otherwise
   it walks or presses the light. What a picture of something being answered
   presses with. A number past the list does nothing, and so does
   lxb_page_action's -1 for nothing, handed straight back. */
void lxb_page_act(lxb_page *page, unsigned long action);

/* How far a wheel or touchpad moved over the spot with this id, counted in
   directions and signed downwards. The directions are already waiting in
   lxb_page_action; this only says which of the page's lists they meant. */
int lxb_page_scrolled(const lxb_page *page, unsigned int id);

void lxb_page_focus(lxb_page *page, unsigned long index);

unsigned long lxb_page_focused(const lxb_page *page);

void lxb_page_glide(lxb_page *page, const float *rect, float strength,
                    float *out);

void lxb_page_place(lxb_page *page, const float *rect, float strength,
                    float *out);

/* The page's light carried to `rect` as lxb_page_glide carries it, or put
   there at once as lxb_page_place puts it where `appears` is 1, and drawn in
   the rectangle's own shape, its corners rounded by `radius` rather than a
   capsule's: a card or a picture lit as itself. The one light the capsule is,
   so it travels between the two. Writes where it is drawn this frame to out,
   which may be NULL, and nothing without a page or a rectangle. */
void lxb_page_light_shaped(lxb_page *page, const float *rect, float radius,
                           float strength, int appears, float *out);

unsigned long lxb_page_press(const lxb_page *page, int lit);

int lxb_page_pressed(lxb_page *page, unsigned int id);

/* Where the pointer is while a press that took hold of this control is still
   held down. Writes x and y to out and answers 1, or answers 0 and writes
   nothing. Asking is what makes the control a bar: a press on one asked about
   on the frame it landed on is held, and is a drag rather than a hover until
   the button comes up; a press held on any other control goes on hovering.
   Ask on every frame the bar is drawn. Only a pointer drags: a finger on the
   same control moves the list. */
int lxb_page_dragging(lxb_page *page, unsigned int id, float *out);

/* Where the pointer rests over the page. Writes x and y to out and answers 1,
   or answers 0 and writes nothing: the pointer has left the window, a menu, a
   dialog or the file chooser stands over the page, a key, a pad or a finger
   has been reached for since it last moved, or a press that took hold of a
   bar is held down, which is a drag (lxb_page_dragging) and not a hover. */
int lxb_page_pointer(const lxb_page *page, float *out);

/* Whether the pointer moved over the page since the last frame: the only time
   hovering may move a light, so one the pad moved stays put under a pointer
   that rests. */
int lxb_page_pointer_moved(const lxb_page *page);

/* The control the pointer moved onto since the last frame, by the number it
   was drawn with, or -1: on a frame it did not move, and over nothing of the
   page's own. Read off the frame it was moving over, so it can be asked
   before anything is drawn. */
int lxb_page_hovered(const lxb_page *page);

/* A field takes text this frame (taking 1), standing at `rect`, or where it
   was said last with NULL: letters and Backspace arrive through lxb_page_typed instead of as
   actions, the desktop's text input is told of the field, and LineXinBar
   raises its on-screen keyboard over it with a controller in hand. Said on
   every frame the field takes text; 0 takes back what was said earlier in
   the frame. */
void lxb_page_taking_text(lxb_page *page, int taking, const float *rect);

/* What was typed since the last frame, in order: its letters as UTF-8, and
   U+0008 for each Backspace. NULL for nothing; the caller owns the rest and
   frees it with lxb_app_string_free. */
char *lxb_page_typed(lxb_page *page);

/* Where LineXinBar's on-screen keyboard stands over the window while it is
   up: writes x, y, width and height to out and answers 1, or answers 0 and
   writes nothing. Stand the field being typed into above its top. */
int lxb_page_board(const lxb_page *page, float *out);

void lxb_page_quit(lxb_page *page);

/* Keep the screen lit, the machine awake, or both, for as long as this is
   said with a non-zero flag; say it with zeros to let go. A film playing holds
   both, music holds only sleep. The desktop is asked once, when it changes. */
void lxb_page_keep_awake(lxb_page *page, int screen, int sleep);

/* Draw the next frame no later than this many seconds from now. Only low-end
   hardware mode waits between frames at all; a page with something running
   that the frame itself notices, such as a song ending, says so here. */
void lxb_page_redraw_within(lxb_page *page, float seconds);

/* Whether the toolkit counts this session as low-end hardware: 1 or 0. The
   shell's low-end mode where Settings has said, and otherwise whether the
   window is drawn in software. A page that moves things itself puts them
   where they are going at once where this is 1. Decided as the window opens,
   and 1 for an application that asked for lxb_app_low_end; a picture
   (lxb_app_shot) answers 0 unless it was asked for of such an application. */
int lxb_page_low_end(const lxb_page *page);

void lxb_page_play(lxb_page *page, unsigned long sound);

void lxb_page_volume(lxb_page *page, float value, int muted);

void lxb_page_cursor(const lxb_page *page, float *out);

void lxb_page_set_cursor(lxb_page *page, const float *rect);

void lxb_page_title(lxb_page *page, const char *text);

void lxb_page_heading(lxb_page *page, const char *text);

void lxb_page_text(lxb_page *page, const char *text);

void lxb_page_note(lxb_page *page, const char *text);

void lxb_page_icon(lxb_page *page, const char *name);

void lxb_page_head(lxb_page *page, const char *icon, const char *title);

void lxb_page_rule(lxb_page *page);

void lxb_page_gap(lxb_page *page);

int lxb_page_button(lxb_page *page, const char *label);

int lxb_page_row(lxb_page *page, const char *label);

int lxb_page_item(lxb_page *page, const char *label);

int lxb_page_row_value(lxb_page *page, const char *label, const char *value);

/* Where the light is standing, for a page that draws its own controls: four
   floats, x, y, width, height. A menu grows out of it. */
void lxb_page_light_at(lxb_page *page, const float *rect);

void lxb_page_menu(lxb_page *page, const char *title,
                   const char *const *commands, unsigned long count);

/* The same menu, with the one command already in force wearing the chosen
   mark, and the light on it as the menu opens. `marked` indexes `commands`;
   anything past the end marks nothing. */
void lxb_page_menu_marked(lxb_page *page, const char *title,
                          const char *const *commands, unsigned long count,
                          unsigned long marked);

/* The same menu of acts, marking nothing, with the light on the command at
   `row` as it opens: what a menu a person comes back to does, the light on
   the row they went in by. A row past the end is the first. */
void lxb_page_menu_on(lxb_page *page, const char *title,
                      const char *const *commands, unsigned long count,
                      unsigned long row);

int lxb_page_chose(lxb_page *page);

void lxb_page_ask(lxb_page *page, const char *title, const char *body,
                  const char *const *answers, unsigned long count);

int lxb_page_answered(lxb_page *page);

int lxb_page_pick(lxb_page *page, unsigned long selection,
                  const char *directory);

int lxb_page_pick_many(lxb_page *page, unsigned long selection,
                       const char *directory);

int lxb_page_save(lxb_page *page, const char *name, const char *directory);

char *lxb_page_picked(lxb_page *page);

char *lxb_page_picked_next(lxb_page *page);

void lxb_app_string_free(char *text);

void lxb_draw_pane(lxb_page *page, const float *rect, unsigned long overlay);

void lxb_draw_card(lxb_page *page, const float *rect, unsigned long surface,
                   unsigned long role, float alpha);

void lxb_draw_label(lxb_page *page, const float *rect, unsigned long text,
                    const char *string, unsigned long role, unsigned long align);

float lxb_draw_paragraph(lxb_page *page, const float *rect, const char *string,
                         unsigned long role);

void lxb_draw_icon(lxb_page *page, const float *rect, const char *name);

/* A picture from a file on this disk (PNG, JPEG, GIF, WebP, BMP, TIFF or
   SVG, an SVG known by its .svg name) in `rect`, fitted as LXB_FIT_CONTAIN or LXB_FIT_COVER says, its corners
   rounded by `radius`, at `alpha`. Answers 1 once it is drawn, and 0 while it
   is still being read, when it cannot be read, or with no page, no path or a
   rectangle without area. Read on a worker thread the first time it is asked for, so ask on
   every frame until it answers 1; a picture of the window (lxb_app_shot)
   reads it at once. */
int lxb_draw_picture(lxb_page *page, const float *rect, float radius,
                     const char *path, unsigned long fit, float alpha);

/* The chosen mark, the tick cut through a bead that says a value is in
   force, in the square `rect` at `alpha`, ringed in the panel's dark glass so
   it is read on a lit control as well as on a resting one. */
void lxb_draw_chosen(lxb_page *page, const float *rect, float alpha);

/* A switch standing in `rect`, on (1) or off (0): a capsule of dark glass,
   the accent's track in it when on, and a knob at the end it is switched to,
   all of the marks' material. `ink` is how strongly it is drawn, 1 on a
   control that can be used. lxb_switch_width says how long one is. */
void lxb_draw_switch(lxb_page *page, const float *rect, int on, float ink);

/* How long a switch `height` tall is drawn. */
float lxb_switch_width(float height);

void lxb_draw_rule(lxb_page *page, const float *rect, unsigned long role);

void lxb_draw_button(lxb_page *page, const float *rect, const char *label,
                     unsigned long press);

void lxb_draw_row(lxb_page *page, const float *rect, const char *name,
                  const char *value, unsigned long press);

void lxb_draw_selection(lxb_page *page, const float *rect, float strength);

void lxb_draw_spot(lxb_page *page, unsigned int id, const float *rect);

/* Blur and fade the top and bottom edges of a scrolling area, after
   everything inside it has been drawn. band is the feather in points; top and
   bottom are how strongly each edge is there. */
void lxb_draw_soft_edges(lxb_page *page, const float *rect, float band,
                         float top, float bottom);

/* The same at the left and right edges of an area: what is carried sideways
   across a pane dissolves into it there. band is the feather in points; left
   and right are how strongly each edge is there. */
void lxb_draw_soft_side_edges(lxb_page *page, const float *rect, float band,
                              float left, float right);

int lxb_page_at(const lxb_page *page, float x, float y);

float lxb_page_metric(const lxb_page *page, unsigned long metric);

float lxb_page_scaled(const lxb_page *page, float reference);

float lxb_page_line(const lxb_page *page, unsigned long text);

float lxb_page_measure(lxb_page *page, unsigned long text, const char *string);

#ifdef __cplusplus
}
#endif

#endif
