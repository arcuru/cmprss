{
  description = "cmprss: a compression client for the CLI";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        nixpkgs-stable.follows = "nixpkgs";
      };
    };
    parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    nci = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        parts.follows = "parts";
      };
    };
    # Universal formatting
    treefmt = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    inputs.parts.lib.mkFlake {inherit inputs;} {
      imports = [
        inputs.nci.flakeModule
        inputs.pre-commit-hooks.flakeModule
        inputs.treefmt.flakeModule
      ];
      systems = ["aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux"];
      perSystem = {
        config,
        pkgs,
        lib,
        ...
      }: {
        nci = {
          projects.cmprss.relPath = "";
          crates.cmprss = {export = true;};
        };
        packages.default = config.nci.outputs.cmprss.packages.release;

        treefmt = {
          projectRootFile = ./flake.nix;
          programs = {
            # Format nix files using alejandra
            alejandra.enable = true;

            # Format Markdown/css/etc
            # The settings are contained in .prettierrc.yaml
            prettier.enable = true;

            # Format rust files using rustfmt
            rustfmt.enable = true;
          };
        };

        pre-commit = {
          # Can't run in `nix flake check` due to the sandbox
          check.enable = false;
          settings = {
            settings.treefmt.package = config.treefmt.build.wrapper;
            hooks = {
              treefmt.enable = true;

              # Ensure no clippy warnings exist
              # FIXME: Re-enable. Fails for unknown reasons
              #clippy.enable = true;

              # Runs `cargo check` to look for errors
              cargo-check.enable = true;
            };
          };
        };

        devShells.default =
          config.nci.outputs.cmprss.devShell.overrideAttrs
          (old: {
            name = "cmprss";
            shellHook = ''
              ${config.pre-commit.installationScript}
            '';
            nativeBuildInputs = with pkgs; [
              act # For running Github Actions locally
              config.treefmt.build.wrapper # `treefmt` to format everything
              nodePackages.prettier
              rust-analyzer
            ];
          });
      };
    };
}
