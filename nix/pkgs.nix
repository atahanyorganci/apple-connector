{inputs, ...}: {
  perSystem = {system, ...}: let
    pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = [inputs.rust-overlay.overlays.default];
      config = {
        allowUnfree = true;
        allowBroken = true;
      };
    };
    craneLib = inputs.crane.mkLib pkgs;
  in {
    _module.args = {
      inherit pkgs craneLib;
    };
  };
}
