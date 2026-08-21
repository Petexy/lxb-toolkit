#!/usr/bin/env python3
"""Write the whole design language out as one page you can look at.

    python3 contact_sheet.py                 # Purple, to contact-sheet.html
    python3 contact_sheet.py Green sheet.html

Everything on the page comes through the toolkit: the colours by role, the
sizes, the type scale, the marks, the timings and the stylesheet itself. The
typeface is embedded from the library, so the page needs nothing installed and
nothing from the network — which is the same reason the toolkit carries it.

It is also the Python binding's own test in the least abstract sense: if a
signature here is wrong, the page comes out wrong in a way you can see.
"""

from __future__ import annotations

import base64
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "python"))

import lxb_toolkit as lxb  # noqa: E402


def mark(name: str, size: float, color: str) -> str:
    """One mark, inline, at `size` pixels.

    The marks are authored as standalone documents, and 53 of the 98 declare
    the same internal names — `bead` and `pierced` among them. A reference
    resolves to the first definition in the *page*, and a nested ``<svg>`` is
    not a new scope for names, so inlined as they are every masked mark after
    the first draws as that first one. Giving each one's names a prefix of its
    own is the whole fix, and the mark's own name is already a safe one.
    """
    source = lxb.glyph(name)
    if source is None:
        return ""
    svg = source.decode("utf-8")
    body = svg[svg.index(">", svg.index("<svg")) + 1 : svg.rindex("</svg>")]
    body = re.sub(r'(id="|href="#|url\(#)', lambda head: f"{head.group(1)}{name}-", body)
    # The marks carry a white fill of their own and none of them uses
    # currentColor, so a CSS `color` on the wrapper reaches nothing. The real
    # shell never needs one — its material computes the colour from the shape —
    # but a flat approximation has to say what it wants, so say it here.
    body = body.replace('"#ffffff"', f'"{color}"').replace('"#fff"', f'"{color}"')
    box = 24 if 'viewBox="0 0 24 24"' in svg else 32
    return (
        f'<svg viewBox="0 0 {box} {box}" width="{size:.0f}"'
        f' height="{size:.0f}">{body}</svg>'
    )


def page(palette_name: str) -> str:
    palette = lxb.palette(palette_name)
    if palette is None:
        raise SystemExit(
            f"no palette is called {palette_name!r}; there are: "
            + ", ".join(p.name for p in lxb.PALETTES)
        )

    colors = palette.colors()
    font = base64.b64encode(lxb.font()).decode("ascii")
    font_bold = base64.b64encode(lxb.font(bold=True)).decode("ascii")

    swatches = "\n".join(
        f'<figure><div class="swatch" style="background:{color}"></div>'
        f"<figcaption>{name}<span>{color}</span></figcaption></figure>"
        for name, color in colors.items()
    )

    marks = "\n".join(
        f'<figure>{mark(name, 44, colors["text"].hex)}'
        f"<figcaption>{name}</figcaption></figure>"
        for name in lxb.GLYPHS
    )

    sizes = "\n".join(
        f"<tr><td>{lxb.Metric(index).name.lower().replace('_', '-')}</td>"
        f"<td class=\"n\">{lxb.size(lxb.Metric(index)):.2f}"
        f"{'' if lxb.is_share(lxb.Metric(index)) else 'px'}</td>"
        f"<td class=\"n\">{lxb.size(lxb.Metric(index), 2160):.2f}"
        f"{'' if lxb.is_share(lxb.Metric(index)) else 'px'}</td></tr>"
        for index in range(len(lxb.Metric))
    )

    type_scale = "\n".join(
        f'<p style="font-size:{lxb.text_size(step):.0f}px;'
        f'font-weight:{700 if lxb.text_is_bold(step) else 400}">'
        f"{step.name.lower()} — {lxb.text_size(step):.0f}px "
        f'<span class="dim">The quick brown fox</span></p>'
        for step in lxb.Text
    )

    durations = "\n".join(
        f'<tr><td>{name}</td><td class="n">{seconds * 1000:.0f}ms</td>'
        f'<td><div class="bar" style="animation-duration:{seconds}s"></div></td></tr>'
        for name, seconds in lxb.DURATIONS.items()
    )

    panes = "\n".join(
        f'<figure><div class="pane" style="'
        f"backdrop-filter:blur({lxb.glass(surface).frost * 24:.0f}px);"
        f"border-width:{max(1, lxb.glass(surface).depth / 8):.0f}px;"
        f'opacity:{0.55 + 0.45 * lxb.glass(surface).gloss:.2f}"></div>'
        f"<figcaption>{surface.name.lower()}<span>"
        f"depth {lxb.glass(surface).depth:.0f} · "
        f"frost {lxb.glass(surface).frost:.2f} · "
        f"gloss {lxb.glass(surface).gloss:.2f}</span></figcaption></figure>"
        for surface in lxb.Surface
    )

    sounds = "\n".join(
        f'<tr><td>{name}</td><td class="n">{len(lxb.sound(name) or b"") / 1024:.1f} KiB</td></tr>'
        for name in lxb.SOUNDS
    )

    return f"""<!doctype html>
<meta charset="utf-8">
<title>{palette.name} — the LineXinBar design language</title>
<style>
@font-face {{
  font-family: "Roboto";
  font-weight: 400;
  src: url(data:font/ttf;base64,{font}) format("truetype");
}}
@font-face {{
  font-family: "Roboto";
  font-weight: 700;
  src: url(data:font/ttf;base64,{font_bold}) format("truetype");
}}
{lxb.stylesheet(palette.name)}
body {{
  margin: 0;
  padding: calc(var(--lxb-panel-padding) * 1.5);
  background: linear-gradient(var(--lxb-sky-top), var(--lxb-sky-bottom)) fixed;
  color: var(--lxb-text);
  font-family: var(--lxb-font), system-ui, sans-serif;
  font-size: var(--lxb-text-body);
  line-height: var(--lxb-line-height);
}}
h1 {{ font-size: var(--lxb-text-display); margin: 0 0 .2em; }}
h2 {{
  font-size: var(--lxb-text-title);
  margin: 2.5rem 0 1rem;
  color: var(--lxb-accent-soft);
}}
.dim {{ color: var(--lxb-text-soft); }}
.grid {{ display: flex; flex-wrap: wrap; gap: var(--lxb-gap); }}
figure {{ margin: 0; text-align: center; }}
figcaption {{
  font-size: var(--lxb-text-caption);
  color: var(--lxb-text-soft);
  padding-top: .4em;
}}
figcaption span {{ display: block; opacity: .6; font-size: .85em; }}
.swatch {{
  width: calc(var(--lxb-tile) * 1.6);
  height: var(--lxb-tile);
  border-radius: calc(var(--lxb-tile) * var(--lxb-tile-radius));
  box-shadow: 0 1px 0 rgb(255 255 255 / .18) inset;
}}
.grid.marks figure {{
  width: calc(var(--lxb-tile) * 1.9);
  padding: var(--lxb-gap) 0;
  border-radius: calc(var(--lxb-tile) * var(--lxb-tile-radius));
  background: rgb(255 255 255 / .04);
}}
.pane {{
  width: calc(var(--lxb-tile) * 3);
  height: calc(var(--lxb-tile) * 1.7);
  border-radius: var(--lxb-panel-radius);
  border: solid rgb(255 255 255 / .35);
  background: rgb(255 255 255 / .06);
}}
.panes {{
  background:
    radial-gradient(60% 120% at 30% 20%, var(--lxb-glow), transparent),
    var(--lxb-accent-deep);
  padding: var(--lxb-panel-padding);
  border-radius: var(--lxb-panel-radius);
}}
table {{ border-collapse: collapse; }}
td {{
  padding: .25em 1.5em .25em 0;
  font-size: var(--lxb-text-label);
  color: var(--lxb-text-soft);
}}
td.n {{ font-variant-numeric: tabular-nums; color: var(--lxb-text); }}
.bar {{
  width: 8rem; height: .5rem;
  border-radius: 999px;
  background: var(--lxb-accent);
  transform-origin: left;
  animation-name: sweep;
  animation-timing-function: var(--lxb-ease);
  animation-iteration-count: infinite;
  animation-direction: alternate;
}}
@keyframes sweep {{ from {{ transform: scaleX(.08) }} to {{ transform: scaleX(1) }} }}
footer {{ margin-top: 3rem; color: var(--lxb-text-soft); font-size: var(--lxb-text-caption); }}
</style>

<h1>{palette.name}</h1>
<p class="dim">The LineXinBar design language, drawn entirely out of
lxb-toolkit {lxb.version()}. Every colour, size, duration, mark and letterform
on this page came through the toolkit — nothing here is installed and nothing
is fetched.</p>

<h2>Colour, by role</h2>
<div class="grid">{swatches}</div>

<h2>Glass</h2>
<p class="dim">A stylesheet can carry a blur and a lit edge. The real material
refracts what is behind it, splits it into colour at the corners and brightens
where the bevel concentrates it — that needs a shader, and one ships with the
toolkit.</p>
<div class="grid panes">{panes}</div>

<h2>Marks</h2>
<div class="grid marks">{marks}</div>

<h2>Type</h2>
{type_scale}

<h2>Sizes</h2>
<table><tr><td class="dim">name</td><td class="dim">1080p</td>
<td class="dim">2160p</td></tr>{sizes}</table>

<h2>Motion</h2>
<p class="dim">Never linear — the bars run on the language's own easing.</p>
<table>{durations}</table>

<h2>Sound</h2>
<table>{sounds}</table>

<footer>Generated by examples/python/contact_sheet.py · library at
{lxb.library_path()}</footer>
"""


def main() -> None:
    palette_name = sys.argv[1] if len(sys.argv) > 1 else "Purple"
    out = Path(sys.argv[2] if len(sys.argv) > 2 else "contact-sheet.html")
    out.write_text(page(palette_name), encoding="utf-8")
    print(f"wrote {out} ({out.stat().st_size / 1024:.0f} KiB)")


if __name__ == "__main__":
    main()
