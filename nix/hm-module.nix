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
      description = "SCSS for style.scss";
    };

    settings = lib.mkOption {
      type = tomlFormat.type;
      default = { };
      description = ''
        Configuration written to 
        {file}`$XDG_CONFIG_HOME/dod-shell/config.toml`.
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
  };
}
