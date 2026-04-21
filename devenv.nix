{ pkgs, ... }:

{
  # Rust stable toolchain (rustc, cargo, clippy, rustfmt, rust-analyzer)
  languages.rust.enable = true;

  packages = with pkgs; [
    # JavaScript runtime + package manager
    bun

    # Tauri CLI
    cargo-tauri

    # Tauri v2 Linux system dependencies
    pkg-config
    dbus
    openssl
    glib
    gtk3
    cairo
    gdk-pixbuf
    librsvg
    webkitgtk_4_1
    libsoup_3
  ];

  env = {
    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
    WEBKIT_DISABLE_COMPOSITING_MODE = "1"; # prevents rendering issues on some setups
  };

  enterShell = ''
    echo "Cerbo dev environment"
    echo "  rustc $(rustc --version)"
    echo "  bun   $(bun --version)"
    cargo tauri --version 2>/dev/null || echo "  cargo-tauri: available"
  '';

  tasks = {
    "app:dev" = {
      exec = "cargo tauri dev";
      cwd = "src-tauri";
    };
    "app:build" = {
      exec = "cargo tauri build";
      cwd = "src-tauri";
    };
    "app:check" = {
      exec = "cargo clippy";
      cwd = "src-tauri";
    };
    "frontend:dev" = {
      exec = "bun run dev";
    };
    "frontend:build" = {
      exec = "bun run build";
    };
    "frontend:check" = {
      exec = "bun run check";
    };
  };
}
