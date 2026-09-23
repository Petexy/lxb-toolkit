#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=packaging/lib.sh
source "$script_dir/../lib.sh"

output_dir="$PACKAGING_DIR/out/fedora"
run_check=true
rpmbuild_extra=()

usage() {
    cat <<'USAGE'
Usage: packaging/fedora/build.sh [OPTIONS] [-- RPMBUILD OPTIONS]

Options:
  --output-dir DIR   Artifact directory (default: packaging/out/fedora)
  --work-dir DIR     Where rpmbuild builds (default: packaging/out/build)
  --no-check         Skip the %check phase, which is most of the build

The build is not run under /tmp: that is a tmpfs on most machines, and the
dev-profile build that %check runs takes about 300 MiB on top of the release
one. LXB_TOOLKIT_WORK_DIR sets the same thing.
USAGE
}

while (($#)); do
    case "$1" in
        --output-dir)
            (($# >= 2)) || package_die "--output-dir requires a value"
            output_dir="$2"
            shift 2
            ;;
        --work-dir)
            (($# >= 2)) || package_die "--work-dir requires a value"
            package_set_work_root "$2"
            shift 2
            ;;
        --no-check)
            run_check=false
            shift
            ;;
        --)
            shift
            rpmbuild_extra=("$@")
            break
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *) package_die "unknown Fedora builder option: $1" ;;
    esac
done

if [[ "$output_dir" != /* ]]; then
    output_dir="$PWD/$output_dir"
fi

require_command rpmbuild
require_command cargo
require_rust_version 1.89

work="$(package_work_dir lxb-toolkit-fedora)"
cleanup() {
    if [[ -n "${work:-}" && "$work" == */lxb-toolkit-fedora.* && -d "$work" ]]; then
        rm -rf -- "$work"
    fi
}
trap cleanup EXIT

if [[ "$run_check" == true ]]; then
    require_free_space "$work" 1024
else
    require_free_space "$work" 512
fi

mkdir -p "$work"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

source_dir="$work/lxb-toolkit-$PACKAGE_VERSION"
snapshot_source "$source_dir"

# Vendor the registry into the source tree, so %build can be offline — which
# is what an RPM build in a clean builder has to be.
package_note "vendoring the locked dependency graph"
mkdir -p "$source_dir/.cargo"
(cd "$source_dir" && cargo vendor --locked vendor > "$source_dir/.cargo/config.toml")

archive_snapshot "$source_dir" "$work/SOURCES/lxb-toolkit-$PACKAGE_VERSION.tar.gz"
install -m0644 "$script_dir/lxb-toolkit.spec" "$work/SPECS/lxb-toolkit.spec"

package_note "building the RPMs"
check_arguments=()
[[ "$run_check" == true ]] || check_arguments+=(--nocheck)
rpmbuild -ba \
    --define "_topdir $work" \
    "${check_arguments[@]}" \
    "${rpmbuild_extra[@]}" \
    "$work/SPECS/lxb-toolkit.spec"

mkdir -p "$output_dir"
collected=0
while IFS= read -r -d '' artifact; do
    install -m0644 "$artifact" "$output_dir/$(basename "$artifact")"
    collected=$((collected + 1))
done < <(find "$work/RPMS" "$work/SRPMS" -type f -name '*.rpm' -print0)
((collected > 0)) || package_die "rpmbuild produced no packages"

package_note "created $collected RPM(s) in $output_dir"
