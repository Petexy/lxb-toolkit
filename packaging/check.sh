#!/usr/bin/env bash

set -euo pipefail

packaging_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=packaging/lib.sh
source "$packaging_dir/lib.sh"

build=true
case "${1:-}" in
    --no-build) build=false ;;
    -h | --help)
        echo "Usage: packaging/check.sh [--no-build]"
        exit 0
        ;;
    "") ;;
    *) package_die "unknown check option: $1" ;;
esac

# The version bumper lives in scripts/ rather than in here, because the version
# is not a packaging detail — but it is the thing that writes the manifests and
# the spec this file checks, so it is checked with them.
shell_scripts() {
    find "$PACKAGING_DIR" -type f -name '*.sh' -print0
    printf '%s\0' "$PROJECT_ROOT/scripts/bump-version.sh"
}

package_note "checking shell syntax"
while IFS= read -r -d '' script; do
    bash -n "$script"
done < <(shell_scripts)
bash -n "$PACKAGING_DIR/arch/PKGBUILD.in"

if command -v shellcheck >/dev/null 2>&1; then
    while IFS= read -r -d '' script; do
        shellcheck -x "$script"
    done < <(shell_scripts)
else
    package_note "shellcheck is not installed; syntax was checked but not linted"
fi

package_note "checking the package version is consistent"
# The package and the generator report the same version, so the workspace is
# the other half of this check: `lxb-new --version` disagreeing with the package
# it was installed from is the kind of thing nobody notices until a bug report.
read_toml_version() {
    local file="$1"
    local wanted_section="$2"
    awk -v wanted="$wanted_section" '
        /^\[/ { section = $0; next }
        section == wanted \
            && match($0, /^version[[:space:]]*=[[:space:]]*"[^"]+"/) {
            line = substr($0, RSTART, RLENGTH)
            sub(/^version[[:space:]]*=[[:space:]]*"/, "", line)
            sub(/"$/, "", line)
            print line
            exit
        }' "$file"
}

workspace_version="$(read_toml_version "$PROJECT_ROOT/Cargo.toml" "[workspace.package]")"
[[ -n "$workspace_version" ]] \
    || package_die "could not read [workspace.package] version from Cargo.toml"
[[ "$workspace_version" == "$PACKAGE_VERSION" ]] \
    || package_die "VERSION says $PACKAGE_VERSION but the workspace says $workspace_version.
Run: scripts/bump-version.sh $PACKAGE_VERSION"

# The Python distribution is a second manifest that cannot read a file, and a
# wheel claiming a version the library does not is a support question nobody
# can answer from the outside.
python_version="$(read_toml_version "$PROJECT_ROOT/python/pyproject.toml" "[project]")"
[[ "$python_version" == "$PACKAGE_VERSION" ]] \
    || package_die "python/pyproject.toml declares $python_version, not $PACKAGE_VERSION.
Run: scripts/bump-version.sh $PACKAGE_VERSION"

# The FFI crate pins the core by exact version. A drifted pin does not fail to
# build — it resolves to a core this pair was never tested as.
grep -Eq "^lxb-toolkit = \{ path = \"\.\./lxb-toolkit\", version = \"${PACKAGE_VERSION//./\\.}\" \}\$" \
    "$PROJECT_ROOT/crates/lxb-toolkit-ffi/Cargo.toml" \
    || package_die "crates/lxb-toolkit-ffi/Cargo.toml does not depend on lxb-toolkit $PACKAGE_VERSION"

# The spec carries a literal version — `Version:` has to be one for the spec to
# be a spec anyone could submit — so compare it against VERSION itself: a
# pattern spelling out the version would agree with a stale spec forever, and
# the mismatch would only surface as rpmbuild failing to find its Source0.
grep -Eq "^Version:[[:space:]]+${PACKAGE_VERSION//./\\.}\$" \
    "$PACKAGING_DIR/fedora/lxb-toolkit.spec" \
    || package_die "fedora/lxb-toolkit.spec does not declare version $PACKAGE_VERSION"
# The rest take the version from VERSION, so check that they still do.
grep -Fqx 'pkgver=@VERSION@' "$PACKAGING_DIR/arch/PKGBUILD.in" \
    || package_die "arch/PKGBUILD.in no longer reads its version from VERSION"
grep -Fq 'builtins.readFile ../../VERSION' "$PACKAGING_DIR/nix/package.nix" \
    || package_die "nix/package.nix no longer reads its version from VERSION"
for control in control.in control-devel.in control-python.in; do
    grep -Fq 'Version: @VERSION@-1' "$PACKAGING_DIR/debian/$control" \
        || package_die "debian/$control no longer reads its version from VERSION"
done
for module in lxb-toolkit lxb-app; do
    grep -Fq '@VERSION@' "$PACKAGING_DIR/files/$module.pc.in" \
        || package_die "files/$module.pc.in no longer reads its version from VERSION"
done

if [[ "$build" == true ]]; then
    package_note "building the release artefacts"
    require_rust_version 1.85
    # Every crate the payload needs. This list was short by lxb-app and
    # lxb-app-ffi and the check still passed, because install.sh found a
    # liblxb_app.so an earlier build had left in target/release — a clean tree
    # would have failed here.
    (cd "$PROJECT_ROOT" && cargo build --locked --release \
        -p lxb-toolkit -p lxb-toolkit-ffi -p lxb-portal -p lxb-app -p lxb-app-ffi \
        -p lxb-new)
fi

target_dir="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
[[ "$target_dir" == /* ]] || target_dir="$PROJECT_ROOT/$target_dir"

package_note "checking the generator reports the packaged version"
generator="$target_dir/release/lxb-new"
[[ -x "$generator" ]] || package_die "missing release binary: $generator
Run without --no-build, or build it first."
reported="$("$generator" --version)"
[[ "$reported" == "lxb-new $PACKAGE_VERSION" ]] \
    || package_die "the generator reports '$reported', not 'lxb-new $PACKAGE_VERSION'"

package_note "staging the payload"
stage="$(package_work_dir lxb-toolkit-check)"
cleanup() {
    if [[ -n "${stage:-}" && "$stage" == */lxb-toolkit-check.* && -d "$stage" ]]; then
        rm -rf -- "$stage"
    fi
}
trap cleanup EXIT

# One site directory for every component, so the partition below compares
# places rather than whatever python3 happens to say between two calls.
sitedir="/usr/lib/python3/site-packages"
stage_component() {
    local component="$1"
    local into="$stage/$component"
    mkdir -p "$into"
    "$PACKAGING_DIR/install.sh" \
        --destdir "$into" \
        --prefix /usr \
        --target-dir "$target_dir" \
        --python-sitedir "$sitedir" \
        --component "$component" >/dev/null
    (cd "$into" && find . -mindepth 1 \( -type f -o -type l \) -printf '%P\n' | sort)
}

library_files="$(stage_component library)"
devel_files="$(stage_component devel)"
python_files="$(stage_component python)"
all_files="$(stage_component all)"

package_note "checking the packages are a partition of the payload"
# A file installed by no package has quietly stopped shipping; a file installed
# by two is a file two packages will fight over at install time. Both are
# invisible until somebody installs the result, so they are checked here.
partition="$(printf '%s\n%s\n%s\n' "$library_files" "$devel_files" "$python_files" | sort)"
if [[ "$partition" != "$all_files" ]]; then
    diff <(printf '%s\n' "$all_files") <(printf '%s\n' "$partition") >&2 || true
    package_die "the three components do not add up to the whole payload"
fi
duplicated="$(printf '%s\n' "$partition" | uniq -d)"
[[ -z "$duplicated" ]] \
    || package_die "installed by more than one package:
$duplicated"

package_note "checking the payload is complete"
for expected in \
    usr/lib/liblxb_toolkit.so \
    usr/lib/liblxb_toolkit.a \
    usr/lib/liblxb_app.so \
    usr/lib/liblxb_app.a \
    usr/lib/pkgconfig/lxb-toolkit.pc \
    usr/lib/pkgconfig/lxb-app.pc \
    usr/include/lxb_toolkit.h \
    usr/include/lxb_app.h \
    usr/bin/lxb-new \
    usr/share/lxb-toolkit/crates/lxb-toolkit/Cargo.toml \
    usr/share/lxb-toolkit/crates/lxb-toolkit/src/lib.rs \
    usr/share/lxb-toolkit/crates/lxb-render/src/lib.rs \
    usr/share/lxb-toolkit/crates/lxb-render/src/ui.wgsl \
    usr/share/lxb-toolkit/crates/lxb-input/src/lib.rs \
    usr/share/lxb-toolkit/crates/lxb-input/src/from_winit.rs \
    usr/share/lxb-toolkit/crates/lxb-sound/src/lib.rs \
    usr/share/lxb-toolkit/crates/lxb-app/src/lib.rs; do
    printf '%s\n' "$all_files" | grep -Fqx "$expected" \
        || package_die "the staged payload is missing $expected"
done

# The Python modules are asked of the checkout rather than listed here: this
# check existed while paint.py was missing from every installed copy, because
# both this list and install.sh's were written out by hand and neither knew
# about it.
while IFS= read -r module; do
    expected="${sitedir#/}/lxb_toolkit/$module"
    printf '%s\n' "$all_files" | grep -Fqx "$expected" \
        || package_die "the staged payload is missing $expected"
done < <(cd "$PROJECT_ROOT/python/lxb_toolkit" && find . -maxdepth 1 -name '*.py' -printf '%P\n' | sort)

package_note "checking the staged Python package imports"
# The list above says the files are present; this says the package works. A
# checkout that imports proves nothing about an install, because the checkout
# has every module whether or not install.sh knows to copy it — which is
# exactly how a package that could not be imported at all came to ship.
# Both libraries are pointed at the staged copies, so nothing here reads
# whatever happens to be installed on this machine.
if command -v python3 >/dev/null 2>&1; then
    PYTHONPATH="$stage/all$sitedir" \
    LXB_TOOLKIT_LIBRARY="$stage/all/usr/lib/liblxb_toolkit.so" \
    LXB_APP_LIBRARY="$stage/all/usr/lib/liblxb_app.so" \
        python3 -c '
import lxb_toolkit
import lxb_toolkit.app
lxb_toolkit.paint.glass
lxb_toolkit.app.App
' || package_die "the staged Python package does not import"
else
    package_note "python3 is not installed; the staged package was not imported"
fi

package_note "checking the API reference is current"
# Generated from the headers, so a header edited without regenerating ships a
# reference that describes a library nobody has.
if command -v python3 >/dev/null 2>&1; then
    (cd "$PROJECT_ROOT" && python3 scripts/make-reference.py --check >/dev/null) \
        || package_die "docs/api-reference.md is out of date: run python3 scripts/make-reference.py"
else
    package_note "python3 is not installed; the API reference was not checked"
fi

package_note "checking the installed crates stand on their own"
# Every crate in the checkout inherits its version, edition and licence from
# [workspace.package]. Lifted out of the workspace those keys resolve to
# nothing, so what is installed has to carry them as literals — and this is the
# check that it still does.
for installed in lxb-toolkit lxb-render lxb-input lxb-sound lxb-app; do
    installed_manifest="$stage/all/usr/share/lxb-toolkit/crates/$installed/Cargo.toml"
    [[ -f "$installed_manifest" ]] \
        || package_die "$installed was not installed as sources"
    grep -Fq "version = \"$PACKAGE_VERSION\"" "$installed_manifest" \
        || package_die "the installed $installed manifest does not carry a literal version"
    ! grep -Fq '.workspace = true' "$installed_manifest" \
        || package_die "the installed $installed manifest still inherits from a workspace it will not have"
done

# And the set has to resolve as a set: each of the four names lxb-toolkit by
# the path it will actually be at once installed, which is beside it.
for installed in lxb-render lxb-input lxb-sound lxb-app; do
    installed_manifest="$stage/all/usr/share/lxb-toolkit/crates/$installed/Cargo.toml"
    grep -Fq 'path = "../lxb-toolkit"' "$installed_manifest" \
        || package_die "the installed $installed does not point at the toolkit beside it"
done

package_note "checking the pkg-config file answers"
if command -v pkg-config >/dev/null 2>&1; then
    pc_dir="$stage/all/usr/lib/pkgconfig"
    for module in lxb-toolkit lxb-app; do
        pc_version="$(PKG_CONFIG_PATH="$pc_dir" pkg-config --modversion "$module")"
        [[ "$pc_version" == "$PACKAGE_VERSION" ]] \
            || package_die "pkg-config reports $pc_version for $module, not $PACKAGE_VERSION"
        PKG_CONFIG_PATH="$pc_dir" pkg-config --validate "$module" \
            || package_die "the generated $module.pc is not valid"
    done
    # And the application layer has to bring the toolkit with it, or a program
    # that asked for one library would be told about half a language.
    PKG_CONFIG_PATH="$pc_dir" pkg-config --libs lxb-app | grep -Fq -- -llxb_toolkit \
        || package_die "lxb-app.pc does not require lxb-toolkit"
else
    package_note "pkg-config is not installed; the .pc file was not exercised"
fi

# And the Fedora file lists have to describe that payload, which nothing above
# asks. Everything before this compares the checkout and the components against
# each other; the spec that ships them is checked only by the greps written out
# by hand in this file. That is how LineXinBar's own spec came to package a
# drawing that had left the tree, and to leave forty-five installed files in no
# package at all — neither visible until rpmbuild reached the end of a build it
# had already paid for in full.
#
# Both directions, because RPM fails on both: an entry with nothing behind it
# is "File not found", and a staged file no entry covers is "Installed (but
# unpackaged) file(s) found".
package_note "checking the Fedora file lists against the staged payload"
spec_root="$stage/all"
spec_lists="$(mktemp -d)"
spec_entries() {
    awk '
        /^%files/ { inside = 1; next }
        /^%(changelog|prep|build|check|install|package|description|pre|post|preun|postun)/ { inside = 0 }
        !inside { next }
        /^[[:space:]]*(#|$)/ { next }
        # %license and %doc are filled by RPM from the source tree, not from
        # the buildroot, so they are not part of what install.sh stages.
        /^%(license|doc)[[:space:]]/ { next }
        {
            entry = $0
            kind = "path"
            if (entry ~ /^%dir[[:space:]]/) { kind = "dir"; sub(/^%dir[[:space:]]+/, "", entry) }
            sub(/^%config\([^)]*\)[[:space:]]+/, "", entry)
            sub(/^%config[[:space:]]+/, "", entry)
            print kind "\t" entry
        }
    ' "$PACKAGING_DIR/fedora/lxb-toolkit.spec"
}

: > "$spec_lists/packaged"
while IFS=$'\t' read -r kind entry; do
    entry="$(printf '%s\n' "$entry" | sed \
        -e 's|%{_bindir}|/usr/bin|g' \
        -e 's|%{_libdir}|/usr/lib|g' \
        -e 's|%{_includedir}|/usr/include|g' \
        -e 's|%{_datadir}|/usr/share|g' \
        -e "s|%{python3_sitelib}|$sitedir|g" \
        -e "s|%{version}|$PACKAGE_VERSION|g" \
        -e 's|%{_prefix}|/usr|g')"
    # Refused rather than skipped: an entry this cannot read is an entry that
    # would go unchecked, which is the state the whole check exists to end.
    if [[ "$entry" == *'%{'* ]]; then
        package_die "check.sh cannot expand the %files entry $entry.
Teach the expansions above the macro rather than leaving the entry unchecked."
    fi
    entry="${entry%/}"
    if [[ "$kind" == dir ]]; then
        # %dir packages the directory itself and none of its contents, so it
        # covers nothing: a file under it still needs an entry of its own.
        [[ -d "$spec_root$entry" ]] \
            || package_die "the spec packages the directory $entry, which nothing creates"
        continue
    fi
    if [[ -d "$spec_root$entry" ]]; then
        (cd "$spec_root" && find ".$entry" \( -type f -o -type l \) -printf '%p\n') \
            | sed 's|^\./||' >> "$spec_lists/packaged"
    elif [[ -f "$spec_root$entry" || -L "$spec_root$entry" ]]; then
        printf '%s\n' "${entry#/}" >> "$spec_lists/packaged"
    else
        package_die "the spec packages $entry, which nothing installs"
    fi
done < <(spec_entries)

sort -u "$spec_lists/packaged" -o "$spec_lists/packaged"
(cd "$spec_root" && find . -mindepth 1 \( -type f -o -type l \) -printf '%P\n' | sort) \
    > "$spec_lists/staged"
if comm -23 "$spec_lists/staged" "$spec_lists/packaged" | grep -q .; then
    package_die "installed and packaged by no %files section: $(
        comm -23 "$spec_lists/staged" "$spec_lists/packaged" | tr '\n' ' ')"
fi
rm -rf -- "$spec_lists"

package_note "all checks passed for lxb-toolkit $PACKAGE_VERSION"
