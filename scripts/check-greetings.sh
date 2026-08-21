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
# Nine views of the tour, and one of the greeting. Byte for byte: a PNG of the
# same pixels is the same file, so anything at all that differs fails here.

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

shots=0
compare() {
    local name=$1 program=$2
    shift 2
    "$root/examples/c/$program" --shot "$out/c-$name.png" "$@" >/dev/null
    PYTHONPATH="$root/python" python3 "$root/examples/python/$program.py" \
        --shot "$out/py-$name.png" "$@" >/dev/null
    cmp -s "$out/c-$name.png" "$out/py-$name.png" \
        || fail "$name" "C and Python drew different frames"
    shots=$((shots + 1))
}

for page in Hello Colour Material Marks Type Motion Sound; do
    compare "tour-$page" tour "$page"
done
compare tour-menu tour Hello menu
compare tour-dialog tour Hello dialog
compare hello hello

# And the short greeting in all three languages, which is one page function
# written three times against one library.
"$root/examples/rust-hello/target/release/hello-lxb-app" --shot "$out/rs-hello.png" >/dev/null
cmp -s "$out/c-hello.png" "$out/rs-hello.png" \
    || fail hello "Rust drew a different frame from C and Python"
shots=$((shots + 1))

printf 'every greeting agrees: %d views, byte for byte, in three languages\n' "$shots"
