#!/usr/bin/env python3
"""Write docs/api-reference.md out of the two C headers.

    python3 scripts/make-reference.py            # write it
    python3 scripts/make-reference.py --check    # fail if it is out of date

Generated rather than written, for the same reason examples/css/lxb.css is:
a copied signature is a signature that goes stale. The headers carry the
prose; this only arranges it.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "api-reference.md"

HEADERS = [
    ("lxb_toolkit.h", ROOT / "crates/lxb-toolkit-ffi/include/lxb_toolkit.h",
     "What the language answers: colours, sizes, motion, type, marks, "
     "recordings and the shapes of its panels. Links nothing."),
    ("lxb_app.h", ROOT / "crates/lxb-app-ffi/include/lxb_app.h",
     "The window, the frame loop, the controls and the sounds — and the page "
     "an application draws into. Carries the GPU stack."),
]

BANNER = re.compile(r"^/\*\s*-+\s*(.*?)\s*-+\s*\*/\s*$")
DECL = re.compile(r"^[A-Za-z_][A-Za-z_0-9 *]*\**\s*(lxb_[a-z_0-9]+)\s*\(")
ENUM_DOC = re.compile(r"^/\*\s*(lxb_[a-z_0-9]+):\s*(.*)$")


def unwrap(block: list[str]) -> str:
    """One C block comment as a paragraph of Markdown."""
    text = []
    for line in block:
        line = line.strip()
        line = re.sub(r"^/\*+", "", line)
        line = re.sub(r"\*+/$", "", line)
        line = re.sub(r"^\*\s?", "", line)
        text.append(line.rstrip())
    out, para = [], []
    for line in text:
        if not line.strip():
            if para:
                out.append(" ".join(para))
                para = []
        else:
            para.append(line.strip())
    if para:
        out.append(" ".join(para))
    return "\n\n".join(p for p in out if p)


def parse(path: Path):
    """(section, [(prose, [(name, signature)])]) in the header's own order.

    A comment in these headers often stands over a run of related
    declarations rather than over one, so consecutive functions sharing a
    comment are kept together and the prose is printed once above them.
    """
    lines = path.read_text().split("\n")
    sections: list[tuple[str, list]] = [("", [])]
    block: list[str] = []
    i, n = 0, len(lines)
    while i < n:
        line = lines[i]
        banner = BANNER.match(line)
        if banner:
            sections.append((banner.group(1), []))
            block = []
            i += 1
            continue
        if line.startswith("/*"):
            start = i
            while i < n and "*/" not in lines[i]:
                i += 1
            block = lines[start:i + 1]
            i += 1
            continue
        decl = DECL.match(line)
        if decl:
            signature = [line]
            while i < n and ";" not in lines[i]:
                i += 1
                signature.append(lines[i])
            text = unwrap(block)
            entry = (decl.group(1),
                     "\n".join(s.rstrip() for s in signature).strip())
            groups = sections[-1][1]
            if block or not groups:
                groups.append([text, [entry]])
            else:
                groups[-1][1].append(entry)
            block = []
            i += 1
            continue
        if line.strip() and not line.startswith(("#", " ", "}")):
            block = []
        i += 1
    return [(name, items) for name, items in sections if items]


def render() -> str:
    out = [
        "# API reference",
        "",
        "Every function the toolkit answers, in the order its headers declare "
        "them.",
        "",
        "**Generated** by `scripts/make-reference.py` from the two C headers, "
        "which are the authoritative surface: a signature copied by hand is a "
        "signature that goes stale. Edit the header, then run the script.",
        "",
        "The three languages are one API. A C function `lxb_page_button` is "
        "`page.button` in Python and `Page::button` in Rust; `lxb_glyph_count` "
        "is `lxb.GLYPHS` and `glyph::ALL`. Where a C call takes an out "
        "parameter, the other two return the value. Enumerations cross as "
        "indices into this library's own lists, so the two shared objects "
        "agree by construction.",
        "",
        "Three shapes repeat and are not written out each time. `X_count` "
        "answers how many of a thing there are; `X_name(index)` answers the "
        "name of one, which is the name the shell itself calls it; and "
        "`X_index(name)` answers the number of a name, however capitalised, "
        "or -1 for a name nobody has. The number is what crosses the ABI, so "
        "walking a list means counting to `X_count` and asking for each.",
        "",
    ]
    total = 0
    for title, path, blurb in HEADERS:
        sections = parse(path)
        count = sum(len(fns) for _, groups in sections for _, fns in groups)
        total += count
        out += [f"## `{title}`", "", blurb, "", f"{count} functions.", ""]
        for name, groups in sections:
            if name:
                out += [f"### {name}", ""]
            for prose, fns in groups:
                if prose:
                    out += [prose, ""]
                for fn, signature in fns:
                    out += [f"`{fn}`", "", "```c", signature, "```", ""]
    out.insert(4, f"{total} functions in all.")
    out.insert(5, "")
    return "\n".join(out).rstrip("\n") + "\n"


if __name__ == "__main__":
    text = render()
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} is out of date: "
                  "run python3 scripts/make-reference.py", file=sys.stderr)
            raise SystemExit(1)
        print(f"{OUT.relative_to(ROOT)} is current")
    else:
        OUT.write_text(text)
        print(f"wrote {OUT.relative_to(ROOT)}: "
              f"{text.count(chr(10))} lines")
