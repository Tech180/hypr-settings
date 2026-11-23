# GTK @define-color tokens for hypr-settings.
# Argument is a nix-colors `colorScheme` attrset (`.palette`).
colorScheme:
let
  p = colorScheme.palette;
in
  ''
    /* Generated from the active nix-colors scheme. */
    @define-color hs_warning #${p.base09};
    @define-color hs_bg #${p.base00};
    @define-color hs_card #${p.base01};
    @define-color hs_border #${p.base02};
    @define-color hs_accent #${p.base0D};
    @define-color hs_fg #${p.base05};
    @define-color hs_dim #${p.base04};
    @define-color hs_danger #${p.base08};
    @define-color hs_teal #${p.base0B};
    @define-color hs_hover alpha(@hs_fg, 0.05);
    @define-color hs_shadow alpha(@hs_bg, 0.35);
    @define-color hs_accent_glow alpha(@hs_accent, 0.15);
    @define-color hs_trial_bg alpha(@hs_warning, 0.12);
    @define-color hs_on_warning @hs_bg;
  ''
