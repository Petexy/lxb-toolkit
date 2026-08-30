# API reference

Every function the toolkit answers, in the order its headers declare them.

229 functions in all.

The reference is the authoritative documentation. `scripts/make-reference.py`
refreshes every C signature from the two headers and refuses missing, extra, or
reordered entries, so adding an API also requires documenting it here.

The three languages are one API. A C function `lxb_page_button` is `page.button` in Python and `Page::button` in Rust; `lxb_glyph_count` is `lxb.GLYPHS` and `glyph::ALL`. Where a C call takes an out parameter, the other two return the value. Enumerations cross as indices into this library's own lists, so the two shared objects agree by construction.

Three shapes repeat and are not written out each time. `X_count` answers how many of a thing there are; `X_name(index)` answers the name of one, which is the name the shell itself calls it; and `X_index(name)` answers the number of a name, however capitalised, or -1 for a name nobody has. The number is what crosses the ABI, so walking a list means counting to `X_count` and asking for each.

## `lxb_toolkit.h`

What the language answers: colours, sizes, motion, type, marks, recordings and the shapes of its panels. Links nothing.

164 functions.

### palettes

`lxb_palette_count`

```c
unsigned long lxb_palette_count(void);
```

`lxb_palette_name`

```c
const char *lxb_palette_name(unsigned long index);
```

The palette of that name, however capitalised, or -1.

`lxb_palette_index`

```c
int lxb_palette_index(const char *name);
```

`lxb_role_count`

```c
unsigned long lxb_role_count(void);
```

`lxb_role_name`

```c
const char *lxb_role_name(unsigned long role);
```

The authored colour: 0xRRGGBB in sRGB.

`lxb_palette_color`

```c
unsigned int lxb_palette_color(unsigned long palette, unsigned long role);
```

The same in linear light, ready to draw.

`lxb_palette_rgba`

```c
lxb_rgba lxb_palette_rgba(unsigned long palette, unsigned long role, float alpha);
```

The conversion everything depends on, both ways.

`lxb_srgb_to_linear`

```c
lxb_rgba lxb_srgb_to_linear(unsigned int hex, float alpha);
```

`lxb_linear_to_srgb`

```c
unsigned int lxb_linear_to_srgb(lxb_rgba color);
```

### changing palette

A whole interface changing colour at once only reads as a system if every surface arrives together, so this holds one blend and everything reads it. Preview shows a colour without choosing it; commit chooses it; restore flows back to what was chosen. Advance once a frame, and read the colours from the accent rather than from the palette while it is moving.

`lxb_accent_new`

```c
lxb_accent *lxb_accent_new(const char *name);  /* NULL if no such palette */
```

`lxb_accent_free`

```c
void lxb_accent_free(lxb_accent *accent);      /* NULL is accepted        */
```

`lxb_accent_preview`

```c
int lxb_accent_preview(lxb_accent *accent, const char *name);
```

`lxb_accent_commit`

```c
int lxb_accent_commit(lxb_accent *accent, const char *name);
```

`lxb_accent_set`

```c
int lxb_accent_set(lxb_accent *accent, const char *name);  /* no animation */
```

`lxb_accent_restore`

```c
void lxb_accent_restore(lxb_accent *accent);
```

Returns 1 while another frame is needed.

`lxb_accent_advance`

```c
int lxb_accent_advance(lxb_accent *accent, float dt);
```

`lxb_accent_color`

```c
lxb_rgba lxb_accent_color(lxb_accent *accent, unsigned long role, float alpha);
```

Which palette owns the setting — not what is on screen during a preview.

`lxb_accent_applied`

```c
const char *lxb_accent_applied(lxb_accent *accent);
```

### the current shell theme

`lxb_icon_style_count`

```c
unsigned long lxb_icon_style_count(void);
```

`lxb_icon_style_name`

```c
const char *lxb_icon_style_name(unsigned long index);
```

`lxb_wallpaper_style_count`

```c
unsigned long lxb_wallpaper_style_count(void);
```

`lxb_wallpaper_style_name`

```c
const char *lxb_wallpaper_style_name(unsigned long index);
```

Read $XDG_CONFIG_HOME/lxb/shell.toml, falling back through HOME. Missing, unreadable and unknown values safely return Purple and Default.

`lxb_shell_theme_load`

```c
lxb_shell_theme lxb_shell_theme_load(void);
```

### the context menu

Every number a context menu is shaped by: its widths, its row, its padding, how far it grows from and how long it takes.

`lxb_context_menu`

```c
lxb_menu lxb_context_menu(void);
```

How many rows of `row` reference pixels a display `height` tall has room for. Never zero.

`lxb_menu_rows_that_fit`

```c
unsigned long lxb_menu_rows_that_fit(float height, float row);
```

The panel's rectangle `travelled` of the way out of its anchor: a miniature of itself over `anchor` at 0, `panel` at 1. All three are four floats, x/y/width/height; `out` may be either input. A null argument is answered by doing nothing.

It is the panel's own shape the whole way — only its scale and its centre move. Interpolating the two rectangles instead carries it through the anchor's proportions, so a wide flat button reshapes into a tall menu on its way, which reads as the button turning into the panel rather than as a panel arriving.

`lxb_menu_growing`

```c
void lxb_menu_growing(const float *anchor, const float *panel, float travelled,
                      float *out);
```

How much of the panel is *there*, `travelled` of the way through its flight: what its glass rides, and what the page gives up its words on. `opening` is non-zero while it is on its way out of its anchor and zero while it is folding back into it.

The two directions are the same journey and not the same curve. It arrives faster than it moves — all of the glass is there by `panel_in` of the way out, so that what comes out of the control reads as a pane rather than as a rectangle being inflated — and it leaves over the whole of the journey, because a panel that held its full colour until the last three frames and then went out did not fade, it blinked.

`lxb_menu_shown`

```c
float lxb_menu_shown(float travelled, int opening);
```

The same for what is written on the panel: it holds back on the way out until there is enough panel to read it on, and leaves with the glass rather than ahead of it.

`lxb_menu_content_shown`

```c
float lxb_menu_content_shown(float travelled, int opening);
```

Measuring one. A panel is laid out once, at the size it settles at, and then flown out of its anchor as one whole shape — lxb_menu_growing says where it is on the way — so that nothing on it moves relative to anything else while it arrives.

Ask lxb_menu_rows_of_that_fit how many rows this display has room for, hand that back as `visible`, and free the layout when the frame is drawn. `title_lines` is nought for a panel raised without a header.

`lxb_menu_layout_new`

```c
lxb_menu_layout *lxb_menu_layout_new(const lxb_menu_row *rows, unsigned long count,
                                     unsigned int title_lines,
                                     const float *anchor, const float *display,
                                     unsigned long first, unsigned long visible,
                                     unsigned long selected, float unfolded,
                                     float extra);
```

`lxb_menu_layout_free`

```c
void lxb_menu_layout_free(lxb_menu_layout *layout);  /* NULL is accepted */
```

`lxb_menu_rows_of_that_fit`

```c
unsigned long lxb_menu_rows_of_that_fit(const lxb_menu_row *rows, unsigned long count,
                                        unsigned int title_lines, float height);
```

Four floats each. The three rect calls return 0 for a row scrolled off the panel, and lxb_menu_aside_rect also for a row with no button.

`lxb_menu_panel`

```c
void lxb_menu_panel(const lxb_menu_layout *layout, float *out);
```

`lxb_menu_rows_top`

```c
float lxb_menu_rows_top(const lxb_menu_layout *layout);
```

`lxb_menu_row_rect`

```c
int lxb_menu_row_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
```

`lxb_menu_chip_rect`

```c
int lxb_menu_chip_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
```

`lxb_menu_aside_rect`

```c
int lxb_menu_aside_rect(const lxb_menu_layout *layout, unsigned long index, float *out);
```

`lxb_menu_highlight_rect`

```c
int lxb_menu_highlight_rect(const lxb_menu_layout *layout, int on_aside, float *out);
```

`lxb_menu_separator`

```c
int lxb_menu_separator(const lxb_menu_layout *layout, unsigned long index, float *out);
```

Two floats: the label's share of what the row has opened out, and the second run's.

`lxb_menu_opened`

```c
void lxb_menu_opened(const lxb_menu_layout *layout, unsigned long index, float *out);
```

`lxb_menu_chip_radius`

```c
float lxb_menu_chip_radius(const lxb_menu_row *row, float drawn, float height);
```

`lxb_menu_aside_radius`

```c
float lxb_menu_aside_radius(const lxb_menu_row *row, float height);
```

`lxb_menu_lines_in`

```c
unsigned int lxb_menu_lines_in(float room, float line);
```

`lxb_menu_title_height`

```c
float lxb_menu_title_height(unsigned int lines);
```

`lxb_menu_title_growth`

```c
float lxb_menu_title_growth(unsigned int lines);
```

The panel's own lengths, in reference pixels: the air it keeps inside its edge, the air on either side of a row's writing, the air above and below a row's chip, and the room one line of each of its runs takes.

`lxb_menu_margin`

```c
float lxb_menu_margin(void);
```

`lxb_menu_label_padding`

```c
float lxb_menu_label_padding(void);
```

`lxb_menu_row_padding`

```c
float lxb_menu_row_padding(void);
```

`lxb_menu_label_line`

```c
float lxb_menu_label_line(void);
```

`lxb_menu_detail_line`

```c
float lxb_menu_detail_line(void);
```

`lxb_menu_stamp_room`

```c
float lxb_menu_stamp_room(void);
```

### the control

The shape every control shares: its radius, its padding, and how far a press takes it down.

`lxb_control`

```c
lxb_control_shape lxb_control(void);
```

The halo's rectangle for a control at `rect`, on a surface `over` wide and `tall` pixels tall — pass a negative `tall` for the control's own height times glow_height. Four floats each; `out` may be `rect`.

`lxb_control_glow_rect`

```c
void lxb_control_glow_rect(const float *rect, float over, float tall,
                           float *out);
```

How much of the lit capsule has arrived over the control at `rect`: 1 when it is sitting on it, 0 while it is still a control away. Measured in the control's own widths and heights, in all four numbers — a capsule sharing a line with a control is not light on it.

`lxb_control_arrival`

```c
float lxb_control_arrival(const float *light, const float *rect);
```

### the press

How big a control is drawn `t` of the way through a press, as a multiple of its own size: down, back past its own size, and settled. 1 outside 0 to 1. A press runs over the `guide-press` duration.

`lxb_press_scale`

```c
float lxb_press_scale(float t);
```

How much of a state change has arrived `t` of the way through a press. Held back until the control is at the bottom of its travel, because a switch is thrown on the way up.

`lxb_fill_arrival`

```c
float lxb_fill_arrival(float t);
```

The breath under whatever is being aimed at: 0 to 1 and back over the `pulse` duration.

`lxb_pulse`

```c
float lxb_pulse(float seconds);
```

Where a control at `rect` is drawn `t` of the way through a press: its own rectangle for a negative `t`, and lxb_press_scale about its own centre otherwise. Four floats each; `out` may be `rect`.

`lxb_pressed`

```c
void lxb_pressed(const float *rect, float t, float *out);
```

One step of the spring a lit capsule glides on, at a highlight's stiffness. position and velocity are updated in place.

`lxb_glide`

```c
void lxb_glide(float *position, float *velocity, float target, float dt);
```

### motion

Two kinds and no third. Something on its own clock is a duration and lxb_ease. Something chasing a target that can move under it is lxb_spring, because a target that moves mid-flight has to be picked up rather than restarted. Nothing is ever linear.

`lxb_ease`

```c
float lxb_ease(float t);
```

`lxb_smoothstep`

```c
float lxb_smoothstep(float t);
```

One step of a critically damped spring; position and velocity are updated.

`lxb_spring`

```c
void lxb_spring(double *position, double *velocity, double target, double rate,
                double dt);
```

`lxb_card_spring`

```c
double lxb_card_spring(void);
```

`lxb_duration_count`

```c
unsigned long lxb_duration_count(void);
```

`lxb_duration_name`

```c
const char *lxb_duration_name(unsigned long index);
```

`lxb_duration`

```c
float lxb_duration(unsigned long index);
```

`lxb_duration_named`

```c
float lxb_duration_named(const char *name);  /* 0 if there is no such one */
```

### sizes and type

The height every size in this language is written against: 1080.

`lxb_reference_height`

```c
float lxb_reference_height(void);
```

What to multiply a written size by at this window's height. Height, never width: a wide window is a wide window, not a large one.

`lxb_scale_for`

```c
float lxb_scale_for(float height);
```

Controls are capsules: the radius of one is half its own height, always.

`lxb_capsule_radius`

```c
float lxb_capsule_radius(float height);
```

`lxb_metric_count`

```c
unsigned long lxb_metric_count(void);
```

`lxb_metric_name`

```c
const char *lxb_metric_name(unsigned long index);
```

On a screen `height` tall. A share comes back unscaled — see below.

`lxb_metric`

```c
float lxb_metric(unsigned long index, float height);
```

`lxb_metric_is_share`

```c
int lxb_metric_is_share(unsigned long index);
```

`lxb_text_count`

```c
unsigned long lxb_text_count(void);
```

`lxb_text_name`

```c
const char *lxb_text_name(unsigned long index);
```

`lxb_text_size`

```c
float lxb_text_size(unsigned long index, float height);
```

`lxb_text_is_bold`

```c
int lxb_text_is_bold(unsigned long index);
```

`lxb_text_line_height`

```c
float lxb_text_line_height(void);
```

### glass

`lxb_surface_count`

```c
unsigned long lxb_surface_count(void);
```

`lxb_surface_name`

```c
const char *lxb_surface_name(unsigned long index);
```

The four numbers that make one of the surfaces: how thick its glass is, how it scatters, how it bends and how it takes the light.

`lxb_surface_glass`

```c
lxb_glass lxb_surface_glass(unsigned long index);
```

Both semantic overlays return the same material by design.

`lxb_overlay_count`

```c
unsigned long lxb_overlay_count(void);
```

`lxb_overlay_name`

```c
const char *lxb_overlay_name(unsigned long index);
```

`lxb_overlay_material_for`

```c
lxb_overlay_material lxb_overlay_material_for(unsigned long index);
```

The shared key light for glass panes/background; writes three floats.

`lxb_key_light`

```c
void lxb_key_light(float *out);
```

`lxb_glass_ior`

```c
float lxb_glass_ior(void);
```

The shading itself, in WGSL. See the comment at the top of that file.

`lxb_glass_wgsl`

```c
lxb_bytes lxb_glass_wgsl(void);
```

The binding-free current Default-water / Simple-silk wallpaper module.

`lxb_wallpaper_wgsl`

```c
lxb_bytes lxb_wallpaper_wgsl(void);
```

### file and folder selection

Start at `directory`. The path is UTF-8 and may be relative; relative paths are made relative to the process's current directory. An unreadable or missing directory still returns a picker whose note explains the problem. NULL means an invalid selection or path.

`lxb_picker_new`

```c
lxb_picker *lxb_picker_new(unsigned long selection, const char *directory);
```

Free a picker. NULL is accepted.

`lxb_picker_free`

```c
void lxb_picker_free(lxb_picker *picker);
```

Borrowed UTF-8, valid until a call that changes `picker` or until it is freed. The location is the directory currently being listed; note is its count, Empty, or This cannot be opened; query is the current case-insensitive substring filter (empty in folder mode, which has no search field).

`lxb_picker_location`

```c
const char *lxb_picker_location(const lxb_picker *picker);
```

`lxb_picker_note`

```c
const char *lxb_picker_note(const lxb_picker *picker);
```

`lxb_picker_query`

```c
const char *lxb_picker_query(const lxb_picker *picker);
```

`lxb_picker_selection`

```c
unsigned long lxb_picker_selection(const lxb_picker *picker);
```

Whether the current listing has a search field. Empty/unreadable file listings have none; a zero-match search keeps its field so it can be cleared. Folder mode uses this position for Select folder instead.

`lxb_picker_can_search`

```c
int lxb_picker_can_search(const lxb_picker *picker);
```

Whether an explicit choose action currently has an answer. A folder picker is choosable only when its current directory opened successfully; a file picker only while a file is focused.

`lxb_picker_can_choose`

```c
int lxb_picker_can_choose(const lxb_picker *picker);
```

Every visible entry. Invalid indices return NULL for strings and -1 for its kind. Folder entries can be entered; file entries are leaves.

`lxb_picker_entry_count`

```c
unsigned long lxb_picker_entry_count(const lxb_picker *picker);
```

`lxb_picker_entry_name`

```c
const char *lxb_picker_entry_name(const lxb_picker *picker, unsigned long index);
```

`lxb_picker_entry_path`

```c
const char *lxb_picker_entry_path(const lxb_picker *picker, unsigned long index);
```

`lxb_picker_entry_kind`

```c
int lxb_picker_entry_kind(const lxb_picker *picker, unsigned long index);
```

The visible entry under focus, or -1 for none. Select and move return non-zero only when focus moved. Movement stops at either end rather than wrapping.

`lxb_picker_selected`

```c
int lxb_picker_selected(const lxb_picker *picker);
```

`lxb_picker_select`

```c
int lxb_picker_select(lxb_picker *picker, unsigned long index);
```

`lxb_picker_move`

```c
int lxb_picker_move(lxb_picker *picker, int delta);
```

Enter the focused folder, or leave to its parent. Both reread lazily and clear the query; they return non-zero only when the directory changed.

`lxb_picker_enter`

```c
int lxb_picker_enter(lxb_picker *picker);
```

`lxb_picker_leave`

```c
int lxb_picker_leave(lxb_picker *picker);
```

Reread the directory as it is now, or replace its case-insensitive substring query and reread it. A listing without a search field, including folder mode, ignores search. NULL query is invalid and does nothing.

`lxb_picker_refresh`

```c
void lxb_picker_refresh(lxb_picker *picker);
```

`lxb_picker_search`

```c
void lxb_picker_search(lxb_picker *picker, const char *query);
```

The current answer, or NULL if a file picker is focused on a folder or has no visible selection. A folder picker answers with a current directory that opened successfully; draw its explicit Select folder control separately so opening a folder never chooses it by accident. Borrowed UTF-8, valid until a call that changes `picker` or until it is freed.

`lxb_picker_choose`

```c
const char *lxb_picker_choose(lxb_picker *picker);
```

### assets

`lxb_glyph_count`

```c
unsigned long lxb_glyph_count(void);
```

`lxb_glyph_name`

```c
const char *lxb_glyph_name(unsigned long index);
```

SVG source. data is NULL if there is no mark of that name.

`lxb_glyph`

```c
lxb_bytes lxb_glyph(const char *name);
```

The square SVG viewBox edge, 24 or 32; zero if there is no such mark.

`lxb_glyph_box`

```c
unsigned int lxb_glyph_box(const char *name);
```

Convert a (size * lxb_glyph_sdf_supersample())-square coverage plane to a size-square signed-distance alpha plane. Both buffers remain caller-owned. Returns 1 on success and 0 for null pointers, overflow, or lengths that do not exactly match.

`lxb_glyph_sdf`

```c
int lxb_glyph_sdf(const unsigned char *coverage, unsigned long coverage_len,
                  unsigned int size, unsigned char *out_alpha,
                  unsigned long out_len);
```

The standalone WGSL module that shades a glyph distance field.

`lxb_glyph_wgsl`

```c
lxb_bytes lxb_glyph_wgsl(void);
```

`lxb_glyph_sdf_range`

```c
float lxb_glyph_sdf_range(void);
```

`lxb_glyph_sdf_supersample`

```c
unsigned int lxb_glyph_sdf_supersample(void);
```

`lxb_glyph_cell`

```c
unsigned int lxb_glyph_cell(void);
```

`lxb_glyph_depth_share`

```c
float lxb_glyph_depth_share(void);
```

`lxb_glyph_lamp`

```c
void lxb_glyph_lamp(float *out);  /* writes three floats; NULL is accepted */
```

`lxb_glyph_shadow`

```c
float lxb_glyph_shadow(void);
```

`lxb_glyph_simple_tint`

```c
float lxb_glyph_simple_tint(void);
```

`lxb_glyph_simple_alpha`

```c
float lxb_glyph_simple_alpha(void);
```

`lxb_glyph_simple_stain`

```c
float lxb_glyph_simple_stain(void);
```

`lxb_sound_count`

```c
unsigned long lxb_sound_count(void);
```

`lxb_sound_name`

```c
const char *lxb_sound_name(unsigned long index);
```

Ogg Vorbis.

`lxb_sound`

```c
lxb_bytes lxb_sound(const char *name);
```

Whether the current shell plays this recording. Unknown names return 0.

`lxb_sound_used`

```c
int lxb_sound_used(const char *name);
```

The shortest gap between two plays of one recording, in seconds.

`lxb_sound_rest`

```c
float lxb_sound_rest(void);
```

How loud a level of 0..1 actually is, as an amplitude. A level is read as loudness, and amplitude is not loudness: a control dragged to the middle should sound half as loud rather than measure half as tall.

`lxb_sound_amplitude`

```c
float lxb_sound_amplitude(float value);
```

How far through a fade of `over` seconds `elapsed` is, eased.

`lxb_sound_fade`

```c
float lxb_sound_fade(float elapsed, float over);
```

How long the one looping recording takes to arrive, and to go.

`lxb_sound_music_fade_in`

```c
float lxb_sound_music_fade_in(void);
```

`lxb_sound_music_fade_out`

```c
float lxb_sound_music_fade_out(void);
```

How long to leave a sound device alone after failing to open it.

`lxb_sound_retry_after`

```c
float lxb_sound_retry_after(void);
```

### what the user pressed, and what it means

`lxb_action_count`

```c
unsigned long lxb_action_count(void);
```

`lxb_action_name`

```c
const char *lxb_action_name(unsigned long index);
```

The action written under this name, or -1.

`lxb_action_named`

```c
int lxb_action_named(const char *name);
```

Whether a held control keeps sending it: the four directions, and no more.

`lxb_action_repeats`

```c
int lxb_action_repeats(int action);
```

What a key means, or -1. The letters are deliberately not in here.

`lxb_action_of_key`

```c
int lxb_action_of_key(int key);
```

The shell's letter shorthands: wasd and hjkl for the directions, y for the context menu. Asked for separately because an application with a field in it must not bind them.

`lxb_action_of_letter`

```c
int lxb_action_of_letter(unsigned int codepoint);
```

What a controller button means, or -1 for the three that mean nothing.

`lxb_action_of_button`

```c
int lxb_action_of_button(int button);
```

How often a controller should be read, in seconds.

`lxb_input_poll_interval`

```c
float lxb_input_poll_interval(void);
```

How long a direction is held before it steps on its own, and then how often.

`lxb_input_initial_repeat`

```c
float lxb_input_initial_repeat(void);
```

`lxb_input_repeat_interval`

```c
float lxb_input_repeat_interval(void);
```

How far a stick is pushed to engage, and comes back to release.

`lxb_input_stick_engage`

```c
float lxb_input_stick_engage(void);
```

`lxb_input_stick_release`

```c
float lxb_input_stick_release(void);
```

How far a device with no notches travels to be worth one.

`lxb_input_scroll_step`

```c
float lxb_input_scroll_step(void);
```

How far a finger may wander and still have been a tap.

`lxb_input_tap_slop`

```c
float lxb_input_tap_slop(void);
```

`lxb_repeat_new`

```c
lxb_repeat *lxb_repeat_new(void);
```

`lxb_repeat_free`

```c
void lxb_repeat_free(lxb_repeat *repeat);
```

Forget every held control, for when this application stops being driven.

`lxb_repeat_reset`

```c
void lxb_repeat_reset(lxb_repeat *repeat);
```

Step to `now` seconds and write the actions that are due into `out`. `pressed` is four ints in left, right, up, down order; `x` and `y` are one stick, with y positive up. Answers how many were due, which may exceed `capacity`.

`lxb_repeat_update`

```c
unsigned long lxb_repeat_update(lxb_repeat *repeat, float now, const int *pressed,
                                float x, float y, int *out, unsigned long capacity);
```

How many whole steps of a wheel `notches` is worth, keeping the rest in `carried` — which starts at nought and is this function's from then on. The remainder is the point of it: a touchpad reports fractions of a notch, and rounding each of them away is a list that never moves under a slow drag.

`lxb_wheel_notches`

```c
int lxb_wheel_notches(float *carried, float notches);
```

Roboto, as the two faces anything here is set in.

`lxb_font`

```c
lxb_bytes lxb_font(int bold);
```

`lxb_version`

```c
const char *lxb_version(void);
```

### drawing the material

Three of the four things above ship as WGSL, and running WGSL needs a renderer. These draw the same three on the processor, over a buffer of pixels: same constants, same terms, same order. A program with a painter rather than a renderer under it gets the real material instead of a flat rectangle with a bright edge painted on.

Each returns 1, or 0 for a canvas it cannot draw on. Each draws on every core the machine has. The scene is thirty-odd transcendental functions per pixel, so a program that wants it cheaply should fill a smaller canvas and scale that up with its own painter — every term in it is broad enough to survive that.

lxb_canvas canvas = { data, width, height, stride }; lxb_scene scene = lxb_scene_for(theme.accent, seconds); scene.soften = lxb_wallpaper_soften(); lxb_paint_wallpaper(&canvas, &scene); lxb_paint_glass(&canvas, &pane);

`lxb_scene_for`

```c
lxb_scene lxb_scene_for(unsigned long palette, float time);
```

The same from a travelling palette, so the scene follows an accent change.

`lxb_accent_scene`

```c
lxb_scene lxb_accent_scene(lxb_accent *accent, float time);
```

`lxb_paint_wallpaper`

```c
int lxb_paint_wallpaper(const lxb_canvas *canvas, const lxb_scene *scene);
```

The canvas is both what a pane refracts and where it lands, so draw back to front: the background, then the panes standing on it, then the panes standing on those.

`lxb_paint_glass`

```c
int lxb_paint_glass(const lxb_canvas *canvas, const lxb_pane *pane);
```

The soft ellipse of light that goes under a pane and behind whatever is being aimed at: a gaussian sealed at its own edge, which is not what a painter's own radial gradient draws. `rect` is four floats. Draw it before the pane, so the pane bends it.

`lxb_paint_light`

```c
int lxb_paint_light(const lxb_canvas *canvas, const float *rect, lxb_rgba tint);
```

Push what is already on the canvas back: blurred `blur` rungs down the same chain a frosted pane reaches into — nought for no blur at all — and dimmed by `tint`. The region comes back opaque.

`lxb_paint_scrim`

```c
int lxb_paint_scrim(const lxb_canvas *canvas, const float *rect, float blur,
                    lxb_rgba tint);
```

Shade one mark out of the distance field lxb_glyph_sdf made of it, which must be lxb_glyph_cell() square. Not the SVG: drawing that directly gets the silhouette and none of this.

`lxb_paint_glyph`

```c
int lxb_paint_glyph(const lxb_canvas *canvas, const lxb_mark *mark,
                    const unsigned char *field, unsigned long field_len);
```

How much the scene is softened when an interface is standing on it.

`lxb_wallpaper_soften`

```c
float lxb_wallpaper_soften(void);
```

### the stylesheet

The whole language as CSS custom properties, for a GTK or Qt application that would rather load a stylesheet than link a renderer. This one allocates: release it with lxb_string_free.

`lxb_stylesheet`

```c
char *lxb_stylesheet(const char *palette_name);  /* NULL if no such palette */
```

`lxb_string_free`

```c
void lxb_string_free(char *text);
```

## `lxb_app.h`

The window, the frame loop, the controls and the sounds — and the page an application draws into. Carries the GPU stack.

65 functions.

A new application. `app_id` is the stable name the desktop entry, the executable and StartupWMClass all have to agree on; `title` is what a person sees. Never null; free it with lxb_app_free.

`lxb_app_new`

```c
lxb_app *lxb_app_new(const char *app_id, const char *title);
```

How large the window asks to be, in logical pixels. A request rather than a demand: the compositor decides, and everything drawn is measured from the height it actually gets.

`lxb_app_size`

```c
void lxb_app_size(lxb_app *app, double width, double height);
```

Draw the whole window rather than the standard page. The pane inset from the window's edges, and the flow inside it, are the common case and are drawn for you; this is for a program that is a screen rather than a panel.

`lxb_app_plain`

```c
void lxb_app_plain(lxb_app *app);
```

Move your own selection: the light is not walked between controls for you, and every action arrives at the page through lxb_page_action.

One list read down the screen is what an application usually is, and for that the default is the whole of the work. An application that is a screen — a column of pages beside the page they are about, a grid, a board — has more than one axis, and the language's own answer is that Up and Down are the outer list while Left and Right are within it. Nothing in the library can know which is which, so this hands the actions over rather than guessing.

`lxb_app_driven`

```c
void lxb_app_driven(lxb_app *app);
```

Open the window and run until it is closed. Zero if it ran; non-zero if it could not, and then lxb_app_trouble says why.

`lxb_app_run`

```c
int lxb_app_run(lxb_app *app, lxb_page_fn page, void *data);
```

One settled frame, written to a PNG at `path`, with no display at all. Zero if it was written; non-zero if it was not, and then lxb_app_trouble says why.

The same page function, the same renderer and the same material as lxb_app_run — there is no second drawing path here, which is the only way a picture is worth anything as a check. `seconds` is the clock it is drawn at, so a moving wallpaper can be caught at a chosen moment.

`lxb_app_shot`

```c
int lxb_app_shot(lxb_app *app, const char *path, unsigned int width,
                 unsigned int height, float seconds, lxb_page_fn page,
                 void *data);
```

Why the window could not open, or null if it did. Owned by the application.

`lxb_app_trouble`

```c
const char *lxb_app_trouble(const lxb_app *app);
```

Give the application back. Null is answered by doing nothing.

`lxb_app_free`

```c
void lxb_app_free(lxb_app *app);
```

What a frame is.

`lxb_page_width`

```c
float lxb_page_width(const lxb_page *page);
```

`lxb_page_height`

```c
float lxb_page_height(const lxb_page *page);
```

Seconds since the window opened: the one clock everything with a movement of its own is measured against.

`lxb_page_seconds`

```c
float lxb_page_seconds(const lxb_page *page);
```

The next thing the keyboard or a controller said since the last frame, as an index into lxb_action, or -1 when there is nothing left. Read it in a loop; nothing in it says which control sent it. Always -1 unless the application asked for lxb_app_driven.

`lxb_page_action`

```c
int lxb_page_action(lxb_page *page);
```

How far a wheel or touchpad moved over the spot written down under `id` with lxb_draw_spot, counted in directions and signed downwards. The directions themselves are already waiting in lxb_page_action, so a page that never asks this still scrolls; ask it to decide *which* of the page's own lists this frame's Up and Down are moving, which is the one question a pointer raises and an action cannot answer.

`lxb_page_scrolled`

```c
int lxb_page_scrolled(const lxb_page *page, unsigned int id);
```

Say which control the light is on, by the order it is drawn in. What a page that moves its own selection does before it draws — see lxb_app_driven.

`lxb_page_focus`

```c
void lxb_page_focus(lxb_page *page, unsigned long index);
```

Which control the light is on. The same number a left click reports; the right button raises the context menu, as the Menu key does.

`lxb_page_focused`

```c
unsigned long lxb_page_focused(const lxb_page *page);
```

The lit capsule that says where the light is, drawn travelling. `out` may be null; otherwise it is filled with where the light actually is this frame, which is not where it was asked to be until it arrives. The light is one object crossing the page rather than a property each control has.

For a page that draws its own controls. Draw it *before* the controls it is behind, so their faces sit over it.

`lxb_page_glide`

```c
void lxb_page_glide(lxb_page *page, const float *rect, float strength,
                    float *out);
```

The same, but the light appears where it is asked for rather than flying to it: what a page does when the row the light was on is gone, because a light that travelled there would be saying the two lists are one.

`lxb_page_place`

```c
void lxb_page_place(lxb_page *page, const float *rect, float strength,
                    float *out);
```

How to draw one of your own controls, as an lxb_press: resting, lit, or part-way through a press. `lit` says whether the light is on this one. The page keeps one press, because the light is in one place — a control the light is not on is resting, however many are in flight elsewhere. The flow's own controls do this for themselves; this is for a page that draws its own, which is a page that asked for lxb_app_driven.

`lxb_page_press`

```c
unsigned long lxb_page_press(const lxb_page *page, int lit);
```

Whether a press landed on the spot written down under `id` with lxb_draw_spot, answered once and by that spot alone. The pointer's half of a page that draws its own controls: the numbers are the page's own, so keep them clear of the numbered controls the flow draws, which are counted from zero in the order they are drawn. Only a page that asked for lxb_app_driven gets these.

`lxb_page_pressed`

```c
int lxb_page_pressed(lxb_page *page, unsigned int id);
```

Where the pointer is while a press that began on `id` is still held down: two floats written to `out`, answering 1, or 0 and nothing written. The one gesture a press and a release cannot describe between them — a bar taken hold of and moved. Only a pointer drags; a finger on the same control moves the list instead, which is what a finger does everywhere else on the page.

`lxb_page_dragging`

```c
int lxb_page_dragging(const lxb_page *page, unsigned int id, float *out);
```

Close the window at the end of this frame.

`lxb_page_quit`

```c
void lxb_page_quit(lxb_page *page);
```

Play one of the language's recordings, by its index in lxb_sound_name's list. The interface's own sounds are played for you; this is for a program with something of its own to say.

`lxb_page_play`

```c
void lxb_page_play(lxb_page *page, unsigned long sound);
```

Turn the interface's sounds down, or off. A level rather than a switch, because that is what a person has. `value` is 0 to 1 and is read as loudness rather than as amplitude.

`lxb_page_volume`

```c
void lxb_page_volume(lxb_page *page, float value, int muted);
```

Where the flow has got to: `out` is filled with x, y, width and height.

`lxb_page_cursor`

```c
void lxb_page_cursor(const lxb_page *page, float *out);
```

Put the flow somewhere else: it lays out inside `rect` from its top.

`lxb_page_set_cursor`

```c
void lxb_page_set_cursor(lxb_page *page, const float *rect);
```

The page's own name, at the top of it.

`lxb_page_title`

```c
void lxb_page_title(lxb_page *page, const char *text);
```

A name for the group of things under it.

`lxb_page_heading`

```c
void lxb_page_heading(lxb_page *page, const char *text);
```

A sentence, wrapped to the page's width however many lines that takes.

`lxb_page_text`

```c
void lxb_page_text(lxb_page *page, const char *text);
```

The same, quieter: a note about the thing above it rather than the thing itself.

`lxb_page_note`

```c
void lxb_page_note(lxb_page *page, const char *text);
```

One of the shell's own marks, at the size a list uses.

`lxb_page_icon`

```c
void lxb_page_icon(lxb_page *page, const char *name);
```

A mark and the page's name on one line, which is how a page in this language introduces itself.

`lxb_page_head`

```c
void lxb_page_head(lxb_page *page, const char *icon, const char *title);
```

The barely-there hairline the shell groups with. A grouping, not a border.

`lxb_page_rule`

```c
void lxb_page_rule(lxb_page *page);
```

One gap's worth of air, for a page that wants a break in it.

`lxb_page_gap`

```c
void lxb_page_gap(lxb_page *page);
```

A button, as wide as its own label. Non-zero on the frame a press lands on it, whether that press came from a key, a controller or a click.

`lxb_page_button`

```c
int lxb_page_button(lxb_page *page, const char *label);
```

A row: the whole width of the page, its label on the left. What a list of things to choose between is made of.

`lxb_page_row`

```c
int lxb_page_row(lxb_page *page, const char *label);
```

One entry of a list: its label, and the lit capsule behind it when the light is on it — and nothing at all when it is not.

The other half of the pair with lxb_page_row, and the difference is what the list is for. A row is a control, so it wears a chip whether or not anything is on it: it says *this can be pressed*. An entry is one of many, and a column of chips says nothing except that there are a lot of them.

`lxb_page_item`

```c
int lxb_page_item(lxb_page *page, const char *label);
```

The same, with what it is currently set to on the right of it — how this language writes a setting: the name and the answer on one line, and pressing it is how the answer is changed.

`lxb_page_row_value`

```c
int lxb_page_row_value(lxb_page *page, const char *label, const char *value);
```

Where the light is standing, for a page that draws its own controls: four floats, x, y, width, height. `lxb_page` records this for its own rows and buttons as they are drawn, and a menu grows out of it; a page that lays out its own cards has to say so itself, or a menu raised over one grows out of the corner of the window.

`lxb_page_light_at`

```c
void lxb_page_light_at(lxb_page *page, const float *rect);
```

Raise a context menu over the control the light is on. `title` may be null for a menu that is about nothing in particular. Nothing happens if one is already up.

`lxb_page_menu`

```c
void lxb_page_menu(lxb_page *page, const char *title,
                   const char *const *commands, unsigned long count);
```

The same menu, with the one command already in force wearing the language's own chosen mark. A menu of alternatives that does not say which one you are on is a menu somebody has to press to find out. `marked` indexes `commands`; anything past the end marks nothing, which is what `lxb_page_menu` passes.

`lxb_page_menu_marked`

```c
void lxb_page_menu_marked(lxb_page *page, const char *title,
                          const char *const *commands, unsigned long count,
                          unsigned long marked);
```

Which command was pressed, on the frame it was pressed, or -1 for none.

`lxb_page_chose`

```c
int lxb_page_chose(lxb_page *page);
```

Ask a question. Modal: while it is up it owns every control, which is what makes it a question. Nothing happens if one is already up.

`lxb_page_ask`

```c
void lxb_page_ask(lxb_page *page, const char *title, const char *body,
                  const char *const *answers, unsigned long count);
```

Which answer was given, on the frame it was given, or -1 for none.

`lxb_page_answered`

```c
int lxb_page_answered(lxb_page *page);
```

Ask for one file, or for a folder, starting at `directory`. The desktop is asked first: where the session runs an `org.freedesktop.portal.FileChooser` — every desktop does — the question goes through it, so the answer comes from the chooser the rest of the machine uses. Where there is no portal the toolkit puts the question itself, in a centred Lattice chooser covering roughly seventy percent of the app and keeping the page behind strong frost and depth; its directory columns form the path trail and it owns keyboard, controller and pointer input until it is answered or cancelled. Pointer hover is inert, so a row takes one click to focus and another to activate. Controller Accept on Search opens the local keyboard; D-pad/left stick moves, A enters, Start finishes, and B or its hide key returns to the lattice with the query intact. `selection` is one of LXB_PICKER_FILE, LXB_PICKER_IMAGE, LXB_PICKER_SCENERY or LXB_PICKER_FOLDER from lxb_toolkit.h, and decides both what is listed and which kinds of file a portal chooser offers. Setting LXB_FILE_PORTAL=0 in the environment uses the toolkit's own chooser always. Zero means another panel or an unanswered question already owns input, the selection was unknown, or `directory` was null.

`lxb_page_pick`

```c
int lxb_page_pick(lxb_page *page, unsigned long selection,
                  const char *directory);
```

Ask for any number of files at once, the same two ways. The answer may be several paths; read it with lxb_page_picked_next. In the toolkit's own chooser each file is ticked with Accept and the row at the head of the column ends the question.

`lxb_page_pick_many`

```c
int lxb_page_pick_many(lxb_page *page, unsigned long selection,
                       const char *directory);
```

Ask where to write a file and what to call it, the same two ways. `name` is what it is called to begin with and may be empty. The answer is one path that need not exist yet; read it with lxb_page_picked.

`lxb_page_save`

```c
int lxb_page_save(lxb_page *page, const char *name, const char *directory);
```

The accepted path since the last frame, once, or null after a cancellation or when there is no new answer. The whole answer is taken: where several files were chosen this is the first of them and the rest are dropped. The returned UTF-8 is owned by the caller; release it with lxb_app_string_free.

`lxb_page_picked`

```c
char *lxb_page_picked(lxb_page *page);
```

The next file of the accepted answer, or null when there are none left. Call it until it answers null to read every file a lxb_page_pick_many question was answered with. Each returned UTF-8 is owned by the caller; release it with lxb_app_string_free.

`lxb_page_picked_next`

```c
char *lxb_page_picked_next(lxb_page *page);
```

Release a path returned by lxb_page_picked. NULL is accepted. This is kept in liblxb_app rather than lxb_string_free in liblxb_toolkit because the two shared libraries deliberately have separate allocation boundaries.

`lxb_app_string_free`

```c
void lxb_app_string_free(char *text);
```

A sheet of the shell's glass. `overlay` indexes lxb_overlay.

`lxb_draw_pane`

```c
void lxb_draw_pane(lxb_page *page, const float *rect, unsigned long overlay);
```

One of the three cuts of glass, tinted by a role.

`lxb_draw_card`

```c
void lxb_draw_card(lxb_page *page, const float *rect, unsigned long surface,
                   unsigned long role, float alpha);
```

One line of writing.

`lxb_draw_label`

```c
void lxb_draw_label(lxb_page *page, const float *rect, unsigned long text,
                    const char *string, unsigned long role, unsigned long align);
```

A sentence, wrapped to the rectangle's width. Answers how tall it came out, so whatever is under it can be placed.

`lxb_draw_paragraph`

```c
float lxb_draw_paragraph(lxb_page *page, const float *rect, const char *string,
                         unsigned long role);
```

One of the shell's own marks, shaded into a bead of water out of its own shape. The style is the shell's current one.

`lxb_draw_icon`

```c
void lxb_draw_icon(lxb_page *page, const float *rect, const char *name);
```

The hairline this language groups with.

`lxb_draw_rule`

```c
void lxb_draw_rule(lxb_page *page, const float *rect, unsigned long role);
```

A button at a rectangle of your own choosing. For anything a person can move onto, prefer lxb_page_button, which numbers it and answers the pointer.

`lxb_draw_button`

```c
void lxb_draw_button(lxb_page *page, const float *rect, const char *label,
                     unsigned long press);
```

A row at a rectangle of your own choosing. `value` may be null.

`lxb_draw_row`

```c
void lxb_draw_row(lxb_page *page, const float *rect, const char *name,
                  const char *value, unsigned long press);
```

The lit capsule that says where the light is, without a control under it.

`lxb_draw_selection`

```c
void lxb_draw_selection(lxb_page *page, const float *rect, float strength);
```

Write down where something you drew yourself went, so that a pointer over it can be answered. `id` comes back from lxb_page_at.

`lxb_draw_spot`

```c
void lxb_draw_spot(lxb_page *page, unsigned int id, const float *rect);
```

Blur and dissolve the top and bottom ends of a scrolling area, after everything inside it has been drawn: surfaces and words soften together as one picture, the middle is untouched, and a menu or dialog still stands clear over it. Blur and translucency grow together, and the last of the fade is exactly what is behind the page's own content there, so a list stops without anything to stop at. `band` is the feather in points; `top` and `bottom` are how much really continues past each end, and an end with nothing past it should be nought so that a finished list looks finished rather than permanently fogged.

`lxb_draw_soft_edges`

```c
void lxb_draw_soft_edges(lxb_page *page, const float *rect, float band,
                         float top, float bottom);
```

What is at a point of the frame that is on screen: the id of the thing there, or -1 for nothing. Read from the last frame drawn, because a pointer event is about the picture the person could see.

`lxb_page_at`

```c
int lxb_page_at(const lxb_page *page, float x, float y);
```

One of the language's named lengths. `metric` indexes lxb_metric.

`lxb_page_metric`

```c
float lxb_page_metric(const lxb_page *page, unsigned long metric);
```

A length written against the 1080p reference, scaled to this window.

`lxb_page_scaled`

```c
float lxb_page_scaled(const lxb_page *page, float reference);
```

How tall one line of a size is, air included. `text` indexes lxb_text.

`lxb_page_line`

```c
float lxb_page_line(const lxb_page *page, unsigned long text);
```

How wide a string is at a size, in this window's pixels. Shaped by the same engine that will draw it, so it is the width it will actually take.

`lxb_page_measure`

```c
float lxb_page_measure(lxb_page *page, unsigned long text, const char *string);
```
