{
  description = "Agent Illustrator - A declarative illustration language for AI agents";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        version = "0.1.25";

        # Map Nix system to GitHub release artifact name
        artifactName = {
          "x86_64-linux" = "agent-illustrator-linux-x86_64";
          "aarch64-linux" = "agent-illustrator-linux-aarch64";
          "x86_64-darwin" = "agent-illustrator-macos-x86_64";
          "aarch64-darwin" = "agent-illustrator-macos-aarch64";
        }.${system} or (throw "Unsupported system: ${system}");

        # Hashes for v0.1.25 release binaries
        artifactHash = {
          "x86_64-linux" = "sha256-7zVLZCP0bYfPqqp2hibrSdF1WRx+2AjFTa15wdYpwrQ=";
          "aarch64-linux" = "sha256-R84hQpBAcg3JwbdvKvNAON9EIW4E6TqInUfTyhP/Fmo=";
          "x86_64-darwin" = "sha256-TK6w+NbXKlbfJtmT2CyVZGAlHDxI3jqTVIz8gf/vlMY=";
          "aarch64-darwin" = "sha256-ZNCJiqty1Oq7MuP+pld5Ofs96RZwjd5qhi8w/DvOP7A=";
        }.${system} or (throw "Unsupported system: ${system}");

      in
      {
        packages.default = pkgs.stdenv.mkDerivation {
          pname = "agent-illustrator";
          inherit version;

          src = pkgs.fetchurl {
            url = "https://github.com/kervel/agent-illustrator/releases/download/v${version}/${artifactName}";
            hash = artifactHash;
          };

          dontUnpack = true;
          dontBuild = true;

          # autoPatchelfHook fixes dynamic linker paths for prebuilt binaries on NixOS
          nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.autoPatchelfHook ];
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.stdenv.cc.cc.lib ];

          installPhase = ''
            mkdir -p $out/bin
            cp $src $out/bin/agent-illustrator
            chmod +x $out/bin/agent-illustrator
          '';

          meta = with pkgs.lib; {
            description = "A declarative illustration language for AI agents";
            homepage = "https://github.com/kervel/agent-illustrator";
            license = licenses.mit;
            platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
          };
        };

        # Also provide a from-source build for development
        packages.from-source = pkgs.rustPlatform.buildRustPackage {
          pname = "agent-illustrator";
          inherit version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ cargo rustc rust-analyzer rustfmt clippy ];
        };
      }
    );
}
