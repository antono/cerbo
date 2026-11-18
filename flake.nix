{
  description = "Cerbo: A minimalist personal knowledge base";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    bun2nix.url = "github:nix-community/bun2nix";
    bun2nix.inputs.nixpkgs.follows = "nixpkgs";
    # Only the flake check of the Home Manager module evaluates against it.
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      bun2nix,
      home-manager,
    }:
    {
      # `services.cerbo.daemon`: see nix/home-manager.nix.
      homeManagerModules.default = import ./nix/home-manager.nix { inherit self; };
    }
    // flake-utils.lib.eachDefaultSystem (
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

        # The Home Manager module, evaluated with a minimal configuration:
        # nothing is added unless enabled, and enabling defines both units.
        checks.home-manager-module =
          let
            lib = pkgs.lib;
            # `withModule = false` leaves the module out altogether.
            hm =
              withModule: extra:
              home-manager.lib.homeManagerConfiguration {
                inherit pkgs;
                modules =
                  lib.optional withModule self.homeManagerModules.default
                  ++ [
                    {
                      home.username = "tester";
                      home.homeDirectory = "/home/tester";
                      home.stateVersion = "24.11";
                    }
                    extra
                  ];
              };
            # No import of the module at all: what a configuration looks like
            # without Cerbo. Importing it with enable=false must change nothing.
            absent = hm false { };
            disabled = hm true { services.cerbo.daemon.enable = false; };
            defaulted = hm true { };
            enabled = hm true {
              services.cerbo.daemon = {
                enable = true;
                vaults.Notes.agent = "approve";
              };
            };
            fakeDaemon = pkgs.runCommand "fake-cerboid" { } ''
              mkdir -p $out/bin
              touch $out/bin/cerboid
            '';
            custom = hm true {
              services.cerbo.daemon = {
                enable = true;
                package = fakeDaemon;
              };
            };
            units = c: c.config.systemd.user;
            # Everything the module could touch, reduced to comparable values.
            footprint = c: {
              packages = map (p: p.outPath) c.config.home.packages;
              services = lib.attrNames (units c).services;
              sockets = lib.attrNames (units c).sockets;
              files = lib.attrNames c.config.home.file;
              xdgConfig = lib.attrNames c.config.xdg.configFile;
            };
            svc = (units enabled).services.cerboid;
            sock = (units enabled).sockets.cerboid;
            asserts = [
              [ "enable=false adds nothing to a configuration without the module" (footprint disabled == footprint absent) ]
              [ "the default adds nothing to a configuration without the module" (footprint defaulted == footprint absent) ]
              [ "the footprint check sees an enabled module" (footprint enabled != footprint absent) ]
              [ "service restarts on failure" (svc.Service.Restart == "on-failure") ]
              [ "service runs cerboid" (lib.all (e: lib.hasSuffix "/bin/cerboid" e) (lib.toList svc.Service.ExecStart)) ]
              [ "a custom package is the one in ExecStart" (lib.toList (units custom).services.cerboid.Service.ExecStart == [ "${fakeDaemon}/bin/cerboid" ]) ]
              [ "a custom package is installed" (builtins.elem fakeDaemon.outPath (footprint custom).packages) ]
              [ "service waits for the socket" (builtins.elem "cerboid.socket" svc.Unit.Requires) ]
              [ "service applies vault levels" (svc.Service.ExecStartPre != [ ]) ]
              [ "socket is owner only" (sock.Socket.SocketMode == "0600" && sock.Socket.DirectoryMode == "0700") ]
              [ "socket path is under the runtime dir" (sock.Socket.ListenStream == "%t/cerbo/cerboid.sock") ]
              [ "enabling installs daemon and CLI" (builtins.length enabled.config.home.packages == builtins.length absent.config.home.packages + 2) ]
            ];
            failed = map (a: builtins.elemAt a 0) (lib.filter (a: !(builtins.elemAt a 1)) asserts);
          in
          assert lib.assertMsg (failed == [ ]) "home-manager module checks failed: ${lib.concatStringsSep ", " failed}";
          pkgs.runCommand "home-manager-module-check" { } ''
            touch $out
          '';

        # The module's generated ExecStartPre script against a fake `cerbo`
        # that exits with the status found in $FAKE_CERBO_STATUS: 0 records the
        # applied level, 2 (vault not registered) warns and lets the service
        # start, anything else fails the start.
        checks.home-manager-exec-start-pre =
          let
            lib = pkgs.lib;
            fakeCerbo = pkgs.writeShellScriptBin "cerbo" ''
              exit "$(cat "$FAKE_CERBO_STATUS")"
            '';
            conf = home-manager.lib.homeManagerConfiguration {
              inherit pkgs;
              modules = [
                self.homeManagerModules.default
                {
                  home.username = "tester";
                  home.homeDirectory = "/home/tester";
                  home.stateVersion = "24.11";
                  services.cerbo.daemon = {
                    enable = true;
                    cliPackage = fakeCerbo;
                    vaults.Notes.agent = "approve";
                  };
                }
              ];
            };
            pre = lib.head (lib.toList conf.config.systemd.user.services.cerboid.Service.ExecStartPre);
          in
          pkgs.runCommand "home-manager-exec-start-pre-check" { } ''
            pre=${pre}
            export FAKE_CERBO_STATUS=$TMPDIR/status
            unset XDG_STATE_HOME

            # Runs the script with a fresh state directory and the given status.
            run() {
              export HOME=$TMPDIR/home-$1
              rm -rf "$HOME"; mkdir -p "$HOME"
              printf '%s' "$1" > "$FAKE_CERBO_STATUS"
              rc=0
              "$pre" 2> "$TMPDIR/err-$1" || rc=$?
              stamps=$(find "$HOME/.local/state/cerbo/declared-agent-levels" -type f 2>/dev/null | wc -l)
            }

            run 0
            [ "$rc" = 0 ] || { echo "exit 0 must succeed, got $rc"; exit 1; }
            [ "$stamps" = 1 ] || { echo "exit 0 must write the stamp"; exit 1; }
            # Applied once: an unchanged level does not call cerbo again, so a
            # failing cerbo no longer matters.
            printf 1 > "$FAKE_CERBO_STATUS"
            "$pre" || { echo "an applied level must be skipped"; exit 1; }

            run 2
            [ "$rc" = 0 ] || { echo "exit 2 must not fail the start, got $rc"; exit 1; }
            [ "$stamps" = 0 ] || { echo "exit 2 must not write the stamp"; exit 1; }
            grep -q "not registered" "$TMPDIR/err-2" || { echo "exit 2 must warn"; exit 1; }

            run 1
            [ "$rc" = 1 ] || { echo "exit 1 must fail the start with 1, got $rc"; exit 1; }
            [ "$stamps" = 0 ] || { echo "exit 1 must not write the stamp"; exit 1; }
            grep -q "could not set" "$TMPDIR/err-1" || { echo "exit 1 must explain"; exit 1; }

            touch $out
          '';

        # The built daemon under a fake systemd: socket passed as descriptor 3
        # with LISTEN_FDS/LISTEN_PID, a client gets the handshake, a second
        # activated instance fails visibly, the socket survives the stop.
        checks.cerboid-socket-activation = pkgs.runCommand "cerboid-socket-activation-check" {
          nativeBuildInputs = [ pkgs.python3 ];
        } ''
          python3 ${./scripts/check-socket-activation.py} ${cerboid}/bin/cerboid ${./core/src/rpc/mod.rs}
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
