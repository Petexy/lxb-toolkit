#!/usr/bin/env python3
"""Refresh the C signatures in docs/api-reference.md from the two headers.

The reference owns its prose and ordering. The headers own declarations. A
missing, extra, or reordered entry is an error so every public function must be
documented deliberately before this script will update copied signatures.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "api-reference.md"

HEADERS = [
    ("lxb_toolkit.h", ROOT / "crates/lxb-toolkit-ffi/include/lxb_toolkit.h"),
    ("lxb_app.h", ROOT / "crates/lxb-app-ffi/include/lxb_app.h"),
]

DECL = re.compile(r"^[A-Za-z_][A-Za-z_0-9 *]*\**\s*(lxb_[a-z_0-9]+)\s*\(")
ENTRY = re.compile(r"(?ms)^`(lxb_[a-z_0-9]+)`\n\n```c\n(.*?)\n```")


def declarations(path: Path) -> list[tuple[str, str]]:
    lines = path.read_text().splitlines()
    found: list[tuple[str, str]] = []
    index = 0
    while index < len(lines):
        match = DECL.match(lines[index])
        if match is None:
            index += 1
            continue
        signature = [lines[index].rstrip()]
        while ";" not in lines[index]:
            index += 1
            if index == len(lines):
                raise SystemExit(f"unterminated declaration for {match.group(1)}")
            signature.append(lines[index].rstrip())
        found.append((match.group(1), "\n".join(signature)))
        index += 1
    return found


def section(text: str, title: str) -> tuple[int, int]:
    marker = f"## `{title}`\n"
    start = text.find(marker)
    if start < 0:
        raise SystemExit(f"docs/api-reference.md has no {title} section")
    end = text.find("\n## `", start + len(marker))
    return start, len(text) if end < 0 else end + 1


def refresh(original: str) -> str:
    text = original
    total = 0
    for title, path in HEADERS:
        declared = declarations(path)
        total += len(declared)
        start, end = section(text, title)
        body = text[start:end]
        entries = list(ENTRY.finditer(body))
        documented = [match.group(1) for match in entries]
        names = [name for name, _ in declared]
        if documented != names:
            missing = [name for name in names if name not in documented]
            extra = [name for name in documented if name not in names]
            raise SystemExit(
                f"{title} documentation differs from its declarations; "
                f"missing={missing}, extra={extra}, or entries are reordered"
            )
        replacements = dict(declared)
        for match in reversed(entries):
            signature_start, signature_end = match.span(2)
            signature = replacements[match.group(1)]
            annotation = match.group(2).find("/*")
            if annotation >= 0:
                while annotation > 0 and match.group(2)[annotation - 1] in " \t":
                    annotation -= 1
                signature += match.group(2)[annotation:]
            body = (
                body[:signature_start]
                + signature
                + body[signature_end:]
            )
        body = re.sub(
            r"(?m)^\d+ functions\.$",
            f"{len(declared)} functions.",
            body,
            count=1,
        )
        text = text[:start] + body + text[end:]
    text = re.sub(
        r"(?m)^\d+ functions in all\.$",
        f"{total} functions in all.",
        text,
        count=1,
    )
    return text


if __name__ == "__main__":
    original = OUT.read_text()
    updated = refresh(original)
    if "--check" in sys.argv:
        if updated != original:
            print(
                "docs/api-reference.md is out of date: "
                "run python3 scripts/make-reference.py",
                file=sys.stderr,
            )
            raise SystemExit(1)
        print("docs/api-reference.md is current")
    else:
        OUT.write_text(updated)
        print(f"wrote docs/api-reference.md: {updated.count(chr(10))} lines")
