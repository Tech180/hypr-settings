{
  description = "Native Hyprland settings app (GTK4 + libadwaita)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" ];
      forEach = f: nixpkgs.lib.genAttrs systems (system: f (pkgsFor system));
      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          config.allowUnfree = true;
          overlays = [ self.overlays.default ];
        };
    in
    {
      overlays.default = final: _prev: {
        hypr-settings = final.callPackage ./nix/package.nix { };
      };

      homeManagerModules.default = ./nix/home-manager.nix;

      packages = forEach (pkgs: {
        inherit (pkgs) hypr-settings;
        cargo = pkgs.callPackage ./nix/cargo.nix { };
        default = pkgs.hypr-settings;
      });

      devShells = forEach (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            kanshi
            ddcutil
            brightnessctl
            hyprsunset
            pkg-config
          ];
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = with pkgs; [
            gtk4
            libadwaita
            glib
            dbus
            bluez
          ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });
    };
}
