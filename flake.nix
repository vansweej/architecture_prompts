{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forEachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in {

      # ── packages ────────────────────────────────────────────────────────────
      #
      # Build the binary with rustPlatform.buildRustPackage.
      # Both src/ and prompts/ must be included because the prompts are
      # embedded at compile time via include_str!().
      #
      # opencode is NOT a Nix dependency — it is managed outside Nix and must
      # be present in PATH at runtime.
      packages = forEachSystem (pkgs: {
        default =
          let
            fs = pkgs.lib.fileset;
          in
          pkgs.rustPlatform.buildRustPackage {
            pname = "architecture_prompts";
            version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;

            src = fs.toSource {
              root = ./.;
              fileset = fs.unions [
                ./Cargo.toml
                ./Cargo.lock
                ./src
                ./prompts
              ];
            };

            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "Activate an architect system prompt for an opencode session";
              mainProgram = "architecture_prompts";
            };
          };
      });

      # ── overlay ─────────────────────────────────────────────────────────────
      #
      # Allows other flakes to compose this package into their own pkgs set:
      #
      #   nixpkgs.overlays = [ inputs.architecture-prompts.overlays.default ];
      #   environment.systemPackages = [ pkgs.architecture-prompts ];
      #
      overlays.default = final: _prev: {
        architecture-prompts = self.packages.${final.system}.default;
      };

      # ── apps ────────────────────────────────────────────────────────────────
      #
      # Enables `nix run github:you/architecture_prompts -- principal`.
      #
      apps = forEachSystem (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.system}.default}/bin/architecture_prompts";
        };
      });

      # ── devShells ───────────────────────────────────────────────────────────
      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            cargo-tarpaulin
          ];
        };
      });
    };
}
