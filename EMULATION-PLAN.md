# Emulation plan — PS2 + PS3 via God of War (2005) and Uncharted: Drake's Fortune

Goal: stand up the PS2 path (PCSX2) and the PS3 path (RPCS3) with one game
each, so the rest of both catalogues (GoW II/III, Uncharted 2/3, Demon's
Souls…) is just "add a disc image" later.

Researched 2026-08-02. Versions and compat statuses below are as of that date.

## Decision: nixpkgs for both, no flatpak

- **PCSX2**: nixpkgs unstable has 2.6.3, which *is* the current upstream
  stable (2026-01-28). Nothing to gain from flatpak.
- **RPCS3**: nixpkgs unstable has 0.0.40-unstable-2026-04-25; upstream is at
  ~0.0.41/0.0.42. Nixpkgs tracks it every 1–2 months. That's fresh enough for
  Drake's Fortune (its compat status predates 0.0.40). The reference doc's
  "flatpak for fast movers" advice is real, but this repo has zero flatpak
  and the philosophy is nixpkgs-native — adopt flatpak *reactively* if a
  specific game turns out to need a fix nixpkgs hasn't shipped, not
  preemptively. (Escape hatches, in order: monthly `nix flake update` ritual
  already bumps it; `nix run nixpkgs#appimage-run -- RPCS3.AppImage` for a
  one-off test of a newer build; flatpak as the durable fallback.)
- Both packages are free software — the global `allowUnfree` in
  `system/core.nix` is irrelevant here, nothing to gate.

## Repo changes (small, one commit)

1. **`home/apps/emulation.nix`** — new leaf, the "app family" form that
   `home/apps/default.nix` explicitly permits:

   ```nix
   # Emulation — PS2 (PCSX2) and PS3 (RPCS3). Both keep mutable state in
   # ~/.config (BIOS, firmware, per-game settings) by design; the configs
   # are runtime-owned, not repo-owned. Game images and BIOS dumps live on
   # ~/gaming/emulation, never in this repo.
   { pkgs, ... }:
   {
     home.packages = with pkgs; [ pcsx2 rpcs3 ];
   }
   ```

2. Add `./emulation.nix` to the import list in `home/apps/default.nix`.
3. Update the file enumeration in `home/AGENTS.md` (repo rule: per-directory
   AGENTS.md must track contents).

Nothing else:

- **No `system/gaming.nix` change.** Both emulators are plain user packages.
- **No controller udev work.** `programs.steam.enable` already enables
  `hardware.steam-hardware`, whose udev rules cover DS3/DS4/DualSense — the
  exact fix for the known RPCS3-on-NixOS controller issue (nixpkgs #326155).
  If a pad still misbehaves, that's a QUIRKS.md entry, not preconfig.
- **No gamescope/gamemode.** DECISIONS.md: gaming is a supported citizen,
  not special; no preemptive scaffolding.
- **No Hyprland windowrules.** Games are normal windows per DECISIONS.md.
- **32-bit GL** already on in `system/nvidia.nix`; Vulkan ICD comes with
  `hardware.graphics.enable` — RPCS3's Vulkan renderer needs nothing extra.
- **No BIOS/firmware in the repo.** The private-repo plaintext rule covers
  rotatable credentials, not copyrighted blobs. They live in `~/gaming/emulation`.

Config-ownership note: PCSX2 and RPCS3 write `~/.config/PCSX2/` and
`~/.config/rpcs3/` themselves. That's a deliberate exception to "all configs
compiled" — same acceptance as btop's grumbling, but bigger: BIOS, firmware,
shader caches and per-game settings are runtime state that can't sensibly be
declared. Documented here so it doesn't get "fixed" later.

## On-parker steps (manual, after `nixos-rebuild switch`)

### Files you must source yourself (dump your own console/discs)

| What | Where it goes |
|---|---|
| PS2 BIOS dump (`*.rom0` minimum, lowercase extensions; `.rom1/.rom2/.erom/.nvm/.mec` if dumped) | `~/.config/PCSX2/bios/` |
| God of War (2005) disc image (ISO/CHD; CHD preferred, smaller) | e.g. `~/gaming/archive/discs/ps2/` |
| Uncharted: Drake's Fortune disc dump | e.g. `~/gaming/archive/discs/ps3/` |
| PS3 firmware `PS3UPDAT.PUP` — the one thing Sony distributes freely; current 4.93 (2026-03-18), from playstation.com PS3 system-software page | anywhere; consumed by RPCS3 install |

### PCSX2 + God of War (do this first — lowest friction, validates BIOS)

1. Launch `pcsx2-qt` (runs under XWayland by design — upstream disabled
   native Wayland; do not fight this).
2. First-run wizard: point BIOS dir at the dump, add the games dir.
3. Play. On 2.6.3 the old GoW-specific EE clamping/rounding fixes are
   auto-applied from the GameDB — change nothing preemptively. Upscale
   (4x–6x is trivial for the 3090); only if bloom/sprite misalignment
   appears: Graphics → Manual Hardware Renderer Fixes → Round Sprite →
   Half. Known-fixed-in-2.x: muted cutscene audio, underwater vertical
   lines — if seen, you're somehow on an old build.

### RPCS3 + Uncharted: Drake's Fortune

1. Launch `rpcs3`, File → Install Firmware → the 4.93 `.PUP`; wait for the
   module compile.
2. Add the game dir. Apply game update 01.01 (in-emulator or via RPCS3's
   compat tooling).
3. Settings — defaults are mostly right on 0.0.40: PPU/SPU Decoder LLVM,
   SPU Threads Auto, Renderer Vulkan. Two deliberate changes:
   - **GPU → Advanced → Write Color Buffers: ON** — without it, white
     screen at Chapter 4.
   - **Manage Game Patches** — enable the official patches for the title.
4. Resolution scale: start 100%, raise stepwise; scaling amplifies RSX cost
   and can expose artifacts in this title.
5. Expectations: status is **Ingame**, not Playable — finishable
   end-to-end, but with shader-compile stutter (first run), drops in heavy
   scenes, some texture flicker. That's the known ceiling, not a config
   error.
6. **Do not pin P-cores preemptively.** Default "Thread Scheduler: Operating
   System" handles the 14700's hybrid topology fine per current reports.
   `taskset -c 0-15 rpcs3` is the reactive move if perf is inexplicably bad
   → QUIRKS.md entry.

### Verify

- PCSX2: GoW intro (Aegean Sea) at target upscale, cutscene audio intact.
- RPCS3: Uncharted reaches gameplay; revisit at Chapter 4 to confirm the
  Write Color Buffers fix held.

## Future paths this opens

- PS2: GoW II, Shadow of the Colossus, SH2/3, MGS3 — same BIOS, just images.
- PS3: Uncharted 2/3 (Ingame, tunable), GoW III (runs well), Demon's Souls
  (Playable) — same firmware install.
- PSP gap-filler (Chains of Olympus): `ppsspp-sdl-wayland`, one more package
  in `emulation.nix`, no BIOS needed.
- If RPCS3 staleness ever actually bites: that's the moment to consider
  flatpak, as its own DECISIONS.md entry.

## Open items for the human

- ~~Confirm storage location~~ — resolved: `~/gaming/emulation/{ps2,ps3,bios}` on the NVMe, discs on the HDD archive (Sept 2026).
- ~~Source the files~~ — resolved: BIOS in `~/.config/PCSX2/bios/` (+ keeper
  copy), firmware + games in `~/gaming/emulation`. The PS3 test title ended up being
  **Red Dead Redemption GOTY (BLUS30758)**, not Uncharted — extracted to
  folder format, decryption verified. The Uncharted notes above still apply
  when that disc arrives; RDR is Playable but one of the heaviest PS3 titles.
- The RPCS3/PCSX2 wikis are Cloudflare-gated to bots; per-game settings
  above came via secondary sources — worth eyeballing
  wiki.rpcs3.net/index.php?title=Uncharted:_Drake%27s_Fortune and
  wiki.pcsx2.net/God_of_War in a browser before deep-diving settings.
