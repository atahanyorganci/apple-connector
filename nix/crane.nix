{inputs, ...}: {
  perSystem = {
    pkgs,
    craneLib,
    self',
    ...
  }: let
    inherit (pkgs) lib;
    projectRoot = ../.;
    rustTarget = "aarch64-apple-darwin";
    rustToolchainFor = p:
      p.rust-bin.selectLatestNightlyWith (
        toolchain:
          toolchain.default.override {
            extensions = ["rust-src" "rustfmt"];
            targets = [rustTarget];
          }
      );
    rustToolchain = rustToolchainFor pkgs;
    craneLibNightly = craneLib.overrideToolchain rustToolchainFor;
    src = lib.fileset.toSource {
      root = projectRoot;
      fileset = lib.fileset.unions [
        (craneLibNightly.fileset.commonCargoSources projectRoot)
        (lib.fileset.maybeMissing (projectRoot + "/sqlx"))
        (lib.fileset.maybeMissing (projectRoot + "/fixtures/messages/attributed-body-hello.bin"))
        (lib.fileset.maybeMissing (projectRoot + "/fixtures/messages/attributed-body-long.bin"))
      ];
    };
    commonArgs = {
      inherit src;
      strictDeps = true;
      buildInputs = [pkgs.libiconv pkgs.sqlite];
      SQLX_OFFLINE = "true";
      SQLX_OFFLINE_DIR = "sqlx";
    };
    cargoArtifacts = craneLibNightly.buildDepsOnly commonArgs;
    individualCrateArgs =
      commonArgs
      // {
        inherit cargoArtifacts;
        inherit (craneLibNightly.crateNameFromCargoToml {inherit src;}) version;
        doCheck = false;
      };

    apple-connector = craneLibNightly.buildPackage (
      individualCrateArgs
      // {
        pname = "apple-connector";
        inherit src;
      }
    );
  in {
    checks = {
      apple-connector-audit = craneLib.cargoAudit {
        inherit src;
        advisory-db = inputs.advisory-db;
      };

      apple-connector-deny = craneLib.cargoDeny {
        inherit src;
      };

      apple-connector-clippy = craneLibNightly.cargoClippy (commonArgs
        // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

      apple-connector-test = craneLibNightly.cargoTest (commonArgs
        // {
          inherit cargoArtifacts;
        });
    };
    packages = {
      "apple-connector" = apple-connector;
      default = apple-connector;
    };
    devShells.default = craneLibNightly.devShell {
      checks = self'.checks;
      packages = [rustToolchain pkgs.cargo-watch];
      RUST_SRC_PATH = "${rustToolchain.passthru.availableComponents.rust-src}/lib/rustlib/src/rust/library";
    };
  };
}
