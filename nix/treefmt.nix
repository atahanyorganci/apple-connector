{inputs, ...}: {
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem = {pkgs, ...}: let
    projectRoot = ../.;
    rustToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
    wrappedRustfmt = pkgs.writeShellScriptBin "rustfmt" ''
      export DYLD_LIBRARY_PATH="${rustToolchain}/lib:$DYLD_LIBRARY_PATH"
      exec ${rustToolchain}/bin/rustfmt "$@"
    '';
  in {
    treefmt = {
      projectRootFile = "flake.nix";
      programs = {
        alejandra.enable = true;
        deadnix.enable = true;
        mdsh.enable = true;
        taplo.enable = true;
      };
      settings.formatter = {
        taplo.options = [
          "--config"
          (builtins.toString (projectRoot + "/taplo.toml"))
        ];
        rustfmt-nightly = {
          command = "${wrappedRustfmt}/bin/rustfmt";
          options = [
            "--edition"
            "2024"
            "--config-path"
            (builtins.toString (projectRoot + "/rustfmt.toml"))
          ];
          includes = ["*.rs"];
        };
      };
    };
  };
}
