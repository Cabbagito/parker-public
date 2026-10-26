# Emulation — PS2 (PCSX2) and PS3 (RPCS3). Both keep mutable state in
# ~/.config (BIOS, firmware, per-game settings, shader caches) by design;
# those configs are runtime-owned, not repo-owned — the btop acceptance,
# writ large. BIOS dumps, PS3 firmware and extracted PS3 games live in
# ~/gaming/emulation/{bios,ps2,ps3}; disc images sit on the HDD archive
# (~/gaming/archive/discs) and are symlinked in. Never in this repo.
{ pkgs, ... }:
{
  home.packages = with pkgs; [
    pcsx2
    rpcs3
  ];
}
