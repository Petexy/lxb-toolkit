#ifndef LXB_TOOLKIT_H
#define LXB_TOOLKIT_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    float r, g, b, a;
} lxb_rgba;

typedef struct {
    float depth;
    float frost;
    float gloss;
    float curve;
} lxb_glass;

typedef struct {
    float radius;
    lxb_glass glass;
    float stain;
    float light_inset;
    float header_x, header_width;
    float header_height, header_height_share;
    float header_light;
    float foot_x, foot_width;
    float foot_height, foot_height_share;
    float foot_light;
    float rim;

    float rim_width;

    unsigned long stain_role, header_role, foot_role, rim_role;
} lxb_overlay_material;

typedef struct {
    float width;
    float extra_width;
    float row;
    float stacked_row;
    float title;
    float title_size;
    float label_size;
    float detail_size;
    float stamp_size;
    float group_gap;
    float gap;
    float glow_reach;
    float scroll_strip;
    float scroll_arrow;
    float dim;
    float scrim;
    float depth;
    float content_in;
    float panel_in;
    float icon;
    float glyph;
    float aside;
    float aside_gap;
    float aside_glyph;
    unsigned int max_lines;
} lxb_menu;

typedef struct {
    unsigned int chip_role;
    float chip_tint;
    float chip_gloss;
    float aside_tint;
    float padding;
    unsigned int lit_role;
    float lit;
    float lit_answer;
    float lit_pulse;
    float glow;
    float glow_pulse;
    float glow_width;
    float glow_height;
    unsigned int out_role;
    float out;
    float out_width;
    float ink;
    float ink_quiet;
    float mark;
    float mark_quiet;
} lxb_control_shape;

typedef struct {
    const unsigned char *data;
    unsigned long len;
} lxb_bytes;

typedef struct lxb_picker lxb_picker;

typedef enum {
    LXB_ICON_STYLE_DEFAULT = 0,
    LXB_ICON_STYLE_SIMPLE = 1,
} lxb_icon_style;

typedef enum {
    LXB_WALLPAPER_STYLE_DEFAULT = 0,
    LXB_WALLPAPER_STYLE_SIMPLE = 1,
    LXB_WALLPAPER_STYLE_CUSTOM = 2,
} lxb_wallpaper_style;

typedef enum {
    LXB_OVERLAY_CONTEXT_MENU = 0,
    LXB_OVERLAY_DIALOG = 1,
} lxb_overlay;

enum {
    LXB_ROLE_ACCENT = 0,
    LXB_ROLE_ACCENT_SOFT = 1,
    LXB_ROLE_ACCENT_DEEP = 2,
    LXB_ROLE_GLASS = 3,
    LXB_ROLE_GLASS_RAISED = 4,
    LXB_ROLE_RIM = 5,
    LXB_ROLE_TEXT = 6,
    LXB_ROLE_TEXT_SOFT = 7,
    LXB_ROLE_DANGER = 8,
    LXB_ROLE_GLOW = 9,
    LXB_ROLE_SKY_TOP = 10,
    LXB_ROLE_SKY_BOTTOM = 11,
    LXB_ROLE_SKY_TOP_ALT = 12,
    LXB_ROLE_SKY_BOTTOM_ALT = 13,
};

enum {
    LXB_METRIC_CARD_RADIUS = 0,
    LXB_METRIC_PANEL_RADIUS = 1,
    LXB_METRIC_PANEL_INSET = 2,
    LXB_METRIC_ROW_HEIGHT = 3,
    LXB_METRIC_ROW_PADDING = 4,
    LXB_METRIC_PANEL_PADDING = 5,
    LXB_METRIC_TILE = 6,
    LXB_METRIC_GAP = 7,
    LXB_METRIC_TILE_GLYPH = 8,
    LXB_METRIC_TILE_RADIUS = 9,
    LXB_METRIC_ITEM_SPACING = 10,
    LXB_METRIC_COLUMN_SPACING = 11,
    LXB_METRIC_ITEM_ICON = 12,
    LXB_METRIC_ITEM_ICON_FOCUSED = 13,
    LXB_METRIC_COLUMN_ICON = 14,
    LXB_METRIC_COLUMN_ICON_FOCUSED = 15,
    LXB_METRIC_MENU_WIDTH = 16,
    LXB_METRIC_DIALOG_WIDTH = 17,
    LXB_METRIC_DIALOG_DIM = 18,
    LXB_METRIC_POWER_WIDTH = 19,
    LXB_METRIC_POWER_DIM = 20,
};

enum {
    LXB_TEXT_DISPLAY = 0,
    LXB_TEXT_TITLE = 1,
    LXB_TEXT_BODY = 2,
    LXB_TEXT_LABEL = 3,
    LXB_TEXT_CAPTION = 4,
};

enum {
    LXB_SURFACE_PANEL = 0,
    LXB_SURFACE_CONTROL = 1,
    LXB_SURFACE_SIDEBAR = 2,
};

typedef struct {
    unsigned long accent;
    int wallpaper;
    int icons;
} lxb_shell_theme;

typedef struct lxb_accent lxb_accent;

typedef struct {
    int stacked;
    int stamp;
    int aside;
    int reading;
    unsigned int group;
    unsigned int lines;
    unsigned int detail_lines;
} lxb_menu_row;

typedef struct lxb_menu_layout lxb_menu_layout;

typedef struct {
    unsigned char *pixels;
    unsigned int width;
    unsigned int height;
    unsigned long stride;
} lxb_canvas;

typedef struct {
    float time;
    float soften;
    unsigned long style;
    lxb_rgba sky[4];
    lxb_rgba accent[3];
    lxb_rgba glow;
} lxb_scene;

typedef struct {
    float x;
    float y;
    float width;
    float height;
    float radius;
    float power;
    lxb_glass glass;
    lxb_rgba tint;
    float opacity;
} lxb_pane;

typedef struct {
    float x;
    float y;
    float width;
    float height;
    lxb_rgba color;
    lxb_rgba accent_soft;
    float gloss;
    int simple;
} lxb_mark;

unsigned long lxb_palette_count(void);
const char *lxb_palette_name(unsigned long index);

int lxb_palette_index(const char *name);

unsigned long lxb_role_count(void);
const char *lxb_role_name(unsigned long role);

unsigned int lxb_palette_color(unsigned long palette, unsigned long role);

lxb_rgba lxb_palette_rgba(unsigned long palette, unsigned long role, float alpha);

lxb_rgba lxb_srgb_to_linear(unsigned int hex, float alpha);
unsigned int lxb_linear_to_srgb(lxb_rgba color);

lxb_accent *lxb_accent_new(const char *name);
void lxb_accent_free(lxb_accent *accent);

int lxb_accent_preview(lxb_accent *accent, const char *name);
int lxb_accent_commit(lxb_accent *accent, const char *name);
int lxb_accent_set(lxb_accent *accent, const char *name);
void lxb_accent_restore(lxb_accent *accent);

int lxb_accent_advance(lxb_accent *accent, float dt);
lxb_rgba lxb_accent_color(lxb_accent *accent, unsigned long role, float alpha);

const char *lxb_accent_applied(lxb_accent *accent);

unsigned long lxb_icon_style_count(void);
const char *lxb_icon_style_name(unsigned long index);
unsigned long lxb_wallpaper_style_count(void);
const char *lxb_wallpaper_style_name(unsigned long index);

lxb_shell_theme lxb_shell_theme_load(void);

lxb_menu lxb_context_menu(void);

unsigned long lxb_menu_rows_that_fit(float height, float row);

void lxb_menu_growing(const float *anchor, const float *panel, float travelled,
                      float *out);

float lxb_menu_shown(float travelled, int opening);

float lxb_menu_content_shown(float travelled, int opening);

lxb_menu_layout *lxb_menu_layout_new(const lxb_menu_row *rows, unsigned long count,
                                     unsigned int title_lines,
                                     const float *anchor, const float *display,
                                     unsigned long first, unsigned long visible,
                                     unsigned long selected, float unfolded,
                                     float extra);
void lxb_menu_layout_free(lxb_menu_layout *layout);

unsigned long lxb_menu_rows_of_that_fit(const lxb_menu_row *rows, unsigned long count,
                                        unsigned int title_lines, float height);

void lxb_menu_panel(const lxb_menu_layout *layout, float *out);
float lxb_menu_rows_top(const lxb_menu_layout *layout);
int lxb_menu_row_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
int lxb_menu_chip_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
int lxb_menu_aside_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
int lxb_menu_highlight_rect(const lxb_menu_layout *layout, int on_aside, float *out);
int lxb_menu_separator(const lxb_menu_layout *layout, unsigned long index, float *out);

void lxb_menu_opened(const lxb_menu_layout *layout, unsigned long index, float *out);

float lxb_menu_chip_radius(const lxb_menu_row *row, float drawn, float height);
float lxb_menu_aside_radius(const lxb_menu_row *row, float height);
unsigned int lxb_menu_lines_in(float room, float line);
float lxb_menu_title_height(unsigned int lines);
float lxb_menu_title_growth(unsigned int lines);

float lxb_menu_margin(void);
float lxb_menu_label_padding(void);
float lxb_menu_row_padding(void);
float lxb_menu_label_line(void);
float lxb_menu_detail_line(void);
float lxb_menu_stamp_room(void);

lxb_control_shape lxb_control(void);

void lxb_control_glow_rect(const float *rect, float over, float tall,
                           float *out);

float lxb_control_arrival(const float *light, const float *rect);

float lxb_press_scale(float t);

float lxb_fill_arrival(float t);

float lxb_pulse(float seconds);

void lxb_pressed(const float *rect, float t, float *out);

void lxb_glide(float *position, float *velocity, float target, float dt);

float lxb_ease(float t);
float lxb_smoothstep(float t);

void lxb_spring(double *position, double *velocity, double target, double rate,
                double dt);
double lxb_card_spring(void);

unsigned long lxb_duration_count(void);
const char *lxb_duration_name(unsigned long index);
float lxb_duration(unsigned long index);
float lxb_duration_named(const char *name);

float lxb_reference_height(void);

float lxb_scale_for(float height);

float lxb_capsule_radius(float height);

unsigned long lxb_metric_count(void);
const char *lxb_metric_name(unsigned long index);

float lxb_metric(unsigned long index, float height);
int lxb_metric_is_share(unsigned long index);

unsigned long lxb_text_count(void);
const char *lxb_text_name(unsigned long index);
float lxb_text_size(unsigned long index, float height);
int lxb_text_is_bold(unsigned long index);
float lxb_text_line_height(void);

unsigned long lxb_surface_count(void);
const char *lxb_surface_name(unsigned long index);

lxb_glass lxb_surface_glass(unsigned long index);

unsigned long lxb_overlay_count(void);
const char *lxb_overlay_name(unsigned long index);
lxb_overlay_material lxb_overlay_material_for(unsigned long index);

void lxb_key_light(float *out);
float lxb_glass_ior(void);

lxb_bytes lxb_glass_wgsl(void);

lxb_bytes lxb_wallpaper_wgsl(void);

enum {
    LXB_PICKER_FILE = 0,
    LXB_PICKER_IMAGE = 1,
    LXB_PICKER_SCENERY = 2,
    LXB_PICKER_FOLDER = 3,
};

enum {
    LXB_PICKER_ENTRY_FOLDER = 0,
    LXB_PICKER_ENTRY_FILE = 1,
};

lxb_picker *lxb_picker_new(unsigned long selection, const char *directory);

void lxb_picker_free(lxb_picker *picker);

const char *lxb_picker_location(const lxb_picker *picker);
const char *lxb_picker_note(const lxb_picker *picker);
const char *lxb_picker_query(const lxb_picker *picker);
unsigned long lxb_picker_selection(const lxb_picker *picker);

int lxb_picker_can_search(const lxb_picker *picker);

int lxb_picker_can_choose(const lxb_picker *picker);

unsigned long lxb_picker_entry_count(const lxb_picker *picker);
const char *lxb_picker_entry_name(const lxb_picker *picker, unsigned long index);
const char *lxb_picker_entry_path(const lxb_picker *picker, unsigned long index);
int lxb_picker_entry_kind(const lxb_picker *picker, unsigned long index);

int lxb_picker_selected(const lxb_picker *picker);
int lxb_picker_select(lxb_picker *picker, unsigned long index);
int lxb_picker_move(lxb_picker *picker, int delta);

int lxb_picker_enter(lxb_picker *picker);
int lxb_picker_leave(lxb_picker *picker);

void lxb_picker_refresh(lxb_picker *picker);
void lxb_picker_search(lxb_picker *picker, const char *query);

const char *lxb_picker_choose(lxb_picker *picker);

unsigned long lxb_glyph_count(void);
const char *lxb_glyph_name(unsigned long index);

lxb_bytes lxb_glyph(const char *name);

unsigned int lxb_glyph_box(const char *name);

int lxb_glyph_sdf(const unsigned char *coverage, unsigned long coverage_len,
                  unsigned int size, unsigned char *out_alpha,
                  unsigned long out_len);

lxb_bytes lxb_glyph_wgsl(void);
float lxb_glyph_sdf_range(void);
unsigned int lxb_glyph_sdf_supersample(void);
unsigned int lxb_glyph_cell(void);
float lxb_glyph_depth_share(void);
void lxb_glyph_lamp(float *out);
float lxb_glyph_shadow(void);
float lxb_glyph_simple_tint(void);
float lxb_glyph_simple_alpha(void);
float lxb_glyph_simple_stain(void);

enum lxb_sound {
    LXB_SOUND_MOVE = 0,
    LXB_SOUND_PRESS = 1,
    LXB_SOUND_GUIDE_MOVE = 2,
    LXB_SOUND_GUIDE_PRESS = 3,
    LXB_SOUND_BACK = 4,
    LXB_SOUND_KEY = 5,
    LXB_SOUND_LAUNCH = 6,
    LXB_SOUND_GUIDE_OPEN = 7,
    LXB_SOUND_SHUTTER = 8,
    LXB_SOUND_AUTHENTICATE = 9,
    LXB_SOUND_NOTIFY = 10,
    LXB_SOUND_TRASH = 11,
    LXB_SOUND_ERROR = 12,
    LXB_SOUND_MUSIC = 13,
};

unsigned long lxb_sound_count(void);
const char *lxb_sound_name(unsigned long index);

lxb_bytes lxb_sound(const char *name);

int lxb_sound_used(const char *name);

float lxb_sound_rest(void);

float lxb_sound_amplitude(float value);

float lxb_sound_fade(float elapsed, float over);

float lxb_sound_music_fade_in(void);
float lxb_sound_music_fade_out(void);

float lxb_sound_retry_after(void);

enum {
    LXB_ACTION_LEFT = 0,
    LXB_ACTION_RIGHT = 1,
    LXB_ACTION_UP = 2,
    LXB_ACTION_DOWN = 3,

    LXB_ACTION_ACCEPT = 4,
    LXB_ACTION_BACK = 5,
    LXB_ACTION_MENU = 6,
    LXB_ACTION_SUBMIT = 7,
    LXB_ACTION_PREVIOUS = 8,
    LXB_ACTION_NEXT = 9,
};

enum {
    LXB_KEY_LEFT = 0,
    LXB_KEY_RIGHT = 1,
    LXB_KEY_UP = 2,
    LXB_KEY_DOWN = 3,
    LXB_KEY_ENTER = 4,
    LXB_KEY_SPACE = 5,
    LXB_KEY_ESCAPE = 6,
    LXB_KEY_BACKSPACE = 7,
    LXB_KEY_TAB = 8,
    LXB_KEY_BACKTAB = 9,
    LXB_KEY_MENU = 10,
    LXB_KEY_F10 = 11,
};

enum {
    LXB_BUTTON_SOUTH = 0,
    LXB_BUTTON_EAST = 1,
    LXB_BUTTON_NORTH = 2,
    LXB_BUTTON_WEST = 3,
    LXB_BUTTON_START = 4,
    LXB_BUTTON_SELECT = 5,
    LXB_BUTTON_LEFT_BUMPER = 6,
    LXB_BUTTON_RIGHT_BUMPER = 7,

    LXB_BUTTON_GUIDE = 8,
    LXB_BUTTON_DPAD_LEFT = 9,
    LXB_BUTTON_DPAD_RIGHT = 10,
    LXB_BUTTON_DPAD_UP = 11,
    LXB_BUTTON_DPAD_DOWN = 12,
};

unsigned long lxb_action_count(void);
const char *lxb_action_name(unsigned long index);

int lxb_action_named(const char *name);

int lxb_action_repeats(int action);

int lxb_action_of_key(int key);

int lxb_action_of_letter(unsigned int codepoint);

int lxb_action_of_button(int button);

float lxb_input_poll_interval(void);

float lxb_input_initial_repeat(void);
float lxb_input_repeat_interval(void);

float lxb_input_stick_engage(void);
float lxb_input_stick_release(void);

float lxb_input_scroll_step(void);

float lxb_input_tap_slop(void);

typedef struct lxb_repeat lxb_repeat;
lxb_repeat *lxb_repeat_new(void);
void lxb_repeat_free(lxb_repeat *repeat);

void lxb_repeat_reset(lxb_repeat *repeat);

unsigned long lxb_repeat_update(lxb_repeat *repeat, float now, const int *pressed,
                                float x, float y, int *out, unsigned long capacity);

int lxb_wheel_notches(float *carried, float notches);

lxb_bytes lxb_font(int bold);

lxb_bytes lxb_font_for(const char *script, int bold);

const char *lxb_version(void);

lxb_scene lxb_scene_for(unsigned long palette, float time);

lxb_scene lxb_accent_scene(lxb_accent *accent, float time);

int lxb_paint_wallpaper(const lxb_canvas *canvas, const lxb_scene *scene);

int lxb_paint_glass(const lxb_canvas *canvas, const lxb_pane *pane);

int lxb_paint_light(const lxb_canvas *canvas, const float *rect, lxb_rgba tint);

int lxb_paint_scrim(const lxb_canvas *canvas, const float *rect, float blur,
                    lxb_rgba tint);

int lxb_paint_glyph(const lxb_canvas *canvas, const lxb_mark *mark,
                    const unsigned char *field, unsigned long field_len);

float lxb_wallpaper_soften(void);

char *lxb_stylesheet(const char *palette_name);
void lxb_string_free(char *text);

#ifdef __cplusplus
}
#endif

#endif
