# Discord — space 4 citizen (summonable, not autostarted furniture).
# Native Wayland via the global NIXOS_OZONE_WL.
{ pkgs, ... }:
{
  home.packages = with pkgs; [ discord ];
}
