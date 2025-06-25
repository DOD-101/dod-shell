{
  description = "Flake for dod-shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      advisory-db,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = (crane.mkLib pkgs).overrideScope (
          final: prev: {
            # We override the behavior of `mkCargoDerivation` by adding a wrapper which
            # will set a default value of `CARGO_PROFILE` when not set by the caller.
            # This change will automatically be propagated to any other functions built
            # on top of it (like `buildPackage`, `cargoBuild`, etc.)
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

        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
            openssl
            wrapGAppsHook
          ];

          buildInputs =
            with pkgs;
            [
              gtk4
              gtk4-layer-shell
              pkg-config
              openssl
              wrapGAppsHook
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.libiconv
            ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
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
              # (craneLib.fileset.commonCargoSources ./crates/my-common)
              # (craneLib.fileset.commonCargoSources ./crates/my-workspace-hack)
              (craneLib.fileset.commonCargoSources crate)
            ];
          };

        make-release =
          drv:
          drv.overrideAttrs (old: {
            CARGO_PROFILE = "release";
          });

        launcher = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "dod-shell-launcher";
            cargoExtraArgs = "-p launcher";
            src = fileSetForCrate ./crates/launcher;
          }
        );
        launcher-release = make-release launcher;

        cli = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "dod-shell-cli";
            cargoExtraArgs = "-p cli";
            # Custom src since the cli depends on all other components
            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.toml
                (craneLib.fileset.commonCargoSources ./crates)
                # (craneLib.fileset.commonCargoSources ./crates/cli)
              ];
            };
          }
        );
        cli-release = make-release cli;

      in
      {
        checks = {
          inherit launcher cli;

          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          docs = craneLib.cargoDoc (
            commonArgs
            // {
              inherit cargoArtifacts;
            }
          );

          fmt = craneLib.cargoFmt {
            inherit src;
          };

          toml-fmt = craneLib.taploFmt {
            src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
          };

          audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          deny = craneLib.cargoDeny {
            inherit src;
          };
        };

        packages = {
          inherit
            launcher
            launcher-release
            cli
            cli-release
            ;

          default = launcher;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = launcher;
        };

        devShells = {
          default = craneLib.devShell {
            # Inherit inputs from checks.
            checks = self.checks.${system};

            # Additional dev-shell environment variables can be set directly
            # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

            packages = [ ];
          };

          full = craneLib.devShell {
            checks = self.checks.${system};
            packages = lib.attrsets.mapAttrsToList (n: v: v) (
              lib.attrsets.filterAttrs (n: v: !(lib.strings.hasSuffix "-release" n)) self.packages.${system}
            );
          };

          full-release = craneLib.devShell {
            checks = self.checks.${system};
            packages = lib.attrsets.mapAttrsToList (n: v: v) (
              lib.attrsets.filterAttrs (n: v: (lib.strings.hasSuffix "-release" n)) self.packages.${system}
            );
          };
        };
      }
    );
}
