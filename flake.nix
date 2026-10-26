{
  description = "parker — declarative NixOS desktop";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # Theming engine (system/theming.nix + themes/ presets). Master branch is
    # the rolling release matching nixos-unstable; updated by the monthly
    # ritual alongside nixpkgs.
    stylix = {
      url = "github:nix-community/stylix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # v5 (native rewrite), cachix branch. Deliberately NO nixpkgs.follows:
    # following would change derivation hashes and miss the binary cache,
    # forcing local C++/GLES builds (upstream docs call this out).
    noctalia.url = "github:noctalia-dev/noctalia/cachix";
    # aaddrick repackages the official Anthropic .deb, tracking it within
    # days. (heytcass/claude-for-linux was tried first for its HM module but
    # is broken against current nixpkgs — needs the removed electron_37.)
    claude-desktop = {
      url = "github:aaddrick/claude-desktop-debian";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    slippi = {
      url = "github:lytedev/slippi-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      nixpkgs,
      home-manager,
      disko,
      sops-nix,
      stylix,
      ...
    }:
    let
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in
    {
      nixosConfigurations.parker = nixpkgs.lib.nixosSystem {
        specialArgs = {
          inherit inputs;
        };
        modules = [
          ./hosts/parker
          disko.nixosModules.disko
          sops-nix.nixosModules.sops
          stylix.nixosModules.stylix
          home-manager.nixosModules.home-manager
          {
            home-manager.useGlobalPkgs = true;
            home-manager.useUserPackages = true;
            home-manager.backupFileExtension = "hm-backup";
            home-manager.extraSpecialArgs = {
              inherit inputs;
            };
            home-manager.users.peki = import ./home;
          }
        ];
      };

      # Dev loop for home/parker/visage (Tauri): the C stack cargo must see —
      # WebKitGTK, GTK, soup — lives here, not in the user profile. Usage:
      #   nix develop ~/parker#visage
      devShells.x86_64-linux.visage = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          pkg-config
          gobject-introspection
          cargo-tauri
          nodejs
        ];
        buildInputs = with pkgs; [
          at-spi2-atk
          cairo
          gdk-pixbuf
          glib
          gtk3
          harfbuzz
          librsvg
          libsoup_3
          pango
          webkitgtk_4_1
          openssl
        ];
      };
    };
}
