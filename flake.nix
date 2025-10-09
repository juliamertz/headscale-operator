{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    filter.url = "github:numtide/nix-filter";
    steiger.url = "github:brainhivenl/steiger/feat/nix-force-host-platform";
  };

  outputs = {
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    steiger,
    filter,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      localSystem: let
        overlays = [(import rust-overlay) steiger.overlays.ociTools];

        pkgs = import nixpkgs {
          inherit overlays;
          system = localSystem;
        };

        crossSystem = let
          steigerTarget = builtins.getEnv "STEIGER_TARGET_SYSTEM";
        in
          if builtins.stringLength steigerTarget == 0
          then pkgs.system
          else steigerTarget;

        pkgsCross = import nixpkgs {
          inherit overlays crossSystem localSystem;
        };

        mkCraneLib = pkgs': (crane.mkLib pkgs').overrideToolchain (p: p.rust-bin.stable.latest.default);
        craneLib = mkCraneLib pkgs;
        craneLibCross = mkCraneLib pkgsCross;

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
          craneLib = craneLibCross;
          inherit filter;
        };
      in {
        packages = {
          default = headscale-operator;
          image = buildImage headscale-operator;
        };

        devShells.default = craneLib.devShell {
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
        };
      }
    );
}
