#!/usr/bin/env bash

# Stage the payload shared by every distro package. This deliberately does not
# install distro-specific documentation or licence metadata.

set -euo pipefail

packaging_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=packaging/lib.sh
source "$packaging_dir/lib.sh"

destdir=""
prefix="/usr"
libdir=""
component="all"
python_sitedir=""
target_dir="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"

usage() {
    cat <<'EOF'
Usage: packaging/install.sh --destdir DIR [--prefix PREFIX] [--libdir DIR]
                            [--target-dir DIR] [--python-sitedir DIR]
                            [--component library|devel|python|all]

Stages one part of lxb-toolkit, or all of it. PREFIX defaults to /usr, --libdir
to PREFIX/lib, --component to all, and --python-sitedir to whatever the running
python3 reports as its purelib.

  library  liblxb_toolkit.so. What anything linking the C ABI needs at run
           time, and what the Python binding loads. Nothing in it is specific
           to building against the toolkit, which is why it is separable.
  devel    The header, the static archive, the pkg-config file, the lxb-new
           generator, and the Rust crate sources a generated project builds
           against. Everything needed to make an application and nothing
           needed to run one.
  python   The ctypes binding. Useless without the library above; the packages
           say so.
  all      Every part, as one tree.

--libdir exists because the distributions disagree: Debian wants a multiarch
triplet under lib/, Fedora wants lib64 on 64-bit, and Arch wants lib.
EOF
}

while (($#)); do
    case "$1" in
        --destdir)
            (($# >= 2)) || package_die "--destdir requires a value"
            destdir="$2"
            shift 2
            ;;
        --prefix)
            (($# >= 2)) || package_die "--prefix requires a value"
            prefix="$2"
            shift 2
            ;;
        --libdir)
            (($# >= 2)) || package_die "--libdir requires a value"
            libdir="$2"
            shift 2
            ;;
        --target-dir)
            (($# >= 2)) || package_die "--target-dir requires a value"
            target_dir="$2"
            shift 2
            ;;
        --python-sitedir)
            (($# >= 2)) || package_die "--python-sitedir requires a value"
            python_sitedir="$2"
            shift 2
            ;;
        --component)
            (($# >= 2)) || package_die "--component requires a value"
            component="$2"
            shift 2
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *) package_die "unknown install option: $1" ;;
    esac
done

case "$component" in
    library | devel | python | all) ;;
    *) package_die "unknown component: $component (library, devel, python or all)" ;;
esac

[[ -n "$destdir" ]] || package_die "--destdir is required"
[[ "$destdir" == /* ]] || package_die "--destdir must be absolute"
[[ -z "$prefix" || "$prefix" == /* ]] || package_die "--prefix must be empty or absolute"

if [[ "$target_dir" != /* ]]; then
    target_dir="$PROJECT_ROOT/$target_dir"
fi
prefix="${prefix%/}"
[[ "$prefix" != "/" ]] || prefix=""
install_root="${destdir}${prefix}"

# Relative to the prefix, because that is how every package definition writes
# it and how the pkg-config file has to refer to itself.
if [[ -z "$libdir" ]]; then
    libdir="$prefix/lib"
fi
[[ "$libdir" == /* ]] || package_die "--libdir must be absolute"
libdir="${libdir%/}"
install_libdir="${destdir}${libdir}"

# Two shared objects, and they are deliberately separate packages' worth of
# dependency. liblxb_toolkit answers what the language is and links nothing at
# all; liblxb_app opens a window, a GPU, the controllers and an audio device. A
# program that only wants the answers must not be made to pull in the second.
stage_library() {
    local name
    for name in lxb_toolkit lxb_app; do
        local library="$target_dir/release/lib$name.so"
        [[ -f "$library" ]] || package_die "missing release library: $library
Build it first: cargo build --release"
        install -Dm0755 "$library" "$install_libdir/lib$name.so"
    done
}

stage_devel() {
    [[ -x "$target_dir/release/lxb-new" ]] \
        || package_die "missing release binary: $target_dir/release/lxb-new"
    install -Dm0755 "$target_dir/release/lxb-new" "$install_root/bin/lxb-new"

    local name
    for name in lxb_toolkit lxb_app; do
        local archive="$target_dir/release/lib$name.a"
        [[ -f "$archive" ]] || package_die "missing static archive: $archive
Build it first: cargo build --release"
        install -Dm0644 "$archive" "$install_libdir/lib$name.a"
    done
    install -Dm0644 "$PROJECT_ROOT/crates/lxb-toolkit-ffi/include/lxb_toolkit.h" \
        "$install_root/include/lxb_toolkit.h"
    install -Dm0644 "$PROJECT_ROOT/crates/lxb-app-ffi/include/lxb_app.h" \
        "$install_root/include/lxb_app.h"

    # pkg-config is how a C consumer finds all of the above without being told
    # where any of it went. The paths written in are the installed ones, not
    # the staged ones: $destdir is where the package is being built, and a
    # reference to it would be a reference to the builder's own machine.
    local module
    for module in lxb-toolkit lxb-app; do
        local pc="$install_libdir/pkgconfig/$module.pc"
        mkdir -p "$(dirname "$pc")"
        sed -e "s|@PREFIX@|${prefix:-/}|g" \
            -e "s|@LIBDIR@|$libdir|g" \
            -e "s|@VERSION@|$PACKAGE_VERSION|g" \
            "$PACKAGING_DIR/files/$module.pc.in" > "$pc"
        chmod 0644 "$pc"
    done

    stage_crate_sources
}

# The Rust crates, as sources.
#
# Cargo does not read $libdir: a Rust dependency is a registry version or a
# path, and nothing else. So an installed toolkit ships the crates themselves
# and lxb-new points generated projects at them — which is what makes the
# generator useful on a machine that has never seen this checkout.
#
# Five of them, always together: lxb-toolkit is what the language answers,
# lxb-render is the material it is drawn in, lxb-input is the controls it is
# driven from, lxb-sound is the noise it answers with, and lxb-app is the
# window they all meet in. They are released as one version, and a project
# built against some of each would be a project built against two releases.
#
# `cargo package` rather than a copy of the directory, because those manifests
# inherit their version, edition and licence from [workspace.package]. Lifted
# out of the workspace those inherited keys resolve to nothing and the crate
# stops parsing, so what gets installed is the normalised manifest Cargo itself
# writes for publication.
stage_crate_sources() {
    local crates="$install_root/share/lxb-toolkit/crates"

    # lxb-toolkit through `cargo package`, which is the normaliser: its
    # manifest inherits version, edition and licence from [workspace.package],
    # and lifted out of the workspace those keys resolve to nothing.
    local crate="$target_dir/package/lxb-toolkit-$PACKAGE_VERSION.crate"
    # Rebuilt whenever the sources have moved under it. Keyed on existence
    # alone, a stale archive from an earlier build is installed in place of the
    # tree being tested, and every check downstream passes against sources
    # nobody has edited for hours.
    local stale=""
    if [[ -f "$crate" ]]; then
        stale="$(find "$PROJECT_ROOT/crates/lxb-toolkit" -newer "$crate" -print -quit)"
    fi
    if [[ ! -f "$crate" || -n "$stale" ]]; then
        package_note "packaging lxb-toolkit for installation"
        # --no-verify: verification is a second full build of a crate this
        # workspace has already built and tested. --allow-dirty: a local build
        # is expected to package the tree the developer is testing, which is
        # the same reason snapshot_source takes untracked files.
        (cd "$PROJECT_ROOT" && cargo package -p lxb-toolkit \
            --locked --offline --no-verify --allow-dirty \
            --target-dir "$target_dir" >/dev/null) \
            || package_die "could not package lxb-toolkit"
    fi
    [[ -f "$crate" ]] || package_die "missing crate archive: $crate"
    mkdir -p "$crates/lxb-toolkit"
    # One directory deep inside the archive, named for the version; strip it so
    # the installed path does not carry a version a consumer would have to know.
    tar -xzf "$crate" -C "$crates/lxb-toolkit" --strip-components=1

    # The others by hand, because `cargo package` cannot normalise them:
    # each depends on lxb-toolkit, which is not on any registry, and packaging
    # a crate resolves its dependencies whether or not it verifies them. So the
    # sources are copied and the same inherited keys are written in, along with
    # the path to the sibling installed beside them.
    local sibling
    for sibling in lxb-render lxb-input lxb-sound lxb-portal lxb-app; do
        mkdir -p "$crates/$sibling"
        cp -r "$PROJECT_ROOT/crates/$sibling/src" "$crates/$sibling/"
        if [[ -d "$PROJECT_ROOT/crates/$sibling/examples" ]]; then
            cp -r "$PROJECT_ROOT/crates/$sibling/examples" "$crates/$sibling/"
        fi
        sed -e "s|^version\.workspace = true$|version = \"$PACKAGE_VERSION\"|" \
            -e 's|^edition\.workspace = true$|edition = "2021"|' \
            -e 's|^license\.workspace = true$|license = "GPL-3.0-only"|' \
            -e 's|^repository\.workspace = true$|repository = "https://github.com/petexy/lxb-toolkit"|' \
            -e 's|^authors\.workspace = true$|authors = ["Piotr Lewandowski"]|' \
            "$PROJECT_ROOT/crates/$sibling/Cargo.toml" > "$crates/$sibling/Cargo.toml"
    done

    # tar and cp restore whatever modes they carried; make them a package's.
    find "$crates" -type d -exec chmod 0755 {} +
    find "$crates" -type f -exec chmod 0644 {} +
}

stage_python() {
    local sitedir="$python_sitedir"
    if [[ -z "$sitedir" ]]; then
        require_command python3
        sitedir="$(python3 -c 'import sysconfig; print(sysconfig.get_path("purelib"))')"
        [[ -n "$sitedir" ]] || package_die "could not determine the Python site directory"
    fi
    [[ "$sitedir" == /* ]] || package_die "--python-sitedir must be absolute"

    # Every module the package has, enumerated rather than named. A list
    # written out by hand is how paint.py came to be missing from every
    # installed copy while the checkout worked perfectly: __init__ imports it
    # unconditionally, so what shipped was a package that could not be
    # imported at all.
    local destination="${destdir}${sitedir}/lxb_toolkit"
    local module modules=()
    while IFS= read -r module; do
        modules+=("$module")
    done < <(find "$PROJECT_ROOT/python/lxb_toolkit" -maxdepth 1 -name '*.py' | sort)
    ((${#modules[@]} > 0)) \
        || package_die "no Python modules found in python/lxb_toolkit"
    for module in "${modules[@]}"; do
        install -Dm0644 "$module" "$destination/$(basename "$module")"
    done

    # Enough metadata for importlib.metadata to see a distribution, and no
    # more: RECORD is pip's account of what pip installed, and nothing here was
    # installed by pip. The distribution's own package manager owns these files.
    local info="${destdir}${sitedir}/lxb_toolkit-$PACKAGE_VERSION.dist-info"
    mkdir -p "$info"
    cat > "$info/METADATA" <<EOF
Metadata-Version: 2.1
Name: lxb-toolkit
Version: $PACKAGE_VERSION
Summary: The LineXinBar design language: colour roles, glass, motion, type, marks and sounds
Home-page: https://github.com/petexy/lxb-toolkit
Author: Piotr Lewandowski
License: GPL-3.0-only
Requires-Python: >=3.9
EOF
    printf 'lxb_toolkit\n' > "$info/top_level.txt"
    chmod 0644 "$info/METADATA" "$info/top_level.txt"
}

if [[ "$component" == library || "$component" == all ]]; then
    stage_library
fi
if [[ "$component" == devel || "$component" == all ]]; then
    stage_devel
fi
if [[ "$component" == python || "$component" == all ]]; then
    stage_python
fi
