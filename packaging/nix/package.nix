{
  lib,
  rustPlatform,
  pkg-config,
  python3,
  src ? ../..,
  # Nix does not split a library from its header: a store path is referenced by
  # what needs it, so there is no dependency graph to keep apart and no file
  # list two packages could fight over. Those are the problems the distro
  # packages split to solve, and Nix has neither — so this is one derivation
  # carrying the library, the header, the generator, the crate sources and the
  # Python binding together.
  #
  # The Python binding is included rather than separated for the same reason:
  # it is one file of pure Python that finds the library through
  # LXB_TOOLKIT_LIBRARY, and pointing it at this derivation's own lib is
  # strictly better than making a consumer line two store paths up.
  withPython ? true,
}:

let
  sourceRoot = toString src;
  cleanSrc = lib.cleanSourceWith {
    inherit src;
    filter =
      path: type:
      let
        relative = lib.removePrefix "${sourceRoot}/" (toString path);
      in
      !(
        relative == ".git"
        || lib.hasPrefix ".git/" relative
        || relative == "target"
        || lib.hasPrefix "target/" relative
        || relative == "packaging/out"
        || lib.hasPrefix "packaging/out/" relative
        || relative == "result"
        || lib.hasPrefix "result-" relative
      );
  };
  version = lib.removeSuffix "\n" (builtins.readFile ../../VERSION);
in
rustPlatform.buildRustPackage {
  pname = "lxb-toolkit";
  inherit version;
  src = cleanSrc;

  cargoLock.lockFile = "${cleanSrc}/Cargo.lock";
  cargoBuildFlags = [ "-p" "lxb-toolkit" "-p" "lxb-toolkit-ffi" "-p" "lxb-app" "-p" "lxb-app-ffi" "-p" "lxb-new" ];
  cargoTestFlags = [ "-p" "lxb-toolkit" "-p" "lxb-toolkit-ffi" "-p" "lxb-app" "-p" "lxb-app-ffi" "-p" "lxb-new" "--all-targets" ];
  checkType = "debug";

  strictDeps = true;
  nativeBuildInputs = [ pkg-config ] ++ lib.optional withPython python3;
  # None. The library has no dependencies of its own, which is the point of it.
  buildInputs = [ ];

  # The crate the installed generator points new projects at, normalised by
  # Cargo so that it parses outside this workspace. Made in the build phase,
  # where the vendored registry is still in place.
  postBuild = ''
    cargo package --offline --frozen --no-verify -p lxb-toolkit \
      --target-dir "$CARGO_TARGET_DIR"
  '';

  # cargoInstallHook would install the binaries and nothing else. install.sh is
  # what every other package definition here uses, and using it means the Nix
  # build cannot quietly ship a different set of files than the .deb does.
  installPhase = ''
    runHook preInstall

    # install.sh reads the release directory of a target dir; buildRustPackage
    # builds under a target triple, so point it at the parent of that.
    targetDir="$(dirname "$(dirname "$(readlink -f target/*/release 2>/dev/null || echo target/release)")")"
    if [ -d "target/release" ]; then targetDir="target"; fi

    bash packaging/install.sh \
      --destdir "$out" \
      --prefix "" \
      --libdir "/lib" \
      --target-dir "$targetDir" \
      ${lib.optionalString withPython ''
        --python-sitedir "/${python3.sitePackages}" \
      ''} \
      --component ${if withPython then "all" else "devel"}

    ${lib.optionalString (!withPython) ''
      bash packaging/install.sh --destdir "$out" --prefix "" \
        --libdir "/lib" --target-dir "$targetDir" --component library
    ''}

    install -Dm0644 LICENSE "$out/share/licenses/lxb-toolkit/GPL-3.0-only.txt"
    install -Dm0644 crates/lxb-toolkit/assets/fonts/LICENSE-Roboto.txt \
      "$out/share/licenses/lxb-toolkit/Roboto-Apache-2.0.txt"
    install -Dm0644 crates/lxb-toolkit/assets/fonts/LICENSE-NotoSansDevanagariUI.txt \
      "$out/share/licenses/lxb-toolkit/NotoSansDevanagariUI-OFL-1.1.txt"
    install -Dm0644 crates/lxb-toolkit/assets/fonts/LICENSE-NotoSansCJKsc.txt \
      "$out/share/licenses/lxb-toolkit/NotoSansCJKsc-OFL-1.1.txt"
    install -Dm0644 README.md "$out/share/doc/lxb-toolkit/README.md"
    install -Dm0644 docs/api-reference.md "$out/share/doc/lxb-toolkit/api-reference.md"

    runHook postInstall
  '';

  # A generated project has to find the crate sources, and a Nix store path is
  # not /usr. The generator looks at its own location first, which works here
  # without help — but a wrapper that says so outright survives a consumer
  # copying the binary somewhere else.
  postFixup = ''
    if [ -e "$out/lib/pkgconfig/lxb-toolkit.pc" ]; then
      substituteInPlace "$out/lib/pkgconfig/lxb-toolkit.pc" \
        --replace-quiet "prefix=/" "prefix=$out"
    fi
  '';

  meta = {
    description = "The LineXinBar design language, for applications built to sit beside it";
    homepage = "https://github.com/petexy/lxb-toolkit";
    license = with lib.licenses; [ gpl3Only asl20 ];
    platforms = lib.platforms.linux;
    mainProgram = "lxb-new";
  };
}
