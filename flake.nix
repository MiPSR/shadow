{
  description = "shadow private communication platform";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { system = system; config.allowUnfree = true; };
        rustToolchain = pkgs.rustc; # placeholder
      in
      {
        devShells = {
          default = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.pkg-config pkgs.openssl ];
          };
          server = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.pkg-config pkgs.openssl pkgs.sqlite ];
          };
          client = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.pkg-config pkgs.openssl pkgs.alsa-lib pkgs.cmake pkgs.opus pkgs.xorg.libxcb pkgs.lld pkgs.ffmpeg pkgs.rustup pkgs.pkgsCross.mingwW64.stdenv.cc pkgs.android-tools ];
          };
        };

        packages = {
          server = pkgs.rustPlatform.buildRustPackage {
            pname = "server";
            version = "0.1.0";
            src = ./.;
            cargoLock = { lockFile = ./Cargo.lock; };
            buildAndTestSubdir = "server";
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl pkgs.sqlite ];
          };
          client = pkgs.rustPlatform.buildRustPackage {
            pname = "client";
            version = "0.1.0";
            src = ./.;
            cargoLock = { lockFile = ./Cargo.lock; };
            buildAndTestSubdir = "client";
            nativeBuildInputs = [ pkgs.pkg-config pkgs.cmake ];
            buildInputs = [ pkgs.openssl pkgs.alsa-lib pkgs.opus pkgs.xorg.libxcb ];
          };
        };
      });
}
