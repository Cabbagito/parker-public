# Hyprland session plumbing: the compositor, its portals, and the
# greeter-less path into it. Window rules, binds, and looks live in the home
# layer (home/desktop/); this file only makes the session exist.
{ ... }:
{
  # Wires the package, portals (xdg-desktop-portal-hyprland), XWayland, and —
  # via UWSM — a proper systemd graphical session.
  programs.hyprland = {
    enable = true;
    withUWSM = true;
  };

  # "Auto-login, no greeter" (DECISIONS.md), now via greetd's initial_session
  # instead of getty autologin: power button → Hyprland, no prompt drawn.
  # default_session = same thing, so logout loops straight back in.
  # tty2+ keep normal login prompts as the fallback shell.
  services.greetd = {
    enable = true;
    settings = rec {
      initial_session = {
        user = "peki";
        command = "uwsm start hyprland-uwsm.desktop";
      };
      default_session = initial_session;
    };
  };

  # Electron/Chromium apps pick Wayland at runtime. No-op until they arrive.
  environment.sessionVariables.NIXOS_OZONE_WL = "1";
}
