# themes/

Theme presets consumed by Stylix (`system/theming.nix`). Pure data, no
modules — one directory per preset. [default.nix](default.nix) names a
`day` and a `night` preset, the families that pair them, the flip schedule
and the transparency; the base system is built from the night preset and
the `day` specialisation from the day one.

Switching: `theme set <family|preset>` (defined in `home/theming/`) edits
the day/night lines and rebuilds (~30s); `theme light|dark|auto` flips
between the two in seconds. `theme list` enumerates the directories here.

visage does not read these. It keeps its own themes, six of them copies of
these presets by name and colour (`home/parker/visage/src/lib/themes.js`),
so retuning a preset here leaves visage as it was.

A preset provides:

- `scheme` — either a base16 yaml file name under
  `${pkgs.base16-schemes}/share/themes/` (string) or a path to the preset's
  own yaml (`./scheme.yaml`)
- `polarity` — `"dark"` or `"light"`
- `wallpaper` (optional) — an image path; becomes `stylix.image`, and the
  stylix noctalia target hands it to the shell's wallpaper. `.svg` paths are
  rasterized to 2560x1440 PNG at build time (resvg, in `system/theming.nix`);
  the sandbox has no system fonts, so any face a wallpaper names must be
  handed in there (currently JetBrains Mono and EB Garamond).
- `accent` + `borderInactive` (optional, paired) — bare hex window border
  colors (active/inactive), consumed by `home/desktop/hyprland.nix`; from
  the theme-lab THEMES record. Without them the stylix hyprland target's
  scheme-derived borders (base0D/base03) apply.

Adding a preset: new directory with a `default.nix`, register it in
`presets` in [default.nix](default.nix), `theme set <name>`.

Wallpaper files in a preset directory:

- `wallpaper.svg` — the one that ships; the only file `default.nix` points at
- `wallpaper-v1.svg` — the first-generation wallpaper, kept for reference
- `themes/hangar/wallpaper-blueprint.svg` — the alternative for the hangar
  slot (an orthographic drafting sheet instead of the exploded 2x12 combo
  amp). Same palette, same PKR-01 stencil; only one of the two ships. Swap by
  pointing `themes/hangar/default.nix` at it.

Anything other than `wallpaper.svg` is inert data — nothing imports it.

Presets: `macchiato/` (stock Catppuccin Macchiato, the original — no
wallpaper or accent of its own) plus the six
[DESIGN.md](DESIGN.md) worlds — `blush`/`velour` (garden), `glacier`/`trench`
(drift), `hangar`/`redline` (forge); day is light, night is dark.
