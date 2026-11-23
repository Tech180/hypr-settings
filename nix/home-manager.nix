# Writes the nix-colors palette into the file the app loads ahead of its built-in tokens.
# No-op when Home Manager has no colorScheme (nix-colors is not imported).
{ config, lib, ... }:
let
  palette = config.colorScheme.palette or null;
in
{
  config = lib.mkIf (palette != null) {
    xdg.configFile."hypr-settings/tokens.css".text = import ./tokens-css.nix config.colorScheme;
  };
}
