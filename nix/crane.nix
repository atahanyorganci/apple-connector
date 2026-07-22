{inputs, ...}: {
  perSystem = {
    pkgs,
    craneLib,
    self',
    ...
  }: let
    inherit (pkgs) lib;
    projectRoot = ../.;
    packageRoot = projectRoot + "/packages/apple-connector";
    packageManifest =
      builtins.fromTOML (builtins.readFile (packageRoot + "/Cargo.toml"));
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
    packageSources =
      lib.fileset.fileFilter
      (file: lib.hasInfix "/packages/apple-connector/" file.name)
      projectRoot;
    src = lib.fileset.toSource {
      root = projectRoot;
      fileset = lib.fileset.unions [
        (craneLibNightly.fileset.commonCargoSources projectRoot)
        packageSources
        (lib.fileset.maybeMissing (packageRoot + "/sqlx"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/messages/attributed-body-hello.bin"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/messages/attributed-body-long.bin"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/messages/balloons"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/messages/chat.schema.sql"))
        (lib.fileset.maybeMissing (projectRoot + "/packages/apple-typedstream/fixtures"))
        (lib.fileset.maybeMissing (projectRoot + "/packages/apple-typedstream/tests/snapshots"))
        (lib.fileset.maybeMissing (projectRoot + "/docs/openapi.json"))
      ];
    };
    workspaceArgs = {
      inherit src;
      strictDeps = true;
      buildInputs = [pkgs.libiconv pkgs.sqlite];
      cargoToml = projectRoot + "/Cargo.toml";
      pname = "apple-connector";
      version = packageManifest.package.version;
      SQLX_OFFLINE = "true";
      SQLX_OFFLINE_DIR = "packages/apple-connector/sqlx";
    };
    commonArgs =
      workspaceArgs
      // {
        cargoExtraArgs = "-p apple-connector";
      };
    cargoArtifacts = craneLibNightly.buildDepsOnly commonArgs;
    individualCrateArgs =
      commonArgs
      // {
        inherit cargoArtifacts;
        doCheck = false;
      };

    apple-connector = craneLibNightly.buildPackage individualCrateArgs;
  in {
    checks = {
      apple-connector-audit = craneLib.cargoAudit (workspaceArgs
        // {
          advisory-db = inputs.advisory-db;
        });

      apple-connector-deny = craneLib.cargoDeny workspaceArgs;

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
