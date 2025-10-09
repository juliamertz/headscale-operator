{
  craneLib,
  filter,
  #
  lib,
  pkg-config,
  stdenv,
  openssl,
  libiconv,
}:
craneLib.buildPackage rec {
  pname = "headscale-operator";
  version = "v0.1.0";

  src = filter {
    root = ./.;
    include = [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };

  cargoVendorDir = craneLib.vendorCargoDeps {inherit src;};

  strictDeps = true;

  nativeBuildInputs =
    [pkg-config]
    ++ lib.optionals stdenv.buildPlatform.isDarwin [libiconv];

  buildInputs = [openssl];
}
