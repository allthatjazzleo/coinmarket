{
  description = "coinmarket tui";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }: 
    {
      overlay = final: prev:
      let
        buildInputs = [] ++ final.lib.optionals final.stdenv.isDarwin (with final.darwin.apple_sdk.frameworks; [
          SystemConfiguration
        ]);
        RUSTFLAGS = "-C link-arg=-s";
      in
      {
        coinmarket = {}: final.rustPlatform.buildRustPackage ({
          inherit buildInputs RUSTFLAGS;
          pname = "coinmarket";
          version = "0.1.0";
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
        }) // { meta.description = "CoinMarket TUI based on Ratatui"; };
      };
    } //
      flake-utils.lib.eachDefaultSystem (system:
        let
          makePkgs = config:
            let
              shouldUseCross = builtins.elem config [
                "aarch64-unknown-linux-musl"
                "x86_64-unknown-linux-musl"
              ];
              crossSystemOptions = if shouldUseCross then {
                crossSystem = {
                  inherit config;
                  rustc = {inherit config;};
                  isStatic = true;
                };
              } else {};
              pkgs = import nixpkgs (crossSystemOptions // {
                inherit system;
                overlays = [ rust-overlay.overlays.default self.overlay ];
              });
            in
              pkgs.callPackage pkgs.coinmarket {};
        in {
          packages = {
            default = makePkgs "${system}";
            x86_64-linux-musl = makePkgs "x86_64-unknown-linux-musl";
            aarch64-linux-musl = makePkgs "aarch64-unknown-linux-musl";
          };
        }
      );
}