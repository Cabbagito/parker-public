# Chrome — space 1 citizen. Native Wayland via the global NIXOS_OZONE_WL.
# Known rough edge on Hyprland: tab drag-out (Chromium ozone bug, not ours).
{ ... }:
{
  # No flags. Until Oct 2026 this carried
  # --disable-features=WaylandWpColorManagerV1 against Hyprland 0.55's
  # reload killing Chromium (QUIRKS.md); re-tested on 0.56.2 without it and
  # Chrome survives a reload.
  programs.google-chrome.enable = true;
}
