{
  description = "Query terminal XTVERSION and print the result";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rust-dev = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "rust-analyzer" "clippy" ];
      };
    in {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "xtver";
        version = "0.1.0";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
        postInstall = ''
          install -Dm644 ${self}/man/man1/xtver.1 $out/share/man/man1/xtver.1
        '';
      };

      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          rust-dev
          pkgs.cargo-watch
          pkgs.cargo-expand
          pkgs.pkg-config
          pkgs.openssl
        ];

        RUST_BACKTRACE = 1;
        RUST_LOG = "debug";
      };
    };
}
