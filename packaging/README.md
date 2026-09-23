# lxb-toolkit packaging

These definitions install the design language system-wide, so that somebody can
build an application for LineXinBar on a machine that has never seen this
checkout. Three packages come out of one source tree:

* **`lxb-toolkit`** — the shared library. What anything linking the C ABI needs
  at run time, and what the Python binding loads. It carries no GPU stack, no
  window system and no audio device, which is the point of it: a design
  language that dragged a dependency tree behind it would be a dependency
  tree. Its one dependency is the Fluent catalogs the controls it draws speak
  from, compiled in — there is no separate translation package and nothing to
  install at run time.
* **`lxb-toolkit-dev`** (`-devel` on Fedora) — the header, the static archive,
  the pkg-config file, the `lxb-new` generator, and both Rust crates as
  sources. Everything needed to *make* an application and nothing needed to run
  one.
* **`python3-lxb-toolkit`** — the `ctypes` binding. One file of pure Python, so
  it is `noarch` everywhere it can be.

Arch does not split a header from its library, so there the first two are one
package and only the Python binding is separate. Nix does not split at all: a
store path is referenced by what needs it, so there is no dependency graph to
keep apart and no file list two packages could fight over — which are the only
problems the split solves.

```text
lxb-toolkit          lib/liblxb_toolkit.so

lxb-toolkit-dev      bin/lxb-new
                     include/lxb_toolkit.h
                     lib/liblxb_toolkit.a
                     lib/pkgconfig/lxb-toolkit.pc
                     share/lxb-toolkit/crates/lxb-toolkit/**
                     share/lxb-toolkit/crates/lxb-render/**

python3-lxb-toolkit  <site-packages>/lxb_toolkit/**
```

The split is a partition, and `packaging/build.sh check` enforces it — a file
installed by neither package is one that has quietly stopped shipping, and a
file installed by both is one two packages will fight over at install time.

## Why the crate sources ship

This is the part that is not like packaging a normal library. Cargo does not
read `/usr/lib`: a Rust dependency is a registry version or a path, and nothing
else. A `lxb-new` that generated a project depending on `lxb-toolkit = "0.2.0"`
would produce something that does not build until that version is on a registry.

So the devel package installs the crates themselves under
`share/lxb-toolkit/crates`, and the generator points new projects at them.
Both, always together: `lxb-toolkit` is what the language answers and
`lxb-render` is the material it is drawn in, they are one release, and a
project built against one of each would be a project built against two.

Neither is *compiled* by a package build. They are Rust sources for a Rust
consumer, so the three packages are built with `-p lxb-toolkit -p
lxb-toolkit-ffi -p lxb-new` rather than `--workspace` — which is also what
keeps a GPU stack out of the vendored source archive. It finds that directory in three ways, most explicit first:

1. `LXB_TOOLKIT_PATH`, if set — used whether or not it exists, so a wrong one is
   reported rather than quietly replaced.
2. `LXB_TOOLKIT_CRATE_DIR`, if it was compiled in when the package was built.
3. Relative to the running binary: `…/bin/lxb-new` looks for
   `…/share/lxb-toolkit/crates/lxb-toolkit`.

The third is what makes a relocated or staged install work, and it is why
nothing here is a compiled-in absolute path alone. If none of them finds
anything, the generated project asks the registry by version, exactly as it did
before there was an installed toolkit to find.

What is installed is not a copy of `crates/lxb-toolkit`. That manifest inherits
its version, edition and licence from `[workspace.package]`; lifted out of the
workspace those keys resolve to nothing and the crate stops parsing. The
packages install the normalised manifest `cargo package` writes for publication,
and `build.sh check` refuses an installed manifest that still says
`.workspace = true`.

## One version, in one file

The version this project releases under is the single line in `VERSION` at the
root of the checkout, and what a package claims and what `lxb-new --version`
reports are the same number because both come from there.

Most things read that file where it stands: the Arch, Debian and Nix
definitions, the source archive's name, the pkg-config file. Three cannot read
a file and carry the number as a literal instead — `[workspace.package]` in
`Cargo.toml`, which is where `--version` gets it; `[project]` in
`python/pyproject.toml`, because a pyproject is data; and `Version:` in the
Fedora spec, which has to be a literal for the spec to be one anyone could
submit. A release is therefore one command that writes them from the file:

```sh
./scripts/bump-version.sh 0.3.0
```

`packaging/build.sh check` refuses to let any of them drift apart.

## Where the build happens, and why not /tmp

Every builder works under `packaging/out/build/`, on whatever filesystem the
checkout is on. Not `${TMPDIR:-/tmp}`, which is the obvious choice and the wrong
one: on a systemd machine /tmp is a tmpfs sized at a fraction of RAM, so
building there means building in memory.

This dependency graph is small — the library carries only its catalogs — but
the `cargo test` that makepkg's `check()` and rpmbuild's `%check` run pulls in
naga to prove the shipped WGSL parses, and a release-plus-test build measures
about 310 MiB. That is enough to exhaust a small tmpfs, which reports it as `No
space left on device` — or, where the tmpfs carries quotas, as `Disk quota
exceeded (os error 122)`.

Send it elsewhere with `--work-dir DIR` on the Arch and Fedora builders, or
`LXB_TOOLKIT_WORK_DIR` for all of them:

```sh
./packaging/build.sh arch --work-dir /var/tmp/lxb-toolkit
LXB_TOOLKIT_WORK_DIR=/var/tmp/lxb-toolkit ./packaging/build.sh fedora
```

A work directory inside the checkout is refused unless it is under
`packaging/out`, because `snapshot_source` picks up untracked files and a build
tree anywhere else would end up inside the source archive built from it.

The builders check free space before extracting anything, so a machine without
the room is told immediately rather than partway through. That check reads `df`,
which cannot see a quota — the default location is what actually solves the
quota case. One thing it cannot route around either: `makepkg.conf` wins over
the environment, so a machine that sets `BUILDDIR` builds there whatever
`--work-dir` said. The Arch builder notices and says so.

## Validate the definitions and their payload

```sh
./packaging/build.sh check
```

This checks shell syntax, confirms every package definition still takes its
version from `VERSION`, builds the release artefacts, confirms `lxb-new
--version` agrees with the package, stages each component, proves the three
components are a partition of the whole payload, proves the installed crate
manifest stands outside the workspace, and asks pkg-config to parse and answer
from the generated `.pc` file. Use `--no-build` when current release artefacts
already exist.

## Arch Linux

```sh
./packaging/build.sh arch
./packaging/build.sh arch -- --syncdeps
./packaging/build.sh arch -- --nocheck
```

The wrapper creates a deterministic source archive and renders a PKGBUILD with
its real SHA-256 checksum before running `makepkg`. Artifacts are copied to
`packaging/out/arch/`.

## Debian

Build on Debian, Ubuntu, or another Debian-derived system. The builder checks
everything it needs before compiling and names whatever is missing in one
`apt install` line — Rust among it as `rustup`, because Debian 13's own is
older than the locked graph allows. A distrobox or toolbox container on a plain
`debian` image is enough:

```sh
./packaging/build.sh debian
```

The builder uses `dpkg-shlibdeps` on the locally linked shared object and on
`lxb-new`, stages three policy-shaped binary packages, and writes them to
`packaging/out/debian/`. The library goes under the host's multiarch triplet,
which is why `install.sh` takes `--libdir` at all.

It compiles into `target/debian` rather than `target/` (or into
`$CARGO_TARGET_DIR` when that is set), so a build in a container that shares
the checkout never replaces the host's own binaries.

`--allow-foreign-host` exists for package-structure testing only, and says so
twice: a `.deb` built against another distribution's libc must not be deployed
on Debian, and on a host with no dpkg database there is nothing to resolve
shared libraries against, so those packages come out with no `Depends` at all
and the builder warns once per package that they have none. What such a build
is good for is looking at the shape — which files landed in which package, and
what each declares about the others.

This is an early-development shape rather than a policy-complete one: the
runtime package is named `lxb-toolkit` rather than `liblxb-toolkit0`, because
the shared object carries no SONAME yet and there is no ABI to track across a
transition. Giving it a versioned package name would be claiming a stability
promise that has not been made.

## Fedora

Build on Fedora with the RPM tools and what the spec asks for, which
`dnf builddep` reads from the spec itself:

```sh
sudo dnf install rpm-build dnf5-plugins git-core
sudo dnf builddep packaging/fedora/lxb-toolkit.spec
./packaging/build.sh fedora
./packaging/build.sh fedora --no-check
```

The builder snapshots the working tree, runs `cargo vendor --locked`, and
creates an offline source archive before invoking `rpmbuild -ba`. Binary and
source RPMs are copied to `packaging/out/fedora/`.

The spec disables the debuginfo subpackage, because Cargo's release profile
emits no DWARF for `find-debuginfo` to collect. Submitting to the Fedora archive
means reversing that: build with `-Cdebuginfo=2 -Cstrip=none` under Fedora's own
path remapping and drop `%global debug_package %{nil}`.

## Nix

```sh
./packaging/build.sh nix
nix build path:.#lxb-toolkit
nix run path:.#lxb-new -- my-player ~/my-player
nix-build packaging/nix
```

The `nix run` form is the one worth noticing: the generator and the crate
sources it points at are in the same store path, so a project made that way
builds against exactly the toolkit that made it.

## After installing

```sh
lxb-new my-player ~/my-player      # says which toolkit it bound the project to
cd ~/my-player && cargo run

cc app.c $(pkg-config --cflags --libs lxb-toolkit)
python3 -c 'import lxb_toolkit; print(lxb_toolkit.version())'
```

These recipes are intended for local and CI packages during early development.
Before submission to an official distribution archive, build in that
distribution's clean builder and complete its dependency-license review.
