/*
 * lxb_toolkit.h — the LineXinBar design language, for C.
 *
 * Colour roles, glass, motion, sizes, type, marks and sounds, from a library
 * with no dependencies of its own. Link against liblxb_toolkit.so, or the
 * static archive beside it.
 *
 *     #include <lxb_toolkit.h>
 *
 *     lxb_accent *accent = lxb_accent_new("Purple");
 *     lxb_accent_preview(accent, "Blue");
 *     while (lxb_accent_advance(accent, 1.0f / 60.0f)) { ... }
 *     lxb_rgba tint = lxb_accent_color(accent, 0, 1.0f);   // role 0 is accent
 *     lxb_accent_free(accent);
 *
 * Three rules hold everywhere:
 *
 *   - Every `const char *` and every `lxb_bytes` points into the library and
 *     lives as long as it does. Do not free them, do not write through them.
 *   - Every enumeration is an index with a matching `_count`, so you can walk
 *     one without a constant in this header going stale. An index past the end
 *     is answered with NULL or zero rather than undefined behaviour.
 *   - Only three things allocate: lxb_accent_new, lxb_menu_layout_new and
 *     lxb_stylesheet. Each has exactly one matching free, and all accept NULL.
 *
 * Colour comes in two forms and the difference matters. `lxb_palette_color`
 * gives the authored value, 0xRRGGBB in sRGB — what a swatch shows and what
 * goes in a config file. Everything ending `_rgba` or `_color` gives linear
 * light, which is what a renderer must have: an sRGB surface interprets what
 * you write as light, so a hex value handed over raw comes out about twice as
 * bright as it was picked.
 *
 * Sizes are written against a 1080-pixel-tall screen; pass the real height and
 * they come back scaled by height, never width, with the shell's 0.6..2.5
 * clamp.
 *
 * Copyright (C) Piotr Lewandowski. GPL-3.0-only.
 */

#ifndef LXB_TOOLKIT_H
#define LXB_TOOLKIT_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* A colour in linear light, with an alpha. */
typedef struct {
    float r, g, b, a;
} lxb_rgba;

/* A pane of glass: depth in reference pixels, then three shares of 0..1. */
typedef struct {
    float depth;  /* thickness, and how wide the rim is rounded over */
    float frost;  /* how much of what it transmits it scatters       */
    float gloss;  /* how strongly it takes the light                 */
    float curve;  /* how much its face bows; 0 for anything compact  */
} lxb_glass;

/*
 * The complete four-layer material shared by context menus and general
 * dialogs. Lengths are reference pixels. The x/width and height_share fields
 * are fractions of the pane; cap each light's scaled height by its share.
 */
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
    /* Scale this reference width, then keep at least one physical pixel. */
    float rim_width;
    /*
     * The palette roles the four layers are drawn in, as indices into the
     * role table: pass one to lxb_role_name, or to lxb_palette_rgba with a
     * palette. Without them a consumer would have to guess which colour
     * stains the pane, which is the one thing a design language is for.
     */
    unsigned long stain_role, header_role, foot_role, rim_role;
} lxb_overlay_material;

/*
 * The context menu's shape: what the overlay material is made of, drawn at the
 * right size. A panel in the right material at the wrong row height is not
 * this menu, it is a menu.
 *
 * Lengths are reference pixels against a 1080-tall screen. glow_reach, dim,
 * scrim, depth, content_in, icon, glyph, aside and aside_glyph are shares of
 * something and must not be scaled.
 */
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

/*
 * The control everything you can act on is cut from — a button, a list row, a
 * switch, a dialog's answer. Three things drawn in order: a halo, the lit
 * capsule that glides onto it, and the control's own chip over the top, which
 * fades out by lxb_control_arrival as the capsule reaches it. Filling the
 * chosen control with the accent instead gets the same still frame and none of
 * the movement.
 *
 * chip_role, lit_role and out_role are indices into the palette. padding and
 * out_width are reference pixels. Everything else is a share or an alpha and
 * must not be scaled.
 */
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

/* Borrowed bytes. Never freed; valid for the lifetime of the library. */
typedef struct {
    const unsigned char *data;  /* NULL if there is no such asset */
    unsigned long len;
} lxb_bytes;

/* The material used for the shell's own marks. */
typedef enum {
    LXB_ICON_STYLE_DEFAULT = 0,
    LXB_ICON_STYLE_SIMPLE = 1,
} lxb_icon_style;

/* What stands behind the shell. Custom is a user-supplied picture or film. */
typedef enum {
    LXB_WALLPAPER_STYLE_DEFAULT = 0,
    LXB_WALLPAPER_STYLE_SIMPLE = 1,
    LXB_WALLPAPER_STYLE_CUSTOM = 2,
} lxb_wallpaper_style;

/* Semantic uses of one shared layered material. */
typedef enum {
    LXB_OVERLAY_CONTEXT_MENU = 0,
    LXB_OVERLAY_DIALOG = 1,
} lxb_overlay;

/*
 * The four enumerations whose index a caller has to pass. Everything below
 * takes a role, a metric, a size or a surface as an `unsigned long`, and
 * without these a C program has to write the number and hope the order never
 * moves. It cannot: the library's own tests read this file and fail if it and
 * the name tables ever disagree.
 *
 * Constants rather than typedefs, because the parameters really are
 * `unsigned long` — and because a type named lxb_metric could not coexist
 * with the function of that name.
 */

/* lxb_role: the colour roles, in the order lxb_role_name reads. */
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

/* lxb_metric: the sizes, in the order lxb_metric_name reads. */
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

/* lxb_text: the type scale, in the order lxb_text_name reads. */
enum {
    LXB_TEXT_DISPLAY = 0,
    LXB_TEXT_TITLE = 1,
    LXB_TEXT_BODY = 2,
    LXB_TEXT_LABEL = 3,
    LXB_TEXT_CAPTION = 4,
};

/* lxb_surface: the three cuts, in the order lxb_surface_name reads. */
enum {
    LXB_SURFACE_PANEL = 0,
    LXB_SURFACE_CONTROL = 1,
    LXB_SURFACE_SIDEBAR = 2,
};

/* The shell choices an application uses to draw in the current theme. */
typedef struct {
    unsigned long accent;  /* index into the palette table */
    int wallpaper;         /* one of lxb_wallpaper_style    */
    int icons;             /* one of lxb_icon_style         */
} lxb_shell_theme;

/* A palette being walked, chosen or left. Opaque. */
typedef struct lxb_accent lxb_accent;

/*
 * What the panel needs to know about one row in order to measure it.
 *
 * The four flags are non-zero for yes: a second run under the label, a quieter
 * one above it, a button on the right-hand end, and a row put on the panel to
 * be read rather than pressed. `group` is which band it belongs to — a change
 * of band opens a gap and rules a hairline in it. `lines` and `detail_lines`
 * are how many lines each of its two runs takes once it has opened out under
 * the highlight; one, or nought, is a row that never opens.
 */
typedef struct {
    int stacked;
    int stamp;
    int aside;
    int reading;
    unsigned int group;
    unsigned int lines;
    unsigned int detail_lines;
} lxb_menu_row;

/* A settled panel: where it stands, and every rectangle on it. Opaque. */
typedef struct lxb_menu_layout lxb_menu_layout;


/*
 * A 32-bit surface you own, for the calls that draw this language's material
 * rather than describing it: words in the machine's own order, alpha in the
 * top byte, premultiplied, sRGB-encoded. That is cairo's ARGB32, Qt's
 * Format_ARGB32_Premultiplied and SDL's ARGB8888, so in practice it is the
 * buffer your painter already has. The stride is bytes per row, separate from
 * the width because every one of those pads its rows.
 */
typedef struct {
    unsigned char *pixels;
    unsigned int width;
    unsigned int height;
    unsigned long stride;
} lxb_canvas;

/*
 * The analytic scene, and everything constant across one picture of it.
 *
 * time is the wallpaper clock in seconds — the same value here and in the
 * shader is the same frame. soften is 0 for the picture itself and up to 1 for
 * the blurred, dimmed variant; lxb_wallpaper_soften() is the amount to use
 * when an interface is standing on it. style is one of lxb_wallpaper_style,
 * and a custom picture is not something arithmetic can produce, so it draws
 * the Default water. Colours are linear light in the order the shader reads
 * them: sky is top, bottom, alternate top, alternate bottom, and accent is
 * normal, soft, deep.
 */
typedef struct {
    float time;
    float soften;
    unsigned long style;
    lxb_rgba sky[4];
    lxb_rgba accent[3];
    lxb_rgba glow;
} lxb_scene;

/*
 * One pane of glass, in pixels of the canvas.
 *
 * power is 2 for a circle-cornered rectangle and 4 for a squircle. tint is the
 * pane's own colour and the alpha it stains with; opacity is what the whole
 * pane is fading at. glass.depth is the one length here that is not in
 * reference pixels: scale it by lxb_scale_for first, exactly as you scale a
 * row height.
 */
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

/*
 * One mark, and everything about it that is not in its distance field.
 *
 * The alpha of color is the mark's opacity. accent_soft is what the Simple
 * material tints towards; read it either way, so one call answers both. simple
 * is non-zero for that plainer material: the same drawing with its bevel,
 * reflection, dispersion and shadow stood down.
 */
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

/* --- palettes ---------------------------------------------------------- */

unsigned long lxb_palette_count(void);
const char *lxb_palette_name(unsigned long index);
/* The palette of that name, however capitalised, or -1. */
int lxb_palette_index(const char *name);

unsigned long lxb_role_count(void);
const char *lxb_role_name(unsigned long role);

/* The authored colour: 0xRRGGBB in sRGB. */
unsigned int lxb_palette_color(unsigned long palette, unsigned long role);
/* The same in linear light, ready to draw. */
lxb_rgba lxb_palette_rgba(unsigned long palette, unsigned long role, float alpha);

/* The conversion everything depends on, both ways. */
lxb_rgba lxb_srgb_to_linear(unsigned int hex, float alpha);
unsigned int lxb_linear_to_srgb(lxb_rgba color);

/* --- changing palette -------------------------------------------------- */
/*
 * A whole interface changing colour at once only reads as a system if every
 * surface arrives together, so this holds one blend and everything reads it.
 * Preview shows a colour without choosing it; commit chooses it; restore flows
 * back to what was chosen. Advance once a frame, and read the colours from the
 * accent rather than from the palette while it is moving.
 */

lxb_accent *lxb_accent_new(const char *name);  /* NULL if no such palette */
void lxb_accent_free(lxb_accent *accent);      /* NULL is accepted        */

int lxb_accent_preview(lxb_accent *accent, const char *name);
int lxb_accent_commit(lxb_accent *accent, const char *name);
int lxb_accent_set(lxb_accent *accent, const char *name);  /* no animation */
void lxb_accent_restore(lxb_accent *accent);

/* Returns 1 while another frame is needed. */
int lxb_accent_advance(lxb_accent *accent, float dt);
lxb_rgba lxb_accent_color(lxb_accent *accent, unsigned long role, float alpha);
/* Which palette owns the setting — not what is on screen during a preview. */
const char *lxb_accent_applied(lxb_accent *accent);

/* --- the current shell theme ------------------------------------------ */

unsigned long lxb_icon_style_count(void);
const char *lxb_icon_style_name(unsigned long index);
unsigned long lxb_wallpaper_style_count(void);
const char *lxb_wallpaper_style_name(unsigned long index);
/*
 * Read $XDG_CONFIG_HOME/lxb/shell.toml, falling back through HOME. Missing,
 * unreadable and unknown values safely return Purple and Default.
 */
lxb_shell_theme lxb_shell_theme_load(void);

/* --- the context menu -------------------------------------------------- */

/* Every number a context menu is shaped by: its widths, its row, its
 * padding, how far it grows from and how long it takes. */
lxb_menu lxb_context_menu(void);

/* How many rows of `row` reference pixels a display `height` tall has room
 * for. Never zero. */
unsigned long lxb_menu_rows_that_fit(float height, float row);

/* The panel's rectangle `travelled` of the way out of its anchor: a miniature
 * of itself over `anchor` at 0, `panel` at 1. All three are four floats,
 * x/y/width/height; `out` may be either input. A null argument is answered by
 * doing nothing.
 *
 * It is the panel's own shape the whole way — only its scale and its centre
 * move. Interpolating the two rectangles instead carries it through the
 * anchor's proportions, so a wide flat button reshapes into a tall menu on its
 * way, which reads as the button turning into the panel rather than as a panel
 * arriving. */
void lxb_menu_growing(const float *anchor, const float *panel, float travelled,
                      float *out);

/* How much of the panel is *there*, `travelled` of the way through its flight:
 * what its glass rides, and what the page gives up its words on. `opening` is
 * non-zero while it is on its way out of its anchor and zero while it is
 * folding back into it.
 *
 * The two directions are the same journey and not the same curve. It arrives
 * faster than it moves — all of the glass is there by `panel_in` of the way
 * out, so that what comes out of the control reads as a pane rather than as a
 * rectangle being inflated — and it leaves over the whole of the journey,
 * because a panel that held its full colour until the last three frames and
 * then went out did not fade, it blinked. */
float lxb_menu_shown(float travelled, int opening);

/* The same for what is written on the panel: it holds back on the way out
 * until there is enough panel to read it on, and leaves with the glass rather
 * than ahead of it. */
float lxb_menu_content_shown(float travelled, int opening);

/*
 * Measuring one. A panel is laid out once, at the size it settles at, and then
 * flown out of its anchor as one whole shape — lxb_menu_growing says where it
 * is on the way — so that nothing on it moves relative to anything else while
 * it arrives.
 *
 * Ask lxb_menu_rows_of_that_fit how many rows this display has room for, hand
 * that back as `visible`, and free the layout when the frame is drawn.
 * `title_lines` is nought for a panel raised without a header.
 */
lxb_menu_layout *lxb_menu_layout_new(const lxb_menu_row *rows, unsigned long count,
                                     unsigned int title_lines,
                                     const float *anchor, const float *display,
                                     unsigned long first, unsigned long visible,
                                     unsigned long selected, float unfolded,
                                     float extra);
void lxb_menu_layout_free(lxb_menu_layout *layout);  /* NULL is accepted */

unsigned long lxb_menu_rows_of_that_fit(const lxb_menu_row *rows, unsigned long count,
                                        unsigned int title_lines, float height);

/* Four floats each. The three rect calls return 0 for a row scrolled off the
 * panel, and lxb_menu_aside_rect also for a row with no button. */
void lxb_menu_panel(const lxb_menu_layout *layout, float *out);
float lxb_menu_rows_top(const lxb_menu_layout *layout);
int lxb_menu_row_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
int lxb_menu_chip_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
int lxb_menu_aside_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
int lxb_menu_highlight_rect(const lxb_menu_layout *layout, int on_aside, float *out);
int lxb_menu_separator(const lxb_menu_layout *layout, unsigned long index, float *out);
/* Two floats: the label's share of what the row has opened out, and the second
 * run's. */
void lxb_menu_opened(const lxb_menu_layout *layout, unsigned long index, float *out);

float lxb_menu_chip_radius(const lxb_menu_row *row, float drawn, float height);
float lxb_menu_aside_radius(const lxb_menu_row *row, float height);
unsigned int lxb_menu_lines_in(float room, float line);
float lxb_menu_title_height(unsigned int lines);
float lxb_menu_title_growth(unsigned int lines);

/* The panel's own lengths, in reference pixels: the air it keeps inside its
 * edge, the air on either side of a row's writing, the air above and below a
 * row's chip, and the room one line of each of its runs takes. */
float lxb_menu_margin(void);
float lxb_menu_label_padding(void);
float lxb_menu_row_padding(void);
float lxb_menu_label_line(void);
float lxb_menu_detail_line(void);
float lxb_menu_stamp_room(void);

/* --- the control -------------------------------------------------------- */

/* The shape every control shares: its radius, its padding, and how far a
 * press takes it down. */
lxb_control_shape lxb_control(void);

/* The halo's rectangle for a control at `rect`, on a surface `over` wide and
 * `tall` pixels tall — pass a negative `tall` for the control's own height
 * times glow_height. Four floats each; `out` may be `rect`. */
void lxb_control_glow_rect(const float *rect, float over, float tall,
                           float *out);

/* How much of the lit capsule has arrived over the control at `rect`: 1 when
 * it is sitting on it, 0 while it is still a control away. Measured in the
 * control's own widths and heights, in all four numbers — a capsule sharing a
 * line with a control is not light on it. */
float lxb_control_arrival(const float *light, const float *rect);

/* --- the press ---------------------------------------------------------- */

/* How big a control is drawn `t` of the way through a press, as a multiple of
 * its own size: down, back past its own size, and settled. 1 outside 0 to 1.
 * A press runs over the `guide-press` duration. */
float lxb_press_scale(float t);

/* How much of a state change has arrived `t` of the way through a press. Held
 * back until the control is at the bottom of its travel, because a switch is
 * thrown on the way up. */
float lxb_fill_arrival(float t);

/* The breath under whatever is being aimed at: 0 to 1 and back over the
 * `pulse` duration. */
float lxb_pulse(float seconds);

/* Where a control at `rect` is drawn `t` of the way through a press: its own
 * rectangle for a negative `t`, and lxb_press_scale about its own centre
 * otherwise. Four floats each; `out` may be `rect`. */
void lxb_pressed(const float *rect, float t, float *out);

/* One step of the spring a lit capsule glides on, at a highlight's stiffness.
 * position and velocity are updated in place. */
void lxb_glide(float *position, float *velocity, float target, float dt);

/* --- motion ------------------------------------------------------------ */
/*
 * Two kinds and no third. Something on its own clock is a duration and
 * lxb_ease. Something chasing a target that can move under it is lxb_spring,
 * because a target that moves mid-flight has to be picked up rather than
 * restarted. Nothing is ever linear.
 */

float lxb_ease(float t);
float lxb_smoothstep(float t);
/* One step of a critically damped spring; position and velocity are updated. */
void lxb_spring(double *position, double *velocity, double target, double rate,
                double dt);
double lxb_card_spring(void);

unsigned long lxb_duration_count(void);
const char *lxb_duration_name(unsigned long index);
float lxb_duration(unsigned long index);
float lxb_duration_named(const char *name);  /* 0 if there is no such one */

/* --- sizes and type ---------------------------------------------------- */

/* The height every size in this language is written against: 1080. */
float lxb_reference_height(void);
/* What to multiply a written size by at this window's height. Height, never
 * width: a wide window is a wide window, not a large one. */
float lxb_scale_for(float height);
/* Controls are capsules: the radius of one is half its own height, always. */
float lxb_capsule_radius(float height);

unsigned long lxb_metric_count(void);
const char *lxb_metric_name(unsigned long index);
/* On a screen `height` tall. A share comes back unscaled — see below. */
float lxb_metric(unsigned long index, float height);
int lxb_metric_is_share(unsigned long index);

unsigned long lxb_text_count(void);
const char *lxb_text_name(unsigned long index);
float lxb_text_size(unsigned long index, float height);
int lxb_text_is_bold(unsigned long index);
float lxb_text_line_height(void);

/* --- glass ------------------------------------------------------------- */

unsigned long lxb_surface_count(void);
const char *lxb_surface_name(unsigned long index);
/* The four numbers that make one of the surfaces: how thick its glass is,
 * how it scatters, how it bends and how it takes the light. */
lxb_glass lxb_surface_glass(unsigned long index);

/* Both semantic overlays return the same material by design. */
unsigned long lxb_overlay_count(void);
const char *lxb_overlay_name(unsigned long index);
lxb_overlay_material lxb_overlay_material_for(unsigned long index);

/* The shared key light for glass panes/background; writes three floats. */
void lxb_key_light(float *out);
float lxb_glass_ior(void);
/* The shading itself, in WGSL. See the comment at the top of that file. */
lxb_bytes lxb_glass_wgsl(void);
/* The binding-free current Default-water / Simple-silk wallpaper module. */
lxb_bytes lxb_wallpaper_wgsl(void);

/* --- assets ------------------------------------------------------------ */

unsigned long lxb_glyph_count(void);
const char *lxb_glyph_name(unsigned long index);
/* SVG source. data is NULL if there is no mark of that name. */
lxb_bytes lxb_glyph(const char *name);
/* The square SVG viewBox edge, 24 or 32; zero if there is no such mark. */
unsigned int lxb_glyph_box(const char *name);

/*
 * Convert a (size * lxb_glyph_sdf_supersample())-square coverage plane to a
 * size-square signed-distance alpha plane. Both buffers remain caller-owned.
 * Returns 1 on success and 0 for null pointers, overflow, or lengths that do
 * not exactly match.
 */
int lxb_glyph_sdf(const unsigned char *coverage, unsigned long coverage_len,
                  unsigned int size, unsigned char *out_alpha,
                  unsigned long out_len);

/* The standalone WGSL module that shades a glyph distance field. */
lxb_bytes lxb_glyph_wgsl(void);
float lxb_glyph_sdf_range(void);
unsigned int lxb_glyph_sdf_supersample(void);
unsigned int lxb_glyph_cell(void);
float lxb_glyph_depth_share(void);
void lxb_glyph_lamp(float *out);  /* writes three floats; NULL is accepted */
float lxb_glyph_shadow(void);
float lxb_glyph_simple_tint(void);
float lxb_glyph_simple_alpha(void);
float lxb_glyph_simple_stain(void);

/* lxb_sound: the recordings, in the order lxb_sound_name reads them, and what
 * each one answers. The value is the index every call here takes.
 *
 * An interface driven from a controller answers a press twice: something moves,
 * and it clicks. A move sounds by where it *landed*, not by what moved it — a
 * click that puts the selection on a row is the same move as the direction that
 * would have walked there — and hovering is not a move at all. A screen with a
 * voice of its own does not borrow another's, which is why the guide has its
 * own pair.
 */
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
/* Ogg Vorbis. */
lxb_bytes lxb_sound(const char *name);
/* Whether the current shell plays this recording. Unknown names return 0. */
int lxb_sound_used(const char *name);
/* The shortest gap between two plays of one recording, in seconds. */
float lxb_sound_rest(void);
/*
 * How loud a level of 0..1 actually is, as an amplitude. A level is read as
 * loudness, and amplitude is not loudness: a control dragged to the middle
 * should sound half as loud rather than measure half as tall.
 */
float lxb_sound_amplitude(float value);
/* How far through a fade of `over` seconds `elapsed` is, eased. */
float lxb_sound_fade(float elapsed, float over);
/* How long the one looping recording takes to arrive, and to go. */
float lxb_sound_music_fade_in(void);
float lxb_sound_music_fade_out(void);
/* How long to leave a sound device alone after failing to open it. */
float lxb_sound_retry_after(void);

/* --- what the user pressed, and what it means ---------------------------- */

/*
 * An interface in this language is driven from a keyboard, a controller and a
 * pointer at once, and the whole point is that they are one interface rather
 * than three. What decides that is not the device; it is the two tables below,
 * which are the same tables the Rust library reads.
 *
 * Three enumerations whose index a caller passes, as with the four above.
 */

/* lxb_action: one thing the user asked for, whichever control asked for it. */
enum {
    LXB_ACTION_LEFT = 0,
    LXB_ACTION_RIGHT = 1,
    LXB_ACTION_UP = 2,
    LXB_ACTION_DOWN = 3,
    /* Act on what is selected. The library calls it "launch". */
    LXB_ACTION_ACCEPT = 4,
    LXB_ACTION_BACK = 5,
    LXB_ACTION_MENU = 6,
    LXB_ACTION_SUBMIT = 7,
    LXB_ACTION_PREVIOUS = 8,
    LXB_ACTION_NEXT = 9,
};

/* lxb_key: the keys that mean something here, and no others. */
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

/* lxb_button: a control on a game controller, by where the thumb finds it.
 * Named positionally, because that is the only naming that survives the pad
 * being an Xbox one, a DualSense, a Switch Pro or a Steam Deck. */
enum {
    LXB_BUTTON_SOUTH = 0,
    LXB_BUTTON_EAST = 1,
    LXB_BUTTON_NORTH = 2,
    LXB_BUTTON_WEST = 3,
    LXB_BUTTON_START = 4,
    LXB_BUTTON_SELECT = 5,
    LXB_BUTTON_LEFT_BUMPER = 6,
    LXB_BUTTON_RIGHT_BUMPER = 7,
    /* The shell's, always: it is the way out of this application. */
    LXB_BUTTON_GUIDE = 8,
    LXB_BUTTON_DPAD_LEFT = 9,
    LXB_BUTTON_DPAD_RIGHT = 10,
    LXB_BUTTON_DPAD_UP = 11,
    LXB_BUTTON_DPAD_DOWN = 12,
};

unsigned long lxb_action_count(void);
const char *lxb_action_name(unsigned long index);
/* The action written under this name, or -1. */
int lxb_action_named(const char *name);
/* Whether a held control keeps sending it: the four directions, and no more. */
int lxb_action_repeats(int action);
/* What a key means, or -1. The letters are deliberately not in here. */
int lxb_action_of_key(int key);
/*
 * The shell's letter shorthands: wasd and hjkl for the directions, y for the
 * context menu. Asked for separately because an application with a field in it
 * must not bind them.
 */
int lxb_action_of_letter(unsigned int codepoint);
/* What a controller button means, or -1 for the three that mean nothing. */
int lxb_action_of_button(int button);

/* How often a controller should be read, in seconds. */
float lxb_input_poll_interval(void);
/* How long a direction is held before it steps on its own, and then how often. */
float lxb_input_initial_repeat(void);
float lxb_input_repeat_interval(void);
/* How far a stick is pushed to engage, and comes back to release. */
float lxb_input_stick_engage(void);
float lxb_input_stick_release(void);
/* How far a device with no notches travels to be worth one. */
float lxb_input_scroll_step(void);
/* How far a finger may wander and still have been a tap. */
float lxb_input_tap_slop(void);

/*
 * The middle of a held direction, which no platform supplies: Wayland hands a
 * client one press and one release, and a pad has no repeat at all. Keep one,
 * put every direction into it, and step it once a poll.
 */
typedef struct lxb_repeat lxb_repeat;
lxb_repeat *lxb_repeat_new(void);
void lxb_repeat_free(lxb_repeat *repeat);
/* Forget every held control, for when this application stops being driven. */
void lxb_repeat_reset(lxb_repeat *repeat);
/*
 * Step to `now` seconds and write the actions that are due into `out`.
 * `pressed` is four ints in left, right, up, down order; `x` and `y` are one
 * stick, with y positive up. Answers how many were due, which may exceed
 * `capacity`.
 */
unsigned long lxb_repeat_update(lxb_repeat *repeat, float now, const int *pressed,
                                float x, float y, int *out, unsigned long capacity);

/*
 * How many whole steps of a wheel `notches` is worth, keeping the rest in
 * `carried` — which starts at nought and is this function's from then on. The
 * remainder is the point of it: a touchpad reports fractions of a notch, and
 * rounding each of them away is a list that never moves under a slow drag.
 */
int lxb_wheel_notches(float *carried, float notches);

/* Roboto, as the two faces anything here is set in. */
lxb_bytes lxb_font(int bold);

const char *lxb_version(void);

/* --- drawing the material ----------------------------------------------- */

/*
 * Three of the four things above ship as WGSL, and running WGSL needs a
 * renderer. These draw the same three on the processor, over a buffer of
 * pixels: same constants, same terms, same order. A program with a painter
 * rather than a renderer under it gets the real material instead of a flat
 * rectangle with a bright edge painted on.
 *
 * Each returns 1, or 0 for a canvas it cannot draw on. Each draws on every
 * core the machine has. The scene is thirty-odd transcendental functions per
 * pixel, so a program that wants it cheaply should fill a smaller canvas and
 * scale that up with its own painter — every term in it is broad enough to
 * survive that.
 *
 *     lxb_canvas canvas = { data, width, height, stride };
 *     lxb_scene scene = lxb_scene_for(theme.accent, seconds);
 *     scene.soften = lxb_wallpaper_soften();
 *     lxb_paint_wallpaper(&canvas, &scene);
 *     lxb_paint_glass(&canvas, &pane);
 */

lxb_scene lxb_scene_for(unsigned long palette, float time);
/* The same from a travelling palette, so the scene follows an accent change. */
lxb_scene lxb_accent_scene(lxb_accent *accent, float time);

int lxb_paint_wallpaper(const lxb_canvas *canvas, const lxb_scene *scene);

/*
 * The canvas is both what a pane refracts and where it lands, so draw back to
 * front: the background, then the panes standing on it, then the panes
 * standing on those.
 */
int lxb_paint_glass(const lxb_canvas *canvas, const lxb_pane *pane);

/*
 * The soft ellipse of light that goes under a pane and behind whatever is
 * being aimed at: a gaussian sealed at its own edge, which is not what a
 * painter's own radial gradient draws. `rect` is four floats. Draw it before
 * the pane, so the pane bends it.
 */
int lxb_paint_light(const lxb_canvas *canvas, const float *rect, lxb_rgba tint);

/*
 * Push what is already on the canvas back: blurred `blur` rungs down the same
 * chain a frosted pane reaches into — nought for no blur at all — and dimmed by
 * `tint`. The region comes back opaque.
 */
int lxb_paint_scrim(const lxb_canvas *canvas, const float *rect, float blur,
                    lxb_rgba tint);

/*
 * Shade one mark out of the distance field lxb_glyph_sdf made of it, which
 * must be lxb_glyph_cell() square. Not the SVG: drawing that directly gets the
 * silhouette and none of this.
 */
int lxb_paint_glyph(const lxb_canvas *canvas, const lxb_mark *mark,
                    const unsigned char *field, unsigned long field_len);

/* How much the scene is softened when an interface is standing on it. */
float lxb_wallpaper_soften(void);

/* --- the stylesheet ---------------------------------------------------- */
/*
 * The whole language as CSS custom properties, for a GTK or Qt application
 * that would rather load a stylesheet than link a renderer. This one
 * allocates: release it with lxb_string_free.
 */
char *lxb_stylesheet(const char *palette_name);  /* NULL if no such palette */
void lxb_string_free(char *text);

#ifdef __cplusplus
}
#endif

#endif /* LXB_TOOLKIT_H */
