{ pkgs, tauri-deps, dev-deps }:
pkgs.mkShell {
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
}
