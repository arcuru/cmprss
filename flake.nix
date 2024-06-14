{
  description = "A compression multi-tool for the command line.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      # Needed because rust-overlay, normally used by crane, doesn't have llvm-tools for coverage
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    advisory-db = {
      # Rust dependency security advisories
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    # Flake helper for better organization with modules.
    flake-parts.url = "github:hercules-ci/flake-parts";

    # For creating a universal `nix fmt`
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    flake-parts,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["aarch64-linux" "x86_64-linux" "aarch64-darwin" "x86_64-darwin"];

      imports = [
        flake-parts.flakeModules.easyOverlay
        inputs.treefmt-nix.flakeModule
      ];

      perSystem = {
        config,
        system,
        pkgs,
        ...
      }: let
        # Use the stable rust tools from fenix
        fenixStable = inputs.fenix.packages.${system}.stable;
        rustSrc = fenixStable.rust-src;
        toolChain = fenixStable.completeToolchain;

        # Use the toolchain with the crane helper functions
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolChain;

        # Clean the src to only have the Rust-relevant files
        src = craneLib.cleanCargoSource (craneLib.path ./.);

        # Common arguments for mkCargoDerivation, a helper for the crane functions
        # Arguments can be included here even if they aren't used, but we only
        # place them here if they would otherwise show up in multiple places
        commonArgs = {
          inherit src cargoArtifacts;
          meta.mainProgram = "cmprss";
        };

        # Build only the cargo dependencies so we can cache them all when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the cargoArtifacts
        cmprss = craneLib.buildPackage commonArgs;
      in {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit cmprss;

          # Run clippy (and deny all warnings) on the crate source
          clippy = craneLib.cargoClippy (commonArgs
            // {
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

          # Check docs build successfully
          doc = craneLib.cargoDoc commonArgs;

          # Check formatting
          fmt = craneLib.cargoFmt commonArgs;

          # Run tests with cargo-nextest
          # Note: This provides limited value, as tests are already run in the build
          nextest = craneLib.cargoNextest commonArgs;
        };

        treefmt = {
          projectRootFile = ./flake.nix;
          programs = {
            alejandra.enable = true;
            prettier.enable = true;
            rustfmt = {
              enable = true;
              package = toolChain;
            };
          };
        };

        packages = {
          default = cmprss;
          cmprss = cmprss;

          # Check code coverage with tarpaulin
          coverage = craneLib.cargoTarpaulin (commonArgs
            // {
              # Use lcov output as thats far more widely supported
              cargoTarpaulinExtraArgs = "--skip-clean --include-tests --output-dir $out --out lcov";
            });

          # Audit dependencies
          audit = craneLib.cargoAudit (commonArgs
            // {
              inherit (inputs) advisory-db;
            });
        };

        apps = rec {
          default = cmprss;
          cmprss.program = self.packages.${system}.cmprss;
        };

        overlayAttrs = {
          inherit (config.packages) cmprss;
        };

        devShells.default = pkgs.mkShell {
          name = "cmprss";
          shellHook = ''
            echo ---------------------
            task --list
            echo ---------------------
          '';

          # Include the packages from the defined checks and packages
          inputsFrom =
            (builtins.attrValues self.checks.${system})
            ++ (builtins.attrValues self.packages.${system});

          # Extra inputs can be added here
          nativeBuildInputs = with pkgs; [
            act # For running Github Actions locally
            alejandra
            deadnix
            git-cliff
            go-task
            gum # Pretty printing in scripts
            nodePackages.prettier
            statix

            # Code coverage
            cargo-tarpaulin

            # For running tests
            diffutils
            # Official tools
            bzip2
            gnutar
            gzip
            xz
          ];

          # Many tools read this to find the sources for rust stdlib
          RUST_SRC_PATH = "${rustSrc}/lib/rustlib/src/rust/library";
        };
      };
    };
}
