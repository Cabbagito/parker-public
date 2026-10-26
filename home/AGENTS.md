# AGENTS.md — home/

home-manager layer, organized by role: `terminal/` (zsh, git, zellij),
`nvim/`, `tuis/` (btop, yazi, lazygit, fastfetch, gh-dash, wiremix), `apps/`
(claude, chrome, spotify, discord, slippi, emulation, gaming, qbittorrent), `desktop/` (hyprland, ghostty, noctalia v5 shell, screenshots),
`theming/` (the `theme` command + the stylix connection map; the engine is
system/theming.nix, presets are data in themes/).

Rules of the house:

- Non-nix config files (lua, kdl, toml, conf) sit next to the module that
  links them via `xdg.configFile`/`home.file`. They are parker's canonical
  copies — edit them here, rebuild to apply.
- All configs are compiled/read-only at runtime. The escape hatch for a tool
  that needs a live edit loop is `mkOutOfStoreSymlink` — currently unused,
  deliberately (DECISIONS.md "all configs compiled").
- A tool that outgrows its role-group gets promoted to its own directory
  (as nvim already is).
- Keep modules Linux-agnostic where cheap — absorbing giskard later stays
  possible.
