#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=packaging/lib.sh
source "$script_dir/../lib.sh"

output_dir="$PACKAGING_DIR/out/debian"
allow_foreign=false

usage() {
    cat <<'EOF'
Usage: packaging/debian/build.sh [--output-dir DIR] [--allow-foreign-host]

Builds the three Debian binary packages from the current working tree. A
deployable package must be built on Debian (or a Debian derivative) so its ABI
and generated shared-library dependencies match the target system.
EOF
}

while (($#)); do
    case "$1" in
        --output-dir)
            (($# >= 2)) || package_die "--output-dir requires a value"
            output_dir="$2"
            shift 2
            ;;
        --allow-foreign-host)
            allow_foreign=true
            shift
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *) package_die "unknown Debian builder option: $1" ;;
    esac
done

if [[ "$allow_foreign" != true ]] && ! host_is_like debian; then
    package_die "build Debian packages on Debian/Ubuntu; use --allow-foreign-host only for metadata testing"
fi
if [[ "$output_dir" != /* ]]; then
    output_dir="$PWD/$output_dir"
fi

require_command dpkg-deb
require_command dpkg-shlibdeps
require_command dpkg
require_command md5sum
require_rust_version 1.85

target_dir="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
if [[ "$target_dir" != /* ]]; then
    target_dir="$PROJECT_ROOT/$target_dir"
fi
package_note "building the release artefacts"
RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$PROJECT_ROOT=/usr/src/lxb-toolkit-$PACKAGE_VERSION" \
    CARGO_TARGET_DIR="$target_dir" \
    cargo build --manifest-path "$PROJECT_ROOT/Cargo.toml" \
    --release --locked -p lxb-toolkit -p lxb-toolkit-ffi -p lxb-app -p lxb-app-ffi -p lxb-new

work="$(package_work_dir lxb-toolkit-debian)"
cleanup() {
    if [[ -n "${work:-}" && "$work" == */lxb-toolkit-debian.* && -d "$work" ]]; then
        rm -rf -- "$work"
    fi
}
trap cleanup EXIT

architecture="$(dpkg --print-architecture)"
# Debian puts a library under its multiarch triplet; the other two
# distributions do not, which is the whole reason install.sh takes --libdir.
multiarch="$(dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || true)"
libdir="/usr/lib${multiarch:+/$multiarch}"
# Debian's own name for the pure-Python directory, which is not what
# sysconfig reports inside a build.
sitedir="/usr/lib/python3/dist-packages"

mkdir -p "$output_dir"
mkdir -p "$work/shlibs/debian"
install -m0644 "$script_dir/source-control" "$work/shlibs/debian/control"

# Build one binary package from one component of the staged tree.
#
#   build_deb COMPONENT NAME CONTROL ARCH_KIND [ELF OBJECT...]
#
# The objects named are the ones handed to dpkg-shlibdeps, which is how each
# package declares only what its own contents actually link against. A package
# with no ELF in it names none and is Architecture: all.
build_deb() {
    local component="$1" name="$2" control="$3" arch_kind="$4"
    shift 4
    local objects=("$@")

    local package_root="$work/$name"
    "$PACKAGING_DIR/install.sh" \
        --destdir "$package_root" \
        --prefix /usr \
        --libdir "$libdir" \
        --target-dir "$target_dir" \
        --python-sitedir "$sitedir" \
        --component "$component"

    # No /usr/share/licenses here: that is the RPM and Arch convention. On
    # Debian the copyright file is the licence record, and it points at the
    # GPL-3 and Apache-2.0 texts every Debian system already carries in
    # /usr/share/common-licenses.
    install -Dm0644 "$script_dir/copyright" "$package_root/usr/share/doc/$name/copyright"

    # Staged before the package is built, so it is weighed by Installed-Size
    # and listed in md5sums.
    case "$component" in
        library)
            install -Dm0644 "$PROJECT_ROOT/README.md" \
                "$package_root/usr/share/doc/$name/README.md"
            ;;
        devel)
            install -Dm0644 "$PROJECT_ROOT/docs/design-language.md" \
                "$package_root/usr/share/doc/$name/design-language.md"
            install -Dm0644 "$PROJECT_ROOT/docs/application-development.md" \
                "$package_root/usr/share/doc/$name/application-development.md"
            install -Dm0644 "$PROJECT_ROOT/docs/api-reference.md" \
                "$package_root/usr/share/doc/$name/api-reference.md"
            ;;
        python)
            install -Dm0644 "$PROJECT_ROOT/python/README.md" \
                "$package_root/usr/share/doc/$name/README.md"
            ;;
    esac

    local package_architecture="$architecture"
    local shlib_depends=""
    if [[ "$arch_kind" == all ]]; then
        package_architecture="all"
    else
        local object
        for object in "${objects[@]}"; do
            [[ -f "$package_root/$object" ]] \
                || package_die "$name does not contain $object"
            if command -v strip >/dev/null 2>&1; then
                # --strip-unneeded on a shared object keeps the dynamic symbol
                # table, which is the only thing a consumer links against.
                strip --strip-unneeded "$package_root/$object"
            fi
        done

        local shlib_arguments=()
        # On a foreign host there is no dpkg database mapping libc and libgcc
        # to Debian packages, so dpkg-shlibdeps fails outright and the flag
        # that exists for structure testing cannot do any. Downgrade that to a
        # warning there and nowhere else: on Debian the strict form is the
        # whole point, because a missed library is a package that installs and
        # then does not run.
        if [[ "$allow_foreign" == true ]]; then
            shlib_arguments+=(--ignore-missing-info)
        fi
        for object in "${objects[@]}"; do
            shlib_arguments+=("-e$package_root/$object")
        done
        local shlib_output
        shlib_output="$({
            cd "$work/shlibs"
            dpkg-shlibdeps -O "${shlib_arguments[@]}"
        })"
        if [[ "$shlib_output" == shlibs:Depends=* ]]; then
            shlib_depends="${shlib_output#shlibs:Depends=}"
        elif [[ "$allow_foreign" == true && -z "$shlib_output" ]]; then
            # Everything was ignored above, so there is nothing left to name.
            # The package's shape is still worth looking at; what it declares
            # is not, and must not be mistaken for a package that could ship.
            package_note "warning: $name has no Depends — this host cannot resolve them"
        else
            package_die "could not determine Debian shared-library dependencies for $name"
        fi
    fi

    local installed_size
    installed_size="$(du -sk "$package_root" | awk '{print $1}')"
    mkdir -p "$package_root/DEBIAN"
    awk \
        -v version="$PACKAGE_VERSION" \
        -v architecture="$package_architecture" \
        -v dependencies="$shlib_depends" \
        -v installed_size="$installed_size" \
        '{
            gsub(/@VERSION@/, version)
            gsub(/@ARCH@/, architecture)
            gsub(/@SHLIB_DEPENDS@/, dependencies)
            gsub(/@INSTALLED_SIZE@/, installed_size)
            print
        }' "$script_dir/$control" > "$package_root/DEBIAN/control"
    # A package with nothing to link against leaves an empty Depends, which is
    # a field dpkg rejects rather than ignores.
    sed -i -e '/^Depends: *$/d' -e 's/^Depends: , /Depends: /' \
        "$package_root/DEBIAN/control"

    (
        cd "$package_root"
        find usr -type f -print0 | sort -z | xargs -0 md5sum
    ) > "$package_root/DEBIAN/md5sums"

    local artifact="$output_dir/${name}_${PACKAGE_VERSION}-1_${package_architecture}.deb"
    dpkg-deb --root-owner-group -Zxz --build "$package_root" "$artifact"
    dpkg-deb --info "$artifact" >/dev/null
    package_note "created $artifact"
}

# The library first: the other two declare a versioned dependency on it, and it
# is the half a machine that only runs applications needs.
build_deb library lxb-toolkit control.in any \
    "${libdir#/}/liblxb_toolkit.so" "${libdir#/}/liblxb_app.so"
build_deb devel lxb-toolkit-dev control-devel.in any usr/bin/lxb-new
build_deb python python3-lxb-toolkit control-python.in all
