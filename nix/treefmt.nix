{
  perSystem = {rustToolchain, ...}: {
    treefmt = {
      projectRootFile = "flake.nix";
      programs = {
        alejandra.enable = true;
        deadnix.enable = true;
        # Nightly rustfmt: rustfmt.toml uses unstable options.
        rustfmt = {
          enable = true;
          package = rustToolchain;
        };
        shfmt.enable = true;
        sql-formatter = {
          enable = true;
          dialect = "sqlite";
        };
      };
    };
  };
}
