# parker — entry point. Composes, never configures.
{ ... }:
{
  imports = [
    ./hardware.nix
    ./disko.nix
    ../../system/core.nix
    ../../system/network.nix
    ../../system/tailscale.nix
    ../../system/users.nix
    ../../system/docker.nix
    ../../system/cooling.nix
    ../../system/sops.nix
    ../../system/nvidia.nix
    ../../system/audio.nix
    ../../system/desktop.nix
    ../../system/misc.nix
    ../../system/searxng.nix
    ../../system/gaming.nix
    ../../system/bluetooth.nix
    ../../system/fonts.nix
    ../../system/theming.nix
  ];

  networking.hostName = "parker";

  # First-install release. Never change this on updates.
  system.stateVersion = "26.05";
}
