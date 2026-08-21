#!/usr/bin/env bash

set -euo pipefail

packaging_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=packaging/lib.sh
source "$packaging_dir/lib.sh"

output="${1:-$PACKAGING_DIR/out/sources/lxb-toolkit-$PACKAGE_VERSION.tar.gz}"
if [[ "$output" != /* ]]; then
    output="$PWD/$output"
fi

work="$(package_work_dir lxb-toolkit-source)"
cleanup() {
    if [[ -n "${work:-}" && "$work" == */lxb-toolkit-source.* && -d "$work" ]]; then
        rm -rf -- "$work"
    fi
}
trap cleanup EXIT

source_dir="$work/lxb-toolkit-$PACKAGE_VERSION"
snapshot_source "$source_dir"
archive_snapshot "$source_dir" "$output"
package_note "created $output"
