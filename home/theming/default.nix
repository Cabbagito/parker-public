# Theming, home side. The engine is system/theming.nix (stylix propagates
# into HM automatically); this file owns the `theme` command, the day/night
# flip timer, and records the connection map:
#
#   auto-themed  ghostty (colors+font), hyprland (border/shadow colors),
#                noctalia (custom palette via its stylix target — replaces
#                the old builtin Catppuccin; the wallpaper needs the IPC
#                push below — noctalia's writable state shadows the
#                config.toml default), btop, lazygit, yazi, zellij
#                (theme file named `default`, auto-picked), fzf, bat, gdu, gtk/qt,
#                TTY console, cursor, spotify-player (stylix target),
#                cava (no palette target — home/tuis/music.nix builds its
#                gradient from the preset border colors)
#   dark-mode only  chrome/discord/spotify: no stylix targets (no palette),
#                but they follow the freedesktop color-scheme preference,
#                which stylix's gnome target flips per variant
#                (prefer-dark on dark polarity, default on light)
#   not reachable  nvim (lazy.nvim manages its own colorscheme — per-preset
#                  alignment is a future step), p10k/fastfetch (static
#                  colors)
#   hardware     case lighting (fans/GPU/RAM/mobo) via `halo sync`
#                (halo.nix) — called on every flip and reasserted by the
#                sync timer; the pump LCD stays CoolerControl's
#
# Day/night model: themes/default.nix names a day and a night preset;
# system/theming.nix pre-builds both (base system = night, `day`
# specialisation = day), so a polarity flip is a seconds-long activation.
# `theme sync` (run by the timer at the schedule boundaries, at login, and
# hourly as an idempotent safety net) keeps the live polarity matched to
# the schedule — or to the manual override `theme light/dark` writes;
# `theme auto` clears the override. The hourly net also self-heals the one
# known wrinkle: a plain `nixos-rebuild switch` always lands in the base
# (night) variant, whatever the clock says.
{ pkgs, lib, osConfig, ... }:
let
  themes = import ../../themes;
  preset = themes.forPolarity osConfig.parker.theme.polarity;

  # Schedule boundaries baked into the script as HHMM integers; `theme set`
  # rebuilds, so they track themes/default.nix.
  dayAt = lib.replaceStrings [ ":" ] [ "" ] themes.schedule.day;
  nightAt = lib.replaceStrings [ ":" ] [ "" ] themes.schedule.night;

  # `theme list` / `theme status` / `theme set <family|preset>` /
  # `theme transparency [v]` / `theme light|dark|auto|sync` — see the case
  # arms below. The repo's
  # permanent home is ~/parker (docs/RUNBOOK.md); override with PARKER_REPO
  # if it ever moves.
  theme = pkgs.writeShellScriptBin "theme" ''
    set -euo pipefail
    repo="''${PARKER_REPO:-$HOME/parker}"
    file="$repo/themes/default.nix"
    state_dir="''${XDG_STATE_HOME:-$HOME/.local/state}/theme"
    override="$state_dir/override"
    profile=/nix/var/nix/profiles/system

    # Baked from themes/default.nix at build time.
    day_name=${themes.day}
    night_name=${themes.night}

    current() {
      if [ "$(readlink -f /run/current-system)" = \
           "$(readlink -f $profile/specialisation/day 2>/dev/null)" ]; then
        echo day
      else
        echo night
      fi
    }

    scheduled() {
      if [ "$((10#$(date +%H%M)))" -ge $((10#${dayAt})) ] \
          && [ "$((10#$(date +%H%M)))" -lt $((10#${nightAt})) ]; then
        echo day
      else
        echo night
      fi
    }

    target() {
      if [ -f "$override" ]; then cat "$override"; else scheduled; fi
    }

    push() {
      # Noctalia persists its own wallpaper in ~/.local/state/noctalia/
      # settings.toml, which shadows the stylix-managed config.toml
      # default — so push the activated preset's wallpaper over IPC (also
      # rewrites that state). Retry briefly: activation restarts noctalia
      # and the socket takes a moment. Presets without a wallpaper
      # (macchiato) skip this and keep the last one.
      export WAYLAND_DISPLAY="''${WAYLAND_DISPLAY:-wayland-1}"
      wp=$(sed -n '/^\[wallpaper.default\]/,/^\[/s/^path = "\(.*\)"/\1/p' \
        "$HOME/.config/noctalia/config.toml" 2>/dev/null | head -1)
      if [ -n "$wp" ]; then
        for _ in 1 2 3 4 5 6 7 8 9 10; do
          if noctalia msg wallpaper-set "$wp" 2>/dev/null; then
            break
          fi
          sleep 1
        done
      fi
      # Open ghostty windows re-read their config (incl. the stylix
      # theme file) on SIGUSR2 — no relaunch needed. TUIs already
      # running inside (btop, zellij's own chrome) still show old
      # colors until restarted; new panes are fine. Match on the
      # cmdline: the nix wrapper's comm is `.ghostty-wrapped`, so
      # `pkill -x ghostty` never fires.
      pkill -USR2 -f 'bin/ghostty' 2>/dev/null || true
    }

    # Activate the pre-built variant for $1 (day|night), then push. `test`
    # (not `switch`) keeps night the boot default; sync at login corrects.
    #
    # The `timeout` is not belt-and-braces, it is the whole point (July
    # 2026, QUIRKS.md "theme flip wedges the rebuild lock"). Activation
    # holds /run/nixos/switch-to-configuration.lock for its whole run, and
    # switch-to-configuration can block forever inside it: when the flip
    # has to reload the *user* dbus-broker, it waits for a JobRemoved
    # signal delivered over the very bus it just reloaded, which never
    # arrives. An hour of that is an hour where `nrs` dies with exit 11.
    # timeout runs as root (inside sudo, so it can actually signal the
    # root child — sudo does not forward signals from outside) and the
    # kernel drops the flock when it dies. 120s is ~4x a normal flip.
    apply() {
      rc=0
      case "$1" in
        day) sudo /run/current-system/sw/bin/timeout 120 \
               "$profile/specialisation/day/bin/switch-to-configuration" test || rc=$? ;;
        night) sudo /run/current-system/sw/bin/timeout 120 \
                 "$profile/bin/switch-to-configuration" test || rc=$? ;;
      esac
      case "$rc" in
        0) ;;
        124) echo "theme: activation timed out after 120s, killed (rebuild lock released)" >&2 ;;
        11) echo "theme: another rebuild holds the lock — flip skipped, next sync retries" >&2 ;;
        *) echo "theme: activation failed (exit $rc)" >&2 ;;
      esac
      # Cosmetics run either way: a partial activation still wants the
      # wallpaper and the LEDs pointed at the right preset.
      push
      halo sync || true
      return "$rc"
    }

    case "''${1:-}" in
      list)
        echo "families (day/night):"
        ${lib.concatStringsSep "\n        " (lib.mapAttrsToList
          (f: p: ''echo "  ${f} = ${p.day} / ${p.night}"'') themes.families)}
        echo "presets:"
        for d in "$repo"/themes/*/; do
          echo "  $(basename "$d")"
        done
        ;;

      status)
        echo "day:   $day_name"
        echo "night: $night_name"
        echo "now:   $(current)"
        if [ "$day_name" = "$night_name" ]; then
          echo "mode:  pinned (single preset, schedule idle)"
        elif [ -f "$override" ]; then
          echo "mode:  manual $(cat "$override") — 'theme auto' resumes the schedule"
        else
          echo "mode:  scheduled (day ${themes.schedule.day}, night ${themes.schedule.night})"
        fi
        ;;

      set)
        name="''${2:?usage: theme set <family|preset>}"
        case "$name" in
          ${lib.concatStringsSep "\n          " (lib.mapAttrsToList
            (f: p: "${f}) d=${p.day}; n=${p.night} ;;") themes.families)}
          *)
            if [ -d "$repo/themes/$name" ]; then
              d=$name; n=$name
            else
              echo "no family or preset '$name' — see: theme list" >&2
              exit 1
            fi
            ;;
        esac
        # The two-space indent anchors these to the top-level pair — the
        # schedule line nests its day/night deeper (see themes/default.nix).
        sed -i "s/^  day = \"[^\"]*\"/  day = \"$d\"/; s/^  night = \"[^\"]*\"/  night = \"$n\"/" "$file"
        # A fresh set means the schedule is live again.
        rm -f "$override"
        # Land directly in the right polarity: base is night, so ask for the
        # day specialisation when the clock says day. (~30s, DECISIONS.md)
        spec=""
        if [ "$d" != "$n" ] && [ "$(scheduled)" = day ]; then
          spec="--specialisation day"
        fi
        sudo nixos-rebuild switch --flake "$repo#parker" $spec
        push
        halo sync || true
        ;;

      transparency)
        v="''${2:-}"
        if [ -z "$v" ]; then
          sed -n 's/^  transparency = \([0-9.]*\);.*/\1/p' "$file"
          exit 0
        fi
        if ! awk -v v="$v" 'BEGIN { exit !(v >= 0.5 && v <= 1) }'; then
          echo "transparency wants 0.5..1 (opacity of unfocused windows; 1 = solid)" >&2
          exit 1
        fi
        sed -i "s/^  transparency = [0-9.]*;/  transparency = $v;/" "$file"
        # Instant feedback on the live session; the rebuild bakes the same
        # values (plus ghostty's background) so flips and reboots keep them.
        a=$(awk -v v="$v" 'BEGIN { a = v + 0.12; if (a > 1) a = 1; printf "%.2f", a }')
        # (Lua config: `hyprctl keyword` is refused, eval is the live path.)
        hyprctl eval "hl.config({ decoration = { inactive_opacity = $v, active_opacity = $a } })" >/dev/null 2>&1 || true
        # Stay in the polarity we're already showing (switch alone would
        # land in base/night).
        spec=""
        if [ "$(current)" = day ]; then spec="--specialisation day"; fi
        sudo nixos-rebuild switch --flake "$repo#parker" $spec
        pkill -USR2 -f 'bin/ghostty' 2>/dev/null || true
        ;;

      light|dark)
        if [ "$day_name" = "$night_name" ]; then
          echo "pinned to '$day_name' — polarity is fixed; 'theme set <family>' first" >&2
          exit 1
        fi
        p=night; [ "$1" = light ] && p=day
        mkdir -p "$state_dir"
        printf '%s\n' "$p" > "$override"
        apply "$p"
        ;;

      auto)
        rm -f "$override"
        ;&

      sync)
        # Pinned pair: one variant exists, nothing to flip — but lighting
        # still wants its login/hourly reassert (fans forget power loss).
        if [ ! -e "$profile/specialisation/day" ]; then
          halo sync || true
          exit 0
        fi
        t=$(target)
        # apply() glows after a flip; the no-flip path reasserts here.
        if [ "$(current)" != "$t" ]; then apply "$t"; else halo sync || true; fi
        ;;

      *)
        echo "usage: theme list | status | set <family|preset> | transparency [0.5..1] | light | dark | auto | sync" >&2
        exit 1
        ;;
    esac
  '';
in
{
  imports = [ ./halo.nix ];

  home.packages = [ theme ];

  # The flip timer: schedule boundaries + hourly safety net, and once at
  # login (covers boots/resumes landing inside either period).
  systemd.user.services.theme-sync = {
    Unit.Description = "Sync theme polarity to schedule/override";
    Service = {
      Type = "oneshot";
      ExecStart = "${theme}/bin/theme sync";
      # systemd defaults Type=oneshot to TimeoutStartSec=infinity, which is
      # how a single wedged flip sat here for 65 minutes holding the rebuild
      # lock. Outer bound only — apply()'s own 120s timeout should always
      # fire first; if this one trips, something upstream of it hung.
      TimeoutStartSec = 180;
      # sudo is the setuid wrapper; the rest mirrors a login shell enough
      # for the script (nixos-rebuild isn't on this path — sync never
      # rebuilds).
      Environment = "PATH=/run/wrappers/bin:/etc/profiles/per-user/%u/bin:/run/current-system/sw/bin";
    };
    Install.WantedBy = [ "default.target" ];
  };
  systemd.user.timers.theme-sync = {
    Unit.Description = "Theme polarity flips at the schedule boundaries";
    Timer = {
      OnCalendar = [
        "*-*-* ${themes.schedule.day}:00"
        "*-*-* ${themes.schedule.night}:00"
        "*-*-* *:15:00"
      ];
      Persistent = true;
    };
    Install.WantedBy = [ "timers.target" ];
  };
}
