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

typedef void (*lxb_page_fn)(lxb_page *page, void *data);

lxb_app *lxb_app_new(const char *app_id, const char *title);

void lxb_app_size(lxb_app *app, double width, double height);

void lxb_app_plain(lxb_app *app);

void lxb_app_driven(lxb_app *app);

int lxb_app_run(lxb_app *app, lxb_page_fn page, void *data);

int lxb_app_shot(lxb_app *app, const char *path, unsigned int width,
                 unsigned int height, float seconds, lxb_page_fn page,
                 void *data);

const char *lxb_app_trouble(const lxb_app *app);

void lxb_app_free(lxb_app *app);

float lxb_page_width(const lxb_page *page);
float lxb_page_height(const lxb_page *page);

float lxb_page_seconds(const lxb_page *page);

int lxb_page_action(lxb_page *page);

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

unsigned long lxb_page_press(const lxb_page *page, int lit);

int lxb_page_pressed(lxb_page *page, unsigned int id);

/* Where the pointer is while a press that began on this control is still held
   down. Writes x and y to out and answers 1, or answers 0 and writes nothing.
   Only a pointer drags: a finger on the same control moves the list. */
int lxb_page_dragging(const lxb_page *page, unsigned int id, float *out);

void lxb_page_quit(lxb_page *page);

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
   mark. `marked` indexes `commands`; anything past the end marks nothing. */
void lxb_page_menu_marked(lxb_page *page, const char *title,
                          const char *const *commands, unsigned long count,
                          unsigned long marked);

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

int lxb_page_at(const lxb_page *page, float x, float y);

float lxb_page_metric(const lxb_page *page, unsigned long metric);

float lxb_page_scaled(const lxb_page *page, float reference);

float lxb_page_line(const lxb_page *page, unsigned long text);

float lxb_page_measure(lxb_page *page, unsigned long text, const char *string);

#ifdef __cplusplus
}
#endif

#endif
