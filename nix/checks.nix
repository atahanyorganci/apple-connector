{
  perSystem = {
    lib,
    pkgs,
    rustPlatform,
    self',
    ...
  }: let
    inherit (self'.packages) apple-connector;
    workspaceSources = lib.fileset.difference ../packages ../packages/raycast-extension;

    # Reuse the package's source, vendored deps, and inputs; replace the build.
    cargoCheck = name: {
      command,
      nativeBuildInputs ? [],
    }:
      apple-connector.overrideAttrs (old: {
        pname = "apple-connector-${name}";
        nativeBuildInputs = old.nativeBuildInputs ++ nativeBuildInputs;
        buildPhase = ''
          runHook preBuild
          ${command}
          runHook postBuild
        '';
        installPhase = "touch $out";
        dontFixup = true;
      });

    scriptCheck = name: script:
      pkgs.runCommand "apple-connector-${name}" {
        src = lib.fileset.toSource {
          root = ../.;
          fileset = lib.fileset.unions [
            ../scripts
            ../packages/apple-connector/src
          ];
        };
        nativeBuildInputs = [pkgs.ripgrep];
      } ''
        bash $src/scripts/${script}
        touch $out
      '';
  in {
    checks = {
      workspace-clippy = cargoCheck "clippy" {
        command = "cargo clippy --frozen --workspace --all-targets -- --deny warnings";
      };

      workspace-test = cargoCheck "test" {
        command = "cargo test --frozen --workspace --all-targets";
      };

      # Advisories need a network fetch; run `cargo deny check` locally for those.
      workspace-deny = cargoCheck "deny" {
        command = "cargo deny --frozen --config ${../deny.toml} check bans licenses sources";
        nativeBuildInputs = [pkgs.cargo-deny];
      };

      workspace-runtime-sql = scriptCheck "runtime-sql" "check-runtime-sql.sh";

      workspace-api-error-leakage = scriptCheck "api-error-leakage" "check-api-error-leakage.sh";

      # fuzz/ is its own workspace with its own lockfile.
      workspace-fuzz-smoke = rustPlatform.buildRustPackage {
        pname = "apple-connector-fuzz-smoke";
        version = "0";
        src = lib.fileset.toSource {
          root = ../.;
          fileset = lib.fileset.unions [
            ../Cargo.toml
            ../fuzz
            ../scripts/fuzz-smoke.sh
            workspaceSources
          ];
        };
        cargoRoot = "fuzz";
        cargoLock.lockFile = ../fuzz/Cargo.lock;
        nativeBuildInputs = [pkgs.cargo-fuzz];
        buildPhase = ''
          runHook preBuild
          FUZZ_MAX_TOTAL_TIME=10 bash scripts/fuzz-smoke.sh
          runHook postBuild
        '';
        installPhase = "touch $out";
        doCheck = false;
        dontFixup = true;
      };
    };
  };
}
