# parker theme system

Six themes, three worlds, each world seen by day and by night. The schedule
flips polarity within the active family; other families are manual moods.

| family     | day     | night   | world                                  |
|------------|---------|---------|----------------------------------------|
| the garden | blush   | velour  | bloom at dawn / the woody velvet dark  |
| the drift  | glacier | trench  | on the ice sheet / the abyss beneath   |
| the forge  | hangar  | redline | the empty bay / the machine, live      |

## Contents

- `<theme>/scheme.yaml` — base16 schemes (classic flat format, Stylix-ready)
- `<theme>/wallpaper.svg` — 2560x1440 source wallpapers, one per theme
- `theme-lab.html` — desktop mockup previewer; the THEMES array is the
  canonical color record (roles, opacity + blur knobs per theme)

## Shared wallpaper grammar ("the same process")

Quiet center (windows live there; lone-window gapsout 120/250 leaves a fat
peripheral ring). Motifs enter from edges; faint large structure crosses the
middle. One spark per frame in the theme's secondary accent. A per-world
"tick" signature: status column (forge), survey stake / sounding line
(drift), trellis (garden). Text only in the forge: PKR-01 stencil.
Uniform grain pass (fractalNoise, ~4.5% alpha) on all six.

## Build notes for Claude Code

- Rasterize SVG -> PNG at build time (resvg or inkscape) for the compositor.
  *Landed:* resvg in `system/theming.nix`, JetBrains Mono handed into the
  sandbox for the PKR-01 stencil text, EB Garamond for blush's plate
  captions.
- PKR-01 currently uses a monospace stack with slit-rects faking stencil;
  final pass should outline a real stencil face (Saira Stencil One /
  Allerta Stencil) to paths for zero font dependencies. Same open end for
  blush's serif captions — live `<text>`, so the sandbox needs the face.
- Wallpapers hardcode each theme's `scheme.yaml` colors (hangar's chassis is
  base08 #E03E4B, its cones base0C #4E97AE) rather than reading the yaml at
  build time — retuning a scheme drifts its wallpaper out of sync.
- `glacier` and `hangar` are fat (~420/320 KB) because the contours and the
  halftone are real geometry, not a texture. They rasterize fine.
- Second generation of wallpapers landed 2026-07-26; the first is kept beside
  each as `wallpaper-v1.svg` (themes/README.md).
- Per-theme opacity/blur knobs live in theme-lab.html THEMES array.
- border_size is currently 0 in hyprland.nix — whether Stylix introduces
  accent borders is an open decision; the lab previews the bordered look.
  *Decided:* borders are in (border_size 2); presets carry the lab's
  accent/borderI as `accent`/`borderInactive`, hyprland.nix prefers them.
- Day/night switching: prefer specialisations (both polarities pre-built,
  near-instant flip via systemd timer or darkman) over rebuild-per-flip.
  *Landed:* base system = night, `day` specialisation = day
  (system/theming.nix); `theme set <family|preset>` picks the pair,
  `theme light/dark/auto` override and resume, a user timer flips at the
  schedule boundaries (themes/default.nix, 08:00/18:30). darkman remains
  the upgrade path if sunset-tracking is ever wanted.
- Deferred ideas: "ash" variant (velour structure, Vesper-peach accent);
  a transparent-heavy preset (transparency is a knob, not a theme).
