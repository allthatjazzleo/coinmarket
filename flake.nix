{
  description = "coinmarket tui";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
  }: 
    flake-utils.lib.eachDefaultSystem (system:
      let
        makePkgs = config:
          import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
            crossSystem = {
              inherit config;
              rustc = {inherit config;};
              isStatic = true;
            };
          };
      in {
        packages = {
          default = (makePkgs "${system}").callPackage ./. {};
          x86_64-linux-musl = (makePkgs "x86_64-unknown-linux-musl").callPackage ./. {};
          aarch64-linux-musl = (makePkgs "aarch64-unknown-linux-musl").callPackage ./. {};
        };
      }
    );
}