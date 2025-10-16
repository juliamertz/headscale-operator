{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    filter.url = "github:numtide/nix-filter";
    steiger.url = "github:brainhivenl/steiger";
    systems.url = "github:nix-systems/default";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    crane,
    rust-overlay,
    steiger,
    filter,
    systems,
    ...
  }: let
    mkCraneLib = pkgs': (crane.mkLib pkgs').overrideToolchain (p: p.rust-bin.stable.latest.default);
    overlays = [(import rust-overlay) steiger.overlays.ociTools];
  in {
    steigerImages = steiger.lib.eachCrossSystem (import systems) (localSystem: crossSystem: let
      pkgs = import nixpkgs {
        inherit overlays;
        system = localSystem;
      };
      pkgsCross = import nixpkgs {
        inherit overlays crossSystem localSystem;
      };

      craneLib = mkCraneLib pkgsCross;

      buildImage = package:
        pkgs.ociTools.buildImage {
          name = package.pname;
          tag = "latest";
          created = "now";

          copyToRoot = pkgsCross.buildEnv {
            name = "${package.pname}-sysroot";
            paths = [
              package
              pkgs.dockerTools.caCertificates
            ];
            pathsToLink = [
              "/bin"
              "/etc"
            ];
          };

          config.Cmd = ["/bin/${package.pname}"];
          compressor = "none";
        };

      headscale-operator = pkgsCross.callPackage ./package.nix {
        craneLib = craneLib;
        inherit filter;
      };
    in {
      headscale-operator = buildImage headscale-operator;
    });

    devShells = nixpkgs.lib.genAttrs (import systems) (
      system: let
        pkgs = import nixpkgs {inherit system overlays;};
        craneLib = mkCraneLib pkgs;
      in {
        default = craneLib.devShell {
          packages = with pkgs; (let
            toolchain = pkgs.rust-bin.stable.latest.default.override {
              extensions = ["rust-src" "rustfmt"];
            };
          in
            with pkgs;
            with toolchain; [
              rust-analyzer
              clippy
              nix-eval-jobs
              steiger.packages.${system}.default
            ]);

          RUST_LOG = "info,headscale_operator=debug";
        };
      }
    );
  };
}
