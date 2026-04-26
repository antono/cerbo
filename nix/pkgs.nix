{ pkgs, tauri-deps }:
let
  cerbo = pkgs.rustPlatform.buildRustPackage {
    pname = "cerbo";
    version = "0.0.11";
    src = ../.;
    cargoLock.lockFile = ../Cargo.lock;
    buildAndTestFocus = "cerbo";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = tauri-deps;
  };

  cerbo-frontend = pkgs.stdenv.mkDerivation {
    pname = "cerbo-frontend";
    version = "0.0.11";
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
    version = "0.0.11";
    src = ../.;
    cargoLock.lockFile = ../Cargo.lock;
    buildAndTestFocus = "cerbo-desktop";
    nativeBuildInputs = [
      pkgs.pkg-config
      pkgs.wrapGAppsHook3
      pkgs.gobject-introspection
      pkgs.jq
    ];
    buildInputs = tauri-deps ++ [ pkgs.gtk3 pkgs.gsettings-desktop-schemas pkgs.adwaita-icon-theme ];

    postPatch = ''
      # Nullify devUrl and point frontendDist to the built frontend in the Nix store
      jq '.build.devUrl = null | .build.frontendDist = "${cerbo-frontend}"' src-tauri/tauri.conf.json > src-tauri/tauri.conf.json.tmp
      mv src-tauri/tauri.conf.json.tmp src-tauri/tauri.conf.json
    '';

    # Ensure Tauri doesn't try to use a dev server
    TAURI_ENV_DEBUG = "false";

    postInstall = ''
      # Install desktop file
      mkdir -p $out/share/applications
      cp src-tauri/cerbo-desktop.desktop $out/share/applications/

      # Install icons
      mkdir -p $out/share/icons/hicolor/scalable/apps
      cp src-tauri/icons/logo.svg $out/share/icons/hicolor/scalable/apps/cerbo-desktop.svg

      # Install fixed-size icons
      for size in 32 64 128; do
        mkdir -p $out/share/icons/hicolor/''${size}x''${size}/apps
        cp src-tauri/icons/''${size}x''${size}.png $out/share/icons/hicolor/''${size}x''${size}/apps/cerbo-desktop.png
      done
    '';
  };
in
{
  inherit cerbo cerbo-frontend cerbo-desktop;
}
