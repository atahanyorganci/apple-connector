{inputs, ...}: {
  # apple-eventkit / apple-contacts `compile_error!` on non-macOS targets.
  systems = ["aarch64-darwin"];
  imports = [
    inputs.treefmt-nix.flakeModule
  ];
  perSystem = {system, ...}: let
    pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = [inputs.rust-overlay.overlays.default];
      config = {
        allowUnfree = true;
        allowBroken = true;
      };
    };
    # One toolchain for the package, checks, dev shell, and rustfmt.
    rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
  in {
    _module.args = {
      inherit pkgs rustToolchain;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
    };
  };
}
