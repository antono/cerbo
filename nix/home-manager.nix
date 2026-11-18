# Home Manager module: `services.cerbo.daemon`.
#
# Runs `cerboid`, the Cerbo daemon, as a systemd user service with socket
# activation. Imported by the flake as `homeManagerModules.default`; nothing is
# added to the configuration unless `services.cerbo.daemon.enable` is set.
{ self }:
{ config, lib, pkgs, ... }:
let
  cfg = config.services.cerbo.daemon;
  inherit (pkgs.stdenv.hostPlatform) system;

  levels = [ "read" "auto" "approve" ];

  # The socket path of cerbo-core's DaemonPaths::for_user: %t is
  # $XDG_RUNTIME_DIR in a user unit.
  socketPath = "%t/cerbo/cerboid.sock";

  # Writes the level declared here into the daemon's configuration, but only
  # when the declared value differs from the one applied last time. A level the
  # user changed with `cerbo vault agent` therefore survives restarts and is
  # overwritten only by a switch that changes the option.
  applyAgentLevels =
    let
      cerbo = "${cfg.cliPackage}/bin/cerbo";
      one = name: vault: ''
        apply ${lib.escapeShellArg name} ${vault.agent}
      '';
    in
    pkgs.writeShellScript "cerboid-apply-agent-levels" ''
      set -u
      state="''${XDG_STATE_HOME:-$HOME/.local/state}/cerbo/declared-agent-levels"
      mkdir -p "$state"
      apply() {
        name="$1"; level="$2"
        stamp="$state/$(printf '%s' "$name" | ${pkgs.coreutils}/bin/sha256sum | cut -d' ' -f1)"
        if [ "$(cat "$stamp" 2>/dev/null)" = "$level" ]; then
          return 0
        fi
        if ${cerbo} vault agent "$name" "$level"; then
          printf '%s' "$level" > "$stamp"
        else
          echo "cerboid: could not set the agent level of vault $name to $level" >&2
        fi
      }
      ${lib.concatStrings (lib.mapAttrsToList one cfg.vaults)}
    '';
in
{
  options.services.cerbo.daemon = {
    enable = lib.mkEnableOption "the Cerbo daemon (cerboid), a systemd user service that owns every registered vault";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${system}.cerboid;
      defaultText = lib.literalExpression "cerbo.packages.\${system}.cerboid";
      description = "The package providing `cerboid`.";
    };

    cliPackage = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${system}.cerbo;
      defaultText = lib.literalExpression "cerbo.packages.\${system}.cerbo";
      description = "The package providing the `cerbo` CLI, installed alongside the daemon.";
    };

    vaults = lib.mkOption {
      default = { };
      description = ''
        Per-vault settings, keyed by the vault's name as registered with
        `cerbo vault add`. Applied when the service starts.
      '';
      example = lib.literalExpression ''{ Notes.agent = "approve"; }'';
      type = lib.types.attrsOf (
        lib.types.submodule {
          options.agent = lib.mkOption {
            type = lib.types.enum levels;
            default = "read";
            description = ''
              What agents connected through `cerbo mcp` may do in this vault:
              `read` only reads, `auto` also writes, `approve` writes after you
              approve each change. A level you set with `cerbo vault agent`
              is replaced only when a switch changes this option.
            '';
          };
        }
      );
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package cfg.cliPackage ];

    systemd.user.services.cerboid = {
      Unit = {
        Description = "Cerbo daemon";
        Documentation = [ "man:cerbo(1)" ];
        Requires = [ "cerboid.socket" ];
        After = [ "cerboid.socket" ];
      };
      Service = {
        Type = "simple";
        ExecStartPre = lib.optional (cfg.vaults != { }) "${applyAgentLevels}";
        ExecStart = "${cfg.package}/bin/cerboid";
        Restart = "on-failure";
        RestartSec = 2;
      };
      Install.WantedBy = [ "default.target" ];
    };

    systemd.user.sockets.cerboid = {
      Unit.Description = "Cerbo daemon socket";
      Socket = {
        ListenStream = socketPath;
        DirectoryMode = "0700";
        SocketMode = "0600";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
  };
}
