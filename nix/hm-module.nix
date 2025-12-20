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
  jsonFormat = pkgs.formats.json { };

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

    scss = {
      text = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = ''
          SCSS written to 
          {file}`$XDG_CONFIG_HOME/dod-shell/style.scss`.

          This is passed to `xdg.configFile."style.scss".text`.
        '';
      };
      source = lib.mkOption {
        type = with lib.types; nullOr path;
        default = null;
        description = ''
          SCSS file linked to 
          {file}`$XDG_CONFIG_HOME/dod-shell/style.scss`.

          Passed to `xdg.configFile."style.scss".source`.
        '';
      };
    };

    config = {
      config = lib.mkOption {
        type = tomlFormat.type;
        default = { };
        description = ''
          Configuration written to 
          {file}`$XDG_CONFIG_HOME/dod-shell/config.toml`.

          Passed to `xdg.configFile."config.toml".source` after being
          generated.
        '';
      };
      source = lib.mkOption {
        type = with lib.types; nullOr path;
        default = null;
        description = ''
          TOML file linked to 
          {file}`$XDG_CONFIG_HOME/dod-shell/config.toml`.

          Passed to `xdg.configFile."config.toml".source`.
        '';
      };
    };

    layouts = {
      config = lib.mkOption {
        type = jsonFormat.type;
        default = { };
        description = ''
          Configuration written to 
          {file}`$XDG_CONFIG_HOME/dod-shell/layouts.json`.

          Passed to `xdg.configFile."layouts.json".source` after being
          generated.
        '';
      };
      source = lib.mkOption {
        type = lib.types.path;
        default = ../test/layouts.json;
        description = ''
          JSON file linked to 
          {file}`$XDG_CONFIG_HOME/dod-shell/layouts.json`.

          Passed to `xdg.configFile."layouts.json".source`.
        '';
      };
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
      "dod-shell/style.scss" = {
        inherit (cfg.scss) text;

        source = lib.mkIf (cfg.scss.source != null) cfg.scss.source;
      };
      "dod-shell/config.toml" = {
        source =
          if cfg.config.config != { } then
            tomlFormat.generate "config.toml" cfg.config.config
          else
            cfg.config.source;
      };
      "dod-shell/layouts.json" = {
        source =
          if cfg.layouts.config != { } then
            jsonFormat.generate "layouts.json" cfg.layouts.config
          else
            cfg.layouts.source;
      };
    };

    systemd.user = {
      targets.dod-shell = {
        Unit = {
          Description = "dod-shell user services";
          Documentation = "https://github.com/DOD-101/dod-shell/blob/master/README.md";
          After = [ "hyprland-session.target" ];
        };
        Install = {
          WantedBy = [ "hyprland-session.target" ];
        };
      };
      services =
        let
          if_in_services = name: lib.mkIf (lib.lists.any (a: a == name) cfg.systemd-services);
        in
        {
          dod-shell-bar = if_in_services "dod-shell-bar" {
            Unit = {
              Description = "dod-shell bar component service";
            };
            Service = {
              ExecStart = "${getComponent "dod-shell-bar"}/bin/dod-shell-bar";
              Type = "exec";
              Restart = "on-failure";
              RestartSec = 3;
              Requires = [ "dod-shell-daemon.service" ];
              After = [ "dod-shell-daemon.service" ];
            };
            Install = {
              WantedBy = [ "dod-shell.target" ];
            };
          };

          dod-shell-osk = if_in_services "dod-shell-osk" {
            Unit = {
              Description = "dod-shell osk component service";
            };
            Service = {
              ExecStart = "${getComponent "dod-shell-osk"}/bin/dod-shell-osk";
              Type = "exec";
              Restart = "on-failure";
              RestartSec = 3;
              Requires = [ "dod-shell-daemon.service" ];
              After = [ "dod-shell-daemon.service" ];
            };
            Install = {
              WantedBy = [ "dod-shell.target" ];
            };
          };

          dod-shell-daemon = if_in_services "dod-shell-daemon" {
            Unit = {
              Description = "dod-shell daemon service";
            };
            Service = {
              ExecStart = "${getComponent "dod-shell-daemon"}/bin/dod-shell-daemon";
              Type = "dbus";
              BusName = "dod.shell.Daemon";
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
