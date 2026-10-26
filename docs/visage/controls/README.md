# ^, — the controls, in visage

Status (Oct 2026): built (DECISIONS.md, "No bar"). This was the design
record, written before any code so the decisions would survive; it is now
the plan it was, with what was built and what is still open at the end
("Built", "Open").

- Design sheet (final): https://claude.ai/artifact/LZGpzvWdJM1dZqECza7o8F,
  "visage — controls". Seven boards: visage at rest with the spaces line,
  a live keyboard sketch of ^,, the mixer open, the keys, parker's changes
  drawn in the transcript, the phone, and the stage they stand on.
- Exploration (how we got here): https://claude.ai/artifact/N6qQwwF2TXDX23jMUdq8YK,
  "parker — the bar". Page "the bar" has 26 bar and 5 control-center
  designs. Page "visage controls" has the first two rounds of this design.

## How we got here

It started as "restyle the SUPER+B bar". What the first round found:

- **noctalia v5 is not Quickshell.** v4 was QML on Quickshell. v5, which we
  run, is native C++/GLES (DECISIONS.md). Our pin is a July beta (rev
  9e406c6); upstream reached stable 5.3.0 on 2026-10-15, plugin API 34.
- **noctalia's bar config** covers a lot: any edge, several named bars,
  floating (`margin_edge`), per-corner radius, `concave_edge_corners`,
  capsules and capsule groups, and font and colour per bar or widget.
- **noctalia's Luau plugins** draw from a fixed node set: box (fill,
  radius, border), label, glyph, image, graph, progress, slider, toggle,
  and a few others. There are no free shapes, paths or shaders. Plugins can
  add bar widgets, panels, desktop widgets and control-center tiles, but
  they cannot replace the control center.
- **Anything shaped** (a notch, a screen frame, morphing panels) needs a
  shell of our own. Quickshell (QML, nixpkgs 0.3.2, pre-1.0) is the only
  option with no ceiling. Waybar and ironbar are CSS only, AGS/Astal is
  slowing down, eww is in maintenance, and HyprPanel is archived in favour
  of Wayle.

Then Peter turned it around (2026-10-17): **no bar at all.** SUPER+P
already brings visage over any window, fullscreen ones included, and
visage has everything a bar needs and more:

- It is a webview, so it can draw anything.
- It is keyboard-first.
- parker is right there to ask.

So the controls live in visage, on `^,`.

What that means for the shell:

- noctalia's bar goes off (`bar.main.enabled = false`).
- noctalia keeps notifications, the OSD, the launcher, the lock screen,
  the wallpaper and the dock.
- SUPER+B and its `noctalia-bar-pin` script go.
- `parker-bar-veil` goes too: it only existed to hide the bar while a face
  was up.
- DECISIONS.md's line "the shell owns bar, launcher, notifications, OSD,
  quick menus" changes with it.

## visage at rest: where you are

The corner gains a fourth line, `space 1 2 3 4`, with the current space in
accent. It is set exactly like the three lines above it (20 px,
`fg-28`). Peter's pick, after trying a top-left mirror of the corner,
four of the face's dots over parker's head, and a minimap of the spaces.

Everything else on the screen stays as shipped. The keys row gains
`^, CONTROLS`.

This bends the design system's rule that "the corner is about his body:
the hour, his load, his heat". The corner now also says where you are.
Update the design-system README when this is built.

Spaces can change while visage is up: visage sits on a special workspace,
so it stays over whatever space is underneath. The line must be live.
Source: `parkerd/src/sense/hypr.rs` already has `workspaces()`. door needs
a feed of it, or Hyprland's event socket.

## ^, — the overlay

- **Layout: the takeover** (Peter's pick). `^,` moves visage into its
  session shape, with parker on the left rail, and the transcript's column
  becomes two columns of rows:
  - left: SOUND, PLAYING, NETWORK, BLUETOOTH
  - right: LOOK, QUIET, PARKER, SESSION

  It adds no new floating object. The input line stays at the foot and
  becomes the find or ask line. Peter also liked the floating sheet's
  look, ^K's surface over the scrim. The live board shows both
  (`layout` tweak), and the sheet may suit the porthole.
- **Rows:** the label, then what it is in `fg-35`, then the value drawn,
  then the value as a number or word. Accent is spent on state only: the
  selected row's band, the current option, the space you are on.
- **Sliders are block cells** (Peter's pick, over the meter ramp, the face's
  dots and a hairline with a caret).
- **States:**
  - selected: the palette's `accent-14` band with a 2 px rule
  - editing: outlined in accent, with an `EDIT` tag
  - in flight: the ledger's act sweep under the row, never a spinner
  - failed: `bad`, always with a word, since the forge's accent is red too
  - unavailable: dimmed, saying why ("bluetooth is off")
  - tower-only on the porthole: read-only, "on the tower only"
  - moved elsewhere: the row says so once ("changed by the volume keys")

## Keys — vim, four modes

The key strip shows at most five things for the current mode. `?` opens
the full card on the palette's surface. Peter found the full map dense at
first glance and intuitive once read, so the strip stays short.

| mode | keys |
|---|---|
| NORMAL, where you land | `j k` move, with a count (`3j`) · `h l` the other column (the next section in the sheet layout) · `gg G` first and last · `^N ^P` as in ^K · `⏎` change the row, or open a group · `space` flip a toggle, play/pause · `m` mute a sound row · `/` find · `i` ask · `?` keys · `^,` put the controls away |
| EDIT, `⏎` on a row | `h l` −1/+1 or the next choice · `H L` −10/+10 · digits type the value · `m` mute · `⏎` keep · `esc` put it back. Session (lock, sleep, reboot, off) is `⏎ ⏎`, and the second press is parker asking: "⏎ again turns me off. esc and I stay." |
| FIND, `/` | typing narrows the rows · `⏎` jumps to the first, or runs a full command: `volume 30`, `volume spotify 20` · `esc` clears |
| ASK, `i` | typing goes to parker, sent marked as from ^, with what ^, shows (the selected row) as context · `⏎` send · `esc` back |

Calls made:

- **`i` to talk, not plain typing.** In NORMAL, `j k h l g i` are all keys,
  and many sentences start with those letters ("just turn it down"). The
  live board's `typing` tweak tries "any unbound key starts the chat".
- **A toggle flips on `⏎` without entering EDIT:** two states need one press.
  Routing everything through EDIT was Peter's other idea and is a one-line
  change.
- **`esc` never closes ^,;** only `^,` does. `esc` clears a find, leaves a
  mode, or undoes an edit.
- **Arrow keys are not bound** (Peter: vim, no arrows).

## One list, three ways in

Each setting is defined once, in parkerd: id, section, label, kind, how to
read it, how to change it, its slash name, and whether it is tower-only.
Kinds are slider, toggle, choice, action, info, transport, session and
group. That one list feeds three things:

1. **^, draws it.** door gains a snapshot command and a set command, next to
   `sense_snapshot`.
2. **^K's "/" is generated from it** into the `THE MACHINE` domain, which
   `visage/src/lib/commands.js` already declares. So `/volume`, `/glass`,
   `/wifi` and the rest exist without a second copy.
3. **parker gets it as a tool** (parkerd `mcp.rs`), so "turn it down a bit"
   or "connect the controller" works from the chat. Today he already has
   `media` (MPRIS and `wpctl` volume), `goto` (spaces) and `focus` (apps).

### What is in the list, and how each one works

Everything here runs as Peter, without root. Checked on 2026-10-17.

| section | row | kind | how | porthole |
|---|---|---|---|---|
| SOUND | all sound | slider | `wpctl set-volume @DEFAULT_AUDIO_SINK@ 42%`, as `parkerd reach media` already does | yes |
| SOUND | apps ▸ (the mixer) | group | see below | yes |
| SOUND | mic | slider | `wpctl set-volume @DEFAULT_AUDIO_SOURCE@` (the CM108 USB mic) | yes |
| SOUND | mute | toggle | `wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle` | yes |
| PLAYING | the track | transport | MPRIS, `reach media` transport | yes |
| NETWORK | wifi | choice | `nmcli` (peki is in the `networkmanager` group); on PEKI | turning wifi off is tower-only |
| NETWORK | tailscale | toggle | `tailscale up/down`, which **needs root today**: add `--operator=peki` (`services.tailscale.extraSetFlags`) | tower-only |
| BLUETOOTH | bluetooth, wireless controller | toggle, action | BlueZ over D-Bus (zbus is already in parkerd) or `bluetoothctl`; works as peki | yes |
| LOOK | desktop (light/dark/auto), family | choice | `theme light/dark/auto`, `theme set <family>` (the sudo rule exists) | yes |
| LOOK | glass | slider 50–100 | `theme transparency <v>`: instant via hyprctl, then baked by a rebuild | yes |
| LOOK | night light | toggle | `noctalia msg nightlight-toggle` | yes |
| LOOK | this screen | choice | visage's own theme, per screen (`/visage` exists) | yes |
| QUIET | do not disturb | toggle | `noctalia msg notification-dnd-toggle` (also `-set`, `-status`) | yes |
| QUIET | notices | info | noctalia's notification history | yes |
| PARKER | brain, dance | choice, toggle | already in ^K (`/brain`, `/dance`) | yes |
| SESSION | lock, sleep, reboot, off | session, ⏎⏎ | `loginctl lock-session`, `systemctl suspend/reboot/poweroff` (logind allows the active session) | off is tower-only; reboot is a hold |

**The mixer (per-app volume).** Peter wants separate levels for Spotify,
games and Discord, plus one for everything.

- `all sound` is the output itself. `apps ▸` opens under it like a
  dropdown, with one row per app making sound right now.
- Streams come from `pw-dump` (`media.class` Stream/Output/Audio, named
  by `application.name`).
- Set one with `wpctl set-volume <id> 45%` and `wpctl set-mute <id> toggle`.
  `pactl` is not installed; `wpctl` and `pw-dump` are.
- An app's level multiplies with `all sound`.
- **WirePlumber already remembers each app's level** by `application.name`
  (`~/.local/state/wireplumber/stream-properties`). spotify and Dolphin
  (Melee) sit near 45% there today, so a level set in ^, sticks across
  restarts for free.
- An app only has a row while it has a stream open. Discord, for example,
  shows during a call.
- **Proton games show up by their `.exe` names**, sometimes twice: the exe
  and a pretty name, like `Titanfall2.exe` and `Titanfall 2`. Launchers add
  noise too: `upc.exe`, `EADesktop.exe`, `SocialClubHelper.exe`. The mixer
  needs a small map to a friendly name, and a filter for launchers.
- There is one output today (HDMI to the monitor's speakers). A device
  choice only matters once a second output exists.

## parker's changes, drawn

visage already receives parker's verbs as tool calls with their arguments
(`parker_media` with an action and percent, `parker_goto` with a space…).
`state.svelte.js` turns them into ⊙ ledger rows today. A renderer per verb
can draw the change instead of only naming it:

- cells going from 45 down to 20 for a volume, the whole output or one app
- paired → connected for bluetooth
- swatches, hangar → redline, for a theme flip
- the space numbers, with the new one lit, for a `goto`

Changes made outside the chat, like the volume keys on the desk, need door
to watch PipeWire (`pw-dump --monitor`, or `pw-mon`). That watch also
gives the "moved elsewhere" row state, and a quiet event line in the
transcript: "all sound 42 → 50 · the keys on the desk, not parker".

## The porthole

The porthole serves the same page, so the controls work from giskard and
the phone too.

- **Touch:** rows at least 44 px, tap a group to open it, sliders drag. The
  phone layout is still the open item it was.
- **Tower-only rows:** wifi off, tailscale and power off. Each would cut
  the line the porthole rides on, or could not be undone from afar.
- **Reboot from the porthole is a hold.** The page comes back about a
  minute later.

## Built

Where each piece landed:

- **The list:** parkerd `src/controls/` — `mod.rs` (rows, the tower rule,
  set, the text form), `sound.rs` (one pw-dump, wpctl, the mixer and its
  `friendly` names), `network.rs` (nmcli, tailscale), `bluetooth.rs`
  (BlueZ over zbus), `desktop.rs` (theme, noctalia, the session). CLI:
  `parker controls [--json]`, `parker controls set <id|name> <value>
  [--away]`.
- **door:** `controls`, `set_control` (id and value checked for shape,
  handed over after `--`), `spaces` and `watch_spaces` (Hyprland's event
  socket). The Tauri window sets with `away` false; the porthole always
  with `away` true and pushes `spaces` down its event stream.
- **visage:** `controls.svelte.js` (state and the four modes' keys),
  `Controls.svelte` (the takeover), App's ^, and its routing, the
  corner's spaces line, `^, CONTROLS` in the keys row, ^K's machine verbs
  generated in `commands.js` (plus `/controls`), renders in
  `state.svelte.js` `changeOf` and Message.svelte, and the ask's hidden
  first line (`⟦parker:controls⟧`, stripped by `visible` and by recall).
- **parker:** the `controls` tool (mcp.rs); `goto` and `media` volume now
  say where they came from.
- **The shell:** noctalia's `bar.main.enabled = false`, SUPER+B and
  `parker-bar-veil` gone, the top gap no longer clears a bar, Tailscale
  with `--operator=peki`.

- **The PipeWire watch** (Oct 2026): door's `watch_levels`, beside
  `watch_spaces`. `pw-dump --monitor` is only the doorbell — a node whose
  params changed, or a node coming or going, never a client — and the
  levels are then read through parkerd (`parker controls --json --only
  sound`, one pw-dump), settled for 250 ms (1.2 s at most), and diffed by
  row. What parkerd set is not said: every sound set in parkerd (^,,
  parker's tool, `reach media` volume, the CLI) is stamped first in
  `$XDG_RUNTIME_DIR/parker/sound-hand` (sound.rs, "whose hand"), and a
  level that moved to a stamped value is the ledger's. A row that just
  appeared is left alone for 3 s (WirePlumber restoring an app's level is
  nobody moving it). "The keys on the desk" means a `wpctl` was connected
  as the level moved, or had left within 300 ms, which is what the
  Super+Alt+J/K binds are; anything else is "changed elsewhere". The event
  is `levels` in both frames (the porthole broadcasts it rather than
  holding it in a watch: a page that connects later must not hear it as
  news). The page keeps the lines itself (`eventLines`, state.svelte.js),
  per thread, woven into the transcript by time, so a refetch never drops
  them; a held key is one line, and a line that comes back to where it
  started goes. While ^, is up, the same event re-reads the list at once
  and the row says "changed by the volume keys".

How it differs from the drawings: the takeover only (no sheet layout); a
toggle flips on ⏎; the transport row's EDIT is h previous, l next, space
play; ^K has no session commands (one ⏎ in a list is too few for off);
`notices` is a group of the latest six, read from noctalia's history.

## Open

- **The PipeWire watch, after the build.** Built (above) and checked on a
  scratch porthole against a silent `parker-test-tone` stream and a 1%
  move of the sink. Not yet seen on the tower itself: it needs this
  switch, and visage rebuilt by hand (`cargo tauri build --no-bundle`).
  Two edges known and left: a `wpctl` that only read a level, a moment
  before an app moved it, makes that move "the keys"; and the keys moving
  a level to a value parker set within the last 5 s are taken for parker.
- **The phone.** The porthole gets the same takeover at the same pixels,
  reachable by `/controls` in ^K; a phone layout (the Phone board) is
  still the open item it was, and so is whether it wants the sheet.
- **Theme swatches in the renders** (hangar → redline): a mode or family
  change is drawn as two words for now.
- **Re-sync the design-system artifact:** the corner rule, `^,` in the keys,
  and the new states.
- **Bump noctalia** from the July beta, or not; nothing here needed it.
