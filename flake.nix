{
  description = "A compression multi-tool for the command line.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    crane = {
      url = "github:ipetkov/crane";
    };

    fenix = {
      # Needed because rust-overlay, normally used by crane, doesn't have llvm-tools for coverage
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    # Flake helper for better organization with modules.
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

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
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

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

        # Common arguments for mkCargoDerivation, a helper for the crane functions
        # Arguments can be included here even if they aren't used, but we only
        # place them here if they would otherwise show up in multiple places
        commonArgs = {
          inherit cargoArtifacts;
          # Clean the src to only have the Rust-relevant files
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.gcc
          ];
        };

        # Build only the cargo dependencies so we can cache them all when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the cargoArtifacts
        cmprss = craneLib.buildPackage (commonArgs
          // {
            doCheck = false; # Tests are run as a separate build with nextest
            meta.mainProgram = "cmprss";
          });

        # Fully static musl build (Linux only)
        staticMusl = pkgs.lib.optionalAttrs pkgs.stdenv.isLinux (
          let
            muslTarget =
              if system == "x86_64-linux"
              then "x86_64-unknown-linux-musl"
              else "aarch64-unknown-linux-musl";

            muslToolchain = inputs.fenix.packages.${system}.combine [
              fenixStable.cargo
              fenixStable.rustc
              inputs.fenix.packages.${system}.targets.${muslTarget}.stable.rust-std
            ];

            craneLibMusl = (inputs.crane.mkLib pkgs).overrideToolchain muslToolchain;

            musl-cc = pkgs.pkgsStatic.stdenv.cc;

            staticArgs = {
              src = craneLibMusl.cleanCargoSource ./.;
              strictDeps = true;
              CARGO_BUILD_TARGET = muslTarget;
              CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
              TARGET_CC = "${musl-cc}/bin/${musl-cc.targetPrefix}cc";
              HOST_CC = "${pkgs.stdenv.cc}/bin/cc";
              nativeBuildInputs = [
                musl-cc
                pkgs.pkg-config
              ];
            };

            cargoArtifactsStatic = craneLibMusl.buildDepsOnly staticArgs;
          in {
            cmprss-static = craneLibMusl.buildPackage (staticArgs
              // {
                cargoArtifacts = cargoArtifactsStatic;
                doCheck = false;
                meta.mainProgram = "cmprss";
              });
          }
        );
        # Source filtered to specific file extensions for lightweight linter checks
        cleanSrc = pkgs.lib.cleanSource ./.;
        sourceWithExts = exts:
          pkgs.lib.cleanSourceWith {
            src = cleanSrc;
            filter = path: type:
              (type == "directory")
              || (pkgs.lib.any (ext: pkgs.lib.hasSuffix ".${ext}" path) exts);
          };

        mkSimpleLinter = {
          name,
          packages ? [],
          src ? cleanSrc,
          command,
        }:
          pkgs.runCommand "lint-${name}" {
            nativeBuildInputs = packages;
            inherit src;
          } ''
            cd $src
            ${command}
            mkdir -p $out
          '';

        linters = {
          statix = mkSimpleLinter {
            name = "statix";
            packages = [pkgs.statix];
            src = sourceWithExts ["nix"];
            command = "statix check .";
          };
          deadnix = mkSimpleLinter {
            name = "deadnix";
            packages = [pkgs.deadnix];
            src = sourceWithExts ["nix"];
            command = "deadnix --fail .";
          };
          shellcheck = mkSimpleLinter {
            name = "shellcheck";
            packages = [pkgs.shellcheck pkgs.findutils];
            src = sourceWithExts ["sh"];
            command = ''find . -name "*.sh" -type f -exec shellcheck {} +'';
          };
          actionlint = mkSimpleLinter {
            name = "actionlint";
            packages = [pkgs.actionlint pkgs.shellcheck pkgs.findutils];
            src = pkgs.lib.cleanSourceWith {
              src = cleanSrc;
              filter = path: type:
                (type == "directory")
                || (pkgs.lib.hasSuffix ".yml" path && pkgs.lib.hasInfix ".github" path);
            };
            command = ''find .github/workflows -name "*.yml" -exec actionlint {} +'';
          };
        };
      in {
        packages =
          {
            default = cmprss;
            inherit cmprss;

            # Check code coverage with tarpaulin
            coverage = craneLib.cargoTarpaulin (commonArgs
              // {
                # Use lcov output as thats far more widely supported
                cargoTarpaulinExtraArgs = "--skip-clean --include-tests --output-dir $out --out lcov";
              });

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
            test = craneLib.cargoNextest commonArgs;

            # Audit dependencies, check licenses, and detect duplicate crates
            deny = craneLib.cargoDeny (commonArgs
              // {
                # advisories excluded: needs network access (blocked by nix sandbox)
                cargoDenyChecks = "bans licenses sources";
              });
          }
          // staticMusl;

        checks =
          {
            inherit cmprss;
            # Build almost every package in checks, with exceptions:
            # - coverage: It requires a full rebuild, and only needs to be run occasionally
            inherit (self.packages.${system}) clippy doc fmt test deny;
          }
          // linters;

        # This also sets up `nix fmt` to run all formatters
        treefmt = {
          projectRootFile = "./flake.nix";
          programs = {
            alejandra.enable = true;
            prettier.enable = true;
            rustfmt.enable = true;
            shfmt.enable = true;
            typos = {
              enable = true;
              configFile = "./.config/typos.toml";
            };
          };
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
            just --list
            echo ---------------------
          '';

          # Include the packages from the defined checks and packages
          # Installs the full cargo toolchain and the extra tools, e.g. cargo-tarpaulin.
          inputsFrom =
            (builtins.attrValues self.checks.${system})
            ++ (builtins.attrValues self.packages.${system});

          # Extra inputs can be added here
          packages = with pkgs; [
            act # For running Github Actions locally
            alejandra
            actionlint
            deadnix
            git-cliff
            gum # Pretty printing in scripts
            just
            nodePackages.prettier
            shellcheck
            statix
            typos

            # For running tests
            diffutils

            # Official tools
            brotli
            bzip2
            gnutar
            gzip
            lz4
            snzip
            unzip
            xz
            zip
            zstd
          ];

          # Many tools read this to find the sources for rust stdlib
          RUST_SRC_PATH = "${rustSrc}/lib/rustlib/src/rust/library";
        };
      };
    };
}
