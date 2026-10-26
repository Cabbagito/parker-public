# Hyprland: monitor, input, looks, window rules, the Super-owned keybind map
# (DECISIONS.md: Super = OS, Alt = terminal-land) and the login startup.
# Lua config since Oct 2026; colours come from the Stylix target.
{ lib, osConfig, ... }:
let
  themes = import ../../themes;
  # Border colors follow the polarity this system variant was built with
  # (base = night, `day` specialisation = day; system/theming.nix).
  preset = themes.forPolarity osConfig.parker.theme.polarity;
in
{
  wayland.windowManager.hyprland = {
    enable = true;
    # UWSM owns the session (system/desktop.nix) — HM must not also manage it.
    systemd.enable = false;

    # Lua config: 0.56 nags on every launch that .conf support is removed in
    # 0.57. Data — config values, monitor, rules — stays structured under
    # `settings`: each attribute renders as one `hl.<name>(...)` call (lists
    # as one call per element), and the stylix target merges its colors into
    # `settings.config`. Binds and startup are plain Lua in extraConfig below,
    # where HM's `_args`/mkLuaInline form would bury them in escaping. (The
    # earlier blocker, home-manager#9468, was its generator passing hyprlang
    # strings through raw — not a problem once the settings are Lua-shaped.)
    configType = "lua";

    settings = {
      # AOC Q27G42XNE — 180Hz declared permanent (DECISIONS.md).
      monitor = {
        output = "DP-3";
        mode = "2560x1440@180";
        position = "auto";
        scale = "1";
      };

      config = {
        input = {
          kb_options = "ctrl:nocaps";
          follow_mouse = 1;
          # Snappier key repeat (default delay 600 feels sluggish).
          repeat_delay = 250;
          repeat_rate = 35;
        };

        general = {
          gaps_in = 10;
          # Even all round: there is no bar to clear (noctalia.nix).
          gaps_out = {
            top = 24;
            right = 24;
            bottom = 24;
            left = 24;
          };
          # Slight outline, previewed in themes/theme-lab.html. Presets
          # carrying the lab's tuned border colors override the stylix
          # hyprland target's generic picks (base0D/base03); presets without
          # them (macchiato) keep the stylix defaults.
          border_size = 2;
          layout = "dwindle";
        }
        // lib.optionalAttrs (preset ? accent) {
          "col.active_border" = lib.mkForce "rgb(${preset.accent})";
          "col.inactive_border" = lib.mkForce "rgb(${preset.borderInactive})";
        };

        # Predictable splits: new tiles always open right/bottom, never on
        # whichever side the cursor happens to be.
        dwindle = {
          force_split = 2;
        };

        decoration = {
          rounding = 12;
          # One outline only — the preset border above. Stylix colors the
          # drop shadow from the scheme (near-white on light themes), which
          # reads as a second border around the accent one.
          shadow.enabled = false;

          # Glass, driven by the one knob in themes/default.nix (`theme
          # transparency <v>` for live tweaks). Unfocused windows sit at the
          # knob, focused stays near-solid; at 1 everything is opaque and
          # the blur below is invisible. Window blur only — the noctalia
          # ghost-strip regression was layerrule blur on the hidden BAR
          # surface; panels/dock get layer-rule blur below, bar stays out.
          active_opacity = lib.min 1.0 (themes.transparency + 0.12);
          inactive_opacity = themes.transparency;
          blur = {
            enabled = true;
            size = 6;
            passes = 3;
            noise = 0.02;
          };
        };
      };

      # A lone window floats in a fraction of the screen; more windows tile
      # with the normal gaps above. One margin for every space — no per-app
      # sizing. Workspaces 1-4 get icon names, surfaced by Noctalia's
      # workspaces widget (display = "name").
      workspace_rule = [
        {
          workspace = "w[tv1]";
          gaps_out = 120;
        }
        # Maximize (Super+Shift+F) escapes the smart margins above: a
        # workspace holding a maximized window drops to normal gaps.
        {
          workspace = "f[1]";
          gaps_out = {
            top = 44;
            right = 24;
            bottom = 24;
            left = 24;
          };
        }
        {
          workspace = "1";
          default_name = "󰖟";
        }
        {
          workspace = "2";
          default_name = "󱚝";
        }
        {
          workspace = "3";
          default_name = "󰆍";
        }
        {
          workspace = "4";
          default_name = "󰝚";
        }
      ];

      window_rule = [
        # Fallback placement for manually-launched citizens (silent: the
        # window goes home, focus stays put). No ghostty rule — terminals
        # belong everywhere; only the startup one lands on space 3.
        {
          name = "chrome";
          match.class = "^(google-chrome)$";
          workspace = "1 silent";
        }
        {
          name = "claude";
          match.class = "^(com\\.anthropic\\.Claude)$";
          workspace = "2 silent";
        }
        {
          name = "discord";
          match.class = "^(discord)$";
          workspace = "4 silent";
        }

        # parker's face: its own special workspace, genuinely fullscreen (the
        # `fullscreen` rule is Super+F's variant, not Super+Shift+F's — no
        # gaps, no rounding, edge to edge).
        #
        # The class MUST contain a dot. ghostty silently ignores `--class`
        # unless it is a valid GTK application id, so the first version of
        # this asked for `parker-face`, got a window still calling itself
        # `com.mitchellh.ghostty`, and this rule matched nothing at all —
        # which is why the face opened on whatever workspace you happened to
        # be on and was never fullscreen. Verified live before and after.
        #
        # `silent` so the login spawn does not drag focus onto a hidden
        # workspace. Mirrored in home/parker (the script) and
        # src/face/mod.rs (sense needs it — see below).
        {
          name = "parker-face";
          match.class = "^(com\\.parker\\.face)$";
          workspace = "special:parker silent";
          fullscreen = true;
        }

        # `parker reach show`: shown things arrive as a floating card over
        # the workspace, not a tiled disruption. Covers the glow pane (the
        # class with the load-bearing dot — reach/show.rs) and imv, which
        # show is the only spawner of today.
        {
          name = "parker-show";
          match.class = "^(com\\.parker\\.show|imv)$";
          float = true;
          center = true;
          size = "monitor_w*0.6 monitor_h*0.7";
        }

        # The summoned utilities (`summon`, home/desktop/default.nix): each a
        # floating card on its own special workspace, so the same key that
        # brings it puts it away. `silent` so the first spawn lands hidden
        # and the script's reveal is the one that shows it.
        #
        # Sizes here are expressions over the monitor, `monitor_w*0.6`, not
        # percentages: under the Lua config "60% 60%" is silently ignored and
        # the window opens at 800×600 (measured on 0.56.2, Oct 2026 —
        # parker-show above had been hit by it since the migration).
        {
          name = "summon-audio";
          match.class = "^(com\\.parker\\.audio)$";
          workspace = "special:audio silent";
          float = true;
          center = true;
          size = "monitor_w*0.6 monitor_h*0.6";
        }
        {
          name = "summon-system";
          match.class = "^(com\\.parker\\.system)$";
          workspace = "special:system silent";
          float = true;
          center = true;
          size = "monitor_w*0.7 monitor_h*0.7";
        }
        {
          name = "summon-devices";
          match.class = "^(com\\.parker\\.devices)$";
          workspace = "special:devices silent";
          float = true;
          center = true;
          size = "monitor_w*0.5 monitor_h*0.55";
        }

        # visage — parker's face as an app, its own special workspace beside
        # the terminal face's. The class is the live app_id (`visage`, not
        # the tauri identifier com.parker.visage — visage/AGENTS.md, July
        # 2026).
        {
          name = "visage";
          match.class = "^(visage)$";
          workspace = "special:visage silent";
          fullscreen = true;
        }
      ];

      layer_rule = [
        # Noctalia surfaces: no animations.
        {
          name = "noctalia";
          match.namespace = "^noctalia-(bar-.+|notification|dock|panel|attached-panel|osd|window-switcher)$";
          no_anim = true;
        }

        # Frosted glass behind panels and the dock (pairs with noctalia's
        # shell.panel glass mode). Since 0.56 Hyprland also honours
        # ext-background-effect-v1, which noctalia asks with directly, so
        # this rule is now redundant for surfaces noctalia already requests
        # blur for — harmless, and the fallback if that request ever stops.
        # Deliberately NOT the bar: blur on its hidden-but-mapped surface is
        # what caused the ghost strip that got blur removed before;
        # ignore_alpha keeps fully transparent regions unblurred. Note a
        # layer rule cannot *remove* protocol-requested blur — shouldBlur()
        # short-circuits on the surface's background-effect region before it
        # ever reads the rule; that is what noctalia-bar-blur.patch is for.
        {
          name = "noctalia-blur";
          match.namespace = "^noctalia-(panel|attached-panel|dock)$";
          blur = true;
          ignore_alpha = 0.2;
        }
      ];
    };

    extraConfig = ''
      -- Apps
      hl.bind("SUPER + Return", hl.dsp.exec_cmd("ghostty"))
      hl.bind("SUPER + Space", hl.dsp.exec_cmd("noctalia msg panel-toggle launcher"))
      hl.bind("SUPER + W", hl.dsp.window.close())

      -- Direct space jumps 1-4 (more spaces on demand), and moving the
      -- focused window there (follows it, like movetoworkspace did). Ctrl
      -- not Shift: Super+Shift+3/4 belongs to screenshots (DECISIONS.md).
      for i = 1, 4 do
        hl.bind("SUPER + " .. i, hl.dsp.focus({ workspace = i }))
        hl.bind("SUPER + CTRL + " .. i, hl.dsp.window.move({ workspace = i }))
      end

      -- Window focus on the current space: Super+hjkl
      hl.bind("SUPER + H", hl.dsp.focus({ direction = "left" }))
      hl.bind("SUPER + J", hl.dsp.focus({ direction = "down" }))
      hl.bind("SUPER + K", hl.dsp.focus({ direction = "up" }))
      hl.bind("SUPER + L", hl.dsp.focus({ direction = "right" }))

      -- Space traversal: the uppercase (shifted) variants. J/K reserved.
      hl.bind("SUPER + SHIFT + H", hl.dsp.focus({ workspace = "e-1" }))
      hl.bind("SUPER + SHIFT + L", hl.dsp.focus({ workspace = "e+1" }))

      -- Super+F true fullscreen; Shift variant maximizes keeping gaps
      hl.bind("SUPER + F", hl.dsp.window.fullscreen({ mode = "fullscreen" }))
      hl.bind("SUPER + SHIFT + F", hl.dsp.window.fullscreen({ mode = "maximized" }))

      -- parker. Special workspaces, so reachable only from here — not by
      -- SUPER+1-4 and not by directional cycling. Scripts spawn on first
      -- use and just reveal afterwards (home/parker). P is visage (the
      -- app); Shift+P keeps the terminal face.
      hl.bind("SUPER + P", hl.dsp.exec_cmd("parker-visage"))
      hl.bind("SUPER + SHIFT + P", hl.dsp.exec_cmd("parker-face"))

      -- Put it on. Starts a dance and brings visage up if it is away —
      -- `--show` rather than a second `parker visage`, because that verb is
      -- a *toggle* and pressing this twice must not put the face away
      -- halfway through the dance it just started (reach/visage.rs).
      --
      -- Alt rather than Shift only because Shift+P is the terminal face.
      -- Same family, one modifier out.
      hl.bind("SUPER + ALT + P", hl.dsp.exec_cmd("parker reach dance auto --for 0 --show"))

      -- Screenshots: full/region to clipboard; +Alt saves to file
      hl.bind("SUPER + SHIFT + 3", hl.dsp.exec_cmd("grim - | wl-copy"))
      hl.bind("SUPER + SHIFT + 4", hl.dsp.exec_cmd('grim -g "$(slurp)" - | wl-copy'))
      hl.bind("SUPER + SHIFT + ALT + 3", hl.dsp.exec_cmd("grim ~/Pictures/screenshots/$(date +%Y-%m-%d_%H-%M-%S).png"))
      hl.bind("SUPER + SHIFT + ALT + 4", hl.dsp.exec_cmd('grim -g "$(slurp)" ~/Pictures/screenshots/$(date +%Y-%m-%d_%H-%M-%S).png'))
      -- …and 5 annotates: a region into satty, which copies on Ctrl+C and
      -- saves beside the others on Ctrl+S (macOS's Shift+5 is the capture
      -- tool too).
      hl.bind("SUPER + SHIFT + 5", hl.dsp.exec_cmd('grim -g "$(slurp)" - | satty --filename - --copy-command wl-copy --output-filename "$HOME/Pictures/screenshots/%Y-%m-%d_%H-%M-%S-annotated.png"'))

      -- Utilities, summoned (DECISIONS.md, Utility principle). The left
      -- hand's home row: A audio, S system, D devices (bluetooth). Each
      -- toggles its own floating card; see `summon`.
      hl.bind("SUPER + A", hl.dsp.exec_cmd("summon audio com.parker.audio wiremix"))
      hl.bind("SUPER + S", hl.dsp.exec_cmd("summon system com.parker.system btop"))
      hl.bind("SUPER + D", hl.dsp.exec_cmd("summon devices com.parker.devices bluetui"))
      -- Clipboard history is noctalia's (it keeps one); this is its panel.
      hl.bind("SUPER + V", hl.dsp.exec_cmd("noctalia msg panel-toggle clipboard"))

      -- Media, on Super+Alt's home row because this keyboard has no media
      -- keys and Fn chords are awkward: J/K volume (held, it repeats), 0
      -- mute, H/L previous/next track, M play-pause. Volume is wpctl
      -- directly — a held key fires at the repeat rate, 35/s, and that is
      -- no job for a process that reads MPRIS — capped at 100%; noctalia
      -- shows the OSD on its own. Tracks go through parker's media verb.
      -- `locked` so they work over the lock screen too.
      hl.bind("SUPER + ALT + K", hl.dsp.exec_cmd("wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+"), { repeating = true, locked = true })
      hl.bind("SUPER + ALT + J", hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"), { repeating = true, locked = true })
      hl.bind("SUPER + ALT + 0", hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"), { locked = true })
      hl.bind("SUPER + ALT + H", hl.dsp.exec_cmd("parker reach media previous"), { locked = true })
      hl.bind("SUPER + ALT + L", hl.dsp.exec_cmd("parker reach media next"), { locked = true })
      hl.bind("SUPER + ALT + M", hl.dsp.exec_cmd("parker reach media toggle"), { locked = true })

      -- Manual sleep (DECISIONS.md: no idle auto-suspend). Deliberately an
      -- awkward chord — SUPER+Escape alone was too easy to hit by accident.
      hl.bind("SUPER + SHIFT + CTRL + Escape", hl.dsp.exec_cmd("systemctl suspend"))

      hl.bind("SUPER + mouse:272", hl.dsp.window.drag(), { mouse = true })
      hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })

      -- Workspace-per-app at login (DECISIONS.md): each citizen spawns to its
      -- space, silently (no focus-stealing during startup). uwsm app -- gives
      -- each its own systemd scope instead of a compositor child. The env
      -- line runs first so portals (screenshare etc.) see the session.
      -- Spotify MUST be placed here — it sets its class late, so class rules
      -- never catch it.
      hl.on("hyprland.start", function()
        hl.exec_cmd("uwsm app -- dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP")
        hl.exec_cmd("uwsm app -- google-chrome-stable", { workspace = "1 silent" })
        hl.exec_cmd("uwsm app -- claude-desktop", { workspace = "2 silent" })
        hl.exec_cmd("uwsm app -- ghostty -e zellij attach -c parker", { workspace = "3 silent" })
        hl.exec_cmd("uwsm app -- spotify", { workspace = "4 silent" })
        -- parker's face, running from login on its own special workspace.
        -- It is not a citizen of a numbered space — SUPER+P reveals it and
        -- nothing else can reach it. Started here rather than on first press
        -- so the face is alive whenever the machine is, and so summoning it
        -- shows something mid-animation instead of a cold start.
        hl.exec_cmd("uwsm app -- ghostty --class=com.parker.face --font-size=32 --window-padding-x=0 --window-padding-y=0 -e parker face --animate")
        -- Last, and the only line that takes focus: space 3 becomes the
        -- session's ground floor, then visage is summoned over it. Everything
        -- above spawns `silent` precisely so this one gets to decide where the
        -- session starts.
        hl.exec_cmd("parker-greet")
      end)
    '';
  };

  # Screenshot file target for the +Alt binds.
  home.file."Pictures/screenshots/.keep".text = "";
}
