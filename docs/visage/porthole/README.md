# the porthole — parker's face from giskard and the phone

Status (Oct 2026): live, and in daily use from giskard. This is the plan
and the design record; the decision itself is summarised in DECISIONS.md.
It was called "the web frame" (crate `visage-web`) until the third pass
named it.

## What and why

visage is a Tauri app that only runs on parker's own screen. The goal is the
same face and the same conversation from the MacBook (giskard) and the phone,
over Tailscale. PARKER.md already decided the shape: *giskard is a terminal,
not a second parker — the MacBook is a window into it.* So parker serves the
page and the other devices only look at it. Nothing is built or installed on
giskard, and the page cannot drift from the parkerd it talks to because nix
deploys the two together.

A native macOS app over SSH was considered and rejected. It would make giskard
a second build target with no nix. It would also go stale whenever parkerd's
face changes, because `face.js` and `catalog.json` must match parkerd
exactly (visage/AGENTS.md). Webapp now, possibly forever. A thin native
wrapper that only knows the URL stays possible later, because it would hold
nothing that can drift.

## The seam that makes this small

`src/lib/engine.js` is already the page's only door to anything, and the Rust
shell behind it is the only part that touches the machine. The Svelte face,
the transcript and the catalog run anywhere unchanged. Going remote means
carrying these calls over HTTP instead of Tauri IPC:

| call | does | over the web |
|---|---|---|
| `create_session` `get_messages` `send_message` `list_sessions` `interrupt_session` `summarize` | HTTP to opencode, 127.0.0.1:4096, password | the same function, behind `POST /api/<name>` |
| engine SSE pump → `engine-event`, `engine-status` | one upstream stream | fanned out to every page over `GET /api/events` |
| `face_state` | reads `$XDG_RUNTIME_DIR/parker/{face,dance,beat}`, polled every 250 ms | **pushed**: the server polls locally and sends `face` on change; the page's 250 ms poll is answered from the last push, never the network |
| `sense_snapshot`, `dance` | run `parker sense --json` / `parker reach dance` | the same function, behind `POST /api/<name>` |
| `theme` | the palette's `/theme`, the `theme` CLI behind door's allowlist | the same function and the same allowlist |
| `dismiss` | `parker visage` (Hyprland) | no-op — there is nothing to put away on a Mac |

## Shape

```
giskard / phone ──tailnet──▶ tailscaled ── tailscale serve, https://parker.tail1b2a09.ts.net
                             (adds Tailscale-User-Login, strips client copies)
                                  │
                                  ▼ http://127.0.0.1:4097
                             porthole  (user service, axum)
                              ├─ static: the same dist the Tauri app embeds
                              ├─ POST /api/<command>   ─┐
                              └─ GET  /api/events (SSE) ├─ visage-door (lib)
                                                        │   engine client · face_state · sense · dance
parker's own screen:  visage (Tauri) ── tauri commands ─┘
                                                        ▼
                                             opencode serve 127.0.0.1:4096
```

Three crates under `home/parker/visage/`:

- **`door/`** (`visage-door`, lib) — the bodies of today's tauri commands,
  moved verbatim with their trap comments: the engine client, the six engine
  calls, `face_state`, `sense_snapshot`, `dance`, and the SSE pump. One
  implementation, two frames. No tauri, no axum.
- **`src-tauri/`** — keeps the window, `dismiss`, `sane_dpi` and the WebKit
  workaround. Its commands become one-line calls into `door`. Behaviour is
  unchanged.
- **`web/`** (`porthole`, bin) — axum: the gate, `/api/*`, `/api/events`,
  and static files from `--dist`. No logic of its own beyond fan-out.

The server is not in parkerd. parkerd is deliberately sync and small, and its
Cargo.toml calls an async runtime a daemon-era decision. The engine client
already lives in visage's shell, so the server goes next to it rather than
duplicating it into the body. If parkerd becomes the daemon PARKER.md
describes (faces → parkerd → engine), `door` is the piece that moves.

## The page: one build, two shells

One `dist` serves both. A new `src/lib/shell.js` exports `invoke` and
`listen`. Under Tauri it re-exports `@tauri-apps/api`. In a browser it does
`fetch('/api/<cmd>')` and `EventSource('/api/events')`, with the same command
names, argument shapes and rejection-as-string. `engine.js` changes only its
import, and `state.svelte.js` does not change at all.

Web-only differences, all keyed on `<html data-shell="web">`:

- the cursor stays visible (`cursor: none` is for the fullscreen tower
  surface only)
- JetBrains Mono is a fallback web font (`JetBrains Mono Web`, served from
  `dist/fonts`). Browsers only fetch a fallback face when the installed one
  is missing, so the Tauri app never downloads it.
- if the event stream drops, the page reports `engine-status: disconnected`.
  The existing status strip then says so, which also covers parker being
  powered off.

## Security

The engine allows every tool, so whoever reaches `/api/send_message` can run
bash as peki on parker. `ssh parker` from giskard already grants exactly that,
so the bar is *as strong as SSH*. Layers:

1. **Tailnet.** Only tailnet devices reach `tailscale serve`, and it is never
   Funnel. The server binds 127.0.0.1 only. The firewall's trusted
   `tailscale0` does not reach it because nothing listens there.
2. **Identity.** Every request, static files included, must carry
   `Tailscale-User-Login` equal to the configured login. Serve adds that
   header and strips client-supplied copies (Tailscale KB 1312). Tagged
   devices get no header and so get nothing. A request that reaches the
   localhost port without coming through serve also has no header and gets
   403. That blocks a page open in parker's own browser.
3. **Cross-site writes.** Every `POST` must carry `Origin` equal to the
   configured origin and `Content-Type: application/json`. A page on another
   site in giskard's browser can neither forge the origin nor send JSON
   without a preflight the server never answers.
4. **Rendered content.** DOMPurify stays between `marked` and `{@html}`. In a
   browser, a script that slipped through would hold the same power the
   Tauri IPC holds on the tower.

A client hanging up must not cancel parker. `send_message` resolves when the
turn ends, which can take minutes, and a phone that sleeps drops the request.
So the server runs every engine call on a detached task. A dropped
connection abandons the answer, never the turn.

## Nix

- `home/parker/porthole.nix` — the frontend (`buildNpmPackage` →
  `dist` plus `fonts/`), the server (`buildRustPackage`, `buildAndTestSubdir
  = "porthole"`, so `cargo test` runs the gate's tests in `nixos-rebuild build`),
  and the `parker-porthole` user service on 127.0.0.1:4097.
- `system/tailscale.nix` — a oneshot `tailscale serve --bg --yes
  --https=443 http://127.0.0.1:4097`, with `serve off` on stop. It is not
  nixpkgs' `services.tailscale.serve`: that option configures Tailscale
  *Services* (`svc:` VIPs on tagged hosts), and tagged traffic carries no
  identity headers.

## Manual steps (Peter)

- Enable **HTTPS Certificates** in the Tailscale admin console (DNS page).
  The tailnet has none today (`CertDomains: null`), and serve's `--https`
  needs them.
- On the phone: sign Tailscale back in (it was last seen 261 days ago).
- Deploy with `switch` when you want it live. Nothing in this change needs
  `boot`.
- On giskard: Cmd+− to about 67%. The layout is the tower's, in px for a
  2560-wide screen. At 100% on a 1512-wide MacBook the view indicator and
  the keybind row overlap. At 67% (≈2256 CSS px) the layout is exactly the
  tower's, WEARS included (verified by headless screenshot). Ctrl+± is
  swallowed by the page, Cmd+± is not, and Safari remembers zoom per site.

## Phases

1. **door** — extract the shell's engine conversation into `visage-door`;
   `src-tauri` calls it. Gate: `cargo check` in the visage dev shell, and the
   Tauri app still builds.
2. **web** — `porthole` with the gate, `/api/*`, `/api/events`, face
   push, and static files. Gate: unit tests for the gate, plus curl against
   the live engine on a spare port.
3. **page** — `shell.js`, `data-shell`, the cursor and the font. Gate: the
   built page in a browser through a header-adding local proxy (test
   scaffolding only, never committed), the Tauri dev loop still fine, and
   `parity.mjs` green.
4. **nix** — packages, the user service, the serve oneshot. Gate:
   `nixos-rebuild build --flake .#parker`.
5. **docs** — visage/AGENTS.md (the two shells, the door, the gate) and
   DECISIONS.md.
6. **live** (Peter) — HTTPS on, `switch`, open
   `https://parker.tail1b2a09.ts.net` on giskard, then Safari → File → Add
   to Dock. Done Oct 10 2026: it works.

## After first use: keys, focus, and where Peter is

Using it from giskard in Chrome turned up three problems, plus one wish
of parker's own:

- **Ctrl+Tab never arrived.** Chrome switches tabs before the page's
  JavaScript runs, so no `preventDefault` can win. The fix is *immersive*
  (Ctrl+F, `/fullscreen`): the Fullscreen API removes the tab strip, and
  Chrome's Keyboard Lock hands the page the browser's own keys, plain Esc
  included. Holding Esc leaves. It was preferred over remapping keys for
  the web: the tower's keys stay the only keys.
- **The input came loose.** On the tower nobody clicks; on a Mac a click
  moves focus to the page, after which only letters found their way back.
  Now a click that selected nothing, the window regaining focus, and
  Backspace, arrows, Enter or paste all reclaim the caret. Verified over
  CDP in headless Chrome: blur plus Backspace edits the box.
- **Ctrl+Space is macOS's** when it has more than one input source, so
  `Ctrl+B` dances too, in both frames.
- **parker asked to know where Peter is.** Each message carries one fixed
  sentence in the engine's per-message `system` field: "at the tower" from
  the Tauri frame, or "away … on giskard (macOS)" from porthole.
  porthole gets the device from serve's `X-Forwarded-For` plus `tailscale
  whois`. The face follows him, but `show`, `look` and `notify` do not, and
  that is the useful part of knowing. Verified: the forwarded address
  resolved to `giskard (macOS)` through the stand-in proxy.

A manifest and icons also landed, so Chrome's "Install page as app" gives
a tab-less window even without immersive.

## Second pass: an app, not a fullscreen

Immersive worked, but on macOS it is a fullscreen Space of its own, and it
takes over the browser window it started in. Under it, Esc misbehaved:
two presses to close the palette, and focus that sometimes left the page
until a click brought it back. Neither reproduced in headless Chrome (CDP
injects keys below the browser), with or without immersive. So the cause is
taken to be Chrome's Mac fullscreen handling, and the answer is not to
need fullscreen at all:

- **Install as an app.** It gets its own window, Dock icon and Cmd+Tab
  entry, and the regular Chrome windows are untouched. The manifest now asks
  for `window-controls-overlay`: hide the title bar with its chevron and
  visage fills the window edge to edge, with the traffic lights floating
  over the empty top-left corner. App.svelte's `.titlebar` strip, sized by
  `env(titlebar-area-*)` and zero everywhere else, is the drag handle.
  Option-click the green button fills the screen without a new Space.
- **Keys arrive anyway.** Chromium's `IsReservedCommandOrKey` returns early
  for app windows ("In Apps mode, no keys are reserved"), and next/previous
  tab are enabled only where a tab strip exists. So `Ctrl+Tab` should
  reach the page in the app window with no Keyboard Lock. This is from
  reading the source; it is unverified on giskard.
- **Ctrl+N / Ctrl+P move in the Ctrl+Y list**, as they already did in the
  palette, in both frames. Verified over CDP (2 down, 1 up, j/k still
  fine, no stray new chat). The palette input also turns off browser
  autocomplete, so no suggestion dropdown can be the thing an Esc closes.

Immersive stays (Ctrl+F) for anyone who wants the whole screen.

## Third pass: a name, and the question marks

- **The porthole.** A round window into the box, like the face. Peter
  chose it knowing what else it sounds like, on one condition: that the
  joke be part of parker. It is, in `mind/identity.md`. He maintains with
  complete dignity that a porthole is nautical, and he is somehow the only
  one who ever brings up the other reading. On the phone the dignity
  slips. The joke is his alone; the docs and the code just say porthole.
  The rename covers the crate (`visage/porthole`, package
  `visage-porthole`, binary `porthole`), `porthole.nix`, the
  `parker-porthole` service and `tailscale-serve-porthole`. The "away"
  sentence parker reads now says "through the porthole", so he always
  has the word.
- **Five question marks, two drawn.** Not input at all. The box held
  `?????`, and JetBrains Mono's ligatures drew `???` as blank cells
  (reproduced on parker, fixed by `font-variant-ligatures: none` on the
  root). Brackets have ligatures too, which is why they acted up. It
  affected the tower as well, just unnoticed.
- **Composition is the text system's.** Keys pressed mid-composition
  (`isComposing`, keyCode 229) are now ignored by App, Chat and the
  palette, so an Enter that commits an accent can no longer send.
- **Still open: Esc twice in the palette on giskard**, in a tab and the
  app window alike, so not fullscreen. It needs a key log from the Mac,
  taken with focus on the page rather than DevTools.

Later, not in this change:

- **A layout below 2560** — the views are px for the tower (`session`
  insets the chat 870px from the left, for one). The MacBook gets by with
  browser zoom. A real narrow layout should scale the design rather than
  reflow it. Automatic CSS `zoom` was considered and left out: the face's
  canvas camera reads the viewport, and zoom-versus-viewport-units is
  exactly where browsers still disagree.
- **Phone ergonomics** — every action is a Ctrl chord today, and a phone has
  none: touch affordances for view, new chat, threads, dance and stop, plus a
  narrow-screen placement for both views. It already loads and renders
  (headless screenshot at 390×844), and the keybind hints overlap the view
  indicator.
- **An asleep page** — a service worker that caches the shell, so a
  powered-off parker gets "parker is asleep — power button, there is no
  WoL" instead of the browser's error. (The manifest and icons are done.)
- **One door** — local visage could talk to `porthole` too, leaving
  `src-tauri` as a window plus `dismiss`. Worth it only if the two frames
  start to differ.
