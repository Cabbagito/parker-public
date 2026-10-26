# DECISIONS.md — parker

Settled during the initial design brainstorm (July 2026). Each entry is a
decision plus the reasoning that produced it. Hardware facts, deferred items,
and build-time verification flags at the bottom. This document records *why*;
the config records *what*. Update it when a decision is revisited, not for
details.

## Philosophy

- **Full declaration.** Everything lives in this repo and is compiled by
  Nix/home-manager. A mutable escape hatch (out-of-store symlinks) exists in
  principle but is currently unused — see "all configs compiled" below.
- **Flakes, pinned to `nixos-unstable`.** The machine's identity (nvidia,
  gaming, CUDA) is where stable channels lag. Safety comes from the lockfile +
  rollback generations, not from channel stability.
- **Monthly update ritual, Claude-driven.** Update the lock, test-build
  (`nixos-rebuild build`), summarize what changed and flag anything spicy
  (driver/kernel bumps), then apply. Regressions → boot previous generation,
  pin back the offender as an exception.
- **parker is self-contained.** giskard (the MacBook) keeps its own dotfiles
  repo. The `home/` layer stays modular and Linux-agnostic where cheap, so
  absorbing giskard via home-manager remains possible later. Not built for now.
- **Impermanence: skipped.** The silent-amnesia failure mode is exactly the
  upkeep flavor to avoid. btrfs subvolume separation is kept sane so this
  stays a config-change-away option.
- **Repo is private.** *Corrected July 2026 — this entry read "repo is
  public" and everything below inherited that threat model.* The repo was
  public only for the install (the runbook clones over https, CI built it on
  a public runner) and went private once parker was up. Consequence: repo
  contents are exposed by a GitHub account compromise or a stolen laptop,
  not by the world — which is roughly the same blast radius as `~` on a
  machine with no disk encryption (see "No disk encryption" below). So
  plaintext credentials in the repo are no longer categorically wrong.
  Working rule: rotatable, scoped, spend-capped secrets (API keys for
  parker's engine, tokens for local daemons) may sit in the repo in
  plaintext; anything catastrophic-if-leaked-in-2040 stays out entirely,
  encrypted or not. sops-nix stays wired for the middle tier and for the day
  this stops being true. **If the repo ever goes public again, that
  plaintext is the migration** — and it is the same cleanup PARKER.md's
  memory section already owes.
- **The public parker is a one-way mirror (October 2026).** This repo stays
  private and stays the only place anything is edited; `mirror/publish`
  builds a public copy from a commit (exclude, overlay, redact) and refuses
  if its tripwire or gitleaks finds anything left. Not "make this repo
  public": that publishes every past version of `mind/peter.md`, the commit
  messages that quote it, and the plaintext above, and rewriting history in
  place would break every hash the notes cite. Not "visage on its own": it
  is half of parkerd's face and a client of the engine, so alone it does
  not build, and a second repo is a second copy that drifts. Snapshots, not
  filtered history: each publish is one commit naming its source, so what
  needs vetting is one tree, never every diff since July. The cost is that
  the commit messages stay home. The mirror also ended the plaintext above
  in practice: the sudo hash left the repo, and the SearXNG key and the
  CoolerControl token moved to files on the machine (engine.env's
  pattern), so the public tree is the config that runs, not a redacted
  copy of it. The rule still permits plaintext; a credential added under
  it gets a `mirror/redact` line in the same commit.

## System

- **No disk encryption.** LUKS was Omarchy's choice, not ours; the headache
  outweighed the threat model for a home desktop. Accepted tradeoff: physical
  theft yields disk contents. Mitigation is hygiene (scoped SSH key), not
  crypto. Bonus: fan control takes effect at boot instead of after unlock.
- **btrfs with zstd compression** on the OS disk — chosen for store
  compression, not snapshots. Subvolumes come along for free. The three ext4
  data drives are declared as mounts, untouched by the install.
- **disko declares the disks.** Bare metal → booted parker is one command.
  Disks addressed by `/dev/disk/by-id/` (see Hardware facts), never by
  `sdX`/`nvmeX` names.
- **Auto-login, no greeter.** Unencrypted disk makes a login screen theater.
  Power button → desktop; reboots come back up whole. sshd is a system
  service, independent of the graphical session (LAN SSH works day one).
  *Mechanism revised at Phase 4 step 1 (July 2026):* getty autologin →
  greetd `initial_session` launching Hyprland via UWSM. Same decision, new
  plumbing; greetd draws no UI. tty2+ keep normal login prompts as fallback.
- **Networking: NetworkManager, wifi-first.** No ethernet available at the
  machine's location. First boot: one `nmtui` login at the keyboard; the
  connection persists and auto-reconnects thereafter. Declaring wifi
  credentials via sops is an optional later purity upgrade.
- **Tailscale: un-deferred (July 2026)** — the remote-node use case became
  real. `system/tailscale.nix`: tailscaled at boot, `tailscale0` trusted, one
  interactive `tailscale up` (nmtui pattern; state in `/var/lib/tailscale`).
  Remote SSH is the existing sshd over the tailnet — never an open port.
  Availability caveats accepted: no remote wake from suspend (wifi-only, no
  WoL; power button is the wake path), so don't suspend before leaving.
  NordVPN remains deferred; when it lands, set
  `networking.firewall.checkReversePath = "loose"`.
- **Docker: declared, not running at boot** (`enableOnBoot = false` —
  socket-activated on first `docker` command). Needed occasionally, not
  furniture.
- **CoolerControl from v0.** `programs.coolercontrol.enable`; manages both
  the nct6798 motherboard headers and the NZXT Kraken AIO; the daemon
  starts at boot so fans are tame before the desktop appears. Curves stay
  daemon state rather than nix (CoolerControl rewrites its own config, so a
  read-only declared file would fight it); a snapshot in
  `docs/cooling/coolercontrol-curves.toml` is the restore path. **Curves
  key on liquid temperature, case fans on max(liquid, GPU)** (Oct 2026):
  the CPU hits TjMax within seconds at any heavy load regardless of
  airflow, so CPU-keyed fans would only add noise; liquid is what the
  radiator actually sheds. Measured case in docs/COOLING.md.
- **zram swap** — carried over from the old install; one module.
- **Plymouth: dropped** (July 2026). Slated for the boot ritual at design
  time, but the theming research verdict was "not viable" and the config
  never enabled it. Boot stays plain; any future splash/greeting ambition
  rides on Noctalia (lock screen) or something small and hand-rolled.
  Stylix's plymouth target is disabled defensively.
- **Sleep is manual** (keybind → suspend-to-RAM). No idle auto-suspend.
  nvidia's preserve-video-memory power-management options declared for clean
  resume. The keybind is deliberately awkward (`SUPER SHIFT CTRL, Escape`);
  plain `SUPER, Escape` was too easy to fat-finger. **Wake is the physical
  power button**, by choice — USB keyboard wake was tried, worked, regressed,
  and was dropped rather than chased (QUIRKS.md).
- **Nvidia via the NixOS module.** Open kernel module (NVIDIA's recommended
  path for Ampere), production driver branch, modeset, preserve-VRAM power
  management. *Revised at Phase 4 step 1 (July 2026):* the hand-rolled Arch
  env-var recipe (QUIRKS.md) was verified obsolete on current drivers —
  explicit sync (555+) fixed the Wayland story and some old vars now cause
  XWayland/screenshare harm. Config starts bare; knobs added on symptom only.
- **PipeWire** for audio.

## Session

- **Hyprland**, chosen on merit after re-examination: the desired combination
  (rounded corners + gaps, per-app window rules, workspace-per-app,
  animations, autostart-to-space) is its native feature set.
- **Workspace-per-app model.** Space 1 Chrome · 2 Claude Desktop · 3
  ghostty/zellij · 4 Spotify/peripheral (Slack cut July 2026 — not needed
  on this machine). Apps auto-spawn to their
  spaces at login (`exec-once` + workspace rules). More spaces on demand.
  Hypothesis to test: space 4's inhabitants may work better as summonable
  scratchpads (special workspaces).
- **The session starts on space 3, behind parker's face** (July 2026). Login
  ends with two deliberate moves: focus space 3, then summon visage. Because
  a special workspace is an overlay, that order makes the terminal the ground
  floor — the face is the first thing you see, and dismissing it returns you
  to space 3 rather than to whatever space the session happened to boot on.
  (The bar it once needed veiling from — `parker-bar-veil`, a listener on
  Hyprland's event socket — went with the bar, Oct 2026.)
- **Claude Desktop: official Linux app** (beta, ships as .deb via apt),
  nix-wrapped via community packaging; chromium `--app` webview as fallback.
- **Modifier ownership: Super = OS, Alt = terminal-land.** Super+1-4 direct
  space jumps, Super+H/L directional cycling, Super+F fullscreen (maximize
  variant keeps gaps/rounding), Super+Space launcher. Caps→Ctrl system-wide
  via xkb `ctrl:nocaps`.
- **Space reordering is scripted, not native** (hyprctl script family; same
  family as profile-swap "modes" like a mechinterp layout). If live
  reordering ever becomes core rather than occasional, Niri is the revisit.
- **Noctalia desktop shell.** *Revised at Phase 4 step 2 (July 2026):*
  originally chosen as a Quickshell/QML prebuilt with the premise "shell
  chrome is owned code" — but upstream froze the Quickshell line (v4, EOL)
  and rewrote Noctalia as native C++/GLES (v5, beta). Adopted **v5**:
  purpose-built for read-only NixOS configs (declarative TOML +separate
  writable state layer), ~6× lighter, built-in theming/wallpaper/lockscreen,
  binary cache (flake input deliberately does NOT follow nixpkgs — cache
  hits require their hashes). Premise downgraded: ownership is config-level,
  not code-level, until v5's plugin story matures; the custom dock idea
  waits on that. *Re-verified late July 2026:* the plugin story has matured
  — Luau plugins (sandboxed VMs, plugin API level 8) cover bar/desktop
  widgets, panels, launcher providers, and background services, and load
  from local paths, so "owned code" returns as in-repo plugins rather than
  a fork; pure-Quickshell rebuild rejected. Dock unblocked: v5 ships a
  built-in dock (off by default, pinned apps + smart auto-hide). Escalation
  path for custom wants: TOML setting → script-backed widget → Luau plugin
  → standalone layer-shell client (Noctalia claims no exclusivity).
  Beta churn accepted — pinned lockfile, deliberate updates.
  The shell owns the launcher, notifications, OSD, lock screen, wallpaper
  and dock. *Revised Oct 2026:* not the bar — it is off, and the settings
  live in visage on ^, (below, "No bar"). The ghost bar it was stays in
  noctalia.nix one line from coming back. Notifications unobtrusive,
  summonable. GUI-written settings land in
  `~/.local/state/noctalia/settings.toml` which overrides the declarative
  layer — keep it pruned; repo stays source of truth.
- **180Hz declared permanent.** *Quirk re-tested at Phase 4 step 1 (July
  2026, driver 595.84 open):* idle clocks downclock properly at 180Hz
  (mem 405 MHz, ~37W) — the quirk is gone, the 60Hz toggle is never built.

## Home

- **Stylix themes the whole machine.** Themes are presets in `themes/`
  (base16 scheme + wallpaper + transparency/blur knobs). Switching = edit
  selection + home-level rebuild (~30s), wrapped in a `theme set <name>`
  command. Chosen over runtime theming: total coherence, zero plumbing;
  accepted cost is losing instant flips (usage pattern is weeks-per-theme).
  Start with one good default (Catppuccin Macchiato), add presets on urge.
  *Landed July 2026:* engine in `system/theming.nix`, presets as data in
  `themes/`. Noctalia is themed via stylix's own noctalia target (custom
  palette generated from the preset's base16 scheme — replaced the builtin
  scheme). btop/lazygit/yazi/zellij moved to their HM modules so targets
  reach them. Out of stylix's reach, accepted: nvim (lazy.nvim owns its
  colorscheme; per-preset alignment is a future step), p10k/fastfetch
  (static colors), chrome/discord/spotify (no targets). *Extended July
  2026:* the six themes/DESIGN.md presets landed with their own base16
  yamls and SVG wallpapers; the preset contract grew local scheme paths,
  and SVG wallpapers rasterize to PNG at build time (resvg, fonts handed
  into the sandbox) rather than at runtime — keeps the compositor on plain
  PNGs, costs one small derivation per theme switch. Accent borders
  decided in: border_size 2, active/inactive colors from the preset's
  `accent`/`borderInactive` (the lab's tuned picks — the stylix hyprland
  target's base0D/base03 read generic next to them; macchiato keeps the
  stylix defaults). Drop shadow off — stylix paints it scheme-white on
  light themes, a second border ring next to the accent one.
  *Extended (transparency, July 2026):* the transparency/blur knobs are
  wired — as ONE global scalar, not per-preset data (chosen over the
  per-preset shape the accent colors use: peki wants a single on-the-fly
  dial, `theme transparency <0.5..1>`, not a per-theme look). The value
  lives in `themes/default.nix` (`transparency`, sed-edited like
  day/night); hyprland derives inactive opacity (= knob) and active
  (knob + 0.12, capped), window blur always on (invisible at 1; layerrule
  blur stays off — the noctalia ghost-strip). Ghostty gets crisp-text
  glass via `stylix.opacity.terminal` (half strength) rather than a raw
  settings key — stylix's target owns background-opacity, so no duplicate
  keys. The subcommand applies instantly via hyprctl for feedback, then
  rebuilds to bake (a bend of "all configs compiled": the live tweak is a
  preview, the compiled value is truth). *Extended (day/night, July 2026):* the
  "instant flips lost" cost above is repealed for polarity: themes/ names
  a day and a night preset (a family = its pair; a single preset pins
  both) and NixOS specialisations pre-build both variants — base system
  is night, `specialisation.day` is day — so a flip is a seconds-long
  `switch-to-configuration test`, no rebuild. Chosen over darkman
  (fixed 08:00/18:30 boundaries suffice; darkman stays the upgrade path
  for sunset-tracking) and over runtime re-theming (unchanged verdict).
  A user timer runs `theme sync` at the boundaries, at login, and hourly
  as an idempotent net — the net also absorbs the accepted wrinkle that
  any plain `nixos-rebuild switch` lands in night regardless of the
  clock. `theme light/dark` writes a sticky override; `theme auto`
  resumes the schedule; `theme set` clears it. The flip escalates via a
  sudoers NOPASSWD rule scoped to exactly the two profile-path
  activation commands. Chrome/discord/spotify now follow polarity (not
  palette) for free: stylix's gnome target flips the freedesktop
  color-scheme key per variant (prefer-dark / default). Cost: every build evaluates the
  system twice (both variants in the closure) — mild on parker (it was
  the expensive half of the retired CI build).
- **Music TUI: spotify-player + cava, composed by a zellij layout.** The
  `music` command (home/tuis/music.nix) opens one window: spotify-player
  above a cava strip. spotify-player chosen over ncspot (feature parity —
  search/playlists/lyrics/queue — plus built-in librespot streaming, MPRIS
  and sixel album art in the nixpkgs default build) and over spicetify for
  bringing Spotify into the palette (spicetify patches the GUI client and
  breaks on its updates; still open as a later add-on for the GUI).
  spotify-player is themed by its stylix target; cava has no palette
  target, so its gradient reuses the preset border story (borderInactive
  floor → accent peaks, base0D solid for accent-less presets). The GUI
  client stays installed as the space-4 citizen. Costs accepted: Premium
  required (already the household reality), one interactive OAuth login
  on first run (token cached in ~/.cache thereafter).
- **All configs compiled, nvim and zellij included.** Current configs are
  stable and port as-is; the promotion-to-mutable path exists if a radical
  experiment ever needs a live edit loop. lazy.nvim keeps managing plugins
  at runtime (accepted undeclared pocket; lockfile must be redirected out of
  the read-only config dir). nixvim-style plugin pinning is a deferred
  purity upgrade, not a porting step.
- **Utility principle: glance via shell, control via TUI.** Every system
  domain gets a summonable keyboard tool: sys=btop, git=lazygit, files=yazi,
  audio=wiremix, bluetooth=bluetui.
- **Screenshots:** grim + slurp + wl-clipboard. Super+Shift+3/4 = full/region
  to clipboard; extra modifier saves to file; satty for the rare annotation.
- **Files:** yazi primary with declared per-type openers — imv (images), mpv
  (video), Chrome (PDFs; zathura in the drawer). Nautilus for the occasional
  GUI-explorer mood.

## Domains

- **The face is entirely parker's (Aug 2026, visage redesign).** The
  original rule — derived states like `thinking` cannot be worn, or the
  involuntary track lies — is reversed. There is no involuntary track
  anymore: visage infers nothing from engine activity, progress lives in
  the transcript (the act ledger and the streaming caret), and parker wears
  every expression himself through `reach face`, per beat. `Expression::
  WEARABLE` is everything but `trouble`, which stays a machine assertion
  (an unreachable engine outranks any face he picked) — **superseded below;
  `trouble` went too.** `VOLUNTARY` remains
  as the spring-easing list only. The old rule's concern — a model claiming
  `thinking` to disguise idleness — is moot once the face carries no status
  at all. Twelve expressions became twenty-four in the same change
  (docs/visage/redesign/README.md); the design's other calls (three views,
  the river travel, the TUI transcript grammar) are recorded there.

- **The face vocabulary is generated, not transcribed (Aug 2026).** The
  redesign gave parker twenty-three wearable faces and told him *when* to
  wear one — before speaking, per beat, a posture while working — but never
  what any of them meant. Only three were ever named outside the enum, so
  the choice between `amused`, `delighted`, `smug` and `conspiratorial` came
  down to the English word. `Expression::gloss()` is now the one place that
  shading is written, and `mcp.rs` hands it to the model beside the enum,
  the same trade `catalog.rs` makes with visage: the tool description is
  built from the faces that exist, so a new expression cannot ship
  undefined (a test asserts every one has a line). The second half was
  never written down at all — `default_hold` splits the set into postures
  that hold at 0 and reactions that expire in 12–20 s, which wants opposite
  habits from the caller (take a posture off; never spend a call undoing a
  reaction), so it is now stated as such. The prompt (now `mind/voice.md`)
  stays terse and carries only the taxonomy and that letting go is a move;
  the definitions live in
  the tool, where they cost nothing until the verb is in front of a model.
  (Twenty-three became twenty-four immediately after — see below.)

- **No automatic face switching at all — `trouble` included (Aug 2026).**
  The redesign above left one carve-out: `trouble` when the engine is
  unreachable, on the argument that a machine fact outranks a face parker
  picked. The argument was sound and the feature was still wrong. What
  killed it was watching it: `pump_events` re-emits `disconnected` on every
  2 s reconnect attempt rather than on the transition, so one ~15 s outage
  flipped parker's face seven times while he was saying nothing. The
  debounce that would fix the flapping was considered and rejected — it
  makes the wrong thing tolerable rather than absent, and leaves the rule
  ("the face is parker's") carrying an asterisk that the next surface will
  read as permission for its own.

  So: `Expression::WEARABLE` is `ALL`, `Expression::from_sense` is deleted,
  `current_expression` falls back to `Idle` rather than to a verdict, and
  visage's `expression()` is `app.worn ?? 'idle'` with no branches.
  `code-hooks.ts` stopped moving the face on session events in the same
  change (its `trouble` call had been dead for weeks — `reach face` had
  been filtering it out all along, into a `catch` that swallowed the
  error). Nothing but parker puts a face on parker.

  The facts themselves are not suppressed, they are relocated: an engine
  that is down is visage's status strip, a permission prompt is a critical
  notification, a finished turn is a normal one. All three are things you
  can miss and then go read. A face is a thing you are already looking at,
  which is exactly why it must not be spent on anything parker did not
  choose to say. `Idle` is not a verdict about parker — it is the absence
  of one.

- **The turn's end takes off a work posture; every posture has a cap (Oct
  2026).** Postures held at 0 — until changed — and parker forgot to change
  them: a `pointing` sat on the face after the work was done, which Peter
  noticed as goofy. Two nets, neither of them the plan:
  - *Work postures* (`thinking`, `speaking`, `skimming`, `digging`,
    `pointing` — `Expression::is_work`) come off when the turn ends:
    door's `send_message` runs `parker reach settle` once its POST returns,
    and `parker-chat` does the same. They mean "mid-work", which is false
    once the work stops, so this removes a face that has become a lie
    rather than imposing one — the rule above still holds: nothing but
    parker *puts* a face on.
  - *Every* posture defaults to a ten-minute hold (`POSTURE_CAP`) instead
    of forever. The state postures (`listening`, `blocked`, `away`,
    `trouble`, `idle`) can honestly outlast a turn, so they are not
    settled — only capped. An explicit hold of 0 still means forever.

  Rejected: clearing *all* faces at turn end (a reaction worn as the last
  word, `smug` say, would vanish with the reply — reactions are left on
  their own clock), and a flat cap with no turn boundary (a forgotten
  `pointing` would still hang there for ten minutes after the work).

- **Two views, not three, and a toggle rather than an axis (Aug 2026).**
  `deep` is cut. It was the far end of a depth axis, and it did not earn the
  distance: the transcript column kept roughly the `session` width and left
  the right third of the screen empty, so stepping into it mostly shrank
  parker (noted while taking the captures — docs/visage/README.md). Paying a
  whole view for a smaller face is the wrong trade when the face is the
  point of the app. What is left, `default` and `session`, is not an axis at
  all but two shapes of the same conversation, so the keybind changed with
  it: `Ctrl+Tab` — "the other one" — replaces `Ctrl+±`, where a directional
  pair over a binary state made half of every press a no-op. `Ctrl+±` and
  `Ctrl+0` are still swallowed, bound to nothing, because WebKit reads them
  as page zoom and a zoomed visage has no way back. Names were revisited in
  the same pass and kept (`facing`/`beside` and `front`/`rail` were the
  candidates): `default` is the honest name for a landing state in a
  config-shaped repo, and both already run through the state store, the
  docs and the capture filenames.

- **visage reaches giskard and the phone as a web page parker serves (Oct
  2026).** The goal was visage on the MacBook. A native macOS app over SSH
  was the other candidate and lost on two counts. It would make giskard a
  second build target with no nix. And it would go stale whenever parkerd's
  face changed, because the page's face code must match parkerd exactly.
  PARKER.md had already settled it: giskard is a window into parker, not a
  second parker. So the page is served from the box. One build serves the
  Tauri window and the browser (`lib/shell.js` picks the transport). The
  shell's engine logic moved into `visage/door`, so the two frames share one
  copy. `porthole` sits on loopback behind `tailscale serve`, and the
  phone came along for free.

  The bar is "as strong as `ssh parker`", because the engine allows every
  tool. That bar holds through Tailscale identity rather than a second
  credential: serve adds `Tailscale-User-Login` and strips client copies,
  and the server checks it on every request. That is only sound because
  the server binds loopback alone, which it enforces in code. Writes
  additionally need our Origin and JSON, because a page from another site
  in giskard's browser rides the same tailnet with the same identity. The
  server is not in parkerd: parkerd stays sync and small until it becomes
  the daemon, and `door` is the piece that moves if it does. Nixpkgs'
  `services.tailscale.serve` was not used, because it configures Tailscale
  Services, whose tagged traffic carries no identity. The design record is
  docs/visage/porthole/README.md.

- **In a browser, the tower's keys win back rather than new keys (Oct
  2026).** Chrome eats Ctrl+Tab, so the porthole gets *immersive*
  (Ctrl+F): real fullscreen plus Keyboard Lock, which hands the browser's
  keys to the page. A web-only keymap was the alternative, and it was
  rejected: two keymaps for one face is the drift the frames exist to
  avoid. The one addition, `Ctrl+B` for dance, is in both frames, because
  macOS can own Ctrl+Space. In the same change parker gained where Peter
  is. One fixed sentence per place rides each message's `system` field
  rather than the transcript. parker asked for it, and the face following
  Peter while `show`/`look`/`notify` do not is the reason it is worth a
  line in his mind.

  *Revised the same day.* On a Mac, an installed app window beats
  immersive. Chromium reserves no keys in app windows, so the tower's
  keymap holds there without Keyboard Lock, a fullscreen Space, or taking
  over the browser. With window controls overlay, visage fills the window.
  Immersive stays as an option, not the way in. (docs/visage/porthole/README.md,
  "Second pass")

- **The second frame is the porthole, and the joke is parker's (Oct
  2026).** Peter named it knowing it sounds like what it sounds like, on
  the condition that the lore own it. So the stance lives in
  `mind/identity.md`, his file: parker insists, with dignity, that it is
  nautical, is the only one who ever raises the other reading, now and
  then, and drops the act on the phone. The docs and the code never make
  the joke; a running gag in the UI would be stale by the third visit.

- **Ctrl+Y shows a card per thread, and the card renames the thread (Oct
  2026).** The list grew to the palette's scale, grouped by day, with the
  selected thread's card beside it (option B of four on the Ctrl+Y board).
  Message snippets were the obvious card and the wrong one: an opening
  line is "sup dude" as often as not. So a `digest` agent writes it
  instead — haiku, no tools, from what was said and one line per act,
  never tool output, the ledger's own shape and rule. It says what the
  thread was about, where it landed, and whether it was left open. What
  can be counted (turns, steps, work, where Peter was) is counted by the
  page and never written by a model. The card describes rather than
  speaks as parker, because something else writing in his voice would be
  a second parker. Cards live on parker, kept by door, so the tower and
  the porthole pay for each one once. No daemon: a card is written when
  Ctrl+Y opens and the thread has moved since the last one.

  The card's title goes back to the engine, at Peter's call. opencode
  names a thread from its first message alone, which gave two "Now
  Playing"s and a "Sup Dude"; the card has read the whole thread. It is
  the engine's own record being rewritten, so the first title is kept in
  the card as `was` and survives every rewrite after it. Renaming does not
  move a thread's `updated`, so the list keeps its order (measured). The
  faces parker wore are in the data and stay out of the card: WEARS is the
  one place visage names an expression.

- **One palette, and Ctrl+Y is gone (Oct 2026).** The list and its cards
  were the best thing on screen and could not be searched; the ^K palette
  could search and showed threads as six bare titles. Two overlays with a
  flag and a toggle each also stacked: ^Y under ^K, a ^Y press closing a
  list nobody could see. Peter's call: one key. The list-and-card became
  the palette's empty state, a query line took its heading, and what is
  typed is the only state — words narrow the threads (title and card at
  once, what was said from three characters), `/` turns the same box to
  commands. Chosen over keeping both keys as two doors into one box
  (VS Code's Ctrl+P and Ctrl+Shift+P): the `/` costs one keystroke, and
  Ctrl+Shift+K was already the line scroll. The cost is j/k in the list:
  the box has focus, so a j types a j, and Ctrl+N/P move as they did.

  Commands are data with a card each (`lib/commands.js`): six verbs by
  what they touch, each saying what is on now, their choices one ⏎ away
  rather than every choice its own row — chosen over the flat list on the
  board, side by side. `/visage` is worn while it is selected and put
  back on Esc. Esc and ^K close from anywhere in one press: the box
  catches both itself, and App hands it every key when it has lost focus.

  The brain card's NAMED FOR made a brain a record in brains.nix (`model`,
  `lore`) rather than a bare model string, so what a brain *is* has one
  place to grow: a field added there reaches `parker brain --json`, door
  and the palette with no second list to keep.

- **Loose ends: next steps, and whose word a thread's state is (Oct
  2026).** LEFT OPEN was the digest's call alone, after the fact, with no
  way to say it was wrong. Now the card also writes what a thread left
  waiting and on whom (`next`: `you` or `parker`, three at most), and
  someone's word can beat it: a *mark*, Peter's from ^K's ^X or the CLI,
  parker's from his `recall mark` verb. Peter's beats parker's — parker
  cannot change one Peter made. Marks live apart from cards because cards
  are door's to write and a mark is not door's word; `parker recall mark`
  is their one writer, and door runs it, as `/brain` runs `parker brain`.

  A mark holds until the thread moves, rather than for ever: a closed
  thread that gets a new message has said more, and a permanent mark would
  hide one reopened and left hanging again. The digest then reads it told
  about the mark, so a closed thread does not come back open on its own old
  steps. Closing one that still has steps keeps them, dim, as SET ASIDE —
  not struck, because visage cannot know whether they were done or
  dropped, and not deleted, so reopening brings them back. A flip that
  agrees with the card clears the mark instead of writing one.

  ^X and not ^D: ^D is the transcript's half-page scroll everywhere else,
  and a key that means two things by context is the kind Peter trips on.
  X as in crossing it off; cut still wins when the box has a selection.
  What is open is pinned above the days (option B of the loose-ends board,
  over the dot alone), and the cursor keeps its row number rather than its
  thread, so ^X ^X ^X works down the pinned list. A closed thread drops
  into its day, by when it last moved, not to the top of the closed: the
  day headings would otherwise lie about when it happened.

  parker learns it from the `recall` verb's description (what) and one
  line in `mind/hands.md` (when: picking up after a gap, check the machine
  before reminding, one thing at most, only if it is Peter's). Not a rule
  he must follow every greeting — the repo-drift readout was cut as
  nagging material, and this would be the same — and not in the
  per-message system line, which must stay byte-identical for the cache.
  So the state is there for parker to read, cards are now also written
  when Peter leaves a thread, not only when ^K opens.

- **parker has a brain, and Peter picks it (Oct 2026).** $100 a month of
  API credit against ~$3 spent July to October made Opus affordable as the
  default, so the model became a choice: four brains in `brains.nix`, named
  for what each is for and never for how clever it is — picking the fast
  one is a choice, not a demotion, and parker has to say the name about
  himself. One per author on Peter's shelf: deep thought (Opus, Adams),
  just read the instructions (Sonnet, Banks), speedy (Haiku, Asimov),
  wintermute (Fable, Gibson). "Brain" because "mode" already means two
  things here (ghost/wired, dance) and PARKER.md's "rented cortex" is the
  engine. Calls made with it:

  - **One brain for all of parker, never one per thread.** invariant 2: a
    parker on Opus in one thread and on Haiku in another is two parkers.
  - **Applied per message, chosen in state.** The engine already takes a
    model per message, so the choice is one line in
    `$XDG_STATE_HOME/parker/brain` and switching needs no rebuild — unlike
    `theme set`, which seds a nix file. State, not runtime: a brain outlives
    a reboot where a face or a dance should not.
  - **Not a verb.** There is no `reach brain`. Spending money on his own
    initiative is what the daemon's gate is for; until then parker knows his
    brain (door names it in each message's `system` line) and asks.
  - **The helpers stay on Haiku.** Titles, `ledger`, `digest` and `probe`
    are not parker, so the brain is not theirs.
  - **`parker code`'s picker shows the brains and only the brains.**
    `enabled_providers` plus a per-provider whitelist, with the brains'
    names on the models — which also turns off opencode's anonymous free
    gateway, where a missing credential used to land without a word.
  - **Spend is not tracked yet.** Month-to-date beside BRAIN is the cheap
    version of the cost governance PARKER.md deferred; it waits until the
    numbers say it is needed.

- **visage has its own themes, apart from the machine's (Oct 2026).** The
  plan had been for nix to template visage's colours from the active Stylix
  preset. Instead visage keeps ten themes of its own (`lib/themes.js`),
  picked with `/visage <name>` in ^K, and `theme set` / `theme light|dark`
  never reach it, nor it them — "for now", in Peter's words. Calls made
  with it:

  - **The worlds' names and colours, not their wiring.** Six of the ten
    are the system presets by name, copied by value: retuning a preset
    does not move visage. glacier is the exception — it wears the light
    twin's colours (cooler, closer to visage's original), because Peter
    liked those better and the two were near enough to share a name.
    midnight, the original placeholder palette, stays the default, and
    newsprint, overcast and blueprint exist only here.
  - **The ladder is defined by contrast, not alpha.** visage's grammar was
    one ink at many alphas, tuned on a dark ground. On light grounds it
    fades: in blush the ledger target fell to 3.7:1, and no light theme's
    accent can be as bright as midnight's blue. Every rung is now solved
    per theme to the contrast it has on midnight, and the accents are held
    at 4.5:1 by moving lightness only. midnight's solve gives back its old
    alphas exactly, so nothing changed on the default. Chosen over the
    plain alphas on a design canvas, side by side in every theme ("tuned
    is superior in each instance").
  - **Per screen, not per parker.** The choice is in the page's
    localStorage, so the tower and the phone can differ. A theme is how
    one screen looks, not who parker is: placement, not pose.

- **Vignettes: the long idle pieces are built, not typed (Oct 2026).** The
  vignette phase (30 min of quiet) had a slot and nothing in it: all five
  beats were short `alert` ones. Nine pieces were designed on a canvas
  ("visage — vignettes"), each played live on visage's renderer, and
  Peter picked them: nap, chores, bonsai, reading and pong for deep idle,
  shine, power-cycle, warp and sort as short set pieces. A tenth, a
  twelve-bar blues on six strings, was cut: it did not read as music.
  Calls made with them:

  - **No new pose fields.** Every one is drawn from what the face already
    has — two cells, the ring, the pupil, the waveform, six strokes, four
    inks. The six-stroke budget turned out to be the design: a tetrahedron,
    Fox's shine and a duster are all exactly six.
  - **Builders, flattened to ordinary beats.** A vignette is minutes of
    keyframes (reading is a thousand), so it is code that writes the table
    (`face/vignette/`), run once into a `Beat`. The player learned two
    things per keyframe, set per section: its motion (a nap breathes) and
    whether the ordinary blink comes through (minutes without a blink
    reads as a frozen screen).
  - **Delivered apart from the catalog.** Flattened, they are about a
    megabyte of JSON, past what belongs in visage's bundle, so `parker
    face --vignettes --json` writes `vignettes.json` beside the catalog
    and visage loads it after first paint. The data still crosses as data
    and carries its own parity answers; only the delivery changed.
  - **Deep idle leads with them.** In the vignette phase the long pieces
    are 70% of picks (`DEEP_SHARE`) and the short beats a sprinkle. An
    even draw would have made deep idle the alert pool on a slower clock.
  - **`nap` is not the sleep state coming back.** That state (removed Aug
    2026) was a place parker went and stayed until woken, a face pretending
    to be off. A nap is a bit he finishes on his own, and any key ends it
    like any beat.

- **visage sets markdown in its own grammar (Oct 2026).** Message.svelte
  styled `p`, `ul`, `ol`, `code` and `em` and nothing else, so every other
  element parker wrote got the browser's defaults: links in #0000EE (about
  2:1 on midnight), headings in 42 px bold, tables flush against the line
  above them, `> [!NOTE]` printed literally, a bullet and a 13 px native
  checkbox side by side, and a click on a link in the porthole navigating
  the tab away from the face. Drawn on a canvas ("visage — what parker can
  write", today beside proposed) and taken as drawn. Calls made with it:

  - **A heading is a chrome label, not a title.** Uppercase, tracked, 19 px
    and quiet, h1/h2 with the palette's day-header rule. The design system
    already said no headings in prose; models write them anyway in long
    answers, so they are made to divide a reply instead of shouting over it.
  - **Accent only on what you can act on.** A link keeps the sentence's ink
    and gets an accent underline. Bold is the one place visage sets weight,
    and only because parker asked for it.
  - **Typing reveals rendered characters, not source.** The old typer
    sliced the markdown and re-parsed the slice, so syntax showed until it
    closed. Rendering once and truncating the result costs a template parse
    per tick and fixes every element at once.
  - **Tables land whole and blocks wait for the sentence before them.** A
    table lifts out of prose like a code fence; whatever follows a prose
    block still typing — a table, code, a ledger row — waits until it is
    done, so a turn reads in the order it was said.
  - **voice.md says when.** Plain prose stays the default; a table to
    compare numbers, tasks for what is open, a callout rarely. Diffs, file
    references, `show` in visage and charts in dots are IDEAS.md, one at a
    time.

- **The faces round: thirteen new faces, eleven redrawn, and entrances
  (Oct 2026).** Thirty-seven candidates were drawn on a canvas ("visage —
  faces"), each live on visage's renderer, in three sets — what parker
  could not say yet, kaomoji, and references off Peter's shelf. He kept
  thirteen: postures `waiting` (an hourglass, for a job left running),
  `weighing` (a balance) and `vibing` (head nodding to music), and
  reactions `disapproval` (ಠ_ಠ), `eye-roll`, `double-take`, `shrug`
  (¯\_(ツ)_/¯ is six strokes exactly), `table-flip`, `dead` (x_x),
  `dizzy`, `glitch`, `creeper` and `uwu`. Calls made with it:

  - **A better look takes an old name, not a new one.** Several picks
    said what an existing face already meant, better: the shades are
    `smug`, >_< is `wince`, ^^; is `sheepish`, the blush is `fond`. parker
    has learned the names and their glosses; the look under a name is free
    to change, and the vocabulary does not grow for it.
  - **The old faces were measured, then redrawn.** Counting the dots two
    held faces share showed what "some don't make sense" meant: blocked and
    listening 85% alike, suspicious and conspiratorial 83% (mirror
    images), amused and fond 78%. Two causes: `squint` shrinks only the
    right eye, and eight faces leaned on it, so they read lopsided rather
    than narrowed; and closed eyes drawn as cells are pills, deadpan's
    shape. So `delighted`, `amused`, `curious`, `melancholy`,
    `suspicious`, `skimming`, `blocked` and `conspiratorial` got new
    looks, smiles became strokes (^ ^), and a test now fails if any two
    faces share 80% of their dots.
  - **Entrances are beats, not a new system.** Seven reactions land with a
    gesture — the roll, the take, the shades coming down, the table turning
    over, the wink snapping shut. An entrance is a `Beat` that departs from
    whatever face was showing (`Beat::departing`) and ends on the new pose
    (`face/entrance.rs`); it crosses to visage in catalog.json as data. Not
    a new pose field and not a new player.
  - **The catalog checks the hand-ported poses now.** `face.js` still
    ports expressions by hand, but the catalog carries every pose, strokes
    and inks included, and samples of the new motions; `parity.mjs` fails
    on any difference. Thirty-seven faces with up to six strokes each is
    past what "line for line" survives unchecked.
  - **Jokes are short, and some faces must not blink.** The gestures and
    jokes let go after six to ten seconds (a table flipped for twenty is
    not a joke); the tool says so. `waiting`'s bulbs are hollowed eyes and
    `conspiratorial`'s open eye would make a wink two eyes shut, so
    `Expression::blinks` keeps the blink off both.

  Cut: searching (a KITT scanner), writing, relieved, concerned, updating,
  greeting, mind blown, woo, flex, ayy, starstruck, BMO, HAL, murderbot,
  iai, `<|endoftext|>`, guitar face, and — after Peter kept them for a
  while — annoyed and touched.

- **No bar: the settings live in visage, on ^, (Oct 2026).** It began as
  restyling noctalia's SUPER+B bar, and the feasibility round found its
  ceiling: v5 is native C++/GLES, not Quickshell; its bar config is broad
  but its Luau plugins draw from a fixed node set and cannot replace the
  control center; anything shaped would mean a shell of our own. Then
  Peter turned it around — SUPER+P already brings visage over anything,
  fullscreen games included, and visage can draw anything, is
  keyboard-first, and has parker in it. So noctalia's bar is off, SUPER+B
  and `parker-bar-veil` are gone, and the whole record — the 26 bars and
  five control centers drawn first, every call, every row's mechanism — is
  docs/visage/controls/README.md. Calls made with it:

  - **One list, three ways in.** Every setting is defined once, in parkerd
    (`parker controls`, controls/): what it is, what it is now, how it
    changes. ^, draws it, ^K's "/" for the machine is generated from it,
    and parker reaches it as the `controls` tool. A setting added there
    is in all three.
  - **The takeover, in vim's keys.** ^, gives the screen the session shape
    and the transcript's column becomes two columns of rows. NORMAL, EDIT,
    FIND and ASK; `i` to talk rather than any letter, since most letters
    are keys; a toggle flips on ⏎; esc never closes ^,; no arrow keys.
    The key strip shows five things and `?` the rest — Peter found the
    whole map dense at first sight and right once read.
  - **A per-app mixer, by WirePlumber's own memory.** `all sound` and
    `apps ▸` under it: one row per app with a stream open, levels set
    with wpctl and remembered by WirePlumber per `application.name`.
    Proton's exe names and launcher streams are folded or dropped
    (`friendly`).
  - **The corner says where you are.** A fourth line, `space 1 2 3 4`,
    following Hyprland's event socket. It bends the design system's
    "the corner is his body"; the corner had the room, and nothing else on
    screen says it now that the bar is gone.
  - **parker's changes are drawn.** A level he moves shows as cells from
    where it was to where it is, a device as a dot, a goto as the spaces;
    the verbs report from → to so the page never guesses. Those rows stay
    out of the ledger's fold.
  - **The porthole is not the tower.** Wifi off, tailscale and power off
    would cut the line the page rides on, so from the porthole they are
    read-only, and parkerd refuses them too (`--away`). Tailscale runs
    with `--operator=peki` so the tower can flip it without root.
  - **Night light is remembered, not read.** noctalia can force it but not
    say whether it is on, so parker keeps what he last set in the runtime
    dir. Notices are read from noctalia's history into visage rather than
    opened in a panel over it.

- **Dev: uv owns Python** (versions, venvs, packages, inline-metadata
  scripts) — no global Python, no Nix-native Python management. `nix-ld`
  enabled system-wide so prebuilt wheels (torch) link, with the driver's
  libcuda on its path (system/nvidia.nix) so they see the GPU. Escalation ladder:
  uv → uv+nix-ld → per-project flake + direnv (only for CUDA toolkit/nvcc or
  odd system libs). Daily CLI tools stay global.
- **Branding opencode without forking it: use the plugin seams, not a
  patched build.** `code.nix`'s header used to say deeper branding "goes
  through source patches on the nixpkgs build". That turned out to be the
  worse of two options. opencode 1.18 ships a documented TUI plugin system
  (`packages/opencode/specs/tui-plugins.md`) whose host renders the home
  wordmark as `<Slot name="home_logo" mode="replace">` — the slot exists so
  it can be replaced. A plugin is one file parker owns, carried across
  opencode bumps untouched; a patch is a `substituteInPlace` that breaks
  whenever upstream reformats the file it targets, plus a local bun bundle
  on every version bump. Verified on 1.18.4 that a plugin loads from a
  read-only store path with no `node_modules` beside it — the JSX runtime
  resolves out of the compiled binary — which is what makes this compatible
  with "all configs compiled". The same seam reaches routes, keybinds,
  sidebar panels and dialogs, so a future parker panel is the same file.
  Patching stays the fallback for anything with no slot.
- **Gaming: supported citizen, not special.** Fullscreen games are normal
  Hyprland windows — instant space-switching, Super+F un-fullscreens in
  place. No gamescope infrastructure; it's a per-game launch-option tool for
  misbehaving titles. Steam via the module, Melee via slippi-nix. Hardware
  quirks handled reactively from `quirks.md`, not preemptively.
- **Emulation: nixpkgs-native, no flatpak.** PCSX2 + RPCS3 as plain user
  packages in `home/apps/emulation.nix`. The community advice of "flatpak
  for the fast movers" was considered and declined: nixpkgs tracked upstream
  closely enough at adoption time (Aug 2026), and the monthly update ritual
  keeps pace. Flatpak becomes its own decision only if a game ever needs a
  fix nixpkgs hasn't shipped. Emulator configs (`~/.config/PCSX2`,
  `~/.config/rpcs3`) are runtime-owned — a deliberate, larger cousin of the
  btop acceptance: BIOS, firmware installs, shader caches and per-game
  settings can't sensibly be compiled. BIOS dumps, PS3 firmware and
  extracted PS3 games live in `~/gaming/emulation/{bios,ps2,ps3}`, disc
  images on the HDD archive (symlinked in), never in the repo.
- **PC games outside Steam: Heroic + umu on GE-Proton, one prefix per game
  (Sept 2026).** Same Proton build Steam uses; no separate Wine — Wine-GE is
  dormant and Lutris/Heroic both launch GE-Proton through umu now. Heroic
  is the GUI (per-game Proton version, prefix, env vars, winetricks),
  `umu-run` the CLI for the same thing, `protontricks`/`protontricks-launch`
  for running an installer or mod exe inside a Steam game's prefix, `limo`
  for native load-order modding; plain "unzip into the game dir" stays the
  default mod install. Lutris and Bottles declined: nixpkgs Lutris has open
  NVIDIA-ICD and umu breakage (#467942, #465252), Bottles' nixpkgs sandbox
  is a no-op (#236590). Layout: `~/gaming/{pc,prefixes,emulation}` on the
  NVMe (games want the fast disk; few installed at once),
  `~/gaming/archive -> /mnt/hdd1/gaming-archive/{installers,discs}` for the
  acquired files so a reinstall is one step. Heroic config is runtime-owned
  like the emulators'. GameMode daemon on, engaged only per title.

- **Bluetooth on, for the DualSense (Sept 2026).** `system/bluetooth.nix`:
  BlueZ, powered at boot, `Experimental` for battery readout,
  `FastConnectable` so a PS-button press reconnects a sleeping pad. The
  radio (Intel CNVi, hci0) and firmware were already live — this only adds
  the daemon. Control via bluetui (utility principle), glance via Noctalia's
  control center; no blueman, no bar widget. The pad's touchpad is hidden
  from libinput by a udev rule (`system/gaming.nix`) so it never drives the
  desktop cursor; games talk hidraw (Steam's uaccess rules cover USB and BT)
  where the touchpad click is a plain button — SDL/Proton expose it as the
  "touchpad" gamepad button, Steam Input can rebind it per title.

## Repo conventions

- Structure as scaffolded: `hosts/parker/`, `system/` (by concern), `home/`
  (by role: desktop, terminal, nvim, tuis, apps, theming), `themes/` (data).
- Entry points compose, never configure. Leaf modules are self-contained and
  deletable. `misc.nix` catches three-liners until they earn a file. A tool
  that outgrows its role-group gets promoted to its own directory.
- `AGENTS.md` per top-level directory (warden docs next to the code);
  `README.md` only where a human would read one. *Renamed from CLAUDE.md,
  October 2026:* three agents read these files — Claude Code (natively
  since 2.1.277), `parker code` and the engine parker (both opencode,
  which looks for AGENTS.md first). One vendor-neutral name serves all
  three with no imports or symlinks; the content is a repo contract, not
  a Claude-specific one. No CLAUDE.md may coexist beside one — Claude Code
  would read only the CLAUDE.md.

## Settled at scaffolding time (July 2026)

- **home-manager as a NixOS module**, not standalone. One command
  (`nixos-rebuild switch`), one atomic generation, one rollback story, one
  CI target. Cost (sudo + full eval for home tweaks) is irrelevant when
  rebuilds are Claude-driven. The ~30s theme switch survives.
- **v0 ships zero secrets.** sops-nix is wired at the flake level but
  nothing depends on it at install. Bootstrap happens post-first-boot from
  parker itself (recipe in `system/sops.nix`): host key → age recipient →
  re-key. Wifi stays interactive nmtui, per the networking decision. *Still
  true as of July 2026 — the bootstrap has never been run.* With the repo
  private (see Philosophy), running it is no longer a prerequisite for the
  first credential; it becomes a deliberate upgrade rather than a gate.
- **The password is not in the repo** (*was `initialHashedPassword` until
  October 2026*). It gates sudo, not entry (autologin), and sshd is
  key-only. The hash sat in the repo from install day, within the threat
  model of the time; the public mirror (Philosophy) is what took it out.
  Users stay mutable, so the password lives in /etc/shadow and removing
  the hash changed nothing on the running system; a reinstall sets it with
  `passwd` (RUNBOOK.md, step 8).
- **hardware.nix pre-baked, verified on install day.** Standard desktop
  initrd module set + Intel microcode + redistributable firmware (iwlwifi!)
  written blind; the runbook diffs it against
  `nixos-generate-config --no-filesystems` in the live env. disko owns all
  filesystem declarations either way.
- **Dotfiles are copied, not referenced.** Config contents (nvim lua,
  zellij kdl, p10k.zsh, TUI configs) were copied from the giskard dotfiles
  repo into `home/` and are now parker's canonical copies. No flake input on
  the dotfiles repo — that would couple the machines (rejected above).
  In transit: `term` CLI retired (superseded by the future `theme` command;
  its curated theme favorites remain inspiration for `themes/` presets),
  thefuck dropped (unmaintained upstream, barely used), macOS yazi openers
  replaced with a Linux baseline, holycode git identity stays on giskard.
- **CI is the gate.** GitHub Actions (determinate-nix-action) builds the
  full closure on every push; giskard is aarch64-darwin and cannot build or
  even lock the flake, so the first CI run generates and commits flake.lock,
  and the green check is the only pre-install build signal. *Retired (July
  2026):* the gate existed because there was no machine to build on. parker
  now boots and rebuilds in seconds-to-minutes, so it is both the target and
  the build check — the runner was re-doing on a cold cache (40+ min, both
  theme variants) what parker does warm. Free-tier minutes ran out, which
  made the redundancy the deciding cost. Consequences accepted: nothing
  proves `main` builds except rebuilding on parker, and flake.lock is now
  hand-locked there (`nix flake lock`) instead of committed by the runner.
  Reinstate if a second host ever joins or the MacBook needs to push
  blind-but-verified changes.

## Hardware facts (captured from the old install, July 2026)

- **OS disk (wiped by install):** Samsung 980 500GB NVMe
  `/dev/disk/by-id/nvme-Samsung_SSD_980_500GB_S64DNL0W226998W`
- **Data drives (ext4, remounted, never touched):**
  - `ata-WDC_WD10EZEX-08WN4A0_WD-WCC6Y2EJ0D5V-part1` — 1TB, label HDD1 → `/mnt/hdd1`
  - `ata-WDC_WD5000AAKX-001CA0_WD-WCAYUEK84641-part1` — 500GB, label HDD2 → `/mnt/hdd2`
  - `ata-KINGSTON_SHFS37A120G_50026B777804CE75-part1` — 120GB, label SSD → `/mnt/ssd`
- **Network:** wifi `wlan0` (Intel iwlwifi) is the live connection; ethernet
  `enp3s0` (Realtek r8169) exists but has no cable at this location.
- **Cooling:** nct6798 motherboard controller (7 fan headers: 3 front
  intakes, rear exhaust, the Kraken's pump tach on fan3, two empty) + NZXT
  Kraken 2023 AIO (pump + one fan channel for the 3 top radiator fans,
  HID). Both managed by CoolerControl; map in docs/COOLING.md.
- **Monitor:** AOC Q27G42XNE, 2560x1440 @ 180Hz, DP-3.
- **Pre-install backup** on `/mnt/hdd1/parker-backup/`: ssh keys, Slippi
  config + replays, coolercontrol config, hardware captures.

## Deferred (real options, deliberately not now)

NordVPN · impermanence · nixvim/plugin
pinning · theme preview mode · ambient/showcase mode · custom dock ·
scratchpad experiment for space 4 · Scarlett low-latency setup · giskard
absorption into the home layer

## Verify at build time

- Freshness of Claude Desktop nix packaging against the official .deb.
- nvidia suspend/resume behavior with power-management options.
- slippi-nix actually shipping the GameCube adapter udev rule (quirks.md).

Answered, kept as a record (Oct 2026 pass): the 180Hz idle draw quirk is
gone (Home → "180Hz declared permanent"); the lazy.nvim lockfile lives in
`stdpath("state")` (`lua/config/lazy.lua`); the Electron citizens run
native Wayland through the global `NIXOS_OZONE_WL` (`system/desktop.nix`)
with no per-app flags.
