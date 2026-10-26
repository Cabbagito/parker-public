# Theme presets, consumed by system/theming.nix. Pure data — no modules.
# `day`/`night` are the two-line switch: `theme set <family|preset>`
# (home/theming) edits them and rebuilds. A family sets its pair; a single
# preset name pins both slots (the schedule then has nothing to flip).
# NOTE: the theme command's sed matches the two-space-indented `day =` /
# `night =` lines below — keep `schedule` on one line so its keys never
# collide with them.
rec {
  day = "hangar";
  night = "redline";

  # Glass knob: opacity of unfocused windows (0.5..1, 1 = solid). Global,
  # not per-preset — `theme transparency <v>` seds this line and rebuilds;
  # hyprland derives the active-window value, ghostty its background alpha.
  transparency = 0.8;

  # Local-time boundaries for `theme sync` and its timer (HH:MM, 24h).
  schedule = { day = "08:00"; night = "18:30"; };

  # The three worlds (DESIGN.md), each seen by day and by night.
  families = {
    garden = { day = "blush"; night = "velour"; };
    drift = { day = "glacier"; night = "trench"; };
    forge = { day = "hangar"; night = "redline"; };
  };

  presets = {
    macchiato = import ./macchiato;
    blush = import ./blush;
    velour = import ./velour;
    glacier = import ./glacier;
    trench = import ./trench;
    hangar = import ./hangar;
    redline = import ./redline;
  };

  # Selection helpers: polarity ("day"|"night") -> preset name / preset.
  # system/theming.nix and home/desktop/hyprland.nix select through these,
  # driven by the parker.theme.polarity option (base system = night, the
  # `day` specialisation = day).
  nameFor = polarity: { inherit day night; }.${polarity};
  forPolarity = polarity: presets.${nameFor polarity};
}
