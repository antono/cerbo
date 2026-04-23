{ pkgs, tauri-deps }:
let
  cerbo = pkgs.rustPlatform.buildRustPackage {
    pname = "cerbo";
    version = "0.1.0";
    src = ../.;
    cargoLock.lockFile = ../Cargo.lock;
    buildAndTestFocus = "cerbo";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = tauri-deps;
  };

  cerbo-frontend = pkgs.stdenv.mkDerivation {
    pname = "cerbo-frontend";
    version = "0.1.0";
    src = ../.;
    nativeBuildInputs = [
      pkgs.bun
      pkgs.bun2nix.hook
    ];
    bunDeps = pkgs.bun2nix.fetchBunDeps {
      bunNix = ./bun.nix;
      src = ../.;
    };
    buildPhase = ''
      bun run build
    '';
    installPhase = ''
      mkdir -p $out
      cp -r build/* $out
    '';
  };

  cerbo-desktop = pkgs.rustPlatform.buildRustPackage {
    pname = "cerbo-desktop";
    version = "0.1.0";
    src = ../.;
    cargoLock.lockFile = ../Cargo.lock;
    buildAndTestFocus = "cerbo-desktop";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = tauri-deps;
    preBuild = ''
      mkdir -p src-tauri/build
      cp -r ${cerbo-frontend}/* src-tauri/build/
    '';
    env.TAURI_DIST_DIR = "../src-tauri/build";
  };
in
{
  inherit cerbo cerbo-frontend cerbo-desktop;
}
