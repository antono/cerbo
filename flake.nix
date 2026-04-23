{
  description = "Cerbo: A minimalist personal knowledge base";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        tauri-deps = with pkgs; [
          pkg-config
          dbus
          openssl
          glib
          gtk3
          dconf
          gsettings-desktop-schemas
          adwaita-icon-theme
          cairo
          gdk-pixbuf
          librsvg
          webkitgtk_4_1
          libsoup_3
          xdg-utils
        ];
        dev-deps = with pkgs; [
          bun
          cargo-tauri
          rustc
          cargo
          clippy
          rustfmt
          rust-analyzer
        ];

        cerbo = pkgs.rustPlatform.buildRustPackage {
          pname = "cerbo";
          version = "0.1.0";
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          # For monorepo support, we might need to filter or specify the member
          buildAndTestFocus = "cerbo";

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = tauri-deps;
        };

        cerbo-frontend = pkgs.stdenv.mkDerivation {
          pname = "cerbo-frontend";
          version = "0.1.0";
          src = ./.;

          nativeBuildInputs = [ pkgs.bun ];

          buildPhase = ''
            bun install --frozen-lockfile
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
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
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
        packages = {
          default = cerbo;
          cerbo = cerbo;
          cerbo-desktop = cerbo-desktop;
          cerbo-frontend = cerbo-frontend;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = tauri-deps ++ dev-deps;

          shellHook = ''
            echo "Cerbo dev environment"
            echo "  rustc $(rustc --version)"
            echo "  bun   $(bun --version)"
            cargo tauri --version 2>/dev/null || echo "  cargo-tauri: available"
          '';

          env = {
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            WEBKIT_DISABLE_COMPOSITING_MODE = "1";
            XDG_DATA_DIRS =
              pkgs.lib.makeSearchPath "share" [
                pkgs.gsettings-desktop-schemas
                pkgs.gtk3
                pkgs.adwaita-icon-theme
              ]
              + ":$XDG_DATA_DIRS";
          };
        };
      }
    );
}
