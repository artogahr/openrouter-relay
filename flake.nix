{
  description = "OpenRouter relay Standby Actor";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = eachSystem (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "apify-openrouter-relay";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
      });

      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            clippy
            rustc
            rustfmt
            nixfmt
          ];
        };
      });

      checks = eachSystem (pkgs: {
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      });

      formatter = eachSystem (pkgs: pkgs.nixfmt);
    };
}
