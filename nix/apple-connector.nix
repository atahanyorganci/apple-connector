{
  perSystem = {
    lib,
    pkgs,
    rustPlatform,
    ...
  }: let
    manifest = lib.importTOML ../packages/apple-connector/Cargo.toml;
    apple-connector = rustPlatform.buildRustPackage {
      pname = "apple-connector";
      inherit (manifest.package) version;

      src = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions [
          ../Cargo.toml
          ../Cargo.lock
          # SQLX_OFFLINE / SQLX_OFFLINE_DIR for the compile-time query macros.
          ../.cargo/config.toml
          # Snapshot compared by the OpenAPI test.
          ../docs/openapi.json
          # Error catalog compared row for row with `ErrorCode`.
          ../docs/errors.md
          (lib.fileset.difference ../packages ../packages/raycast-extension)
        ];
      };

      cargoLock.lockFile = ../Cargo.lock;
      cargoBuildFlags = ["--package" "apple-connector"];

      # libsqlite3-sys is built with `sqlite-unbundled`.
      nativeBuildInputs = [pkgs.pkg-config];
      buildInputs = [pkgs.sqlite];

      # Workspace tests run as `checks.workspace-test`.
      doCheck = false;

      meta = {
        description = "HTTP API over Apple Messages, Reminders, Notes, Calendar, and Contacts";
        license = lib.licenses.mit;
        mainProgram = "apple-connector";
        platforms = lib.platforms.darwin;
      };
    };
  in {
    packages = {
      inherit apple-connector;
      default = apple-connector;
    };
  };
}
