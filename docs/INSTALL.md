# INSTALL.md — parker install & first-boot plan (high level)

The install is the execution of the repo, not an interactive process. The
repo must be green (CI build passing) before install day. This is the
sequence; the detailed copy-pasteable runbook is [RUNBOOK.md](RUNBOOK.md).

## Phase 1 — before touching the machine

1. Scaffold the repo (separate Claude Code session, from the MacBook):
   skeleton per DECISIONS.md, disko for the Samsung 980 by-id + the three
   ext4 data mounts, and the v0 system target:
   **boots to TTY · NetworkManager (wifi) · sshd · zram · Docker
   (not-on-boot) · CoolerControl · sops-nix wired · user with zsh, git, uv,
   the TUI fleet · flakes enabled.** No desktop in v0.
2. GitHub Actions workflow builds the flake on every push. Milestone:
   green check. Nothing proceeds until it exists. (The first CI run also
   generates and commits flake.lock — the MacBook has no nix and cannot.
   `git pull` after it goes green.)
3. Flash the **graphical NixOS ISO** to a USB stick (from Omarchy). The
   graphical ISO is chosen for NetworkManager/`nmtui` in the live
   environment (wifi-only location) — its guided installer goes unused.

## Phase 2 — install day (order of operations)

1. Confirm the `/mnt/hdd1/parker-backup/` salvage exists (already done:
   ssh, Slippi, coolercontrol, hardware captures).
2. Boot the USB. Open a terminal, ignore the graphical installer.
3. `nmtui` → connect wifi.
4. Clone the repo.
5. Sanity-check the pre-baked hardware.nix:
   `nixos-generate-config --no-filesystems --show-hardware-config` and diff
   against `hosts/parker/hardware.nix`. Fold in anything unexpected
   (initrd modules, firmware) before proceeding.
6. Run disko against the OS disk (partitions + btrfs; data drives untouched).
7. `nixos-install --flake .#parker`
8. Before rebooting: `nixos-enter` into the new system, confirm the user
   exists and `/mnt/hdd1` mounts. Thirty seconds against a surprise reboot.
9. Reboot into parker. Remove USB.

## Phase 3 — first boot (~15 minutes)

1. Auto-login lands on a TTY as the declared user.
2. `nmtui` once → wifi connects; persists and auto-reconnects from now on.
3. Fans: CoolerControl daemon is already running; restore the curves from
   `docs/cooling/coolercontrol-curves.toml` (recipe in its header).
4. Restore from `/mnt/hdd1/parker-backup/`: `~/.ssh` (GitHub key works
   immediately), Slippi dirs when gaming gets built.
5. Clone the repo onto parker. Verify `nixos-rebuild build` works locally.
6. Optional sanity: SSH into parker from the MacBook over LAN.

## Phase 4 — iterate (the actual fun)

Claude Code moves in. The desktop gets built decision by decision from
DECISIONS.md, each step a rebuild away from rollback, roughly:

1. Hyprland + keybinds + spaces (usable desktop)
2. Quickshell shell (Noctalia base) — bar, launcher, notifications
3. Stylix theming + the `theme` command
4. Apps to their spaces (Chrome, Claude Desktop, Spotify...)
5. Gaming layer (Steam, slippi-nix; udev quirks reactively)
6. Utilities polish (screenshots, openers, wiremix/bluetui binds)

Deferred list in DECISIONS.md is the backlog after that.
