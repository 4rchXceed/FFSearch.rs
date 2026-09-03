{
  description = "Rust Devshell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, fenix, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      toolchain = fenix.packages.${system}.stable.toolchain;
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = [
          toolchain
          pkgs.rust-analyzer
          pkgs.pkg-config
          pkgs.protobuf
          pkgs.openssl
          pkgs.stdenv.cc
          pkgs.rust-analyzer
          pkgs.expat
          pkgs.fontconfig
          pkgs.freetype
          pkgs.rustfmt
          pkgs.freetype.dev
          pkgs.libGL
          pkgs.pkg-config
          pkgs.libx11
          pkgs.libxcursor
          pkgs.libxi
          pkgs.libxrandr
          pkgs.wayland
          pkgs.libxkbcommon
          pkgs.rustc
          pkgs.cargo
          pkgs.protobuf
          pkgs.openssl

        ];
        buildInputs = with pkgs; [
          stdenv.cc
          rust-analyzer
          expat
          fontconfig
          freetype
          rustfmt
          freetype.dev
          libGL
          pkg-config
          libx11
          libxcursor
          libxi
          libxrandr
          wayland
          libxkbcommon
          rustc
          cargo
          protobuf
          openssl
          bzip2
        ];

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
          pkgs.vulkan-loader
          pkgs.libGL
          pkgs.libx11
          pkgs.wayland
          pkgs.libxkbcommon
        ];
      };
    };
}
