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
  };

  outputs = inputs:
    inputs.parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.nci.flakeModule inputs.pre-commit-hooks.flakeModule ];
      systems =
        [ "aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux" ];
      perSystem = { config, pkgs, lib, ... }: {
        nci = {
          projects.cmprss.relPath = "";
          crates.cmprss = { export = true; };
        };
        packages.default = config.nci.outputs.cmprss.packages.release;

        pre-commit.settings = {
          hooks = {
            # Format rust files using rustfmt
            rustfmt.enable = true;

            # Ensure no clippy warnings exist
            # FIXME: Re-enable. Fails due to mismatched compiler versions because of the devToolchain
            #clippy.enable = true;

            # Runs `cargo check` to look for errors
            # FIXME: Re-enable. Fails due to network errors in the sandbox.
            #cargo-check.enable = true;

            # Format nix files using nixfmt
            # This hook will format the files for you, so on a failure all
            # that's needed to fix is to re-run the commit.
            nixfmt.enable = true;

            # Format Markdown/css/etc
            # The settings are contained in .prettierrc.yaml
            prettier.enable = true;
          };
        };

        devShells.default = config.nci.outputs.cmprss.devShell.overrideAttrs
          (old: {
            name = "cmprss";
            shellHook = ''
              ${config.pre-commit.installationScript}
            '';
            nativeBuildInputs = with pkgs; [
              act # For running Github Actions locally
              nodePackages.prettier
              rust-analyzer
            ];
          });
      };
    };
}
