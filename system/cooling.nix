# CoolerControl daemon from boot so fans are tame before any desktop exists.
# Manages the nct6798 motherboard headers + NZXT Kraken 2023 AIO. Curves are
# daemon state in /etc/coolercontrol/config.toml, not nix: liquid-keyed,
# tuned Oct 2026. Fan map, rationale and test numbers in docs/COOLING.md;
# restorable snapshot in docs/cooling/coolercontrol-curves.toml.
{ pkgs, ... }:
{
  programs.coolercontrol.enable = true;

  # The nct6798 Super-I/O chip needs its driver loaded explicitly (nct6775
  # covers the whole family) — without it no motherboard fan headers exist
  # in hwmon and CoolerControl only sees the Kraken + GPU.
  boot.kernelModules = [ "nct6775" ];

  # Case/fan RGB (PARKER.md, wire-verified July 2026). CoolerControl covers
  # neither controller, so these are direct writers with no daemon conflict:
  # - NZXT 2023 RGB Controller (1e71:2012), 3 colour channels via liquidctl,
  #   e.g. `liquidctl -m "2023 RGB" set led1 color fixed ff4653`
  # - ASRock Polychrome USB (26ce:01a2), the motherboard ARGB, via openrgb
  # Both packages ship uaccess udev rules, so the logged-in user can write
  # to them without root. CLI only, no services — parkerd is the intended
  # eventual caller, not a lighting daemon.
  environment.systemPackages = [
    pkgs.liquidctl
    pkgs.openrgb
  ];
  services.udev.packages = [
    pkgs.liquidctl
    pkgs.openrgb
  ];
}
