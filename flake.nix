{
  description = "The LineXinBar design language, for applications built to sit beside it";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          lxb-toolkit = pkgs.callPackage ./packaging/nix/package.nix { src = ./.; };
        in
        {
          inherit lxb-toolkit;
          default = lxb-toolkit;
        });

      # `nix run .#lxb-new -- my-player ~/my-player` creates an application
      # against the toolkit in the same store path as the generator that made
      # it, which is the whole reason the crate sources ship with the binary.
      apps = forAllSystems (system: rec {
        lxb-new = {
          type = "app";
          program = "${self.packages.${system}.lxb-toolkit}/bin/lxb-new";
        };
        default = lxb-new;
      });
    };
}
