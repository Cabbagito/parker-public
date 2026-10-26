# AGENTS.md — home/parker/

parker the entity. Read PARKER.md at the repo root first — it records why
this exists and what it is meant to become; this file is the warden doc for
the code.

## Where things are

- `default.nix` — the role: installs the `parker` binary and the Rust
  toolchain for the dev loop.
- `package.nix` — the derivation. `rustPlatform.buildRustPackage`,
  `cargoLock.lockFile`, no extra flake input.
- `parkerd/` — the crate. **Cargo sources only.** The package name is
  `parkerd` (where it is going); the binary is `parker` (what you type).
- `engine.nix` — the `parker-engine` user service: `opencode serve` on
  127.0.0.1:4096, password-gated, consuming `parker mcp` plus the full
  built-in toolset (permissions are allow-all since August 2026 — PARKER.md
  records the why; `websearch` alone stays denied, replaced by the `search`
  verb). Its header records every wire-verified trap; read it before
  touching the engine config.
- `code.nix` — `parker code`'s config: opencode's TUI with all coding tools
  plus the parker verbs, themed from the active Stylix scheme. The opposite
  trade from engine.nix, same seam. Writes two files, because opencode splits
  its config in two: `code.json` (agent — MCP, sharing) behind
  `OPENCODE_CONFIG`, and `tui.json` (theme, keybinds, TUI plugins) behind
  `OPENCODE_TUI_CONFIG`.
- `code-logo.tsx` — parker's wordmark on `parker code`'s home screen, as an
  opencode TUI plugin filling the `home_logo` slot. Not a fork: upstream
  declares that slot `mode="replace"` on purpose. `docs/OPENCODE.md` is the
  capability reference for what else that seam reaches; DECISIONS.md
  ("branding opencode without forking it") records why it beat patching.
- `code-status.tsx` — the machine on screen: a hint bar of the slash commands
  (`home_footer`), a compact readout beside the prompt
  (`session_prompt_right`), the MACHINE panel (`sidebar_content`) and the
  statusline (`sidebar_footer`). The readouts all come from `parker sense
  --json` on a timer, so the panel and the agent can never disagree about the
  machine.
- `code-theme.tsx` — live day/night. Watches the generated theme file and
  re-installs it when a specialisation switch rewrites it, which is the one
  seam a rebuild could not close: the TUI process outlives the rebuild.
- `code-hooks.ts` — the **server** plugin, and the only thing here that runs
  in the agent loop rather than the terminal. Injects the sense block into a
  session's first message, hands bash `PARKER_*` and parker's git author,
  enforces the house rules as walls rather than comments (nothing but Cargo
  inputs in `parkerd/`; no request-socket calls; no switch from an agent),
  and sends notifications. It no longer touches the face — only parker does.
- `visage/` — parker's face as a fullscreen app (own warden doc inside).
- `porthole.nix` — the same face for the tailnet. It packages the page
  and the `porthole` server, and runs them as the `parker-porthole`
  user service on 127.0.0.1:4097. `system/tailscale.nix` publishes that
  port with `tailscale serve`.
- `brains.nix` — parker's brains: a name we chose → a record of opencode's
  `provider/model` and the `lore` it is named for (visage's ^K card shows
  it), plus the default. Pure data. engine.nix and code.nix take the
  default's model as their config model; default.nix compiles the set to
  `~/.config/parker/brains.json` for `parker brain` (parkerd's `brain.rs`),
  the one place a name becomes a model. Which brain is in use is state
  (`$XDG_STATE_HOME/parker/brain`), so switching needs no rebuild.
- `mind.nix` + `mind/` — parker's prompt, in four files stitched in a fixed
  order at build time: `identity.md` (who he is — ours, parker never edits
  it), `peter.md` (who Peter is — ours too; its "As of" section dates and
  is the part to revisit), `voice.md` (how he speaks; the face as a
  channel), `hands.md` (how he works). The engine uses it as the whole
  agent prompt; `parker code` appends it to every mode. All four always
  load, or the build fails.

What reaches a parker model, and who owns each part:

| layer | where | owns |
|---|---|---|
| mind | `mind/*.md` via `mind.nix` | persona, who Peter is, voice, working habits |
| repo contract | `AGENTS.md` files (engine: root by name; parker code: discovered) | map, conventions, building |
| verbs | `parkerd/src/mcp.rs` + `Expression::gloss()` | what each verb does; the face vocabulary and its mechanics |
| modes | `code.nix` agent prompts | ghost / wired / subagent postures only |
| live state | `code-hooks.ts`, `visage/door` | the `<machine>` sense block (parker code); where Peter is and which brain parker is on, with each visage message (engine) |

A rule lives in exactly one row. If you are about to write it in a second,
point at the first instead.

The agents are parker's, not opencode's: `plan` and `build` are disabled in
`code.nix` and replaced by two modes — **ghost** (read-only, touches nothing)
and **wired** (full tools, the default) — with **probe** (locate, cheap
model), **redteam** (adversarially verify) and **autopsy** (debug) as
subagents.

Two modes and not three on purpose. A `calibrate` mode existed briefly and
was wrong: fmt → clippy → test → build is a checklist, and you do not switch
modes to run a fixed sequence. It is `/calibrate` now.

Slash commands wrap the verbs — `/sense`, `/look`, `/drift`, `/rebuild`,
`/calibrate`, `/verify`, `/theme` — and they paste shell *output* into the
prompt rather than asking the model to call a tool, which is the difference
between a command and an MCP verb.

`ghost` is a posture, not a sandbox: edit/write/patch are denied, but bash is
open, because an allow-list of read commands is a list you are still editing a
year later. The refusals that matter live in `code-hooks.ts`, where they apply
to every agent and can explain themselves — a permission glob cannot say
*why* it said no, and a thrown error can.

## Two abstractions, and only two

**sense** — where parker gets information (`src/sense/`).
**reach** — what parker can change (`src/reach/`).

Everything else is a loop and a socket. Resist adding a third: no trait
until there is a third implementation of it.

## State: v1

```
parker sense [--json] [--watch [SECS]]
parker search <query...>         # the web, via the machine's own SearXNG
parker recall list [--title T] [--open] [--since AGO] [-n N] [--json]   # past threads, via the engine API; --open: what is still waiting
parker recall search <query...> [--exact] [--since AGO] [-n N] [--json]
parker recall ask <thread> <question...>   # the Flatline (engine.nix `recall`) reads it and answers
parker recall mark <thread> done|open|clear [--note TEXT] [--json]   # your word on a thread (visage's ^X runs it)
parker reach notify <text> [--urgency low|normal|critical]
parker reach look [--target window|screen]
parker reach goto <space>
parker reach focus <app>
parker reach media <action> [--player NAME] [--percent N]   # play/pause/next · louder/quieter/volume
parker reach show <what>         # a URL, a file, or text — floated in front of you
parker reach face <expression> [--for SECS] [--sound]
parker reach settle              # turn over: take off a work posture, if worn
parker reach dance [routine|auto|stop] [--for SECS] [--show]
parker face [--expression NAME] [--phase P] [--animate] [--sheet [--json]]
parker face --log [--since AGO] [-n N] [--json]   # what he has worn, and for how long
parker face --catalog --json     # the idle/dance tables, for visage
parker face --vignettes --json   # the long idle pieces, apart for their size
parker visage                   # up, or away again — the same as SUPER+P
parker dance                    # dancing, or not — the same as Ctrl+Space
parker brain [name...]          # which model he thinks with; a name picks one
parker brain --list | --model | --json
parker mcp                      # JSON-RPC over stdio, for a model
parker code [args...]           # opencode's coding TUI, wearing parker
parker-chat <message>           # one message through the engine, text back
```

The face has **three** layers, not two (MOTION.md): a *pose* you interpolate
to, a *motion* that loops forever, and a *beat* — something parker does once
and then stops doing. Beats are data (`face/beat.rs`), they are deliberately
not `Expression` variants (a glance must never be mistakable for `Curious`),
and they drive both idle flair (`face/idle.rs`) and dance (`face/dance.rs`).
The long ones — vignettes, minutes each — are built rather than typed
(`face/vignette/`), flattened once into ordinary beats.
`face/catalog.rs` writes the tables out so visage reads them instead of
transcribing them, the vignettes in a file of their own.

Thirty-seven expressions since the faces round (Oct 2026, DECISIONS.md), and
**every one of them is wearable, `trouble` included** —
`Expression::WEARABLE` is `ALL`, and no surface infers a face anymore;
`VOLUNTARY` survives only as the spring-easing list. `trouble` was the last
machine-asserted expression and it was removed too: nothing but parker puts
a face on parker, so there is no `from_sense` and no disconnected-engine
override. An engine that is down is a line of text, not an expression.

A few reactions land with a gesture — an *entrance* (`face/entrance.rs`,
MOTION.md Part Four): a short beat that departs from wherever the face was
and ends on the new pose. The look under a name may change (the faces round
redrew eleven); the name and its meaning are what parker has learned.

What each face *means* is `Expression::gloss()`, and it is written only
there: `mcp.rs` builds the `face` tool's description from it — split by
`is_posture()`, because postures hold until changed and reactions expire on
their own — so the vocabulary a model is handed cannot drift from the faces
that exist. A new expression needs a gloss or the tests fail.

Postures have two nets under them (DECISIONS.md, Oct 2026): every one
defaults to a ten-minute hold (`POSTURE_CAP`), and the work postures
(`is_work()`) come off at the turn's end through `parker reach settle`,
which door runs when a send returns. A new posture must say whether it is
work.

Every change to the face is also appended to a diary, `face/log.rs` →
`$XDG_STATE_HOME/parker/face.log`: one JSON line per wear, clear or settle
(a settle that took nothing off writes nothing). End times are never logged
— a face ends at its deadline or the next line, whichever is first — and
`parker face --log` derives the spans. Writes are best effort and the file
trims itself to its newest 2000 lines; a new writer of the face slot must
go through `state.rs`, or the diary lies.

`parker face` is the surface (draw it); `parker reach face` is the choice
(wear it); `parker reach dance` is a *mode*, which is why it is its own verb
rather than another wearable name — a mode is not a face, and Esc must be
able to end it without undressing him.

**Dance is a mode, not a window.** It launches nothing. It writes one line to
`$XDG_RUNTIME_DIR/parker/dance` and every face parker has — visage, the
terminal face, anything later — reads that line and dances. So it is
orthogonal to `parker visage`, which only decides whether the app is on
screen: starting a dance while visage is away means parker is dancing where
you cannot see him, and bringing visage up then shows a dance already in
progress.

Two spellings, and the split is the same one `face` makes:

- `parker dance` — the human gesture. One word, no flags, toggles. You can
  never be stranded by forgetting the word for "stop", because there isn't
  one.
- `parker reach dance <routine|auto|stop> [--for SECS] [--show]` — the knobs,
  and the one a model calls over MCP.

`Ctrl+Space` in visage runs the `reach` form with no deadline, which is why
a dance toggled on there outlives the window. That is deliberate (a timer
would end it mid-song) and `parker dance` is how you end it from anywhere. `parker visage` is the app on `special:visage`: one word, no
flags, flipping it up or away exactly as SUPER+P does — and it is the one
summon path, with the `parker-visage` keybind script and `parker-greet` both
running it. Typed rather than bound so the face can be reached over SSH,
from the couch.

sense reads Hyprland (`hyprctl -j`), MPRIS over D-Bus, and the machine
(nvidia-smi, /proc, systemctl) in ~90 ms. reach notifies through
org.freedesktop.Notifications (noctalia owns it), screenshots via grim, and
navigates via `hyprctl dispatch`.

`parker mcp` has three clients. Two are parker instances: the
`parker-engine` service (engine.nix), where the parker agent gets the verbs
plus opencode's full built-in toolset (allow-all since August 2026;
`websearch` alone denied — it is Exa-hosted, and `search` via the local
SearXNG replaces it), and `parker code` (code.nix), which is the mind
plus every coding tool. The third is Claude Code, the first client — the
seam proven by an engine parker does not ship with. It keeps the verbs, so
the maintenance model can see the machine and wear the face; the
registration is asserted on every switch by `home/apps/claude-code.nix`
against the profile path, never a store path (a store path silently pinned
a six-verb build for two months). Provider: Anthropic via `opencode auth login`
(state in `~/.local/share/opencode/auth.json`, outside repo and store).

**The brain is per message, and it is Peter's.** `parker brain` picks which
model parker thinks with — deep thought (Opus), just read the instructions
(Sonnet), speedy (Haiku), wintermute (Fable); brains.nix has the set. Door
names it on every visage send (each thread carries a model of its own, so
leaving it out is not neutral) and tells parker which one he is on in the
message's `system` line;
parker-chat sends it too, and `parker code` launches on it. One brain for
all of parker, never one per thread (PARKER.md invariant 2). It is not a
verb: parker cannot pick his own, because spending money on his initiative
is what the daemon's gate is for. The helpers — `small_model`, `title`,
`ledger`, `digest`, `probe` — stay on Haiku whatever the brain is; they are
not parker.

Still no daemon. The engine is a service but parkerd itself remains
one-shot: the verbs turned out to be as one-shot as the senses.

Deliberately absent, because a single reading cannot honestly produce them:
**idle time** and **focus duration**. `--watch` fakes the second one within
one watch session, which is a preview of why the daemon has to exist rather
than a substitute for it. Repo drift (uncommitted count, lock age) was
considered and cut — it is nagging material, not sensing.

## Rules of the house

- **Every input degrades to "I don't know."** An input that cannot answer
  returns `None`; it never panics and never takes the process down. parker is
  never in the critical path (PARKER.md invariant 1), and that starts here.
- **`Sense.version` is never removed and fields are never reordered.** That
  is all that survived the replay invariant: anything that later logs a
  decision can log the `Sense` beside it for free.
- **Text is what parker reads; JSON is for tests and parsers.** The block goes
  into every prompt, and the JSON of the same content costs ~3× the tokens. A
  line with nothing to say is omitted entirely — no `media: null`.
- **Nothing but Cargo inputs inside `parkerd/`.** A stray file there is part
  of the derivation's source hash, so touching it rebuilds all 102
  dependency crates. New source directories go in `package.nix`'s fileset.
- **`hyprctl`, not `.socket.sock`.** Hyprland evaluates request-socket
  connections synchronously — a client holding one open freezes the
  compositor until a 5 s timeout. The daemon will read `.socket2.sock` (the
  event socket, no such hazard); it must never hold the request socket.
- **Hyprland JSON is parsed through `serde_json::Value`, not typed structs.**
  It changes between minor releases without saying so; a missing field should
  cost one number, not the whole read.
- **In `parker mcp`, stdout belongs to the protocol.** One stray `println!`
  corrupts the stream and the client hangs confusingly. Diagnostics go to
  stderr, always.
- **Gating is about initiative, not obedience.** PARKER.md gates `notify` on
  `interruptible`; that applies to things parker decides to do, never to
  things it was asked to do. Every v1 call is user-initiated, so nothing is
  gated. The gate arrives with the daemon.

## Working on it

Inner loop, ~0.2 s:

```bash
cd ~/parker/home/parker/parkerd && cargo run -- sense
```

The package's checkPhase runs `cargo test`, so `nixos-rebuild build` (root
AGENTS.md, Building) proves parkerd too — ~20 s. Deploying is the same
command with `switch`, on the same terms as everything else.

Formatting is stock `rustfmt`, no `rustfmt.toml` — the ecosystem default is
the one choice nobody has to argue about. Before committing:

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

## Known gaps, on purpose

- The privacy denylist (`src/sense/policy.rs`) **fails open**. In a browser
  the title is the page, so an unlisted sensitive site gets through — there
  is a test asserting exactly that, so nobody rediscovers it as a surprise.
  The fix is not a longer word list; it is that parker looks at a screen only
  when asked. Keep the list short and assume it is porous.
- `inferred.mode` is a class lookup and `interruptible` is "are you
  fullscreen". Both are placeholders for something that needs history.
