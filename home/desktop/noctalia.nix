# Noctalia v5 (native rewrite, beta) — launcher, notifications, OSD, lock
# screen, wallpaper, dock. Not the bar: it is off, and the controls live in
# visage on ^, (docs/visage/controls/README.md, DECISIONS.md). Pinned via
# flake.lock; update deliberately (beta churn is accepted, DECISIONS.md).
#
# Settings here are the declarative layer (~/.config/noctalia, read-only).
# The GUI writes to ~/.local/state/noctalia/settings.toml which loads LAST
# and silently wins — if a shell setting ever seems stuck, prune that file.
# Schema notes (v5 beta): bars are NAMED sections (bar.main) — flat [bar]
# keys are silently ignored; per-widget config lives under widget.<name>.
{ inputs, pkgs, lib, ... }:
{
  imports = [ inputs.noctalia.homeModules.default ];

  # home-manager (2026-08) grew its own programs.noctalia module, which
  # collides with the flake's home-module above. The flake module matches our
  # pinned noctalia rev (and our patches), so it stays the owner. (HM moved
  # it to a directory in 2026-10; the path must track that.)
  disabledModules = [ "programs/noctalia" ];

  programs.noctalia = {
    enable = true;
    # UWSM session → systemd user service is the clean autostart.
    systemd.enable = true;

    # Spotlight launcher is a parker patch (noctalia-spotlight.patch): a new
    # shell.launcher.spotlight flag — empty query shows a bare input strip,
    # the card grows with results. Upstream declined for v4 (#1902) and v5
    # has no equivalent; re-check and re-diff on every pin bump. Building
    # from source instead of the cachix binary is the accepted cost.
    #
    # noctalia-bar-blur.patch is the second: noctalia asks the compositor to
    # blur the bar's whole background rect whenever the bar is on screen,
    # never consulting background_opacity. That was free for as long as
    # Hyprland ignored the request; 0.56 implements
    # ext-background-effect-v1 and the ghost bar below (opacity 0.0) grew a
    # frosted strip across the top of the wallpaper. The patch skips the
    # blur region when the background is fully transparent. Drop it if
    # upstream gains a bar blur toggle.
    #
    # noctalia-dock-exclude.patch is the third: show_running is all-or-nothing,
    # so parker's always-on face windows earned dock icons for something the
    # user never launches, focuses or quits. The patch adds a `dock.exclude`
    # app-id list that show_running skips (matched against the compositor app
    # id and the desktop entry it resolves to). Drop it if upstream grows a
    # per-app dock filter.
    package =
      inputs.noctalia.packages.${pkgs.stdenv.hostPlatform.system}.default.overrideAttrs
        (old: {
          patches = (old.patches or [ ]) ++ [
            ./noctalia-spotlight.patch
            ./noctalia-bar-blur.patch
            ./noctalia-dock-exclude.patch
          ];
        });

    settings = {
      # The bar is off (Oct 2026): visage is the control surface — SUPER+P
      # over anything, ^, for the settings, the corner for the hour and the
      # space you are on (docs/visage/controls/README.md). Kept whole and
      # one line away from coming back: the ghost bar it was — fully
      # transparent, widgets floating on the wallpaper, workspace hint left,
      # clock center, controls right, no tray.
      bar.main = {
        enabled = false;
        position = "top";
        auto_hide = false;
        show_on_workspace_switch = false;
        reserve_space = false;
        background_opacity = 0.0;
        shadow = false;
        border_width = 0.0;
        scale = 1.15;
        start = [ "workspaces" ];
        center = [ "clock" ];
        end = [
          "network"
          "volume"
          "notifications"
          "control-center"
          "session"
        ];
      };

      # Bare pager: dots + a wide highlighted strip on the active space,
      # no labels at all. Must be style "regular" — minimal/focus_hint
      # coerce display "none" back to Id and numbers reappear.
      widget.workspaces = {
        display = "none";
        style = "regular";
        hide_when_empty = false;
      };

      # macOS-style dock: hidden until the pointer hits the bottom edge,
      # magnifying icons, running dots. Pinned set = the daily citizens;
      # running-but-unpinned apps append after them. Trailing grid-dots
      # button opens the launcher. Runtime extras for free: drag to
      # reorder pins, right-click for window list / jump-list / close.
      # mkForce: the upstream HM module ships its own dock defaults at
      # normal priority; without force every key we set here conflicts.
      dock = lib.mkForce {
        enabled = true;
        auto_hide = true;
        position = "bottom";
        magnification = true;
        # Zoom easing (kHoverZoomLerp) is compiled into noctalia — the
        # smoothness levers we have are base size, zoom ratio, spacing.
        magnification_scale = 1.7;
        icon_size = 64;
        item_spacing = 12;
        show_dots = true;
        show_instance_count = false;
        show_running = true;
        reserve_space = false;
        background_opacity = 0.75;
        radius = 20;
        margin_edge = 6;
        launcher_position = "none";
        # Core four only — anything else (discord etc.) appends itself
        # while running via show_running.
        pinned = [
          "google-chrome"
          "com.anthropic.Claude"
          "com.mitchellh.ghostty"
          "spotify"
        ];
        # ...except parker's own faces. Both are always up and live on special
        # workspaces summoned by key (hyprland.nix); a dock icon offers to
        # launch or quit something that is neither launched nor quit. Needs
        # noctalia-dock-exclude.patch above.
        exclude = [
          "visage"
          "com.parker.face"
        ];
      };

      # Slightly gentler global animations (dock hover felt twitchy).
      shell.animation.speed = 0.9;

      # Track-change OSD off — every skip on the desk popped a card. The
      # other OSD kinds (volume, brightness, wifi, etc.) stay on; this is
      # the one per-kind gate under osd.kinds.
      osd.kinds.media = false;

      # Launch apps as transient systemd user units (systemd-run --user,
      # app.slice, ExitType=cgroup) instead of children of noctalia.service.
      # Without this, KillMode=control-group takes every launched app down
      # whenever the shell restarts (nrs, noctalia crash/relaunch). Applies
      # to launcher, dock and taskbar alike; requires the shell itself to
      # run under the user manager, which systemd.enable above guarantees.
      shell.launch_apps_as_systemd_services = true;

      shell.launcher = {
        sort_by_usage = true;
        # No category filter buttons, tighter rows. app_grid = true is the
        # Launchpad-style alternative (spotlight ignores it).
        categories = false;
        compact = true;
        # Bare blurred strip until typing starts (parker patch, see above).
        spotlight = true;
      };

      # Frosted panels: glass keeps the card translucent; the blur behind it
      # is the compositor's. Hyprland 0.56 does implement noctalia's native
      # blur protocol (ext-background-effect-v1), so panels no longer need
      # the layerrule in hyprland.nix — it is kept as the belt to that
      # protocol's braces, and costs nothing when both agree.
      shell.panel.transparency_mode = "glass";

      # No theme block here: stylix's noctalia target owns it (source =
      # "custom" + a full palette from the active preset's base16 scheme,
      # mode from polarity). Was builtin Catppuccin pre-stylix.
    };
  };
}
