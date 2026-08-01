{
  description = "Flake for dod-shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        inherit (pkgs) lib;

        craneLib = (crane.mkLib pkgs).overrideScope (
          final: prev: {
            mkCargoDerivation =
              args:
              prev.mkCargoDerivation (
                {
                  CARGO_PROFILE = "dev";
                }
                // args
              );
          }
        );

        src = lib.fileset.toSource {
          root = ./.;

          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources ./.)
            ./icons
          ];
        };

        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
            openssl

            wrapGAppsHook4
          ];

          buildInputs = with pkgs; [
            gtk4
            gtk4-layer-shell
            libpulseaudio
            libadwaita
            libxkbcommon
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
        };

        fileSetForCrate =
          crate:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./icons
              (craneLib.fileset.commonCargoSources ./crates/common)
              (craneLib.fileset.commonCargoSources ./crates/daemon)
              (craneLib.fileset.commonCargoSources crate)
            ];
          };

        make-release =
          drv:
          drv.overrideAttrs (old: {
            CARGO_PROFILE = "release";
          });

        filter-packages =
          release:
          lib.attrsets.mapAttrsToList (n: v: v) (
            lib.attrsets.filterAttrs (
              n: v: (lib.strings.hasSuffix "-release" n) == release
            ) self.packages.${system}
          );

        launcher = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "dod-shell-launcher";
            cargoExtraArgs = "-p launcher";
            src = fileSetForCrate ./crates/launcher;
          }
        );
        launcher-release = make-release launcher;

        bar = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "dod-shell-bar";
            cargoExtraArgs = "-p bar";
            src = fileSetForCrate ./crates/bar;
          }
        );
        bar-release = make-release bar;

        osk = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "dod-shell-osk";
            cargoExtraArgs = "-p osk";
            src = fileSetForCrate ./crates/osk;
          }
        );
        osk-release = make-release osk;

        daemon = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "dod-shell-daemon";
            cargoExtraArgs = "-p daemon";
            src = fileSetForCrate ./crates/daemon;
          }
        );
        daemon-release = make-release daemon;

        cli = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "dod-shell-cli";
            cargoExtraArgs = "-p cli";
            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.toml
                (craneLib.fileset.commonCargoSources ./crates/common)
                (craneLib.fileset.commonCargoSources ./crates/cli)
              ];
            };
          }
        );
        cli-release = make-release cli;

      in
      {
        checks = lib.attrsets.genAttrs' (filter-packages false) (
          p: lib.nameValuePair (p.name + "-build") p
        );

        packages = {
          inherit
            launcher
            launcher-release
            bar
            bar-release
            osk
            osk-release
            daemon
            daemon-release
            cli
            cli-release
            ;

          default = launcher;
        };

        devShells = {
          default = craneLib.devShell {
            packages =
              with pkgs;
              [
                wev
                watchexec
                prek
                cargo-deny
                cargo-audit
                typos-lsp
                (writeShellScriptBin "dod-watch" (builtins.readFile ./dod-watch.sh))
              ]
              ++ commonArgs.buildInputs
              ++ commonArgs.nativeBuildInputs;

            shellHook = ''
              prek install
            '';
          };
          full = craneLib.devShell {
            packages = filter-packages false;
          };

          full-release = craneLib.devShell {
            packages = filter-packages true;
          };
        };
      }
    )
    // {
      homeManagerModules = rec {
        default = dod-shell;
        dod-shell = import ./nix/hm-module.nix self;
      };
    };
}
