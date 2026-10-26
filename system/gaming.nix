# Gaming layer, growing decision by decision (DECISIONS.md: supported
# citizen, not special). Melee via slippi-nix; its default module carries the
# GameCube adapter (WUP-028) support — verify the udev rule landed, QUIRKS
# has the vendor/product if it didn't.
{
  inputs,
  config,
  lib,
  pkgs,
  ...
}:
let
  kernel = config.boot.kernelPackages.kernel;
in
{
  imports = [ inputs.slippi.nixosModules.default ];

  # The adapter's overclock module, swapped for a patched copy. nixpkgs
  # (2026-10) signs kernel modules, and the modules aggregate now wants a .ko.cer
  # beside every out-of-tree module; gcadapter-oc-kmod copies its .ko by hand and
  # ships none, which fails the whole system build (NixOS/nixpkgs#575192).
  # Installing through modules_install signs it, same as NixOS/nixpkgs#576926.
  # Drop this block, mkForce included, once that lands.
  gamecube-controller-adapter.overclocking-kernel-module.enable = lib.mkForce false;
  boot.extraModulePackages = [
    (config.boot.kernelPackages.gcadapter-oc-kmod.overrideAttrs (old: {
      makeFlags = old.makeFlags ++ [
        "INSTALL_MOD_DIR=extra"
        "INSTALL_MOD_STRIP=1"
        "DEPMOD=true"
      ];
      installPhase = null;
      installTargets = [ "modules_install" ];
      installFlags = [
        "-C"
        "${kernel.dev}/lib/modules/${kernel.modDirVersion}/build"
        "M=$(PWD)"
      ];
    }))
  ];
  boot.kernelModules = [ "gcadapter_oc" ];

  programs.steam = {
    enable = true;
    extraCompatPackages = [ pkgs.proton-ge-bin ];
  };

  # The daemon behind the per-game GameMode toggle (Steam launch option,
  # Heroic checkbox). Only ever engaged per title — no global "gaming mode".
  programs.gamemode.enable = true;

  # DualSense: hid-playstation splits the pad into three evdev nodes —
  # gamepad, touchpad, motion. libinput treats the touchpad node as a real
  # touchpad and the pad turns into a mouse on the desktop. Hide it from
  # libinput (Hyprland) only; games read the pad over hidraw (SDL/Proton,
  # uaccess from Steam's udev rules), where the touchpad click is a plain
  # button. The kernel names the pad by what it reports about itself, which
  # is just "Wireless Controller" (USB adds a "Sony Interactive Entertainment "
  # prefix, BT does not) — "DualSense" never appears, so match on Sony's
  # vendor id + the Touchpad suffix; that covers DS4 and Edge too.
  services.udev.extraRules = ''
    ACTION=="add|change", SUBSYSTEM=="input", KERNEL=="event*", ATTRS{id/vendor}=="054c", ATTRS{name}=="*Controller Touchpad", ENV{LIBINPUT_IGNORE_DEVICE}="1"
  '';
}
