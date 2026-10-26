# `music` — the composed music window: spotify-player stacked on a cava
# strip, one zellij session (layout in music.kdl, linked by store path so
# the terminal/zellij layouts dir stays untouched). spotify-player streams
# through its built-in librespot (nixpkgs default features: rodio, MPRIS,
# sixel album art) — needs Premium; the first run walks an OAuth login.
# Colors: spotify-player via its stylix target; cava has no palette target,
# so its gradient rides the preset border story (borderInactive floor to
# accent peaks), stylix base0D when a preset has no accent (macchiato).
{ pkgs, config, osConfig, ... }:
let
  themes = import ../../themes;
  preset = themes.forPolarity osConfig.parker.theme.polarity;

  music = pkgs.writeShellScriptBin "music" ''
    if zellij list-sessions 2>/dev/null | grep -q music; then
      exec zellij attach music
    fi
    exec zellij --session music --new-session-with-layout ${./music.kdl}
  '';
in
{
  home.packages = [ music ];

  programs.spotify-player = {
    enable = true;
    # Spotify Connect name; everything else stays at spotify-player's
    # defaults (the stylix target selects its theme).
    settings.device.name = "parker";
  };

  programs.cava = {
    enable = true;
    settings.color =
      if preset ? accent then
        {
          gradient = 1;
          gradient_count = 2;
          gradient_color_1 = "'#${preset.borderInactive}'";
          gradient_color_2 = "'#${preset.accent}'";
        }
      else
        { foreground = "'#${config.lib.stylix.colors.base0D}'"; };
  };
}
