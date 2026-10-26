# Stylix: one theming engine, presets as data in themes/. The flake wires
# stylix.nixosModules.stylix; this file points it at the day/night pair
# (themes/default.nix) and pre-builds BOTH polarities: the base system is
# the night preset, the `day` specialisation the day one — so flipping is a
# seconds-long activation (`theme light/dark/sync`, home/theming), not a
# rebuild. home-manager runs as a NixOS module, so stylix auto-imports its
# HM side into each variant and targets auto-enable for whatever the home
# layer turns on (ghostty, hyprland, noctalia, btop, lazygit, yazi, zellij,
# fzf, gdu, gtk/qt) — connection notes live in home/theming/.
{ pkgs, lib, config, ... }:
let
  themes = import ../themes;
  presetName = themes.nameFor config.parker.theme.polarity;
  preset = themes.presets.${presetName};

  # A preset's scheme is either a string (file under pkgs.base16-schemes) or
  # a path to its own yaml (the six DESIGN.md worlds ship theirs).
  scheme =
    if builtins.isString preset.scheme then
      "${pkgs.base16-schemes}/share/themes/${preset.scheme}"
    else
      preset.scheme;

  # SVG wallpapers rasterize to PNG at build time (DESIGN.md). The sandbox
  # has no fonts, so every face a wallpaper names is handed in explicitly:
  # JetBrains Mono for the forge pair's PKR-01 stencil and every other
  # wallpaper's mono labels, EB Garamond for blush's engraved plate captions
  # (its stack is "EB Garamond, Iowan Old Style, Georgia, serif" and the text
  # is live <text>, not outlined paths — without a serif here the plate falls
  # back to the mono face and reads wrong). Only ever forced when the preset
  # has a wallpaper — the optionalAttrs guard below must stay on the pure
  # `preset ? wallpaper` (an attr-presence condition that needs pkgs recurses
  # infinitely).
  wallpaper =
    if lib.hasSuffix ".svg" (toString preset.wallpaper) then
      pkgs.runCommand "wallpaper-${presetName}.png"
        { nativeBuildInputs = [ pkgs.resvg ]; }
        ''
          resvg --width 2560 --height 1440 \
            --skip-system-fonts \
            --use-fonts-dir ${pkgs.jetbrains-mono}/share/fonts/truetype \
            --use-fonts-dir ${pkgs.eb-garamond}/share/fonts \
            --monospace-family "JetBrains Mono" \
            --serif-family "EB Garamond" \
            ${preset.wallpaper} $out
        ''
    else
      preset.wallpaper;
in
{
  # Which half of the themes/ pair this system variant is built with.
  # Everything preset-driven (stylix here, hyprland borders in the home
  # layer via osConfig) selects through it.
  options.parker.theme.polarity = lib.mkOption {
    type = lib.types.enum [ "day" "night" ];
    default = "night";
    description = "Half of the themes/ day/night pair this variant uses.";
  };

  config = {
    stylix = {
      enable = true;
      base16Scheme = scheme;
      inherit (preset) polarity;

      # The font stack fonts.nix installs, told to stylix so targets (ghostty,
      # noctalia's shell font, gtk) agree on it.
      fonts = {
        monospace = {
          package = pkgs.nerd-fonts.meslo-lg;
          name = "MesloLGS Nerd Font";
        };
        sansSerif = {
          package = pkgs.inter;
          name = "Inter";
        };
        serif = {
          package = pkgs.noto-fonts;
          name = "Noto Serif";
        };
        emoji = {
          package = pkgs.noto-fonts-color-emoji;
          name = "Noto Color Emoji";
        };
        # Matches the pre-stylix ghostty font-size.
        sizes.terminal = 17;
      };

      # Was home.pointerCursor in home/desktop; stylix owns it now
      # (hyprcursor stays off there — apple-cursor is XCursor only).
      cursor = {
        package = pkgs.apple-cursor;
        name = "macOS";
        size = 24;
      };

      # Terminal glass at half the global knob's strength (themes/default.nix
      # `transparency`, live-tweaked via `theme transparency <v>`): the
      # ghostty target turns this into background-opacity, so only the
      # background fades — text stays crisp. knob 0.85 -> 0.925; 1 -> solid.
      opacity.terminal = (1.0 + themes.transparency) / 2.0;

      # Plymouth is dropped entirely (DECISIONS.md) — never let stylix
      # resurrect it.
      targets.plymouth.enable = false;
    }
    # stylix.image is optional; presets with a `wallpaper` get it (noctalia's
    # target then sets it as the shell wallpaper).
    // lib.optionalAttrs (preset ? wallpaper) { image = wallpaper; };

    # The pre-built day half. Only emitted for a real pair — a pinned single
    # preset builds one system and the theme command's flips no-op. (NixOS
    # clears `specialisation` inside the child itself, so no recursion.)
    specialisation = lib.mkIf (themes.day != themes.night) {
      day.configuration.parker.theme.polarity = "day";
    };

    # `theme light/dark/sync` flips polarity from the user session (and its
    # timer) without a password prompt — scoped to exactly the two pre-built
    # activation commands, nothing else.
    #
    # Wrapped in `timeout` so the kill happens as root: a wedged activation
    # holds the global rebuild lock, and sudo will not forward a signal from
    # the unprivileged side (QUIRKS.md "theme flip wedges the rebuild lock").
    # The rules match argv exactly, so they must stay byte-identical to the
    # invocations in home/theming/default.nix `apply()`.
    security.sudo.extraRules = [
      {
        users = [ "peki" ];
        commands = [
          {
            command = "/run/current-system/sw/bin/timeout 120 /nix/var/nix/profiles/system/bin/switch-to-configuration test";
            options = [ "NOPASSWD" ];
          }
          {
            command = "/run/current-system/sw/bin/timeout 120 /nix/var/nix/profiles/system/specialisation/day/bin/switch-to-configuration test";
            options = [ "NOPASSWD" ];
          }
        ];
      }
    ];

    # Backend for the gtk target (stylix flips HM's gtk.enable, which wants
    # dconf present system-side).
    programs.dconf.enable = true;
  };
}
