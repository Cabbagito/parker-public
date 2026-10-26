# AGENTS.md — system/

System layer, one concern per file, each self-contained and deletable.
Composed only by `hosts/parker/default.nix` — nothing in here imports
siblings. Growth pattern: a new domain gets a new file (e.g. `nvidia.nix`,
`audio.nix`, `gaming.nix` in Phase 4); three-liners without a home
accumulate in a `misc.nix` until they earn one.

Current: `core.nix` (boot, nix, locale, zram, oomd slices, generation
pruning + store optimise, nix-ld) · `network.nix`
(NetworkManager + sshd) · `tailscale.nix` (tailscaled, one interactive
`tailscale up`, and `tailscale serve` publishing the porthole) · `users.nix` (peki) · `docker.nix` (not-on-boot) ·
`cooling.nix` (CoolerControl; curves are daemon state, see
docs/COOLING.md) · `sops.nix` (wired, zero secrets, never
bootstrapped — recipe inside; the repo is private, so plaintext is an option
too, see DECISIONS.md) · `nvidia.nix` (3090, open module, libcuda on nix-ld's path for uv's CUDA wheels; otherwise deliberately bare) ·
`audio.nix` (PipeWire) · `desktop.nix` (Hyprland + UWSM, greetd autologin,
Ozone) · `theming.nix` (Stylix engine pointed at the themes/ day/night pair; base
system = night preset, `day` specialisation = day, flipped by the `theme`
command in home/theming) · `searxng.nix` (parker's own web search —
metasearch on 127.0.0.1:8888, JSON API for `parker search`) ·
`gaming.nix` (Steam + GE-Proton, gamemode, Slippi, DualSense touchpad
hidden from libinput) · `bluetooth.nix` (BlueZ for the DualSense) ·
`fonts.nix` (Meslo NF, Inter, JetBrains Mono, symbols, Noto) · `misc.nix`
(empty — the three-liner drawer).
