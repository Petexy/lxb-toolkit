#!/usr/bin/env bash
set -euo pipefail

# Every greeting draws the same frames, and they have to be the same frames.
#
# Usage: scripts/check-greetings.sh [OUTPUT-DIR]
#
# Three programs in three languages call one renderer through one C ABI. That
# they *can* is what the examples show; that they come out identical is what
# this checks — and it is the only check that catches a binding which crosses a
# value correctly and then uses it slightly differently.
#
# Fifteen views of the tour, and two greeting checks. Byte for byte: a PNG of
# the same pixels is the same file, so anything at all that differs fails here.

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
out=${1:-$(mktemp -d)}
mkdir -p "$out"

fail() {
    printf 'greeting mismatch [%s]: %s\n' "$1" "$2" >&2
    exit 1
}

for needed in "$root/target/release/liblxb_app.so" "$root/examples/c/tour" \
    "$root/examples/c/hello" "$root/examples/rust-hello/target/release/hello-lxb-app"; do
    [[ -e "$needed" ]] || fail setup "missing $needed
Build them first: cargo build --release, then make -C examples/c hello tour,
then cargo build --release --manifest-path examples/rust-hello/Cargo.toml"
done

# The tour never opens a person's home directory. Both programs receive the
# same source-controlled miniature tree, which makes the picker listing and
# the location it names deterministic even when this script is run elsewhere.
picker_files="$root/examples/tour-files"
[[ -d "$picker_files" ]] || fail setup "missing picker fixture: $picker_files"
export LXB_TOUR_FILES="$picker_files"

shots=0
compare() {
    local name=$1 program=$2
    shift 2
    "$root/examples/c/$program" --shot "$out/c-$name.png" "$@" >/dev/null
    env -u PYTHONPATH python3 "$root/examples/python/$program.py" \
        --shot "$out/py-$name.png" "$@" >/dev/null
    cmp -s "$out/c-$name.png" "$out/py-$name.png" \
        || fail "$name" "C and Python drew different frames"
    shots=$((shots + 1))
}

for page in Hello Colour Material Marks Type Motion Sound Picker; do
    compare "tour-$page" tour "$page"
done
compare tour-menu tour Hello menu
compare tour-dialog tour Hello dialog
compare tour-picker-file tour Picker picker-file
compare tour-picker-many tour Picker picker-many
compare tour-picker-folder tour Picker picker-folder
compare tour-picker-save tour Picker picker-save
compare tour-picker-image tour Picker picker-image
compare hello hello

# And the short greeting in all three languages, which is one page function
# written three times against one library.
"$root/examples/rust-hello/target/release/hello-lxb-app" --shot "$out/rs-hello.png" >/dev/null
cmp -s "$out/c-hello.png" "$out/rs-hello.png" \
    || fail hello "Rust drew a different frame from C and Python"
shots=$((shots + 1))

# Say what was actually compared. All but one of these are the C and Python
# tours against each other; the last is the short greeting, which is the only
# view that exists in all three languages. The Rust tour is a richer program
# with prose of its own and cannot be a third copy of this one — what covers it
# is its own suite, which `cargo test --workspace` does not reach either. See
# examples/README.md.
printf 'every greeting agrees: %d views byte for byte — %d across C and Python, and the greeting in all three\n' \
    "$shots" "$((shots - 1))"
