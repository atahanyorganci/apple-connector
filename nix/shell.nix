{
  perSystem = {
    pkgs,
    rustToolchain,
    self',
    ...
  }: {
    devShells.default = pkgs.mkShell {
      inputsFrom = [self'.packages.apple-connector];
      packages = with pkgs; [
        rustToolchain
        cargo-deny
        cargo-fuzz
        cargo-watch
        # scripts/sqlx-prepare-all.sh
        sqlx-cli
        # Raycast extension; corepack provides the pnpm pinned in package.json.
        nodejs-slim
        corepack
      ];
    };
  };
}
