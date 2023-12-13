{
  description = "A compression multi-tool for the command line.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        nixpkgs-stable.follows = "nixpkgs";
      };
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

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
  };

  outputs = {self, ...} @ inputs:
    inputs.flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import inputs.nixpkgs {
        inherit system;
      };

      inherit (pkgs) lib;

      # Use the stable rust tools from fenix
      fenixStable = inputs.fenix.packages.${system}.stable;
      rustSrc = fenixStable.rust-src;
      toolChain = fenixStable.completeToolchain;

      # Use the toolchain with the crane helper functions
      craneLib = inputs.crane.lib.${system}.overrideToolchain toolChain;

      # Clean the src to only have the Rust-relevant files
      src = craneLib.cleanCargoSource (craneLib.path ./.);

      # Common arguments for mkCargoDerivation, a helper for the crane functions
      # Arguments can be included here even if they aren't used, but we only
      # place them here if they would otherwise show up in multiple places
      commonArgs = {
        inherit src cargoArtifacts;
      };

      # Build only the cargo dependencies so we can cache them all when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the cargoArtifacts
      cmprss = craneLib.buildPackage commonArgs;
    in {
      checks =
        {
          # Build the crate as part of `nix flake check` for convenience
          inherit cmprss;

          # Run clippy (and deny all warnings) on the crate source
          cmprss-clippy = craneLib.cargoClippy (commonArgs
            // {
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

          # Check docs build successfully
          cmprss-doc = craneLib.cargoDoc commonArgs;

          # Check formatting
          cmprss-fmt = craneLib.cargoFmt commonArgs;

          # Run tests with cargo-nextest
          # Note: This provides limited value, as tests are already run in the build
          cmprss-nextest = craneLib.cargoNextest commonArgs;

          # Audit dependencies
          crate-audit = craneLib.cargoAudit (commonArgs
            // {
              inherit (inputs) advisory-db;
            });
        }
        // lib.optionalAttrs (system == "x86_64-linux") {
          # Check code coverage with tarpaulin runs
          cmprss-tarpaulin = craneLib.cargoTarpaulin commonArgs;
        }
        // {
          # Run formatting checks before commit
          # Can be run manually with `pre-commit run -a`
          pre-commit-check = inputs.pre-commit-hooks.lib.${system}.run {
            src = ./.;
            tools.rustfmt = toolChain;
            hooks = {
              alejandra.enable = true; # Nix formatting
              prettier.enable = true; # Markdown formatting
              rustfmt.enable = true; # Rust formatting
            };
          };
        };

      packages = {
        default = cmprss;
        cmprss = cmprss;
      };

      apps = rec {
        default = cmprss;
        cmprss = inputs.flake-utils.lib.mkApp {
          drv = self.packages.${system}.cmprss;
        };
      };

      devShells.default = pkgs.mkShell {
        name = "cmprss";
        shellHook = ''
          ${self.checks.${system}.pre-commit-check.shellHook}
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
        ];

        # Many tools read this to find the sources for rust stdlib
        RUST_SRC_PATH = "${rustSrc}/lib/rustlib/src/rust/library";
      };
    })
    // {
      overlays.default = final: prev: {
        cmprss = self.packages.${final.system}.cmprss;
      };
    };
}
