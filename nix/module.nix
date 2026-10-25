{ config, lib, pkgs, ... }:
let
  cfg = config.services.shadow-server;
in {
  options.services.shadow-server = {
    enable = lib.mkEnableOption "shadow server";
    package = lib.mkOption { type = lib.types.package; default = pkgs.shadow-server or (throw "package not provided"); };
    bind = lib.mkOption { type = lib.types.str; default = "0.0.0.0:8080"; };
    databaseUrl = lib.mkOption { type = lib.types.str; default = "sqlite:/var/lib/shadow/shadow.db?mode=rwc"; };
  };
  config = lib.mkIf cfg.enable {
    systemd.services.shadow-server = {
      description = "Shadow server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/server";
        Restart = "on-failure";
        StateDirectory = "shadow";
        Environment = [
          "SHADOW_BIND=${cfg.bind}"
        ];
      };
    };
  };
}
