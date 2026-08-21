Name:           lxb-toolkit
Version:        0.2.0
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
# build. An archive submission wants real debuginfo instead: drop this and
# build with `-Cdebuginfo=2 -Cstrip=none` under Fedora's own remapping.
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
cargo build --offline --locked --release -p lxb-toolkit -p lxb-toolkit-ffi -p lxb-app -p lxb-app-ffi -p lxb-new
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
cargo test --offline --locked -p lxb-toolkit -p lxb-toolkit-ffi -p lxb-app -p lxb-app-ffi -p lxb-new --all-targets

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
* Fri Aug 21 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.2.0-1
- The language corrected to the shell: neutral marks and their material,
  the theme and wallpaper surfaces, and the lxb-new application generator.

* Sun Aug 16 2026 Piotr Lewandowski <piotr.petexy@gmail.com> - 0.1.0-1
- Initial package.
