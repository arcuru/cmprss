{
  description = "cmprss: a compression client for the CLI";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, fenix, flake-utils, naersk, nixpkgs, pre-commit-hooks, }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        mkToolchain = fenix.packages.${system}.combine;

        # Use stable rust releases by default
        toolchain = fenix.packages.${system}.stable;

        buildToolchain = mkToolchain (with toolchain; [ cargo rustc ]);

        devToolchain = mkToolchain (with toolchain; [
          cargo
          clippy
          rust-src
          rustc
          rustfmt
          rust-analyzer
        ]);
      in {
        # Use naersk to create a default package from the src files
        packages.default = (pkgs.callPackage naersk {
          cargo = buildToolchain;
          rustc = buildToolchain;
        }).buildPackage { src = ./.; };

        devShells.default = pkgs.mkShell {
          inherit (self.checks.${system}.pre-commit-check) shellHook;

          # Rust Analyzer needs to be able to find the path to default crate
          # sources, and it can read this environment variable to do so. The
          # `rust-src` component is required in order for this to work.
          RUST_SRC_PATH = "${devToolchain}/lib/rustlib/src/rust/library";

          nativeBuildInputs = [ devToolchain ];
        };

        checks = {
          packagesDefault = self.packages.${system}.default;
          devShellsDefault = self.devShells.${system}.default;

          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              # Format rust files using rustfmt
              rustfmt.enable = true;

              # Ensure no clippy warnings exist
              # FIXME: Re-enable. Fails due to mismatched compiler versions because of the devToolchain
              #clippy.enable = true;

              # Runs `cargo check` to look for errors
              cargo-check.enable = true;

              # Format nix files using nixfmt
              # This hook will format the files for you, so on a failure all
              # that's needed to fix is to re-run the commit.
              nixfmt.enable = true;

              # Format Markdown/css/etc
              # The settings are contained in .prettierrc.yaml
              prettier.enable = true;
            };
          };
        };
      });
}
