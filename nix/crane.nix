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
      (file: lib.hasInfix "/packages/" file.name)
      projectRoot;
    src = lib.fileset.toSource {
      root = projectRoot;
      fileset = lib.fileset.unions [
        (craneLibNightly.fileset.commonCargoSources projectRoot)
        packageSources
        (lib.fileset.maybeMissing (packageRoot + "/build.rs"))
        (lib.fileset.maybeMissing (packageRoot + "/sqlx"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/messages/attributed-body-hello.bin"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/messages/attributed-body-long.bin"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/messages/balloons"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/messages/chat.schema.sql"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/reminders/reminders.schema.sql"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/reminders/seed.sql"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/notes/notes.schema.sql"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/notes/seed.sql"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/notes/bodies/checklist.bin"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/notes/bodies/plain-text.bin"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/notes/bodies/acnp"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/calendar/calendar.schema.sql"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/calendar/seed.sql"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/contacts/contacts.schema.sql"))
        (lib.fileset.maybeMissing (packageRoot + "/fixtures/contacts/seed.sql"))
        (lib.fileset.maybeMissing (projectRoot + "/packages/apple-typedstream/fixtures"))
        (lib.fileset.maybeMissing (projectRoot + "/packages/apple-typedstream/tests/snapshots"))
        (lib.fileset.maybeMissing (projectRoot + "/docs/openapi.json"))
      ];
    };
    workspaceArgs = {
      inherit src;
      strictDeps = true;
      buildInputs =
        [pkgs.sqlite pkgs.clang]
        ++ lib.optionals pkgs.stdenv.isDarwin [pkgs.libiconv];
      LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
      cargoToml = projectRoot + "/Cargo.toml";
      pname = "apple-connector";
      version = packageManifest.package.version;
      SQLX_OFFLINE = "true";
      SQLX_OFFLINE_DIR = "packages/apple-connector/sqlx";
    };
    workspaceCheckArgs =
      workspaceArgs
      // {
        cargoExtraArgs = "--workspace";
      };
    cargoArtifacts = craneLibNightly.buildDepsOnly workspaceCheckArgs;
    individualCrateArgs =
      workspaceCheckArgs
      // {
        inherit cargoArtifacts;
        doCheck = false;
        cargoExtraArgs = "-p apple-connector";
      };

    apple-connector = craneLibNightly.buildPackage individualCrateArgs;
  in {
    checks = {
      workspace-audit = craneLib.cargoAudit (workspaceArgs
        // {
          advisory-db = inputs.advisory-db;
        });

      workspace-deny = craneLib.cargoDeny workspaceArgs;

      workspace-clippy = craneLibNightly.cargoClippy (workspaceCheckArgs
        // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

      workspace-test = craneLibNightly.cargoTest (workspaceCheckArgs
        // {
          inherit cargoArtifacts;
          cargoTestExtraArgs = "--all-targets";
        });

      workspace-runtime-sql =
        pkgs.runCommand "apple-connector-runtime-sql-check" {
          inherit src;
          nativeBuildInputs = [pkgs.ripgrep pkgs.bash];
        } ''
          cd $src
          mapfile -t matches < <(rg 'sqlx::query(_as)?\(' packages/apple-connector/src -n | rg -v '!' || true)
          for entry in "''${matches[@]}"; do
            file="''${entry%%:*}"
            case "$file" in
              *fixtures.rs|*attachments.rs) continue ;;
              *) echo "runtime SQL API not allowed: $entry" >&2; exit 1 ;;
            esac
          done
          touch $out
        '';
    };
    packages = {
      "apple-connector" = apple-connector;
      default = apple-connector;
    };
    devShells.default = craneLibNightly.devShell {
      checks = self'.checks;
      packages = [rustToolchain pkgs.cargo-watch];
      RUST_SRC_PATH = "${rustToolchain.passthru.availableComponents.rust-src}/lib/rustlib/src/rust/library";
      SQLX_OFFLINE = "true";
      SQLX_OFFLINE_DIR = "packages/apple-connector/sqlx";
      shellHook = ''
        export DYLD_LIBRARY_PATH="${rustToolchain}/lib''${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
      '';
    };
  };
}
