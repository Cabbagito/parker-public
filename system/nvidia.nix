# RTX 3090 via the NixOS nvidia module. Deliberately bare beyond this: the
# old Arch env-var recipe (QUIRKS.md) is obsolete in the explicit-sync era —
# some of those vars now cause harm. Add knobs reactively, on symptom only.
{ config, ... }:
{
  hardware.graphics.enable = true;
  # 32-bit GL for the gaming layer later; harmless now.
  hardware.graphics.enable32Bit = true;

  # Despite the name this just loads/wires the driver — no X server runs.
  services.xserver.videoDrivers = [ "nvidia" ];

  hardware.nvidia = {
    # Open kernel module — NVIDIA's recommended path for Ampere.
    open = true;
    # KMS; required for Wayland.
    modesetting.enable = true;
    # Preserve VRAM across suspend/resume (manual-sleep decision).
    powerManagement.enable = true;
    package = config.boot.kernelPackages.nvidiaPackages.production;
  };

  # Symptom: uv's torch wheel saw no GPU ("Found no NVIDIA driver") — libcuda
  # lives in /run/opengl-driver/lib, off the search path of nix-ld programs.
  # Putting the driver on nix-ld's path (core.nix) fixes it for every uv
  # project; same package as above, so it always matches the loaded module.
  # Appends to nix-ld's base library list, doesn't replace it.
  programs.nix-ld.libraries = [ config.hardware.nvidia.package ];
}
