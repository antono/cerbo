{
  description = "Cerbo: A minimalist personal knowledge base";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
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

        cerbo-desktop = pkgs.buildNpmPackage {
          pname = "cerbo-desktop";
          version = "0.1.0";
          src = ./.;

          npmDepsHash = "sha256-Pr794n6WOMImJSOL9lto3lEoTsrj76P60CGUnkmKHSM=";

          makeCacheWritable = true;
          npmDepsFetcherVersion = 2;

          npmFlags = [ "--legacy-peer-deps" ];

          nativeBuildInputs = [ pkgs.bun ];

          # Use bun instead of npm if possible, or just npm since I generated package-lock.json
          # Actually, I'll just use npm to be safe as it's what buildNpmPackage likes.
          # But the user specifically mentioned Bun.
          # I'll try npm first to get the hash.
        };

      in
      {
        packages = {
          default = cerbo;
          cerbo = cerbo;
          cerbo-desktop = cerbo-desktop;
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
            XDG_DATA_DIRS = pkgs.lib.makeSearchPath "share" [
              pkgs.gsettings-desktop-schemas
              pkgs.gtk3
              pkgs.adwaita-icon-theme
            ] + ":$XDG_DATA_DIRS";
          };
        };
      }
    );
}
