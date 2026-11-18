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
        inherit (cerbo-pkgs) cerbo cerboid cerbo-frontend cerbo-desktop tauri-driver;

        src = builtins.path { path = ./.; name = "cerbo-src"; };
        releaseWorkflowCheck = pkgs.writeShellApplication {
          name = "release-workflow-check";
          runtimeInputs = [ pkgs.actionlint ];
          text = ''
            actionlint ${src}/.github/workflows/release.yml
          '';
        };

        # scripts/screenshot.sh with its tools and the desktop app built in:
        # `nix run .#screenshot -- -s 390x844` from the repo root.
        screenshot = pkgs.runCommand "cerbo-screenshot" {
          nativeBuildInputs = [ pkgs.makeWrapper ];
          meta.mainProgram = "cerbo-screenshot";
        } ''
          install -Dm755 ${./scripts/screenshot.sh} $out/bin/cerbo-screenshot
          wrapProgram $out/bin/cerbo-screenshot \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.xvfb-run pkgs.xdotool pkgs.imagemagick ]} \
            --set-default CERBO_DESKTOP_BIN ${cerbo-desktop}/bin/cerbo-desktop
        '';
      in
      {
        packages = {
          default = cerbo;
          inherit cerbo cerboid cerbo-frontend cerbo-desktop tauri-driver;
          release-workflow-check = releaseWorkflowCheck;
          inherit screenshot;
        };

        checks.release-workflow = pkgs.runCommand "release-workflow-check" {
          nativeBuildInputs = [ pkgs.actionlint ];
        } ''
          ${releaseWorkflowCheck}/bin/release-workflow-check
          touch $out
        '';

        # Every vault write must go through cerbo_core::fsio::write_atomic.
        checks.atomic-writes = pkgs.runCommand "atomic-writes-check" {
          nativeBuildInputs = [ pkgs.python3 ];
        } ''
          python3 ${src}/scripts/check-atomic-writes.py ${src}/core/src
          touch $out
        '';

        apps.screenshot = {
          type = "app";
          program = "${screenshot}/bin/cerbo-screenshot";
        };

        apps.release-workflow-check = {
          type = "app";
          program = "${releaseWorkflowCheck}/bin/release-workflow-check";
        };

        devShells.default = import ./nix/devshell.nix {
          inherit pkgs tauri-deps dev-deps tauri-driver;
        };
      }
    );
}
