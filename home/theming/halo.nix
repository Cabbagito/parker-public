# halo — case lighting follows the theme (research in PARKER.md, July
# 2026). One writer per device, permanently: liquidctl owns the NZXT 2023
# RGB Controller (the three top fan rings), openrgb owns the I2C devices
# (GPU, RAM sticks, mobo/case strip), CoolerControl owns the pump LCD.
# Deliberately a CLI and not a daemon: the resting state is the theme's
# accent, reasserted by the theme-sync timer, and the eventual caller for
# anything livelier is parkerd (a reach verb), not a lighting service.
#
# Persistence differs per device and the timer papers over all of it:
# the RAM sticks store `custom` on-stick (fire-and-forget), the GPU holds
# Direct until power loss (a one-time "Save To Device" in `openrgb --gui`
# makes the accent its power-on default), the fans and mobo are reasserted
# at login/hourly by `theme sync`.
{ pkgs, ... }:
let
  themes = import ../../themes;
  # macchiato has no accent; a neutral grey keeps halo total (every input
  # degrades, never fails — the house rule applies to outputs too).
  # Presets may set `halo` for lighting separately from `accent`: LEDs
  # render brighter and less saturated than pixels, so the case wants a
  # deeper shade than the window borders (redline: lipstick red).
  fallback = "888888";
  lightFor = p: p.halo or (p.accent or fallback);
  dayAccent = lightFor themes.presets.${themes.day};
  nightAccent = lightFor themes.presets.${themes.night};

  halo = pkgs.writeShellScriptBin "halo" ''
    # No -e: lighting must degrade quietly, never break a theme flip.
    set -u
    profile=/nix/var/nix/profiles/system
    day_accent=${dayAccent}
    night_accent=${nightAccent}

    current() {
      if [ "$(readlink -f /run/current-system)" = \
           "$(readlink -f $profile/specialisation/day 2>/dev/null)" ]; then
        echo day
      else
        echo night
      fi
    }

    accent() {
      if [ "$(current)" = day ]; then
        echo "$day_accent"
      else
        echo "$night_accent"
      fi
    }

    # The RAM's two writes are not atomic: `-m custom` lands the mode with
    # no colour behind it, so the sticks sit white for the ~2s the colour
    # process needs to start up and reach the SMBus. On a theme flip that
    # is invisible (the whole case is changing anyway); on the hourly
    # reassert it is a white blink in a red case, which is what it looked
    # like. The sticks store mode *and* colour on-stick, so a reassert of
    # the colour they already hold is pure cost — skip it. The marker
    # lives on tmpfs: a power cycle does reset the sticks, and clearing
    # the marker with it earns them a full re-program on the next sync.
    ram_state="''${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/halo.ram"

    # $1 = RRGGBB. Two invocations, always: `-m custom -c <hex>` in one
    # command silently drops the colour (the sticks end up white; verified
    # July 2026 by eye), and combining them with other devices
    # mis-associates modes across `-d` blocks. Mode first, then colour,
    # each its own process.
    ram() {
      [ "$(cat "$ram_state" 2>/dev/null)" = "$1" ] && return 0
      openrgb --noautoconnect -d Vengeance -m custom \
        >/dev/null 2>&1 || echo "halo: openrgb ram mode failed" >&2
      if openrgb --noautoconnect -d Vengeance -c "$1" >/dev/null 2>&1; then
        printf '%s\n' "$1" >"$ram_state" 2>/dev/null || true
      else
        echo "halo: openrgb ram colour failed" >&2
        # Unknown state is not a state to cache against.
        rm -f "$ram_state" 2>/dev/null || true
      fi
    }

    # $1 = RRGGBB. Fans first (fast HID), then openrgb for the I2C
    # devices. Sequential processes still serialize the SMBus traffic,
    # which is the safety that matters.
    apply() {
      for ch in led1 led2 led3; do
        liquidctl --match "2023 RGB" set "$ch" color fixed "$1" \
          >/dev/null 2>&1 || echo "halo: fans unreachable ($ch)" >&2
      done
      openrgb --noautoconnect \
        -d Suprim -m direct -c "$1" \
        -d B760 -m static -c "$1" \
        >/dev/null 2>&1 || echo "halo: openrgb gpu/mobo failed" >&2
      ram "$1"
    }

    case "''${1:-}" in
      sync)
        apply "$(accent)"
        ;;

      off)
        for ch in led1 led2 led3; do
          liquidctl --match "2023 RGB" set "$ch" color off \
            >/dev/null 2>&1 || true
        done
        openrgb --noautoconnect \
          -d Suprim -m off \
          -d B760 -m off \
          >/dev/null 2>&1 || echo "halo: openrgb off failed" >&2
        # The Corsair sticks have no off mode; stored black is off, which
        # ram() reaches the same way it reaches any other colour.
        ram 000000
        ;;

      status)
        echo "day:   #$day_accent"
        echo "night: #$night_accent"
        echo "now:   $(current) -> #$(accent)"
        ;;

      "")
        echo "usage: halo sync | off | status | <RRGGBB>" >&2
        exit 1
        ;;

      *)
        hex=$(printf '%s' "$1" | tr -d '#')
        case "$hex" in
          [0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F])
            apply "$hex"
            ;;
          *)
            echo "usage: halo sync | off | status | <RRGGBB>" >&2
            exit 1
            ;;
        esac
        ;;
    esac
  '';
in
{
  home.packages = [ halo ];
}
