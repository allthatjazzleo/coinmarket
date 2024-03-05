{rustPlatform, openssl_3_2, pkg-config}:
rustPlatform.buildRustPackage {
  name = "coinmarket";
  src = ./.;
  nativeBuildInputs = [
    pkg-config
  ];
  buildInputs = [openssl_3_2];
  cargoLock.lockFile = ./Cargo.lock;
  RUSTFLAGS="-C link-arg=-s";
}