{
  description = "Cerbo: A minimalist personal knowledge base";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    bun2nix.url = "github:nix-community/bun2nix";
    bun2nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      bun2nix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ bun2nix.overlays.default ];
        };

        deps = import ./nix/deps.nix { inherit pkgs; };
        inherit (deps) tauri-deps dev-deps;

        cerbo-pkgs = import ./nix/pkgs.nix { inherit pkgs tauri-deps; };
        inherit (cerbo-pkgs) cerbo cerbo-frontend cerbo-desktop;
      in
      {
        packages = {
          default = cerbo;
          inherit cerbo cerbo-frontend cerbo-desktop;
        };

        checks.release-workflow =
          let
            src = builtins.path { path = ./.; name = "cerbo-src"; };
          in
          pkgs.runCommand "release-workflow-check" {
            nativeBuildInputs = [ pkgs.actionlint ];
          } ''
          actionlint ${src}/.github/workflows/release.yml
          touch $out
          '';

        devShells.default = import ./nix/devshell.nix {
          inherit pkgs tauri-deps dev-deps;
        };
      }
    );
}
