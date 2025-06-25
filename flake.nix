{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
  };

  outputs = { self, nixpkgs, utils, fenix }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        toolchain = with fenix.packages.${system};
          combine (with complete; [
            rustc
            cargo
            rust-src
            clippy
            rustfmt
            rust-analyzer
          ]);
        armToolchain = fenix.packages.${system}.targets.thumbv7m-none-eabi.latest.rust-std;
      in
      {
        devShell = with pkgs; mkShell rec {
          buildInputs = [
            toolchain
            armToolchain
            ldproxy
            openocd
            udev
            pkg-config
            gdb
            probe-rs-tools
            gcc-arm-embedded
          ];

          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
          #LIBCLANG_PATH = "${pkgs.llvmPackages_19.libclang.lib}/lib";
          RUST_BACKTRACE = 1;
          shellHook = ''
          export CC=arm-none-eabi-gcc
          echo "Using CC=$CC"
          '';
        };
      }
    );
}

