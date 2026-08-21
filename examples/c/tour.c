/*
 * A tour of the LineXinBar design language, in C.
 *
 * Seven pages: what the language answers, its colour, its material, its marks,
 * its type, its motion and its sounds. Every one of them is drawn by the
 * shell's own renderer on the GPU — the same wallpaper shader, the same glass
 * shader and the same glyph shader the shell itself runs — through two
 * libraries and one page function.
 *
 *     make tour && ./tour                 # the window
 *     ./tour --shot tour.png              # one frame of it, with no display
 *     ./tour --shot tour.png Colour
 *
 * Up and Down walk the pages; Left and Right walk within one. Enter presses;
 * Escape leaves; the Menu key or the right mouse button raises a context
 * menu. A controller does all four, and so does a pointer.
 *
 * This application asks to drive itself — lxb_app_driven — because it is a
 * screen rather than a list: it has two axes, and only the application knows
 * which is which. An application that is one list of things has none of this
 * and is forty lines; see hello.c.
 *
 * It is a transcription of examples/python/tour.py, line for line, and the two
 * are compared frame for frame: see examples/README.md.
 */

#include <math.h>
#include <stdio.h>
#include <string.h>
#include <strings.h>

#include <lxb_app.h>

#define COUNT(array) ((long)(sizeof(array) / sizeof((array)[0])))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define MAX(a, b) ((a) > (b) ? (a) : (b))

static const char *const SECTIONS[]
    = { "Hello", "Colour", "Material", "Marks", "Type", "Motion", "Sound" };

#define GREETING "Hello, world"

/* Six of the thirty-four durations, chosen because each is a different kind of
 * arrival. */
static const struct {
    const char *name;
    const char *what;
} SHOWN_DURATIONS[] = {
    { "panel", "a panel arrives" },
    { "flight", "a card flies" },
    { "accent-change", "the accent travels" },
    { "guide-press", "a control is pressed" },
    { "scenery-fade", "a scene crossfades" },
    { "launch-open", "an application opens" },
};

static const char *const MENU_ROWS[] = { "Simple marks", "Ask a question",
                                         "Copy this page's values", "Close" };

/* The whole of the application: which page, and where in it. */
struct tour {
    long section;
    /* Which page the light is on, so it appears on a new one rather than
     * flying across from the last. */
    long lit_section;
    long cursor[COUNT(SECTIONS)];
    lxb_shell_theme theme;
    long palette;
    int asked;
    /* A panel to raise on the first frame, for a picture of one. */
    const char *raise_panel;
};

/* How many things the current page has to walk between. */
/* Where this page's own items are, so a pointer over one can be answered.
 * The numbers are this program's, and are kept clear of the sidebar's, which
 * lxb_page_item numbers from zero in the order it draws them. */
#define ITEM_SPOT 0x200u

static long page_count(const struct tour *tour)
{
    switch (tour->section) {
    case 1:
        return (long)lxb_palette_count();
    case 2:
        return (long)lxb_surface_count();
    case 3:
        return (long)lxb_glyph_count();
    case 4:
        return (long)lxb_text_count();
    case 5:
        return COUNT(SHOWN_DURATIONS);
    case 6:
        return (long)lxb_sound_count();
    default:
        return 1;
    }
}

static long at(const struct tour *tour)
{
    return tour->cursor[tour->section];
}

/* A library name as a heading. The names are the keys a value is stored under,
 * so they are lowercase; what is written at the head of a card is not. One
 * buffer, because only one of these is ever on screen at a time. */
static const char *capitalised(const char *name)
{
    static char said[32];
    snprintf(said, sizeof said, "%s", name);
    if (said[0] >= 'a' && said[0] <= 'z') {
        said[0] = (char)(said[0] - 'a' + 'A');
    }
    return said;
}

/* A rectangle on its way across, in double and cast once.
 *
 * Every length here is worked out in double and narrowed exactly once, where it
 * crosses. Doing the arithmetic in float instead rounds at every step, and the
 * frames stop matching Python's — which does all of its arithmetic in double —
 * by one part in ten million. That is invisible on screen and fatal to the
 * comparison the two greetings exist to make. */
static void rect_of(float out[4], double x, double y, double w, double h)
{
    out[0] = (float)x;
    out[1] = (float)y;
    out[2] = (float)w;
    out[3] = (float)h;
}

/* -- what the controls mean ----------------------------------------------- */

static void walk_page(struct tour *tour, lxb_page *page, long delta)
{
    long want = MIN(MAX(tour->section + delta, 0), COUNT(SECTIONS) - 1);
    if (want != tour->section) {
        tour->section = want;
        lxb_page_play(page, LXB_SOUND_MOVE);
    }
}

static void walk(struct tour *tour, lxb_page *page, long delta)
{
    long want = MIN(MAX(at(tour) + delta, 0), page_count(tour) - 1);
    if (want != at(tour)) {
        tour->cursor[tour->section] = want;
        lxb_page_play(page, LXB_SOUND_MOVE);
        /* The recordings page plays what it is on rather than what has been
         * chosen: walking previews, and Enter plays it again. */
        if (tour->section == 6) {
            lxb_page_play(page, (unsigned long)want);
        }
    }
}

/* What pressing this page's selection does, without the click it is answered
 * with: a pointer's press has already been sounded by the time the page hears
 * about it, and a control that said it twice would be saying the pointer is a
 * different kind of thing. */
static void apply(struct tour *tour, lxb_page *page)
{
    if (tour->section == 1) {
        tour->palette = at(tour);
    } else if (tour->section == 6) {
        lxb_page_play(page, (unsigned long)at(tour));
    }
}

static void press(struct tour *tour, lxb_page *page)
{
    lxb_page_play(page, LXB_SOUND_PRESS);
    apply(tour, page);
}

/* One action, whichever control sent it. The single place it happens, so that
 * the same move made two ways cannot end up doing two different things. */
static void act(struct tour *tour, lxb_page *page, int action)
{
    switch (action) {
    case LXB_ACTION_UP:
    case LXB_ACTION_PREVIOUS:
        walk_page(tour, page, -1);
        break;
    case LXB_ACTION_DOWN:
    case LXB_ACTION_NEXT:
        walk_page(tour, page, 1);
        break;
    case LXB_ACTION_LEFT:
        walk(tour, page, -1);
        break;
    case LXB_ACTION_RIGHT:
        walk(tour, page, 1);
        break;
    case LXB_ACTION_ACCEPT:
    case LXB_ACTION_SUBMIT:
        press(tour, page);
        break;
    case LXB_ACTION_MENU:
        lxb_page_menu(page, SECTIONS[tour->section], MENU_ROWS,
                      (unsigned long)COUNT(MENU_ROWS));
        break;
    case LXB_ACTION_BACK: {
        /* Nothing left to step out of, so this is the way out — asked rather
         * than taken. */
        static const char *const answers[] = { "Leave", "Stay" };
        lxb_page_ask(page, "Leave the tour?",
                     "The window closes. Nothing here is saved, because "
                     "nothing here is yours: every value on these pages is the "
                     "toolkit's answer, not this program's.",
                     answers, 2);
        tour->asked = 1;
        break;
    }
    default:
        break;
    }
}

/* -- the seven pages ------------------------------------------------------ */

static void head(lxb_page *page, const char *title, const char *mark)
{
    float cursor[4], rule[4];
    if (mark == NULL) {
        lxb_page_heading(page, title);
    } else {
        lxb_page_head(page, mark, title);
    }
    lxb_page_cursor(page, cursor);
    rect_of(rule, cursor[0], cursor[1] - lxb_page_metric(page, LXB_METRIC_GAP) * 0.5,
            cursor[2], MAX(lxb_page_scaled(page, 2.0f), 1.0f));
    lxb_draw_rule(page, rule, LXB_ROLE_ACCENT_SOFT);
}

static void page_hello(struct tour *tour, lxb_page *page)
{
    char value[160];
    float cursor[4], box[4];
    double column = lxb_page_metric(page, LXB_METRIC_COLUMN_SPACING);
    double line = lxb_page_line(page, LXB_TEXT_BODY);
    unsigned long accent = (unsigned long)tour->palette;

    head(page, GREETING, "launch");
    for (int index = 0; index < 6; index++) {
        const char *name = NULL;
        switch (index) {
        case 0:
            name = "theme";
            snprintf(value, sizeof value, "%s, %s wallpaper, %s marks",
                     lxb_palette_name(accent),
                     tour->theme.wallpaper == 0 ? "Default" : "Simple",
                     tour->theme.icons == 0 ? "Default" : "Simple");
            break;
        case 1:
            name = "colour";
            snprintf(value, sizeof value, "text #%06x over sky-top #%06x",
                     lxb_palette_color(accent, LXB_ROLE_TEXT) & 0xffffffu,
                     lxb_palette_color(accent, LXB_ROLE_SKY_TOP) & 0xffffffu);
            break;
        case 2:
            name = "size";
            snprintf(value, sizeof value, "title %.0fpx in a %.0fpx row",
                     lxb_page_line(page, LXB_TEXT_TITLE),
                     lxb_page_metric(page, LXB_METRIC_ROW_HEIGHT));
            break;
        case 3:
            name = "motion";
            snprintf(value, sizeof value,
                     "a panel arrives over %.0fms, and never linearly",
                     lxb_duration_named("panel") * 1000.0f);
            break;
        case 4:
            name = "mark";
            snprintf(value, sizeof value, "launch, drawn in a 32-unit square");
            break;
        default:
            name = "sound";
            snprintf(value, sizeof value, "press-selected, %lu recordings in all",
                     lxb_sound_count());
            break;
        }
        lxb_page_cursor(page, cursor);
        rect_of(box, cursor[0], cursor[1], column, line);
        lxb_draw_label(page, box, LXB_TEXT_BODY, name, LXB_ROLE_TEXT, LXB_ALIGN_LEFT);
        rect_of(box, cursor[0] + column, cursor[1], cursor[2] - column, line);
        lxb_draw_label(page, box, LXB_TEXT_CAPTION, value, LXB_ROLE_TEXT_SOFT,
                       LXB_ALIGN_LEFT);
        rect_of(box, cursor[0], cursor[1] + line, cursor[2], 0.0);
        lxb_page_set_cursor(page, box);
    }
    lxb_page_gap(page);
    lxb_page_note(page, "Every value on this page was named rather than picked, "
                        "and every one of them depends on the height it was "
                        "asked at — resize the window and read them again.");
    lxb_page_text(page, "Everything here is the shell's own material: the water "
                        "behind it is the wallpaper shader, every pane bends "
                        "what is behind it through the glass shader, and the "
                        "mark above is a distance field shaded into a bead of "
                        "water by the glyph shader. This page asks for them by "
                        "name and never touches one.");
}

/* The lit capsule, travelling. A light that has never been on this page
 * appears where it is asked for rather than flying in from the last page's
 * row: the two are different lists, and a light crossing between them would
 * be saying they are one. */
static void light(struct tour *tour, lxb_page *page, const float rect[4])
{
    if (tour->lit_section == tour->section) {
        lxb_page_glide(page, rect, 1.0f, NULL);
    } else {
        lxb_page_place(page, rect, 1.0f, NULL);
        tour->lit_section = tour->section;
    }
}

/* A row of capsules, one of them lit: what choosing between a handful of
 * things looks like in this language. */
static void chips(struct tour *tour, lxb_page *page, const char *const *names,
                  long count, long chosen)
{
    float cursor[4], box[4], rects[8][4];
    double gap = lxb_page_metric(page, LXB_METRIC_GAP) * 0.5;
    double height = lxb_page_scaled(page, 44.0f);
    double left;

    lxb_page_cursor(page, cursor);
    left = cursor[0];
    for (long index = 0; index < count; index++) {
        double room = lxb_page_measure(page, LXB_TEXT_LABEL, names[index])
                      + 2.0 * lxb_page_metric(page, LXB_METRIC_ROW_PADDING);
        rect_of(rects[index], left, cursor[1], room, height);
        left += room + gap;
    }

    /* The light first, and the faces over it: one object crossing the row
     * rather than a property each capsule has, so walking the row is the light
     * travelling and not five capsules taking turns. */
    light(tour, page, rects[chosen]);
    for (long index = 0; index < count; index++) {
        lxb_draw_spot(page, ITEM_SPOT + (unsigned int)index, rects[index]);
        /* Asked of the page rather than decided here: a control the light is
         * on is part-way through a press whenever one is in flight, and this
         * program has no way of its own to know that. */
        lxb_draw_button(page, rects[index], names[index],
                        lxb_page_press(page, index == chosen));
    }
    rect_of(box, cursor[0], cursor[1] + height + gap, cursor[2], 0.0);
    lxb_page_set_cursor(page, box);
}

static void page_colour(struct tour *tour, lxb_page *page)
{
    char hex[16];
    const char *names[8];
    float cursor[4], box[4];
    double gap, step, chip, column;
    long roles = (long)lxb_role_count();
    long per = (roles + 1) / 2;
    long palette = tour->cursor[1];

    head(page, "Colour", NULL);
    lxb_page_cursor(page, cursor);
    gap = lxb_page_metric(page, LXB_METRIC_GAP);
    step = lxb_page_line(page, LXB_TEXT_BODY) * 1.35;
    chip = lxb_page_metric(page, LXB_METRIC_TILE);
    column = (cursor[2] - gap) / 2.0;

    for (long index = 0; index < roles; index++) {
        double left = cursor[0] + (double)(index / per) * (column + gap);
        double top = cursor[1] + (double)(index % per) * step;
        double height = chip * 0.42;
        rect_of(box, left, top + (step - height) / 2.0, chip, height);
        lxb_draw_card(page, box, LXB_SURFACE_CONTROL, (unsigned long)index, 1.0f);
        rect_of(box, left + chip + gap * 0.5, top, column, step);
        lxb_draw_label(page, box, LXB_TEXT_BODY, lxb_role_name((unsigned long)index),
                       LXB_ROLE_TEXT, LXB_ALIGN_LEFT);
        snprintf(hex, sizeof hex, "#%06x",
                 lxb_palette_color((unsigned long)palette, (unsigned long)index)
                     & 0xffffffu);
        rect_of(box, left, top, column, step);
        lxb_draw_label(page, box, LXB_TEXT_CAPTION, hex, LXB_ROLE_TEXT_SOFT,
                       LXB_ALIGN_RIGHT);
    }

    rect_of(box, cursor[0], cursor[1] + (double)per * step + gap, cursor[2], 0.0);
    lxb_page_set_cursor(page, box);
    for (long index = 0; index < (long)lxb_palette_count() && index < 8; index++) {
        names[index] = lxb_palette_name((unsigned long)index);
    }
    chips(tour, page, names, MIN((long)lxb_palette_count(), 8), palette);
    lxb_page_gap(page);
    {
        char said[256];
        snprintf(said, sizeof said,
                 "%s is what the shell has. Left and Right choose, Enter "
                 "applies — and the %lu colours above travel to their new "
                 "values over %.0fms, in linear light.",
                 lxb_palette_name((unsigned long)tour->palette), lxb_role_count(),
                 lxb_duration_named("accent-change") * 1000.0f);
        lxb_page_text(page, said);
    }
}

static void page_material(struct tour *tour, lxb_page *page)
{
    char value[24];
    float cursor[4], box[4];
    double gap, card, line, height;
    long surfaces = (long)lxb_surface_count();

    head(page, "Material", NULL);
    lxb_page_cursor(page, cursor);
    gap = lxb_page_metric(page, LXB_METRIC_GAP);
    card = (cursor[2] - 2.0 * gap) / (double)surfaces;
    line = lxb_page_line(page, LXB_TEXT_CAPTION);
    height = lxb_page_line(page, LXB_TEXT_TITLE) + 4.0 * line + 2.0 * gap;

    for (long index = 0; index < surfaces; index++) {
        lxb_glass glass = lxb_surface_glass((unsigned long)index);
        double left = cursor[0] + (double)index * (card + gap);
        int lit = index == tour->cursor[2];
        double top;
        rect_of(box, left, cursor[1], card, height);
        lxb_draw_spot(page, ITEM_SPOT + (unsigned int)index, box);
        lxb_draw_card(page, box, (unsigned long)index,
                      lit ? LXB_ROLE_ACCENT_SOFT : LXB_ROLE_GLASS, lit ? 1.0f : 0.8f);
        rect_of(box, left + gap, cursor[1] + gap * 0.5, card - 2.0 * gap,
                lxb_page_line(page, LXB_TEXT_TITLE));
        lxb_draw_label(page, box, LXB_TEXT_TITLE, capitalised(lxb_surface_name(
                                                      (unsigned long)index)),
                       LXB_ROLE_TEXT, LXB_ALIGN_LEFT);
        top = cursor[1] + gap * 0.5 + lxb_page_line(page, LXB_TEXT_TITLE);
        for (int which = 0; which < 4; which++) {
            static const char *const names[] = { "depth", "frost", "gloss", "curve" };
            switch (which) {
            case 0:
                snprintf(value, sizeof value, "%.0fpx", (double)glass.depth);
                break;
            case 1:
                snprintf(value, sizeof value, "%.2f", (double)glass.frost);
                break;
            case 2:
                snprintf(value, sizeof value, "%.2f", (double)glass.gloss);
                break;
            default:
                snprintf(value, sizeof value, "%.0f", (double)glass.curve);
                break;
            }
            rect_of(box, left + gap, top, card - 2.0 * gap, line);
            lxb_draw_label(page, box, LXB_TEXT_CAPTION, names[which],
                           LXB_ROLE_TEXT_SOFT, LXB_ALIGN_LEFT);
            lxb_draw_label(page, box, LXB_TEXT_CAPTION, value, LXB_ROLE_TEXT,
                           LXB_ALIGN_RIGHT);
            top += line;
        }
    }

    rect_of(box, cursor[0], cursor[1] + height + gap, cursor[2], 0.0);
    lxb_page_set_cursor(page, box);
    lxb_page_text(page, "Three cuts, and everything is made of one of them: a "
                        "compact slab for a panel, a clear lozenge for what you "
                        "can act on, and a broad shallow sheet for a sidebar. "
                        "How much each one shows of the page it is standing on "
                        "is the material rather than an alpha.");
    lxb_page_note(page, "The two panes on this screen are the fourth thing: the "
                        "layered menu-and-dialog material — a stain, two lights "
                        "under it and a hairline over it. Every one of them "
                        "refracts what is behind it.");
}

static void page_marks(struct tour *tour, lxb_page *page)
{
    char said[192];
    float cursor[4], box[4];
    double gap, cell;
    long across, rows, chosen = tour->cursor[3];
    long marks = (long)lxb_glyph_count();

    head(page, "Marks", NULL);
    lxb_page_cursor(page, cursor);
    gap = lxb_page_metric(page, LXB_METRIC_GAP) * 0.5;
    cell = lxb_page_metric(page, LXB_METRIC_ITEM_ICON) + gap;
    across = MAX((long)(cursor[2] / cell), 1);

    for (long index = 0; index < marks; index++) {
        double left = cursor[0] + (double)(index % across) * cell;
        double top = cursor[1] + (double)(index / across) * cell;
        if (index == chosen) {
            rect_of(box, left - gap * 0.5, top - gap * 0.5, cell, cell);
            light(tour, page, box);
        }
        rect_of(box, left - gap * 0.5, top - gap * 0.5, cell, cell);
        lxb_draw_spot(page, ITEM_SPOT + (unsigned int)index, box);
        rect_of(box, left, top, cell - gap, cell - gap);
        lxb_draw_icon(page, box, lxb_glyph_name((unsigned long)index));
    }

    rows = (marks + across - 1) / across;
    rect_of(box, cursor[0], cursor[1] + (double)rows * cell + gap, cursor[2], 0.0);
    lxb_page_set_cursor(page, box);
    lxb_page_heading(page, lxb_glyph_name((unsigned long)chosen));
    snprintf(said, sizeof said,
             "All %ld of the shell's marks, shipped in the library. Each is a "
             "shape, and the shader makes it a bead of water: the same one the "
             "shell draws, at whatever size it is asked for.",
             marks);
    lxb_page_note(page, said);
}

static void page_type(struct tour *tour, lxb_page *page)
{
    char said[96];
    float cursor[4], box[4];
    double gap, top;
    long sizes = (long)lxb_text_count();

    head(page, "Type", NULL);
    lxb_page_cursor(page, cursor);
    gap = lxb_page_metric(page, LXB_METRIC_GAP);
    top = cursor[1];

    for (long index = 0; index < sizes; index++) {
        double line = lxb_page_line(page, (unsigned long)index);
        if (index == tour->cursor[4]) {
            rect_of(box, cursor[0] - gap * 0.5, top, cursor[2] + gap, line);
            light(tour, page, box);
        }
        rect_of(box, cursor[0] - gap * 0.5, top, cursor[2] + gap, line);
        lxb_draw_spot(page, ITEM_SPOT + (unsigned int)index, box);
        rect_of(box, cursor[0], top, cursor[2], line);
        lxb_draw_label(page, box, (unsigned long)index,
                       capitalised(lxb_text_name((unsigned long)index)),
                       LXB_ROLE_TEXT, LXB_ALIGN_LEFT);
        snprintf(said, sizeof said, "%.0fpx, line %.0fpx, %s",
                 lxb_text_size((unsigned long)index, lxb_page_height(page)), line,
                 lxb_text_is_bold((unsigned long)index) ? "bold" : "regular");
        /* Against the far edge rather than in a second column: Display is three
         * times the height of Caption, so no one column start clears every name
         * on this page. */
        rect_of(box, cursor[0], top, cursor[2] - gap, line);
        lxb_draw_label(page, box, LXB_TEXT_CAPTION, said, LXB_ROLE_TEXT_SOFT,
                       LXB_ALIGN_RIGHT);
        top += line + gap * 0.5;
    }

    rect_of(box, cursor[0], top + gap * 0.5, cursor[2], 0.0);
    lxb_page_set_cursor(page, box);
    lxb_page_text(page, "Five sizes and no others, each a share of the display's "
                        "height. The face is shipped with the library, so a "
                        "machine with no fonts installed draws exactly this.");
}

static void page_motion(struct tour *tour, lxb_page *page)
{
    char said[256];
    float cursor[4], box[4];
    double gap, line, column, caption, track, bead, top;

    head(page, "Motion", NULL);
    lxb_page_cursor(page, cursor);
    gap = lxb_page_metric(page, LXB_METRIC_GAP);
    line = lxb_page_line(page, LXB_TEXT_BODY);
    column = lxb_page_metric(page, LXB_METRIC_COLUMN_SPACING);
    /* The bar and what it says are two columns, not one: a caption laid over
     * the track it describes is a caption with a bead running through it twice
     * a second. */
    caption = column * 1.4;
    track = cursor[2] - column - caption - gap;
    bead = lxb_page_scaled(page, 14.0f);
    top = cursor[1];

    for (long index = 0; index < COUNT(SHOWN_DURATIONS); index++) {
        double over = lxb_duration_named(SHOWN_DURATIONS[index].name);
        /* Each runs on its own clock, over and back, so the whole page is one
         * loop and every bar is the same journey at its own length. */
        double through = fmod(lxb_page_seconds(page), over * 2.0) / over;
        double travelled = lxb_ease((float)(through <= 1.0 ? through : 2.0 - through));
        int lit = index == tour->cursor[5];

        rect_of(box, cursor[0], top, cursor[2], line);
        lxb_draw_spot(page, ITEM_SPOT + (unsigned int)index, box);
        rect_of(box, cursor[0], top, column, line);
        lxb_draw_label(page, box, LXB_TEXT_BODY, SHOWN_DURATIONS[index].name,
                       LXB_ROLE_TEXT, LXB_ALIGN_LEFT);
        rect_of(box, cursor[0] + column, top + line * 0.5, track,
                MAX(lxb_page_scaled(page, 2.0f), 1.0f));
        lxb_draw_rule(page, box, LXB_ROLE_ACCENT_SOFT);
        rect_of(box, cursor[0] + column + travelled * (track - bead),
                top + (line - bead) * 0.5, bead, bead);
        lxb_draw_card(page, box, LXB_SURFACE_CONTROL,
                      lit ? LXB_ROLE_ACCENT : LXB_ROLE_ACCENT_SOFT, 1.0f);
        snprintf(said, sizeof said, "%.0fms, %s", over * 1000.0f,
                 SHOWN_DURATIONS[index].what);
        rect_of(box, cursor[0] + column + track + gap, top, caption, line);
        lxb_draw_label(page, box, LXB_TEXT_CAPTION, said, LXB_ROLE_TEXT_SOFT,
                       LXB_ALIGN_RIGHT);
        top += line + gap * 0.5;
    }

    rect_of(box, cursor[0], top + gap * 0.5, cursor[2], 0.0);
    lxb_page_set_cursor(page, box);
    snprintf(said, sizeof said,
             "%lu named durations, and not one of them is linear: everything in "
             "this language leaves and arrives gently, because nothing physical "
             "starts at full speed.",
             lxb_duration_count());
    lxb_page_text(page, said);
}

static void page_sound(struct tour *tour, lxb_page *page)
{
    float cursor[4], box[4];
    double row, pad, top;
    long chosen = tour->cursor[6];

    head(page, "Sound", NULL);
    lxb_page_cursor(page, cursor);
    row = lxb_page_metric(page, LXB_METRIC_ROW_HEIGHT) * 0.7;
    pad = lxb_page_metric(page, LXB_METRIC_ROW_PADDING);
    top = cursor[1];

    for (long index = 0; index < (long)lxb_sound_count(); index++) {
        int lit = index == chosen;
        /* The light and nothing else: a column of fourteen chips says only that
         * there are fourteen of them. This is what lxb_page_item draws, laid
         * out here because the page moves its own selection. */
        if (lit) {
            rect_of(box, cursor[0], top, cursor[2], row);
            light(tour, page, box);
        }
        rect_of(box, cursor[0], top, cursor[2], row);
        lxb_draw_spot(page, ITEM_SPOT + (unsigned int)index, box);
        rect_of(box, cursor[0] + pad, top, cursor[2] - 2.0 * pad, row);
        lxb_draw_label(page, box, LXB_TEXT_BODY, lxb_sound_name((unsigned long)index),
                       lit ? LXB_ROLE_TEXT : LXB_ROLE_TEXT_SOFT, LXB_ALIGN_LEFT);
        top += row;
    }

    rect_of(box, cursor[0], top + lxb_page_metric(page, LXB_METRIC_GAP), cursor[2],
            0.0);
    lxb_page_set_cursor(page, box);
    lxb_page_text(page, "Fourteen recordings, shipped with the library. Left and "
                        "Right walk them and each one plays as it is reached; "
                        "Enter plays it again. No clip is ever laid over a copy "
                        "of itself.");
}

/* -- the screen ----------------------------------------------------------- */

/* The pages, as a column beside the page they are about.
 *
 * These are the application's only numbered controls, so the light is told
 * where it is — the tour moves its own selection — and a click on one of them
 * comes back as a press on that row. */
static void draw_sidebar(struct tour *tour, lxb_page *page, const float rect[4])
{
    static const struct {
        const char *name;
        const char *what;
    } KEYS[] = {
        { "Up / Down", "the page" },   { "Left / Right", "within it" },
        { "Enter", "act on it" },      { "Menu", "a context menu" },
        { "Escape", "close, or leave" },
    };
    char version[64];
    float box[4];
    double pad = lxb_page_metric(page, LXB_METRIC_PANEL_PADDING);
    double left, line, foot;

    lxb_draw_pane(page, rect, LXB_OVERLAY_DIALOG);
    lxb_page_focus(page, (unsigned long)tour->section);
    rect_of(box, rect[0] + pad, rect[1] + pad, rect[2] - 2.0 * pad,
            rect[3] - 2.0 * pad);
    lxb_page_set_cursor(page, box);

    snprintf(version, sizeof version, "lxb-toolkit %s", lxb_version());
    lxb_page_note(page, version);
    for (long index = 0; index < COUNT(SECTIONS); index++) {
        if (lxb_page_item(page, SECTIONS[index])) {
            tour->section = index;
        }
    }

    /* The keys, at the foot of it. Written down rather than discovered, because
     * an interface driven three ways has to say so. */
    left = lxb_page_measure(page, LXB_TEXT_LABEL, "Left / Right")
           + lxb_page_metric(page, LXB_METRIC_GAP);
    line = lxb_page_line(page, LXB_TEXT_LABEL);
    foot = rect[1] + rect[3] - pad - (double)COUNT(KEYS) * line;
    for (long index = 0; index < COUNT(KEYS); index++) {
        rect_of(box, rect[0] + pad, foot, rect[2], line);
        lxb_draw_label(page, box, LXB_TEXT_LABEL, KEYS[index].name, LXB_ROLE_TEXT,
                       LXB_ALIGN_LEFT);
        rect_of(box, rect[0] + pad + left, foot, rect[2], line);
        lxb_draw_label(page, box, LXB_TEXT_LABEL, KEYS[index].what,
                       LXB_ROLE_TEXT_SOFT, LXB_ALIGN_LEFT);
        foot += line;
    }
}

static void draw(lxb_page *page, void *data)
{
    struct tour *tour = data;
    double inset, gap, sidebar, tall, x, pad;
    float rect[4], content[4];
    int action, chosen;

    while ((action = lxb_page_action(page)) >= 0) {
        act(tour, page, action);
    }
    if (tour->raise_panel != NULL) {
        act(tour, page,
            strcmp(tour->raise_panel, "menu") == 0 ? LXB_ACTION_MENU
                                                   : LXB_ACTION_BACK);
        tour->raise_panel = NULL;
    }

    /* A press on one of this page's own items. The pointer's half of Left and
     * Right: it carries the selection there and acts on it, which is the two
     * steps a walk and an Enter are, in one. */
    for (long index = 0; index < page_count(tour); index++) {
        if (lxb_page_pressed(page, ITEM_SPOT + (unsigned int)index)) {
            tour->cursor[tour->section] = index;
            apply(tour, page);
            break;
        }
    }

    chosen = lxb_page_chose(page);
    if (chosen == 1) {
        static const char *const answers[] = { "Good", "Close" };
        lxb_page_ask(page, "A question",
                     "This is the shell's own panel: the same glass, the same "
                     "arrival, the same answer capsules.",
                     answers, 2);
    } else if (chosen == 3) {
        lxb_page_quit(page);
    }
    if (tour->asked && lxb_page_answered(page) == 0) {
        lxb_page_quit(page);
    }

    inset = lxb_page_metric(page, LXB_METRIC_PANEL_INSET);
    gap = lxb_page_metric(page, LXB_METRIC_GAP);
    sidebar = MIN(lxb_page_metric(page, LXB_METRIC_MENU_WIDTH),
                  lxb_page_width(page) * 0.34f);
    tall = lxb_page_height(page) - 2.0 * inset;
    rect_of(rect, inset, inset, sidebar, tall);
    draw_sidebar(tour, page, rect);

    x = inset + sidebar + gap;
    rect_of(content, x, inset, lxb_page_width(page) - x - inset, tall);
    lxb_draw_pane(page, content, LXB_OVERLAY_DIALOG);
    pad = lxb_page_metric(page, LXB_METRIC_PANEL_PADDING);
    rect_of(rect, content[0] + pad, content[1] + pad, content[2] - 2.0 * pad,
            content[3] - 2.0 * pad);
    lxb_page_set_cursor(page, rect);

    switch (tour->section) {
    case 1:
        page_colour(tour, page);
        break;
    case 2:
        page_material(tour, page);
        break;
    case 3:
        page_marks(tour, page);
        break;
    case 4:
        page_type(tour, page);
        break;
    case 5:
        page_motion(tour, page);
        break;
    case 6:
        page_sound(tour, page);
        break;
    default:
        page_hello(tour, page);
        break;
    }
}

int main(int argc, char **argv)
{
    struct tour tour = { 0 };
    lxb_app *app;
    int shot, code;

    tour.theme = lxb_shell_theme_load();
    tour.palette = (long)tour.theme.accent;
    tour.cursor[1] = tour.palette;

    shot = argc > 2 && strcmp(argv[1], "--shot") == 0;
    if (shot && argc > 4) {
        tour.raise_panel = argv[4];
    }
    if (shot && argc > 3) {
        for (long index = 0; index < COUNT(SECTIONS); index++) {
            if (strcasecmp(argv[3], SECTIONS[index]) == 0) {
                tour.section = index;
            }
        }
    }

    app = lxb_app_new("com.example.LxbTour", "lxb-toolkit");
    lxb_app_plain(app);
    lxb_app_driven(app);
    code = shot ? lxb_app_shot(app, argv[2], 1280, 800, 10.0f, draw, &tour)
                : lxb_app_run(app, draw, &tour);
    if (code != 0) {
        fprintf(stderr, "%s\n", lxb_app_trouble(app));
    }
    lxb_app_free(app);
    return code;
}
