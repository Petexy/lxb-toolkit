Name:           lxb-toolkit
Version:        0.9.0
Release:        1%{?dist}
Summary:        The LineXinBar design language, for applications built to sit beside it

# The library itself, the embedded Roboto faces, and the locked Rust dependency
# graph the tests build against.
License:        GPL-3.0-only AND Apache-2.0 AND MIT AND Apache-2.0 WITH LLVM-exception AND Unicode-3.0
URL:            https://github.com/petexy/lxb-toolkit
Source0:        lxb-toolkit-%{version}.tar.gz

ExclusiveArch:  x86_64 aarch64

# Cargo's release profile emits no DWARF, so find-debuginfo would produce an
# empty debugsourcefiles.list and rpmbuild would fail on it after the whole
# build. An archive submission wants real debuginfo instead: drop this, and
# with it the -Cdebuginfo=0 in %build that holds Fedora's own -Cdebuginfo=2 off,
# so the DWARF is built and packaged rather than built and binned.
%global debug_package %{nil}

# python3-devel supplies this on Fedora. Defining it only when it is missing
# lets the spec also build where those macros are not installed, rather than
# handing %install the unexpanded macro name as if it were a path.
%{!?python3_sitelib: %global python3_sitelib %(python3 -c "import sysconfig; print(sysconfig.get_path('purelib'))" 2>/dev/null || echo /usr/lib/python3/site-packages)}

BuildRequires:  cargo >= 1.85
BuildRequires:  rust >= 1.85
BuildRequires:  gcc
BuildRequires:  pkgconfig
BuildRequires:  python3-devel

%description
The interface vocabulary of the LineXinBar shell — its colour roles, its glass,
its motion, its type, its 98 marks and its recordings — packaged so that
something which is not that shell can be built out of the same material and
belong beside it.

This package is the shared library. It has no dependencies of its own: a design
language that dragged a dependency tree behind it would be a dependency tree.

%package        devel
Summary:        Header, static library and application generator for lxb-toolkit
Requires:       %{name}%{?_isa} = %{version}-%{release}
# The generated project is a Cargo project, and Cargo reads a path or a
# registry rather than the library directory. So this package also carries
# the crate sources lxb-new points new projects at, and cargo builds them.
Requires:       cargo

%description    devel
The C header, the static archive, the pkg-config file, and lxb-new: the
generator that creates a runnable Wayland application with a matching desktop
entry and icon, already speaking the design language.

Also installs the toolkit's Rust crate sources under
%{_datadir}/lxb-toolkit, which is what a generated project builds against on a
machine that has never seen the upstream checkout.

%package        -n python3-%{name}
Summary:        The LineXinBar design language, for Python
Requires:       %{name}%{?_isa} = %{version}-%{release}
BuildArch:      noarch

%description    -n python3-%{name}
The same design language through the same native core as the Rust and C APIs.
The module is ctypes over the C ABI, so there is no build step and no wheel per
Python version.

%prep
%autosetup -n lxb-toolkit-%{version}

%build
export RUSTUP_TOOLCHAIN=stable
export CARGO_TARGET_DIR=target
# Fedora exports its own %%{build_rustflags} into RUSTFLAGS before this runs, and
# they carry -Cdebuginfo=2 -Cstrip=none. RUSTFLAGS is appended after the release
# profile's own flags and wins, so every crate here was generating full DWARF —
# and with %%global debug_package %%{nil} above, no package was ever made of it.
# -Cdebuginfo=0 last turns that off. This profile asks for `lto = true`, which
# is fat LTO rather than thin, and the two together are the largest compiler in
# any of these repositories: the final rustc measures 1898 MiB with the DWARF
# and 1578 MiB without.
export RUSTFLAGS="${RUSTFLAGS:-} -Cdebuginfo=0"

# And Cargo takes its job count from the core count alone, knowing nothing about
# how much memory the machine has to hold that many of those at once. The sister
# repository's shell was killed by the kernel's OOM killer twice on an 8 GiB
# Apple M1 for want of exactly this.
#
# Arithmetic rather than %%limit_build, the Fedora macro meant for this, which
# swallowed the remainder of the script it was used in on Fedora Asahi.
build_jobs="%{_smp_build_ncpus}"
build_room="$(awk '/^MemTotal:/ { n = int($2 / 1024 / 2048); print (n < 1 ? 1 : n) }' /proc/meminfo 2>/dev/null || true)"
if [ -n "$build_room" ] && [ "$build_room" -lt "$build_jobs" ]; then
    build_jobs="$build_room"
fi
echo "building with $build_jobs of %{_smp_build_ncpus} jobs, for the memory this machine has"
cargo build --offline --locked --release -p lxb-toolkit -p lxb-toolkit-ffi -p lxb-app -p lxb-app-ffi -p lxb-new -j"$build_jobs"
# The crate the installed generator points new projects at, normalised by Cargo
# so that it parses outside this workspace. Built here rather than in %install,
# which is not a build phase.
cargo package --offline --locked --no-verify -p lxb-toolkit

%install
export CARGO_TARGET_DIR=target
./packaging/install.sh \
    --destdir %{buildroot} \
    --prefix %{_prefix} \
    --libdir %{_libdir} \
    --target-dir target \
    --python-sitedir %{python3_sitelib} \
    --component all

%check
export RUSTUP_TOOLCHAIN=stable
export CARGO_TARGET_DIR=target
# The same two as %%build. The dev profile asks for full DWARF and this phase
# builds the graph a second time to get it, with no package made of it either;
# a failing test still names its file and line, which the panic carries rather
# than DWARF.
export RUSTFLAGS="${RUSTFLAGS:-} -Cdebuginfo=0"
build_jobs="%{_smp_build_ncpus}"
build_room="$(awk '/^MemTotal:/ { n = int($2 / 1024 / 2048); print (n < 1 ? 1 : n) }' /proc/meminfo 2>/dev/null || true)"
if [ -n "$build_room" ] && [ "$build_room" -lt "$build_jobs" ]; then
    build_jobs="$build_room"
fi
cargo test --offline --locked -p lxb-toolkit -p lxb-toolkit-ffi -p lxb-app -p lxb-app-ffi -p lxb-new --all-targets -j"$build_jobs"

%files
%license LICENSE
%doc README.md
%{_libdir}/liblxb_toolkit.so
%{_libdir}/liblxb_app.so

%files devel
%license LICENSE crates/lxb-toolkit/assets/fonts/LICENSE-Roboto.txt
%doc docs/design-language.md docs/application-development.md docs/api-reference.md
%{_bindir}/lxb-new
%{_includedir}/lxb_toolkit.h
%{_includedir}/lxb_app.h
%{_libdir}/liblxb_toolkit.a
%{_libdir}/liblxb_app.a
%{_libdir}/pkgconfig/lxb-toolkit.pc
%{_libdir}/pkgconfig/lxb-app.pc
%{_datadir}/lxb-toolkit/

%files -n python3-%{name}
%license LICENSE
%doc python/README.md
%{python3_sitelib}/lxb_toolkit/
%{python3_sitelib}/lxb_toolkit-%{version}.dist-info/

%changelog
* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.9.0-1
- One number, across four projects. LineXinBar, this toolkit, the greeter and
  the store now release under the same version, so that what somebody has
  installed can be read off one number instead of four that move apart. Nothing
  else changed: this is 0.3.11's code, and the API, the ABI and the payload are
  what they were. An application asking for lxb-app 0.3.11 has to be moved to
  0.9.0 to build against it, because a caret requirement on a 0.x version pins
  the minor.

* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.11-1
- A touchscreen, and a bar for the hands that are not on a pad. A finger was
  the one thing an application could not be worked with at all: no press
  reached it and no list moved under it. A drag is now turned into the same
  directions a wheel sends, so every page that already scrolled scrolls under a
  finger without knowing a finger exists — pulled down is the list coming back
  up, the way every touchscreen agrees it is — and a finger that stayed where
  it was put is a tap and presses what it is on. Ui::scroll_bar draws a bar
  down the edge of a list, and Page::dragging is the one gesture a press and a
  release cannot describe between them: a bar taken hold of and moved.

* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.10-1
- A plus, transcribed from the shell. The set had every verb for a thing that
  already exists — copy it, move it, rename it, throw it away — and no mark at
  all for the one that brings a new one into being, so a row that adds wore
  whatever was nearest and said something else by it. Like every other mark it
  is a shape the material is computed from, so it takes the bead and the gloss
  under theme-icons = "Default" and comes out flat under "Simple". 109 marks.

* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.9-1
- Escape reaches a page that steers itself. It closed the window, which on any
  application with more than one screen made going back impossible: every
  screen behind a press read Action::Back and not one of them ever saw it. A
  page the toolkit walks the controls for has no screen to go back to, so
  there Escape is still the way out, and a dialog, a menu or the file chooser
  still takes it for itself.
- A menu's frost no longer reaches outside its own pane. It was grown a little
  so that the refraction at the pane's edges would find frost rather than the
  sharp page; the stain is nearly black, so what that really drew was a dark
  ring hanging outside the pane with nothing over it. This language does not
  draw shadows.

* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.8-1
- A context menu frosts what is under its own pane. The dialog has done this
  since 0.3.5; a menu had not, and over an application's page — which is free
  to be a grid of icons and screenshots — its rows were being read over
  whatever happened to be beneath them. Under the pane and no further: a menu
  is not modal, and the page around one is still in play. It follows the
  pane's own growth, because the pane is drawn at its final rectangle and
  flown to the growing one, and a frost left where the pane was going would
  sit off to one side for the whole of the opening.

* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.7-1
- Page::light_at / lxb_page_light_at, so a page that lays out its own controls
  can say where the light is standing. A menu grows out of that rectangle, and
  lxb_page only recorded it for the rows and buttons it draws itself — so a
  menu raised over a card a page laid out itself grew out of the corner of the
  window rather than out of the thing it was raised over.
- Page::menu_marked / lxb_page_menu_marked, the same menu with the one command
  already in force wearing the language's own chosen mark. A menu of
  alternatives that does not say which one you are on is a menu somebody has to
  press to find out. Both are mirrored in the C ABI and the Python binding.

* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.6-1
- A dialog's body is white ink made quiet by its alpha, not by its hue. It was
  Role::TextSoft, which every palette cuts as a pale cast of the accent: over
  the shell's own dark surfaces that passes for a soft white, and over the
  bright patch of somebody's screenshot showing through the pane it came back
  as lavender on grey, at almost no contrast, in the middle of a sentence. The
  answers below it already quieted themselves with Role::Text at
  control::INK_QUIET; the body is the one part of a dialog that did not.

* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.5-1
- The dialog's frost now reaches under the dialog. A pane of glass samples the
  source texture rather than the picture being painted, so 0.3.4's frost — laid
  beside the pane on the overlay layer — frosted the whole window except the
  rectangle it was for: the pane read the page back out of the source, sharp
  and bright. It is laid on SOFTEN now, where the pane can see it. The stain
  goes from 0.12 to 0.55 and the frost to a panel's own 0.95: blur takes away
  detail and leaves brightness, so a screenshot of a light desktop window was
  still a white slab under the words however deep the pyramid went.

* Sun Aug 30 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.4-1
- A dialog frosts the page it covers. Page::ask already dimmed what was
  behind it; over an application's own page that was not enough, because a
  page is free to be a photograph and a dialog's words then sit on top of one.
  The page now goes into the blur pyramid as well, far enough that nothing on
  it can be read at body size and no further, so somebody asked to confirm a
  removal can still see what they are removing. The dialog's own pane is
  unchanged: it is still the clearer sidebar cut, and what changed is what it
  is looking at.

* Sat Aug 29 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.3-1
- A wheel and a touchpad are directions like every other control: they arrive
  as Up and Down in the ordinary action queue, in the order they happened, and
  keep their fractions between events, so every application scrolls in all
  three languages. Page::scrolls, Page::scrolled and lxb_page_scrolled say
  where the pointer was, for a page with more than one list to move the one
  under the hand. Ui::soft_edges and lxb_draw_soft_edges blur and dissolve
  the ends of a scrolling area after its surfaces and its words have been
  drawn together, leaving the controls around it sharp. Blur and translucency
  grow together and end in whatever is behind the page's own content, so a
  list stops without anything to stop at.

* Sat Aug 29 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.2-1
- Ui::written and Ui::cut_between, so a page can cut everything it drew
  between two marks down to a rectangle. A list whose rows are taller than the
  room it has can now end at an edge instead of at a whole row, which is what
  a list needs to say that it runs on without drawing over whatever it stops
  short of.

* Fri Aug 28 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.1-1
- A page can really be typed into: what it asks for is in force between one
  frame and the next, which is when a key arrives, and the window is told so
  through zwp_text_input_v3, which is what summons an on-screen keyboard.
  Page::pad_in_hand says which control the user last reached for, watched the
  way the shell watches it.

* Fri Aug 28 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.3.0-1
- Ui::picture and Fit, so an application can draw a picture of its own;
  Page::taking_text and Page::typed, so a page can be typed into; and the
  thumbnail cell at 512 px.

* Fri Aug 21 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.2.0-1
- The language corrected to the shell: neutral marks and their material,
  the theme and wallpaper surfaces, and the lxb-new application generator.

* Sun Aug 16 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.1.0-1
- Initial package.
