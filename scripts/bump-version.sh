#!/usr/bin/env bash

# Set the version this project releases under.
#
# `VERSION` at the root of the checkout is that number, and almost everything
# reads it where it stands: the Arch, Debian and Nix definitions, the source
# archive's name, the pkg-config file, and the installed crate's path.
#
# Three places cannot read a file and carry the number as a literal instead.
# Cargo's manifest is one: `lxb-new --version` compiles in `CARGO_PKG_VERSION`,
# and `[workspace.package] version` is where that comes from. The Python
# distribution is another, because a pyproject.toml is data and cannot read a
# file either. The Fedora spec is the third: `Version:` has to be a literal for
# the spec to be one anyone could submit. This writes all of them, which is what
# makes a release one command rather than four edits that have to agree.

set -euo pipefail

scripts_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd "$scripts_dir/.." && pwd)"
version_file="$project_root/VERSION"
manifest="$project_root/Cargo.toml"
pyproject="$project_root/python/pyproject.toml"
spec="$project_root/packaging/fedora/lxb-toolkit.spec"

die() {
    echo "error: $*" >&2
    exit 1
}

note() {
    echo "==> $*"
}

usage() {
    cat <<'USAGE'
Usage: scripts/bump-version.sh X.Y.Z

Writes the version into VERSION, [workspace.package] in Cargo.toml, Cargo.lock,
python/pyproject.toml and packaging/fedora/lxb-toolkit.spec. Every other package
definition reads VERSION for itself.

Run with the version already in VERSION to write the other four back into
agreement with it.
USAGE
}

case "${1:-}" in
    -h | --help)
        usage
        exit 0
        ;;
    "")
        usage >&2
        exit 1
        ;;
esac

new_version="$1"
shift
[[ $# -eq 0 ]] || die "unexpected argument: $1"

# The same shape packaging/lib.sh insists on when it reads the file back, and
# the same shape a Cargo version and an RPM Version: can both be.
[[ "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
    || die "version must be X.Y.Z, not $new_version"

[[ -f "$manifest" ]] || die "no manifest at $manifest"
[[ -f "$pyproject" ]] || die "no Python manifest at $pyproject"
[[ -f "$spec" ]] || die "no spec at $spec"

note "VERSION -> $new_version"
printf '%s\n' "$new_version" > "$version_file"

# Only inside [workspace.package]: the member manifests say `version.workspace
# = true`, and a blind substitution would also rewrite the FFI crate's
# dependency on the core, which has to stay a requirement rather than become
# whatever this file says today.
note "Cargo.toml [workspace.package] -> $new_version"
awk -v version="$new_version" '
    /^\[/ { section = $0 }
    section == "[workspace.package]" && /^version[[:space:]]*=/ && !done {
        print "version = \"" version "\""
        done = 1
        next
    }
    { print }
' "$manifest" > "$manifest.tmp"
mv "$manifest.tmp" "$manifest"

# The FFI crate depends on the core by an exact version, so that a mismatched
# pair cannot be resolved rather than being resolved to something untested.
# Every crate here that names another one names its version too, so that a
# published release cannot resolve against a different one. All of them move
# together or the workspace stops resolving; rewriting only the FFI crate's is
# what shipped 0.2.0 with five stale requirements.
#
# The Rust examples are in this list for the same reason and are easy to
# forget, because they are deliberately outside the workspace and so no
# workspace command reaches them. They are built the way an application is,
# which is exactly why a stale requirement there is worth catching.
for manifest in "$project_root"/crates/*/Cargo.toml "$project_root"/examples/*/Cargo.toml; do
    if ! grep -qE '^lxb-[a-z-]+[[:space:]]*=.*version[[:space:]]*=' "$manifest"; then
        continue
    fi
    note "${manifest#"$project_root/"} lxb-* -> $new_version"
    awk -v version="$new_version" '
        /^lxb-[a-z-]+[[:space:]]*=/ {
            sub(/version[[:space:]]*=[[:space:]]*"[^"]*"/, "version = \"" version "\"")
        }
        { print }
    ' "$manifest" > "$manifest.tmp"
    mv "$manifest.tmp" "$manifest"
done

note "python/pyproject.toml -> $new_version"
awk -v version="$new_version" '
    /^\[/ { section = $0 }
    section == "[project]" && /^version[[:space:]]*=/ && !done {
        print "version = \"" version "\""
        done = 1
        next
    }
    { print }
' "$pyproject" > "$pyproject.tmp"
mv "$pyproject.tmp" "$pyproject"

note "packaging/fedora/lxb-toolkit.spec Version: -> $new_version"
awk -v version="$new_version" '
    /^Version:[[:space:]]/ && !done {
        printf "Version:        %s\n", version
        done = 1
        next
    }
    { print }
' "$spec" > "$spec.tmp"
mv "$spec.tmp" "$spec"

# Every packaged build is `--locked` or `--frozen`, so a lock file left behind
# is a build that refuses to start rather than one that quietly updates.
note "Cargo.lock"
(cd "$project_root" && cargo update --workspace --offline >/dev/null 2>&1) \
    || (cd "$project_root" && cargo update --workspace >/dev/null)

# Each example keeps a lock file of its own, because each is built the way an
# application is rather than as part of the workspace. A lock still naming the
# old version is what makes `cargo --locked` refuse the whole example.
for example in "$project_root"/examples/*/Cargo.lock; do
    [ -e "$example" ] || continue
    directory="$(dirname "$example")"
    note "${directory#"$project_root/"}/Cargo.lock"
    (cd "$directory" && cargo update --workspace --offline >/dev/null 2>&1) \
        || (cd "$directory" && cargo update --workspace >/dev/null)
done

note "done. The spec's %changelog is the one thing only a person can write:"
echo "    packaging/fedora/lxb-toolkit.spec"
echo "Then: packaging/build.sh check"
