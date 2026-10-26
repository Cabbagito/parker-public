# quirks.md — hardware gotchas from the old install
 
**How to use this file:** do NOT proactively configure anything here. These are
symptom→cause pairs from problems already solved once on this hardware (under
Arch). Only reach for this file when something misbehaves and a symptom
matches — then check whether the old cause applies on NixOS. Some of these may
never come up, or may be handled automatically by NixOS modules/flakes.
 
## Symptom → likely cause
 
- **USB mic or controller randomly disconnects / drops out**
  → USB autosuspend. Old fix: `usbcore autosuspend=-1` (modprobe option).
- **Slippi doesn't detect the GameCube adapter**
  → missing udev rule. Old rule: vendor `057e`, product `0337`, `MODE="0666"`.
  (The slippi-nix flake is supposed to ship this — verify before hand-adding.)
- **DualShock touchpad moves the mouse cursor**
  → libinput sees the touchpad as a pointer. Old fix: udev rule setting
  `LIBINPUT_IGNORE_DEVICE=1` for "Wireless Controller Touchpad" (USB + BT names).
- **CoolerControl shows only Kraken + GPU, no motherboard fan headers**
  → nct6798 driver not loaded; nothing auto-loads it. Solved on NixOS (July
  2026): `boot.kernelModules = [ "nct6775" ]` in `system/cooling.nix`
  (family driver covers nct6798). Daemon needs a restart after the module
  first loads.
- **A motherboard fan header ignores every duty and runs ~1500–3000 rpm**
  (fan3; "slowing it down" can even read faster)
  → it is not a fan: the Kraken's pump tach lead, so it tracks the pump's
  speed (Oct 2026). Don't put a curve on it. Map in docs/COOLING.md.
- **Kraken fan channel set to 0% reports 0 rpm, but the radiator fans keep
  spinning**
  → the Kraken won't stop them and the readout drops to 0 regardless.
  Identify or test that channel by driving it to 100% (audible), not 0%.
- **Keyboard doesn't wake the machine from suspend (only the power button)**
  → USB wakeup disabled by kernel default along the whole chain (root hubs +
  device). Old fix (July 2026): udev rules in `system/misc.nix` enabling
  `power/wakeup` for the root hubs (1d6b) + BG64MV2 keyboard (2233:003b);
  mouse receiver deliberately kept non-waking. Worked, then regressed after
  an unidentified later change. *Abandoned (July 2026)* — rules removed, the
  physical power button is the accepted wake path (DECISIONS.md). Restore
  from git history if keyboard wake is ever wanted again; suspects for the
  regression are the xHCI controller's `/proc/acpi/wakeup` entry and BIOS
  ErP.
- **F-keys act as media keys on the Apple-style keyboard**
  → `hid_apple` default. Old fix: `options hid_apple fnmode=2`.
- **Wayland session glitchy/broken on the 3090** (flicker, apps not rendering,
  wrong GPU picked)
  → old working recipe was: DRM modeset on (`nvidia_drm modeset=1`), nvidia
  modules loaded early (initramfs), plus env vars `LIBVA_DRIVER_NAME=nvidia`,
  `__GLX_VENDOR_LIBRARY_NAME=nvidia`, `NVD_BACKEND=direct`. NixOS's nvidia
  module covers most of this via options — compare, don't copy blindly.
  Known-good driver at time of capture: nvidia-open 595.71.05.
- **After a `switch`, newly started GPU apps die at once** — the bar
  (noctalia: `fatal: eglGetDisplay failed`, then systemd's start limit),
  hyprpaper aborting in its OpenGL renderer — while everything already
  running is fine
  → the switch brought a new NVIDIA userspace (595.99 → 595.104, Oct 2026)
  but the kernel still has the old module loaded; every *new* EGL client
  mismatches until reboot. Compare `/proc/driver/nvidia/version` with the
  version under `/run/opengl-driver/lib`. Fix: reboot. Prevention: a
  monthly update that bumps the driver deploys with `nixos-rebuild boot`
  + reboot, not `switch` (root AGENTS.md, Building).
- **GPU memory clock pegged / high idle power draw & heat**
  → the 180Hz refresh rate does this. Old workaround: toggle DP-3 between
  180Hz and 60Hz when idle (script existed for this).
- **Wifi device completely absent** (no `wlan0` in `ip link`, zero iwlwifi
  lines in dmesg, bluetooth still present)
  → iwlwifi never loaded — seen once (July 2026, NixOS) after several
  hard-power-button resets. `sudo modprobe iwlwifi` revived it instantly.
  If it ever recurs on clean boots: declare `boot.kernelModules =
  [ "iwlwifi" ]` to force-load it.
- **Every Hyprland keybind stops working at once** — Super does nothing, no
  space switching, but the session is otherwise fine
  → binds are alive but parked in an inactive **submap**. Check with
  `hyprctl binds | grep submap` (healthy is `submap:` blank) and
  `hyprctl submap` (healthy is `default`). Cause: something ran
  `hyprctl keyword submap <name>`, which leaves the config *parser* in submap
  context — it is a scope opener, not a one-shot setting. The next
  `hyprctl reload` (home-manager runs one on every switch) then re-reads
  hyprland.conf with that scope still open and attaches every `bind =` to
  that submap. Fix without rebuilding:
  `hyprctl keyword submap reset && hyprctl reload`.
  Seen July 2026, caused by a Claude subagent probing config syntax live.
  **Never run `hyprctl keyword submap` against a live session** — it is the
  one keyword that changes parser state rather than a value. *(Written for
  the hyprlang config. Under the Lua config, Oct 2026, `hyprctl keyword` is
  refused outright, so this cause is unreachable; the symptom check still
  applies if binds ever go dead with the session otherwise healthy.)*
- **Chrome / Chromium / Electron apps (Spotify, Discord) all die the instant
  a `nixos-rebuild switch` runs** (SIGTRAP core dumps, same second as
  home-manager's `onFilesChange` → `hyprctl reload`)
  → Hyprland 0.55's reload also does a monitor reload, which invalidates
  Wayland color-management objects; Chromium ≥141 crashes referencing the
  destroyed `wp_image_description_v1` (hyprwm/Hyprland discussions #11843,
  #14611). Diagnosed on NixOS (July 2026): Chrome carries
  `--disable-features=WaylandWpColorManagerV1` in `home/apps/chrome.nix` as
  the workaround. Electron apps still die on switch — accepted until a fixed
  Hyprland arrives via the monthly bump; re-test by removing the flag then.
  **Gone on Hyprland 0.56.2 (re-tested Oct 2026):** Chrome 155 *without*
  the flag, Claude Desktop, Spotify and every other client kept their pids
  through a switch that reloaded the config and through a bare `hyprctl
  reload`; no core dumps. The flag is removed. If this symptom returns,
  the flag is the first thing to put back.

- **visage (WebKitGTK) dies at launch: `Gdk-Message: Error 71 (Protocol
  error) dispatching to Wayland display`**
  → WebKitGTK's dmabuf renderer vs this NVIDIA + Hyprland combination, the
  ecosystem's long-standing pairing bug. Seen at visage's first launch
  (July 2026). Fixed in code: visage's `main()` sets
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` before GTK initializes (env wins if
  already set). Any future WebKitGTK-based window will need the same.

- **visage (WebKitGTK) comes up completely blank** — the window is there,
  mapped, focused and fullscreen (`hyprctl clients` agrees), SUPER+P reveals
  it, and it paints its background colour and nothing else. No crash, no
  stderr, no JS error.
  → GDK has no screen resolution to report. `gdk_screen_get_resolution()`
  returns **-1** when unset, WebKitGTK divides that by 96, and the webview
  comes up at `devicePixelRatio = -1/96`. Every CSS length then goes
  *negative*: on a 2560x1440 window `innerWidth` measured -245760, `100vh`
  resolved below zero, and the whole document laid out at zero size. Nothing
  fills the value in under Hyprland — there is no XSETTINGS daemon, and the
  generated `~/.config/gtk-3.0/settings.ini` carries theme, cursor and font
  but no `gtk-xft-dpi`. Most GTK apps never notice because they read -1 as
  "assume 96"; the webview is the one that does the arithmetic.
  Fixed in code (July 2026): `sane_dpi()` in visage's `main()` sets
  `gtk-xft-dpi` to 96 dpi when it is unset, before the window is built. A
  real HiDPI setting is left alone. The machine-wide alternative is the same
  key in home-manager's gtk config, which would cover any other WebKit app
  that trips on this — worth doing if a second one ever does.

- **Shell shortcuts stop working over SSH and print escape codes instead** —
  Ctrl+F (accept autosuggestion) inserts something like `[102;5u` on the line;
  affects the bare zsh prompt, not any one app. Comes and goes on its own.
  → the kitty keyboard protocol left enabled on the PTY. Ghostty (giskard)
  speaks it, so an app that negotiates CSI-u and then exits or detaches
  uncleanly leaves the mode on; zle can't parse CSI-u and inserts the tail
  literally. Self-heals whenever something pops the mode or a fresh session
  starts, which is why it looks intermittent — it is terminal *state*, never
  config, so nothing in this repo can be "the change that broke it".
  Unstick a live session: `printf '\033[<u'` (and `printf '\033[>4;0m'` for
  the xterm modifyOtherKeys equivalent). Seen July 2026. Mitigated by
  `support_kitty_keyboard_protocol false` in `home/terminal/zellij/config.kdl`
  — zellij was the likeliest leaker and sits in the SSH path. Claude Code's
  TUI also enables the protocol; if this recurs with zellij ruled out,
  suspect whatever TUI exited last.

- **A theme flip wedges the rebuild lock and `nrs` dies with exit 11** —
  `nixos-rebuild` fails with "Could not acquire lock ... returned non-zero
  exit status 11", and at the same time the theme refuses to change: `theme
  dark` appears to do nothing, the timer keeps failing, the desktop stays on
  the other polarity. Two symptoms, one cause — worth knowing, because it
  reads as two unrelated bugs.
  → `theme sync` ran an activation that hung *inside* the lock. Seen 28 July
  2026: `theme-sync.service` sat in `activating` for 65 minutes, last log
  line `reloading the following user units: dbus-broker.service`, with a
  `switch-to-configuration` child stuck in `poll()`. switch-to-configuration
  reloads changed user units and then blocks waiting for systemd's
  `JobRemoved` signal — but the unit it reloaded *is the session bus that
  signal travels over*, so the reply never arrives. It only comes up when
  dbus-broker itself changed between the running system and the target, i.e.
  a polarity flip that crosses a nixpkgs bump. `Type=oneshot` defaults to
  `TimeoutStartSec=infinity`, so nothing ever timed it out.
  Mitigated (not fixed) in `home/theming/default.nix`: `apply()` wraps
  activation in `timeout 120` *inside* sudo — root has to be the one to kill
  it, since sudo does not forward signals inward — plus
  `TimeoutStartSec=180` on the unit as an outer bound. The lock releases when
  the killed process dies, so the worst case is a skipped flip and a warning
  instead of an hour of blocked rebuilds. The underlying s-t-c behaviour is
  upstream and untouched.
  Unstick a live one: kill the `switch-to-configuration` child running as
  your own uid (the root parents unwind with it), then re-run.
- **FitGirl/DODI-style repack installer dies on its first file** — "Unable to
  write data to disk! Unarc.dll returned an error code: -11 / archive data
  corrupted", the install folder is empty afterwards, MD5 of the .bin files is
  fine.
  → The unpacker (ISDone/unarc + srep/magic2/lollypop plugins) reserves ~2.9 GB
  in two blocks and peaks at 4.18 GB VmPeak inside the 32-bit `setup.tmp`,
  i.e. it needs the whole 4 GB a 32-bit process can address; Windows leaves
  it nearly empty, Wine's own layout fragments enough that the reservation
  fails. Verified 14 Sep 2026 over 17 runs: identical under GE-Proton 11-6,
  Proton Experimental 11 and plain Wine 11 in a win32 prefix; unaffected by
  the "Limit installer to 2 GB RAM" box, vcrun2022, CLS.ini memory patches,
  CPU-count limits, fsync/esync, LAA=0, WinXP, or the x64/x86 helper
  processes (they run). Not a settings problem — get a plain-files release
  instead (scene ISO / pre-installed zip, `7zz x` it). Ordinary Inno
  installers are fine, and run headless with `/VERYSILENT /DIR=...`.
- **FLT release (2022) — "Failed to initialize Steam" or "cannot be started
  while Steam is not running"**, whether real Steam is open or not.
  → FLT's crack is a patched *real* `steamclient64.dll` plus a near-stock
  `steam_api64.dll`, which resolves the client through the Windows-Steam
  registry (`HKCU\Software\Valve\Steam` InstallPath/ActiveProcess pid). A
  Proton prefix pre-populates those keys pointing at Proton's fake Steam, so
  the crack is never reached (its `flt.ini` is never even opened); delete the
  keys and the "is Steam running" check fails instead. Fix (14 Sep 2026):
  replace `steam_api64.dll` with Goldberg (gbe_fork, regular/x64) +
  `steam_appid.txt` + `steam_settings/` (account_name, language, offline=1,
  `steam_interfaces.txt` from `generate_interfaces_x64.exe` on the original
  dll), drop `steamclient64.dll`, launch with `WINEDLLOVERRIDES=steam_api64=n,b`.
  Works for Steamworks-only titles (no Denuvo). Chrome refuses the emulator
  download ("dangerous") — fetch it with curl.

- **DualSense moves the mouse cursor / clicks on the desktop** → the pad's
  touchpad is a separate evdev node (`Wireless Controller Touchpad` —
  the kernel uses the pad's self-reported name, "DualSense" never appears) and libinput treats it as a real touchpad. Handled by the
  `LIBINPUT_IGNORE_DEVICE` udev rule in `system/gaming.nix`; if it recurs,
  check `libinput list-devices` for the Touchpad node and `udevadm info
  -a` for a renamed device string (Edge/Access pads).
- **DualSense pairs but drops / won't reconnect on PS press** → hold PS +
  Create (the small button left of the touchpad) ~5s until the light bar
  blinks fast, then `bluetui` → pair again. The pad remembers one host at a
  time, so a pad last used on a PS5 must be re-paired here.
- **parker's words go missing in visage — mid-turn lines, or the whole
  closing reply; the turn shows only acts** → not a bug: Opus 5.5's
  *progress updates* (Anthropic docs, "Progress updates between tool
  calls"), hidden by the default `thinking.display`. The text is missing
  from the engine's store too, so it is not visage, and not opencode
  (13 Oct: proxy capture + raw replay).
  - Mechanism: on Opus 5.5 (also Sonnet 5.5, Fable 5/5.1, Mythos 5.1),
    prose written *before a tool call* comes back as its own `thinking`
    block, not `text`. Under the default `display: "omitted"` its text is
    empty, so opencode stores an empty `reasoning` part. Billed at full
    length; the model gets its full note back through the signature on the
    next step, so parker believes he said it and Peter saw nothing.
  - A tool call *after* the closing reply makes the closing reply a progress
    update too. Every lost closing reply on 13 Oct ended in a `face` call.
    Prose with no tool call after it is a `text` block and survives.
  - `display: "updates"` (beta header `thinking-display-updates-2026-08-18`)
    returns each update as **summary text by a separate model**, not
    verbatim; any non-empty thinking block is then an update and reasoning
    stays empty. `"summarized"` returns summaries for both,
    indistinguishable — do not use it for this. Verbatim mid-turn text needs
    a tool (Anthropic's lever: a send-message tool, declared from the first
    request, since adding a tool invalidates earlier thinking blocks).
  - Opus 5.5 rejects `thinking: disabled` (400). Opus 5 writes `text`
    between tools, thinking on or off (6/6 each) — rejected on 13 Oct, we
    do not go back a model to dodge this.
  - Not the trigger: the `thinking-binding-controls` beta / `block_binding`
    (5/6 either way). Frequency depends on context: a bare hand-written tool
    loop rarely shows it; opencode's request does (5/6 on replay).
  - Prompting: Opus 5.5 got a `reasoning_extraction` refusal (empty
    content) for "narrate your work… what you found so far and what you
    will check next". Anthropic's progress-update wording passed.
  - Check: `select data from part where message_id=…` in
    `~/.local/share/opencode/opencode-stable.db` (no sqlite3 on parker:
    `nix shell nixpkgs#python3`). A second empty `reasoning` part where the
    prose should be, no `text`, is this. Its signature (base64 → protobuf)
    names `narration` in the first ~40 bytes; undocumented, diagnose with
    it, never build on it.
  - Rig: `/tmp/opencode/t2/` — `run.sh` (one `opencode run --pure` +
    tally), `proxy.py` on :8787 via `OPENCODE_CONFIG=proxy-config.json`,
    `replay5.py <cap> asis|nobind|summ|off N <model>`. /tmp does not survive
    a reboot.

## Facts (not quirks)
 
- Monitor: AOC Q27G42XNE — 2560x1440 @ 180Hz, was on DP-3
- Data drives to remount: Kingston SSD + WD 1TB + WD 500GB (ext4)
- Calibrated data worth its weight: `~/.config/SlippiOnline/` (controller
  profiles), `~/Slippi/` (replays), `~/.ssh/parker` (GitHub key)
