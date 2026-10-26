# visage — captured states

Screenshots of the live app (release binary on `special:visage`, DP-3,
2560×1440), taken 2026-08-12 by driving the real keymap against a real engine
session. They exist to iterate on the design without reproducing each state by
hand.

The axis is depth into the conversation — `home` > `prompt` > `session` >
`deep`, parker shrinking monotonically along it
(`../../home/parker/visage/AGENTS.md`).

| file | state |
|---|---|
| `01-home.png` | `home`, idle. Just parker, fullscreen. |
| `02-prompt-typing.png` | `prompt` before sending — opened mid-keystroke by typing at `home`; box low and narrow, parker over the top two thirds. |
| `03-prompt-thinking.png` | `prompt`, request in flight: user bubble, `…` placeholder, box reading "parker is thinking…", face in the thinking ink. |
| `04-prompt-answer.png` | `prompt` with an answer — parker leans back to about half, the sense block and reply fill the band between him and the box. |
| `05-session-transition.png` | the move into `session`, caught ~1.2 s in: parker mid-morph and running off the left edge, the chat already reflowed. |
| `06-session-streaming.png` | `session` mid-stream, tool calls arriving. |
| `07-session-answer.png` | `session` with tool cards (find / ls / grep) and prose. |
| `08-deep.png` | `deep` — parker a marble on the left, transcript owning the screen. |
| `09-history.png` | Ctrl+Y earlier-threads overlay over `deep`, first row selected. |
| `10-dance-a.png`, `11-dance-b.png` | dance mode (Ctrl+Space) at `home`, two frames of the lens. |
| `12-home-amused.png` | a worn expression at `home` (`parker reach face amused`). |

Two things visible in the captures, noted while taking them:

- The **"engine unreachable — retrying"** banner sits bottom-right in `03`
  and `04` while the request was in fact streaming fine, and the face carries
  the error ink through the whole reply. The banner is reporting the SSE bus,
  not the request.
- The `prompt` → `session` transition (`05`) carries parker off the left
  edge of the screen before he settles into the rail — a second capture at
  the same moment would tell whether that overshoot is the helix or a clip.
- In `deep` the transcript column keeps roughly the `session` width and
  leaves the right third of the screen empty, so the step from `session` to
  `deep` mostly moves parker rather than giving the chat the space it claimed.

To retake: bring visage up (`parker visage`), drive it with real
keystrokes while it is focused — `wtype` from the pinned nixpkgs (`nix
build github:NixOS/nixpkgs/<flake.lock rev>#wtype`), e.g. `wtype -M ctrl
k -m ctrl` for Ctrl+K — and capture with `grim -o DP-3 <file>` (needs
`WAYLAND_DISPLAY` and `HYPRLAND_INSTANCE_SIGNATURE` in a non-graphical
session). The old `hyprctl dispatch sendshortcut …` form is a Lua syntax
error since the Lua config.
