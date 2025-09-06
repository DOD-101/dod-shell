self:
{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.dod-shell;

  tomlFormat = pkgs.formats.toml { };

  filter-packages =
    release:
    lib.attrsets.mapAttrsToList (n: v: v) (
      lib.attrsets.filterAttrs (
        n: v: (lib.strings.hasSuffix "-release" n) == release
      ) self.packages.${pkgs.stdenv.hostPlatform.system}
    );

  getComponent =
    pname:
    let
      component = lib.lists.findFirst (p: p.pname == pname) null cfg.components;
    in
    if builtins.isNull component then
      throw "component ${pname} not found in dod-shell.components"
    else
      component;

in
{

  options.dod-shell = {
    enable = lib.mkEnableOption "dod-shell";

    components = lib.mkOption {
      type = with lib.types; listOf package;
      default = filter-packages true;
      description = "Components of the shell to install";
    };

    scss = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = ''
        SCSS written to 
        {file}`$XDG_CONFIG_HOME/dod-shell/style.scss`.
      '';
    };

    settings = lib.mkOption {
      type = tomlFormat.type;
      default = { };
      description = ''
        Configuration written to 
        {file}`$XDG_CONFIG_HOME/dod-shell/config.toml`.
      '';
    };

    systemd-services = lib.mkOption {
      type = with lib.types; listOf str;
      default = map (p: p.pname) cfg.components;
      description = ''
        Systemd services to create / enable

        By default systemd services are created for all components 
        that are enabled via `dod-shell.components` if there are 
        units associated with them.

        This will also create a `dod-shell.target` systemd target
        that can be used to control all services simultaneously.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = cfg.components;
    xdg.configFile = {
      "dod-shell/style.scss".text = cfg.scss;
      "dod-shell/config.toml" = lib.mkIf (cfg.settings != { }) {
        source = tomlFormat.generate "config.toml" cfg.settings;
      };
    };

    systemd.user = {
      targets.dod-shell = {
        Unit = {
          Description = "dod-shell user services";
          # Documentation  TODO: add documentation link
          After = [ "hyprland-session.target" ];
        };
        Install = {
          WantedBy = [ "hyprland-session.target" ];
        };
      };
      services =
        lib.mkIf (cfg.systemd-services != [ ]) { }
        // lib.mkIf (lib.lists.any (a: a == "dod-shell-bar") cfg.systemd-services) {
          dod-shell-bar = {
            Unit = {
              Description = "dod-shell bar component service";
            };
            Service = {
              ExecStart = "${getComponent "dod-shell-bar"}/bin/dod-shell-bar";
              Type = "exec";
              Restart = "on-failure";
              RestartSec = 3;
              Requires = [ "dod-shell-deamon.service" ];
              After = [ "dod-shell-deamon.service" ];
            };
            Install = {
              WantedBy = [ "dod-shell.target" ];
            };
          };
          dod-shell-deamon = {
            Unit = {
              Description = "dod-shell deamon service";
            };
            Service = {
              ExecStart = "${getComponent "dod-shell-deamon"}/bin/dod-shell-deamon";
              Type = "dbus";
              BusName = "dod.shell.Deamon";
              Restart = "on-failure";
              RestartSec = 3;
            };
            Install = {
              WantedBy = [ "dod-shell.target" ];
            };
          };
        };
    };
  };
}
