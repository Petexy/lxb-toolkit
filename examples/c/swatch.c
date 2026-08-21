/*
 * The design language from C, and one thing done with it.
 *
 *     make && ./swatch
 *     ./swatch Blue 2160
 *
 * It prints the tokens, then writes `swatch.svg` — the palette, the type scale
 * and a few of the marks, laid out with the sizes the toolkit gives, so that
 * the C path is proved by something you can look at rather than by a number
 * coming back from a function.
 *
 * Nothing here allocates except the stylesheet, and nothing needs freeing but
 * that: every string and every asset is borrowed from the library.
 */

#include <lxb_toolkit.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Paste a mark's body, giving every name it defines a prefix of its own.
 *
 * The marks are authored as standalone documents, and several of them use the
 * same internal names: 53 of the 98 declare both `bead` and `pierced`. SVG
 * resolves a reference to the first definition in the *document*, so pasted
 * side by side without this, every masked mark after the first comes out drawn
 * as that first one — silently, and only visible by looking at the picture.
 */
static void write_scoped(FILE *out, const char *body, size_t length,
                         unsigned scope)
{
    /* The three ways these files name something and refer to it. */
    static const char *const heads[] = { "id=\"", "href=\"#", "url(#" };

    for (size_t index = 0; index < length; index++) {
        int matched = 0;
        for (size_t head = 0; head < sizeof heads / sizeof *heads; head++) {
            size_t width = strlen(heads[head]);
            if (index + width <= length
                && strncmp(body + index, heads[head], width) == 0) {
                fwrite(heads[head], 1, width, out);
                fprintf(out, "m%u_", scope);
                index += width - 1;
                matched = 1;
                break;
            }
        }
        if (!matched) {
            fputc(body[index], out);
        }
    }
}

/* Drop the XML declaration and the comment off a mark, so it can be pasted
 * inside an <svg> as a group. The marks are authored as standalone files. */
static void write_mark(FILE *out, const char *name, float x, float y,
                       float size, unsigned scope)
{
    lxb_bytes mark = lxb_glyph(name);
    if (!mark.data) {
        return;
    }
    const char *source = (const char *)mark.data;
    const char *open = strstr(source, "<svg");
    if (!open) {
        return;
    }
    const char *body = strchr(open, '>');
    const char *close = strstr(source, "</svg>");
    if (!body || !close) {
        return;
    }
    body++;

    /* The marks come in two square boxes; read which from the file rather than
     * assuming, or a 24-unit mark drawn as a 32-unit one is three quarters the
     * size it should be. */
    double box = strstr(source, "viewBox=\"0 0 24 24\"") ? 24.0 : 32.0;

    fprintf(out,
            "  <g transform=\"translate(%.1f %.1f) scale(%.4f)\">\n",
            x, y, size / box);
    write_scoped(out, body, (size_t)(close - body), scope);
    fprintf(out, "  </g>\n");
}

int main(int argc, char **argv)
{
    const char *wanted = argc > 1 ? argv[1] : "Purple";
    float height = argc > 2 ? (float)atof(argv[2]) : lxb_reference_height();

    int index = lxb_palette_index(wanted);
    if (index < 0) {
        fprintf(stderr, "no palette is called %s. There are %lu:\n", wanted,
                lxb_palette_count());
        for (unsigned long i = 0; i < lxb_palette_count(); i++) {
            fprintf(stderr, "  %s\n", lxb_palette_name(i));
        }
        return 1;
    }

    printf("lxb-toolkit %s — %s at %.0fp\n\n", lxb_version(),
           lxb_palette_name((unsigned long)index), height);

    for (unsigned long role = 0; role < lxb_role_count(); role++) {
        unsigned int hex = lxb_palette_color((unsigned long)index, role);
        lxb_rgba light = lxb_palette_rgba((unsigned long)index, role, 1.0f);
        printf("  %-16s #%06x   linear %.3f %.3f %.3f\n", lxb_role_name(role),
               hex, light.r, light.g, light.b);
    }

    printf("\n  %-16s %.1fpx (a control's radius is half its own height)\n",
           "capsule", lxb_capsule_radius(lxb_metric(LXB_METRIC_ROW_HEIGHT, height)));
    printf("  %-16s %.0fms\n", "flight", lxb_duration_named("flight") * 1000.0f);
    printf("  %-16s %.2f\n", "ease(0.25)", lxb_ease(0.25f));

    /* A spring, stepped the way an interface would step it. */
    double at = 0.0, speed = 0.0;
    int frames = 0;
    while (at < 0.999 && frames < 600) {
        lxb_spring(&at, &speed, 1.0, lxb_card_spring(), 1.0 / 60.0);
        frames++;
    }
    printf("  %-16s %d frames to arrive, never crossing\n", "spring", frames);

    /* The one thing that allocates. */
    char *css = lxb_stylesheet(wanted);
    if (css) {
        printf("  %-16s %zu bytes of custom properties\n", "stylesheet",
               strlen(css));
        lxb_string_free(css);
    }

    /* And the picture. */
    float scale = lxb_scale_for(height);
    float tile = lxb_metric(LXB_METRIC_TILE, height);
    float gap = lxb_metric(LXB_METRIC_GAP, height);
    float pad = lxb_metric(LXB_METRIC_PANEL_PADDING, height);
    float width = pad * 2.0f + (tile + gap) * (float)lxb_role_count();

    FILE *out = fopen("swatch.svg", "w");
    if (!out) {
        perror("swatch.svg");
        return 1;
    }
    fprintf(out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"%.0f\" "
            "height=\"%.0f\" viewBox=\"0 0 %.0f %.0f\">\n",
            width, 360.0f * scale, width, 360.0f * scale);

    /* The background is the palette's own sky, top to bottom. */
    fprintf(out, "  <defs><linearGradient id=\"sky\" x1=\"0\" y1=\"0\" x2=\"0\" "
                 "y2=\"1\">\n");
    fprintf(out, "    <stop offset=\"0\" stop-color=\"#%06x\"/>\n",
            lxb_palette_color((unsigned long)index, 10)); /* sky-top */
    fprintf(out, "    <stop offset=\"1\" stop-color=\"#%06x\"/>\n",
            lxb_palette_color((unsigned long)index, 11)); /* sky-bottom */
    fprintf(out, "  </linearGradient></defs>\n");
    fprintf(out, "  <rect width=\"100%%\" height=\"100%%\" fill=\"url(#sky)\"/>\n");

    /* One tile per role, a squircle-ish rounded square with its own colour. */
    for (unsigned long role = 0; role < lxb_role_count(); role++) {
        float x = pad + (tile + gap) * (float)role;
        fprintf(out,
                "  <rect x=\"%.1f\" y=\"%.1f\" width=\"%.1f\" height=\"%.1f\" "
                "rx=\"%.1f\" fill=\"#%06x\"/>\n",
                x, pad, tile, tile, tile * lxb_metric(LXB_METRIC_TILE_RADIUS, height),
                lxb_palette_color((unsigned long)index, role));
        fprintf(out,
                "  <text x=\"%.1f\" y=\"%.1f\" fill=\"#%06x\" font-family=\"Roboto\" "
                "font-size=\"%.1f\" text-anchor=\"middle\">%s</text>\n",
                x + tile / 2.0f, pad + tile + lxb_text_size(LXB_TEXT_CAPTION, height) * 1.4f,
                lxb_palette_color((unsigned long)index, 7), /* text-soft */
                lxb_text_size(LXB_TEXT_CAPTION, height) * 0.7f, lxb_role_name(role));
    }

    /* And a row of marks under them, at the size a tile's mark is drawn. */
    const char *marks[] = {"launch", "search", "volume", "brightness",
                           "setting-accent", "screenshot", "notifications",
                           "shutdown"};
    float mark_size = tile * lxb_metric(LXB_METRIC_TILE_GLYPH, height);
    for (size_t i = 0; i < sizeof(marks) / sizeof(*marks); i++) {
        float x = pad + (tile + gap) * (float)i + (tile - mark_size) / 2.0f;
        write_mark(out, marks[i], x, pad * 3.0f + tile, mark_size,
                   (unsigned)i);
    }

    fprintf(out,
            "  <text x=\"%.1f\" y=\"%.1f\" fill=\"#%06x\" font-family=\"Roboto\" "
            "font-weight=\"700\" font-size=\"%.1f\">%s</text>\n",
            pad, 320.0f * scale, lxb_palette_color((unsigned long)index, 6),
            lxb_text_size(LXB_TEXT_TITLE, height), lxb_palette_name((unsigned long)index));
    fprintf(out, "</svg>\n");
    fclose(out);

    printf("\nwrote swatch.svg (%.0fx%.0f)\n", width, 360.0f * scale);
    return 0;
}
