#!/bin/sh
# Write lxb.css beside hello.html: the design language as custom properties.
#
#     cargo build --release -p lxb-toolkit-ffi   # from the repository root
#     ./generate.sh                              # this machine's own accent
#     ./generate.sh Green                        # or any palette, by name
#     xdg-open hello.html
#
# CSS is the one route with no library at run time, so the palette has to be
# chosen when the sheet is written rather than read when the page is opened.
# The sheet is generated rather than checked in for the same reason nothing
# else here hardcodes a value: a copied number is a number that goes stale.

set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$here/../..

PYTHONPATH=$root/python python3 - "${1:-}" > "$here/lxb.css" <<'PY'
import sys

import lxb_toolkit as lxb

wanted = sys.argv[1] or lxb.ShellTheme.load().accent.name
sys.stdout.write(lxb.stylesheet(wanted))
PY

echo "wrote $here/lxb.css — now open $here/hello.html"
