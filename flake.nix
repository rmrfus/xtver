{
  description = "xtver rust dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "rust-analyzer" "clippy" ];
      };
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          rust
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
