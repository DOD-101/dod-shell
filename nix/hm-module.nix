self:
{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.dod-shell;

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
  };

  config = lib.mkIf cfg.enable {
    home.packages = cfg.components;
    xdg.configFile."dod-shell/style.scss".text = cfg.scss;
  };
}
