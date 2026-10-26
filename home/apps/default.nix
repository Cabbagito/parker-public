# App role — composes one file per app (or app family).
{ ... }:
{
  imports = [
    ./claude-code.nix
    ./claude-desktop.nix
    ./chrome.nix
    ./spotify.nix
    ./discord.nix
    ./slippi.nix
    ./emulation.nix
    ./gaming.nix
    ./qbittorrent.nix
  ];
}
