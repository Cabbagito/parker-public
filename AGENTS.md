# AGENTS.md — parker

NixOS flake for the desktop "parker". Read DECISIONS.md before proposing
structural changes — it records why things are the way they are; this repo
records what. QUIRKS.md is reactive-only: consult on matching symptom, never
preemptively configure from it. IDEAS.md holds unbuilt, undecided features;
add to it freely, build from it only when asked.

This file is read by every agent that works here — Claude Code, `parker
code`, and parker himself through his engine. Directories with their own
AGENTS.md have rules for that part of the repo: read the nearest one before
changing anything under it.

## Map

- `flake.nix` — inputs (nixpkgs unstable, home-manager, disko, sops-nix,
  stylix, noctalia, claude-desktop, slippi), one output:
  `nixosConfigurations.parker` (plus the `visage` dev shell)
- `hosts/parker/` — entry point (composes, never configures), disko, hardware
- `system/` — system layer by concern, one file each, self-contained and
  deletable
- `home/` — home-manager layer by role: `terminal/`, `nvim/`, `tuis/`,
  `apps/`, `desktop/`, `theming/`, and `parker/` — parker the entity: the
  `parker` binary (parkerd), his engine, his mind, visage (PARKER.md is the
  design record)
- `themes/` — theme preset data consumed by Stylix (`system/theming.nix`);
  `themes/default.nix` names the day and night presets (night = base system,
  day = specialisation), switched via `theme set` / `theme light|dark|auto`.
  visage is not among the consumers: it keeps its own themes (visage/AGENTS.md)
- `mirror/` — builds the public copy of this repo, one way; never published
  itself (mirror/AGENTS.md)

## Conventions

- Entry points compose, never configure. Leaf modules are self-contained.
- A three-liner without a home goes in a `misc.nix` until it earns a file.
- Disks are addressed by `/dev/disk/by-id/`, never `sdX`.
- Non-nix config files (nvim lua, zellij kdl, p10k.zsh) live next to the
  module that links them and are the canonical copies — the giskard dotfiles
  repo is upstream inspiration, not a dependency.

## Building

The one statement of how this repo is built and deployed; nested AGENTS.md
files carry only their component's inner loop (`cargo run`, `cargo tauri
dev`) and point here for the rest.

- This repo builds x86_64-linux only, and parker is the only builder. On the
  MacBook there is no local nix and no CI — a change written here is unproven
  until it builds on parker.
- **Test**: `nixos-rebuild build --flake ~/parker#parker`. This is the gate:
  it evaluates everything and runs parkerd's `cargo test`. No sudo needed.
- **Deploy**: `sudo nixos-rebuild switch --flake ~/parker#parker` (passwordless
  for this exact command). Switch when Peter asks for a change to go live,
  never as a test. A switch restarts services — the engine among them — and
  reloads the compositor under a live session.
- **Boot instead of switch** when the tree changes what a running session
  cannot take live: the compositor's config type, the kernel, the GPU driver.
  `sudo nixos-rebuild boot …`, then reboot. Before any switch, read
  `git log -5` and the diff for a commit that says so.
- Flakes see only git-tracked files: `git add` a new file before building, or
  the build silently does not see it.
- flake.lock is edited on parker: adding a flake input means `nix flake lock`
  there (it adds entries for new inputs, never bumps existing pins — the
  monthly ritual owns those), then commit the lock alongside the change.
- Update ritual (monthly): `nix flake update`, test-build, summarize what
  changed, flag driver/kernel bumps (those deploy with `boot`), then deploy.

## Gotchas

- `dotfiles/` (if present) is an ignored local reference clone — never commit
  it, never treat it as part of this repo.
- `home/nvim/config` is linked read-only into `~/.config/nvim`; the
  lazy.nvim lockfile is redirected to `stdpath("state")` in
  `lua/config/lazy.lua`. Keep that line when touching lazy setup.
- btop saves its config on exit and will grumble about the read-only
  `btop.conf` symlink — cosmetic, accepted.
- v0 has zero sops secrets by design and the bootstrap has never been run;
  the recipe is in `system/sops.nix`. The repo is **private** — plaintext
  rotatable/scoped credentials are acceptable in it (DECISIONS.md), so sops
  is a deliberate upgrade, not a prerequisite. A public copy is built from
  it, though, so in practice credentials live on the machine, not in the
  repo (mirror/AGENTS.md has the pattern), and a new file about Peter gets
  a line in `mirror/` in the same commit.
