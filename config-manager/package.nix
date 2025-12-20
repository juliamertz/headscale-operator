{
  craneLib,
  filter,
  #
  lib,
  pkg-config,
  stdenv,
  libiconv,
}:
craneLib.buildPackage rec {
  pname = "config-manager";
  version = "v0.1.0";

  src = filter {
    root = ../.;
    include = [
      ../Cargo.toml
      ../Cargo.lock
      ../headscale-operator/Cargo.toml
      ./Cargo.toml
      ./src
    ];
  };

  cargoExtraArgs = "-p config-manager";
  cargoVendorDir = craneLib.vendorCargoDeps {inherit src;};

  strictDeps = true;

  nativeBuildInputs =
    [pkg-config]
    ++ lib.optionals stdenv.buildPlatform.isDarwin [libiconv];
}
