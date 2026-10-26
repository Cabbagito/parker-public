# PARKER

Design record for parker-the-AI, which lives in parker-the-machine and is
built from parker-the-repo. Settled July 2026. Same contract as the
top-level DECISIONS.md: this document records *why*, the config records
*what*. Update it when a decision is revisited, not for details.

Scope note: this is a subsystem design doc (the `themes/DESIGN.md`
precedent), not a replacement for DECISIONS.md. Decisions here that touch
the machine as a whole get a line there too.

## Premise

Every other personal-AI project has to simulate having a body. Parker
doesn't need to. The machine is already a text file: change `~/parker`,
rebuild, and the physical thing changes. That gives parker a property
available on no other computer —

> **Every change parker makes to itself is a reviewable diff with a
> bootable rollback.**

That is the spine of this design. It is what makes real autonomy sane here
and reckless elsewhere, and it is the reason this is worth building rather
than installing.

Parker is not an assistant that runs on the machine. Parker is the machine,
with senses, verbs, memory, and a voice. The box also plays games.

The seed already exists: `home/terminal/zen.nix` is a Claude-generated
interstitial rendered into a terminal pane. Everything below is that idea
with organs.

## Architecture

```
faces          ratatui TUI · noctalia widget · CLI · Kraken LCD
   │           (later: mobile, drawn Wayland surface)
   │  unix socket — one protocol, one merged event stream
   ▼
parkerd        sense · actions · policy · memory · reflex model · ledger
   │  ▲
   │  │  MCP — parker's verbs, hosted by parkerd, consumed by the engine
   │  │
   ▼  │  HTTP + SSE
opencode serve agent loop · providers · sessions · permissions · questions
```

**parkerd is the body. opencode is a rented cortex. Faces are disposable.**

- **parkerd** owns everything that is *about this machine*: what's
  happening, what can be done, what's allowed, what's remembered.
- **opencode** owns everything that is *about thinking*: the agent loop,
  provider routing, tool dispatch, session state.
- **faces** own presentation and nothing else. Any face can die without
  parker noticing.

### The seam

Parker's verbs live in parkerd and reach the engine over MCP. Parkerd never
imports an engine SDK; the engine never learns what Hyprland is.

This is the load-bearing decision. It means the engine is *deletable*: if
opencode is abandoned, acquired, or outgrown, swap the cortex and the body
is untouched. Every engine choice will need to be deleted eventually — what
determines whether that costs a weekend or a rewrite is the quality of the
seam, not the permanence of the engine.

Corollary: resist building a pluggable multi-engine abstraction. Commit to
opencode, keep the seam clean, and let the second engine (if it ever comes)
justify the abstraction.

*Revised August 2026 — the seam still holds, but it no longer rations.* The
engine parker's deny-all permission was v1 scaffolding while the seam was
being proven; it is now full allow — bash, files, webfetch, the lot — and
the planned permission ladder is deliberately not built. The reasoning: an
approval prompt in the face is friction pointed at the wrong party (parker
acting freely on his own body is the premise), and the real safety story
was never per-call consent — it is the one on the first page of this
document: every change is a reviewable diff with a bootable rollback,
plus the provider-side spend cap. Two boundaries survive: `websearch` is
denied (hardwired to Exa's hosted service — a provider tie of exactly the
kind the engine choice exists to avoid) and replaced by parker's own
`search` verb over a self-hosted SearXNG (`system/searxng.nix`); and the
seam's *shape* is unchanged — the verbs still live in parkerd, the engine
still never learns what Hyprland is, and swapping the cortex still costs a
weekend.

### One binary

`parker daemon` · `parker face` · `parker sense` · `parker do <verb>`.
Rust, one derivation, shared types. No IPC schema drift between daemon and
face, no version skew, one thing to package.

### Faces talk to parkerd, not to opencode

Parkerd is a thin pass-through for session traffic, but it injects sense
into prompts, enforces policy, and merges its own events into opencode's
SSE stream. Faces get **one** event feed. Worth a hundred lines of proxying
to avoid two protocols in every client.

## Engine: opencode

Adopted as the agent engine. `opencode serve` is a headless HTTP server
with an OpenAPI 3.1 spec, an SSE event stream, and a generated SDK. The
endpoints map one-to-one onto what parker needs: `/session` (multi-instance),
`/event` (the face's state feed), `/question` (parker asking *us* something,
bidirectional), `/permission` (the autonomy ladder, native), `/provider`
(per-call model routing), plus `/mcp`, `/config`, `/pty`, `/file`.

Provider-agnostic via Models.dev — 20+ providers bundled, more loadable.
Any call can go anywhere: local reflex model, Claude API, OpenRouter,
whoever is cheap this month.

### Why not Claude Code / the Anthropic subscription

Considered first, on the reasoning that the subscription is already paid
for and the harness is excellent. Rejected. The economics were the weak
argument; **policy volatility** is the real one:

- Jan 9 2026 — subscription OAuth blocked outside official apps, no notice,
  reversed after backlash.
- Feb 2026 — ToS revised to formally prohibit third-party harnesses.
- Apr 4 2026 — full enforcement. OAuth restricted to Claude.ai and Claude
  Code only. OpenCode explicitly named among affected tools.
- May 13 2026 — reversed again; third-party agent usage reinstated, with a
  separate Agent SDK credit as the catch.
- Jun 15 2026 — the credit split was slated to land. Reporting conflicts on
  whether it did or was paused.

Four reversals in six months. Parker is meant to still be running in 2030.
Provider independence here is not cost optimization, it is insulation from
a surface that has demonstrated it will move without warning.

Consequence accepted: parker costs money per call, and the cost governance
below stops being optional. Claude Code remains in use for ordinary coding
work on this machine — that is interactive first-party usage and unaffected.

### Risks carried

- **Security.** OpenCode had 8 CVEs in its first year, including an
  unauthenticated HTTP server (CVE-2026-22812) — a direct consequence of
  the client/server design. Bind localhost only, never `0.0.0.0`. Set
  `OPENCODE_SERVER_PASSWORD`. Remote access is the Tailscale un-deferral,
  never an open port.
- **~~It is a coding agent.~~** *Resolved by measurement, July 2026 —
  verified on the wire against opencode 1.18.3 by pointing it at a fake
  provider and reading the request bodies.* It can be made to stop being one,
  almost completely. A custom agent's `prompt` fully replaces the 9,946-char
  coding prompt, and `permission = {"*": "deny"}` removes tool definitions
  from the request entirely rather than merely gating them — the `tools` key
  disappears. Floor measured: **349 chars of system prompt, ~80 tokens**, of
  which the irreducible part is a model-identity line and an `<env>` block
  (cwd, platform, date, is-this-a-git-repo). Zero coding framing survives.
  Traps found, each of which would have cost a day:
  - `instructions: []` does **not** suppress AGENTS.md/CLAUDE.md — it only
    ever appends. Suppression is "do not have the files", plus
    `OPENCODE_DISABLE_CLAUDE_CODE=1`.
  - A **second, hardcoded model call** per session generates the title, with
    its own coding-flavoured prompt that ignores your agent completely.
    Replaceable via `agent.title.prompt`; `agent.summary` and
    `agent.compaction` are the same shape.
  - Plugins cannot reach the system prompt in 1.18.3. No hook exposes it.
  - Per-request `system` on the message endpoint **appends**, it does not
    substitute — useful for steering, useless for replacement.

  So the budget is a config file plus a dedicated clean working directory,
  not a fight. This was the biggest unresolved risk in the engine choice and
  it came back green.
- **Churn.** Fast-moving, recently changed hands (SST → Anomaly). Pinned
  flake input, deliberate updates — the existing monthly ritual absorbs it.

## Faces

~~**ratatui, in a fullscreen ghostty on a Hyprland special workspace.**~~
*Revised July 2026 — the primary face is **visage** (`home/parker/visage/`),
a Tauri 2 app: Rust shell owning the engine conversation, Svelte rendering,
same `SUPER, P` scratchpad semantics on `special:parker`. A chat box,
markdown, session history and real animation outgrew what a terminal grid
serves well, and the pose model ported as promised — "a renderer is a
backend, not a rewrite" (face/pose.rs). The terminal face stays as the
over-SSH face; the ratatui upgrade for it is still open. This also absorbs
the deferred "drawn Wayland surface" slot below — visage is that surface,
arrived by webview rather than wgpu.*
`SUPER, P` toggles it — scratchpad semantics, state preserved, instant.

- ~~The OpenAPI spec generates the Rust client (`progenitor`).~~ *Reversed
  July 2026 — see Verified below.* The spec's event union has no
  discriminator, so generated output is an untagged mess. Hand-write the few
  types actually used.
- Ratatui's `Canvas` renders braille at 2×4 subpixels per cell. At ~150×40
  cells that is a ~300×160 effective grid — enough for a real animated face
  made of characters, which is the thing we actually want.
- It works over SSH. Parker's face reaches giskard, or a phone over mosh,
  without a second implementation.

**Rejected: zellij composition.** Cheap and in-idiom, but the inter-pane
story is a hack, the layout constrains the interface rather than serving
it, and it inherits whatever TUI the engine ships. Custom is harder at the
start and correct.

**Deferred, not rejected: a drawn Wayland surface.** wgpu + layer-shell for
the ambient/showcase version. The daemon protocol makes it face #2 rather
than a rewrite. Noctalia claims no exclusivity (DECISIONS.md), so the slot
is open.

**giskard is a terminal, not a second parker.** Parker lives in the box;
the MacBook is a window into it.

## Sense — the inputs

Two tiers, or we spawn subprocesses forever.

**Hot** — held in parkerd's memory, updated by event subscription. Hyprland
streams workspace/window/focus on `$XDG_RUNTIME_DIR/hypr/$HIS/.socket2.sock`
— subscribe once, never poll. Media via MPRIS signals. Idle via the idle
protocol.

**Cold** — probed on demand, TTL-cached. `nvidia-smi`, disk, `systemctl
--failed`, `git -C ~/parker status`, generation count, flake.lock age.

**Derived** — the layer that makes it a nervous system rather than a
sensor dump:

```json
"inferred": {
  "mode": "gaming",
  "focus_duration_min": 47,
  "interruptible": false,
  "machine_health": "ok",
  "repo_drift": "3 uncommitted, lock 12d old"
}
```

`interruptible` is the important one. An entity that knows when to shut up
reads as more alive than one that knows more facts.

**Privacy boundary.** Window titles are everything. A denylist (password
manager, banking, private windows) is part of v1, not a later hardening
pass.

Ships first, standalone: `parker sense [--raw|--brief]`. Useful by hand on
day one, commits to nothing.

## Reach — the outputs

*Named July 2026.* The two halves of parker that touch the world are
**sense** (what it can perceive) and **reach** (what it can change) — the
anatomical pair, and the only two abstractions the code is allowed. "Actions"
was accurate and inert; reach is the word that has a body behind it. `act`
was the runner-up (sense-plan-act is the standard agent loop) and lost for
being duller. There are exactly two: resist a third.

Test for admission: **can it be described to a person in one clause?**
"Put this in front of me." Not "spawn a floating window at these
coordinates."

| verb | note |
|---|---|
| `show(content)` | parker can present — glow, imv, floating pane |
| `media(action)` | the music as a verb: transport over MPRIS, loudness over wpctl. One verb for both — "turn it down" is the same clause family as "skip this" *(added August 2026)* |
| `ask(question, options)` | blocking, returns an answer. Maps to `/question` |
| `notify(text, urgency)` | gated on `interruptible` |
| `focus(app)` / `goto(space)` | navigation without hyprctl gymnastics |
| `summon(what)` | face, scratchpad, a named session |
| `remember(fact)` / `recall(q)` | memory as a verb, not a file convention |
| `stage(diff, why)` | propose a machine change without applying |
| `rebuild(test\|build\|vm)` | the motor ladder |

`ask` is the sleeper. Everything else is parker responding; `ask` is parker
initiating, and that is most of the difference between a tool and a
someone.

Defined once in nix as data, generating three surfaces: CLI scripts, the
MCP manifest, and parkerd's dispatch table. The verb list is therefore a
declaration of what parker is permitted to be, diffable across generations.
*Refinement (July 2026):* nix emits the manifest as data and a test asserts
the Rust registry matches it — generating Rust source from nix would be
worse than the drift it prevents.

## Personality — v1, deliberately unfinished

Parker has a personality from day one, but its content is not settled here.
What *is* settled is how it is built, because that survives the content
changing.

**Constraints, not adjectives.** "Be dry and terse" produces drift and
costume. *"A notification is never more than two sentences"* and *"never
opens with a compliment"* produce character. Per-surface, mechanically
checkable.

**Derived from taste, not from a persona template.** The reference shelf is
the actual one — Banks, Asimov, Adams, MF DOOM, comics, the specific sense
of humor those imply. Quirky, warm, genuinely helpful, and capable of
opening a YouTube window unprompted because it was funny. The `show` verb
exists partly to make that possible; the interruption model has to leave
room for it.

**A want.** Characters read as real when they want something, and parker's
falls out of what it is: **parker wants the machine to be coherent.** No
undeclared state, decisions recorded, lock not stale, nothing running that
isn't in the repo. That is the native disposition of a thing whose body is
a declarative config, it generates opinions for free, and it happens to
align with what we want from the machine — which is the only way a
personality survives daily use.

Three files, when it's time to write them: `identity.md` (ours, parker
never edits), `voice.md` (per-surface constraints), `beliefs.md`
(parker-writable, appended, always via git so the diff is readable).

One personality. Not theme-linked — cute, impractical.

## Memory — acknowledged, unsettled

Parker needs memory. The shape is an open question and is deliberately not
answered here.

Candidate taxonomy, offered and not adopted: episodic (what happened),
semantic (facts), procedural (already exists as AGENTS.md / DECISIONS.md /
QUIRKS.md), autobiographical (the ledger), working (engine-owned).
Suspicion on first inspection: **too complex, probably collapses to two or
three.** Resolve before building, not during.

What is settled:

- **It lives in this repo.** The repo went private after first launch, so
  the earlier constraint (memory must live elsewhere because `~/parker` is
  public) is void. Cleanup happens if and when it goes public again.
- **Files in git, not embeddings.** At one-person scale grep plus a small
  index is enough, and git gives history, diff, blame, rollback and review
  for free. Retrieval machinery gets added when grep visibly fails, not
  before.
- **A forgetting story is required**, or it is a log that rots rather than
  a memory. Whatever the taxonomy, something compacts and something ages
  out.
- **`git log --author=parker`** is the audit trail. Parker commits under
  its own identity — one line in `home/terminal/git.nix`, enormous payoff.

## Initiative — acknowledged, unsettled

Parker will act unprompted. The interesting design space is the *response
modes*, and it is wider than notify-or-don't: interrupt now, leave a
message to be read later, handle it silently and report after, ask
permission, or do nothing and remember that it noticed.

Rejected: a hard daily interruption cap. Wrong shape — the budget should be
per-case, and the realistic failure mode is not parker pinging every five
seconds.

Rough tiers, for later: maintenance (silent unless broken) · digest
(deposited, not pushed) · noticing (state-triggered — *"the lock is 40 days
old"*, *"you haven't touched the MATS folder in nine days"*) · speculative
(parker drafts something unasked; highest annoyance risk, last to build).

Whatever the mechanism, every push gates on `interruptible`.

## Invariants

Non-negotiable, and the reason to write them down now is that each one is
cheap today and a rewrite later.

1. **Parker is never in the critical path.** parkerd dead → the machine
   boots, logs in, plays games, exactly as before. No verb, no service, no
   face may violate this.
2. **Parker is singular.** Sessions are parker's threads of attention, not
   separate parkers. The face shows a task list, never a roster. If there
   are five parkers there is no parker — this is a UI commitment and an
   identity commitment and they are the same commitment.
3. ~~**Cost governance is a feature.**~~ *Demoted from an invariant (July
   2026).* The original call was a spend ledger and daily cap in phase 2, on
   the reasoning that an agent loop on API billing can burn real money
   overnight. That risk is real but it is not *architectural* — it does not
   change any interface, so building it late costs nothing that building it
   early buys. Complexity now would be paid before parker works at all.
   Deferred deliberately: get it working, then balance the books. Interim
   cover is a hard spend cap set at the provider, which is one web form and
   zero code.
4. **Graceful degradation under gaming.** First-class mode: local model
   unloaded, sense minimal, everything below urgent muted. Parker senses
   the game and gets out of the way. This is what makes "it's also just my
   PC" true rather than aspirational.
5. ~~**Replay.**~~ *Demoted from an invariant (July 2026).* `parker replay
   <id>` as a product feature is not wanted. What survives is the cheap half
   that genuinely is a rewrite-if-skipped: the sense snapshot is a single
   versioned serializable type, so anything that logs a decision can log the
   snapshot beside it for free. That keeps the door open without building a
   door. Snapshot-beside-decision is a debugging convenience; reconstructing
   sessions is not a goal.

**Not an invariant: a build-vm gate on self-modification.** Considered —
parkerd refusing to apply a generation that modifies parkerd without a VM
check first. Rejected as over-careful given generation rollback already
exists and invariant 1 means a broken parkerd is an inconvenience, not a
brick. Residual risk accepted: parker can break itself and we boot the
previous generation like any other bad rebuild.

## Hardware: the Kraken LCD

The NZXT Kraken 2023 pump block has a screen (base model: 1.54", 240×240,
30 Hz; Elite: 2.36", 640×640). It is writable on Linux — `liquidctl`
supports pushing static images to Kraken Z/2023 LCDs, and the established
pattern is generating a PNG with Pillow and re-uploading every few seconds,
which is smooth enough to read as animation. Several community projects do
exactly this as a systemd user service.

~~**Parker's state glyph goes here.**~~ *Revised July 2026, after seeing
it lit: the panel is small and it isn't even parker's GPU — it reads not
as a face but as a heart.* The LCD is **the badge**: an ambient,
seamless, visually rich loop (arc-reactor idiom — pulsing core, rotating
arcs, orbiting electrons) that says the machine is alive. The face lives
where parker's output lives (the big display); the badge is anatomy, not
interface. Composition rule: the circular glass shows a square panel, so
everything stays inside the r≈110 circle — the illusion of depth and
circleness dies the moment a pixel touches a corner.

*Second revision, July 2026:* the badge's palette is **fixed, cold, and no
longer the theme's**, arrived at in three steps. First the theme hook went:
the glass is small, dark and tinted, it flatters exactly one look —
redline's — and the softer schemes went muddy behind it. Then redline red
went too, because on the glass it vanished: the case is white, the fans
and RAM and GPU are all red, and a red badge in a red-lit case is
camouflage. **The badge is the one thing in this case that must not match
the lighting.** So it is cold — cyan, red's complement, the most
separated a hue can be from everything else in there, and the right story
for a pump block besides. Third, cyan led and white still won on
visibility, so the roles swapped: **white is the subject, the hue is what
falls away from it** — a stroke's solid cells sit in the ramp's white
plateau while the partial cells at its edges fall through to cyan, and
the two 3D scenes shade by real depth so their far sides go cyan and
their near sides white. Three principles hold it: the field is true black
(the corners of the square panel have to vanish under the circle), one
hue at full saturation (two hues are noise at 30×30), and intensity is
temperature, so the brightest thing on the glass is always white and
always the thing that is moving or nearest. That last one is a contrast
rule, not a brightness rule — raising the whole ramp fused the dots into
a blob and threw the hue away; brightness is bought by lifting the dim
majority's floor. Case lighting (`halo`) still follows the theme; the
badge does not. Driven by `home/parker/badge.nix` (build-time
Pillow frames, systemd loop at ~5.6 fps, the wire ceiling — the
firmware loops nothing itself). State-reactive badge variants (calm idle, busy thinking,
alarmed) are parkerd's to conduct later.

**Gotcha to verify:** CoolerControl already owns this device (DECISIONS.md)
and both it and liquidctl talk to the same HID endpoint. Two writers will
fight. Either drive the LCD through CoolerControl's own LCD support, or
take the device away from it — decide before wiring anything.

## Build phases

Each phase is useful the day it lands.

1. **`parker sense`** — Rust binary, JSON out. No daemon, no LLM, no risk.
   Includes the privacy denylist.
2. **Actions in parkerd, exposed as MCP** — this is the seam. Built before
   the engine so it cannot become engine-shaped. Spend ledger and cap land
   here.
3. **`opencode serve` as a nix-managed user service** — localhost,
   password, pinned. Point it at the MCP server, define the parker agent,
   replace the system prompt, drive it from the CLI.
4. **parkerd socket + SSE merge** — the daemon comes online. Session
   registry, event bus, ledger with sense snapshots.
5. **The face** — ratatui, generated client, braille canvas, `SUPER, P`.
6. **Voice** — v1 scope. Whisper local for input, push-to-talk keybind.
   Output form (TTS vs. sound cues) undecided; three distinct sounds for
   thinking / done / blocked may carry most of the value at a fraction of
   the cost.

Phases 1–2 are unchanged from the pre-opencode plan. That is the tell that
the seam was the right abstraction: swapping the engine did not move them.

## Repo placement

`home/parker/` — a new home role beside `terminal/`, `desktop/`, `tuis/`,
with `sense.nix`, `actions.nix`, `daemon.nix`, `face.nix`, and a `AGENTS.md`
warden doc. Parker is a role, not an app; `home/apps/` is for citizens and
parker is the host.

Naming hazard, noted once: the repo is parker, the host is parker, the home
role is parker, and the entity is parker. Intentional. In prose, "parker"
without qualification means the entity.

## Deferred (real options, deliberately not now)

Memory taxonomy · interruption model · personality content · expanded
inputs (screen capture, clipboard stream, calendar/mail, typing rhythm,
reading the gym tracker and comics library) · expanded outputs (TTS,
wallpaper/lockscreen as ambient surfaces) · drawn Wayland face · mobile
face · modes (`parker mode mechinterp`) · Tailscale for remote parker ·
multi-provider abstraction

## Known gaps (built, but not finished)

Deferred above is what we chose not to build; this is what exists and falls
short. Add a line when you hit a shortcoming, strike it when it is fixed.
Detail belongs in the nearest AGENTS.md; this is the index.

- **visage is not a nix package.** The tower's window is a dev-loop binary
  (`cargo tauri build --no-bundle`, found at
  `~/parker/home/parker/visage/src-tauri/target/release/visage` by
  `reach/visage.rs`), so a visage change is a manual build on top of the
  rebuild. The porthole half is packaged already (`porthole.nix`); the Tauri
  frame wants the same: a derivation for `src-tauri` (GTK/WebKit stack as in
  the `visage` dev shell, page from `visage-page`), installed on PATH, with
  `reach/visage.rs` and the `parker-visage` comment in `home/parker/default.nix`
  switched to it. Tauri must be built through the CLI, not plain
  `cargo build --release`, or it loads devUrl (visage/AGENTS.md).
- **No narrow layout or touch controls** for the phone porthole
  (docs/visage/porthole/README.md, "Later").
- **Palette needs two Escs on giskard**, unreproduced off the Mac
  (visage/AGENTS.md, "Open").

## Verified (was: verify at build time)

Answered July 2026, mostly by measurement rather than reading.

- **Kraken LCD — drive it through CoolerControl, not liquidctl.** The base
  2023 does have a writable 240×240 LCD (`1e71:300e`, liquidctl's
  `KrakenZ3`). The feared conflict is real but the resolution is the
  opposite of expected: CoolerControl does not compete with liquidctl, it
  **embeds** it (liqctld folded into coolercontrold in v3), already holds
  `/dev/hidraw0`, and reports an `lcd` channel. There is no per-device
  opt-out, so taking the Kraken away means turning CoolerControl off and
  losing the fan curves. *Wire-verified July 2026 on coolercontrold 4.3.1,
  face pushed and blinking:*
  - The endpoint is
    `PUT https://127.0.0.1:11987/devices/{uid}/settings/lcd/lcd/images?log=false`
    (channel name in the path — `lcd` appears twice; HTTPS, self-signed, so
    `-k`/no-verify). Multipart form: `mode=image`, optional `brightness`
    (0–100) and `orientation` (0/90/180/270), exactly one file in
    `images[]` (jpg/png/gif/tiff/bmp — daemon resizes and re-encodes).
    `?log=false` exists precisely so scripted updates don't spam the log.
  - Auth is `Authorization: Bearer cc_<uuid>`; v4 401s *everything*,
    even `/health`. A write-scoped token named `parker` is minted in UI →
    Access Protection (revocable there) and kept on the machine, in
    `$XDG_STATE_HOME/parker/coolercontrol.token` (badge.nix). The device UID
    needs no API at all — it's in the world-readable
    `/etc/coolercontrol/config.toml`: the Kraken is
    `b308185de6658c811067b6e74271d1607b5340c8e2f9bdb93d3ca518c403796b`.
  - **The ~5s refresh fear was wrong: measured ~0.2s per applied frame**,
    a 4-frame blink streamed at ~5fps with all 200s. The bug reports were
    about older liqctld, not this path.
  - **Animated GIFs are rejected by Kraken 2023 firmware 2.x** (liquidctl
    issue #631) — upload succeeds only for static images, so animation is
    frame streaming by re-upload, and the device shows a static frame
    between pushes. Design expressions as poses-plus-occasional-frames,
    not as video.
  - The last pushed image persists (`/etc/coolercontrol/lcd_image.png`)
    and is reapplied on boot — parkerd should assert its own face on
    startup rather than trust what's there. Reverting to the stock liquid
    readout: `PUT .../settings/lcd/lcd` with JSON
    `{"mode":"liquid","brightness":100,"orientation":0,"colors":[]}`.
    v4.3.1 also has a `carousel` mode and a separate
    `PUT .../lcd/shutdown-image` — a parker-is-down face is a freebie.
  - Streaming morphs works in practice: 92 lerped pose frames × 2 loops
    (184 consecutive PUTs) applied with zero failures at ~0.2s each. The
    pose.rs numbers-not-pixels model ports directly; only the renderer
    (Pillow/`image` crate instead of braille) is new.
- **Fan/case RGB exists and is reachable, but not through CoolerControl.**
  The AIO itself is the base SKU (`1e71:300e` "NZXT Kraken Base") — no
  LEDs on it, and CC shows zero lighting channels anywhere. But the bus
  has two RGB controllers (July 2026, from `/sys/bus/usb`):
  - `1e71:2012` **NZXT 2023 RGB Controller** — liquidctl ≥1.16 drives it
    (`Nzxt2023RgbController`: 0 fan channels, **3 colour channels**,
    modes incl. fixed/super-fixed/breathing/pulse/spectrum-wave).
    coolercontrold 4.3.1 detects it and logs "currently not supported,
    please create a feature request", then leaves it alone — so a direct
    liquidctl writer does **not** conflict with CC here, unlike the
    Kraken. Needs hidraw access: root, or liquidctl's udev rules
    (`services.udev.packages = [ pkgs.liquidctl ]`).
  - `26ce:01a2` **ASRock Polychrome USB** — the motherboard's own ARGB
    controller, OpenRGB territory (`services.hardware.openrgb`).
  Which controller owns which physical LEDs (fan rings vs strips vs mobo
  headers) needs eyes on the case: light each channel and look. Unverified
  beyond detection — no udev rules are installed yet, so neither has been
  written to.
- **`socket2` covers what sense needs**, with sharp edges: it truncates
  data at 1024 bytes, sends addresses *without* the `0x` prefix that
  `hyprctl -j` includes, drops the client entirely if it falls 64 events
  behind, and has no snapshot — every reconnect must re-prime from
  `hyprctl -j`. Geometry, DPMS and idle are not on it and need polling.
- **opencode's system prompt is ~fully replaceable** — see Risks carried
  above for the measured floor and the traps.
- **Do not generate the Rust client.** The OpenAPI spec is real (3.1, 162
  paths, 472 schemas) but its `Event` type is an 89-variant `anyOf` with
  **no `discriminator`**, despite every variant carrying a single-value
  `type` enum. Generators emit an untagged union needing try-every-variant
  deserialization where what you want is `#[serde(tag = "type")]`. Also: no
  `securitySchemes`, 11 schema names containing `.`/`-`, and SSE that no
  generator handles. Hand-write the handful of types actually used.
- **Do not use `hyprland-rs`.** Ten months stale, maintainer stepped back,
  open unfixed breakage against 0.55+, and its read loop has no cross-read
  line buffering — an event burst splits a line and the stream terminates
  permanently. The daemon would not crash, it would go silently deaf.
  `BufReader::lines()` plus a tolerant parser is both safer and less code.

Phase 3 measurements (July 2026, against the live service in
`home/parker/engine.nix`):

- **`opencode serve` cannot bind a unix socket** — 1.18.3 offers
  `--hostname`/`--port` only. The CVE class is mitigated, not removed:
  127.0.0.1 plus `OPENCODE_SERVER_PASSWORD` (HTTP basic, user `opencode`),
  measured to 401 without it.
- **`permission = {"*": "deny", "parker_*": "allow"}` is the seam's whole
  enforcement.** On the wire the request carries exactly the six parker
  verbs and not one built-in tool. The July recon only measured deny-all;
  the allow-glob carve-out holds.
- The endpoint for sending a prompt is `POST /session/{id}/message`.
  `/session/{id}/prompt` does not exist and **silently returns the embedded
  web UI as HTML with HTTP 200**, which reads as a mysterious no-op.
- **With no provider configured, opencode silently routes to its own
  anonymous free gateway** (`opencode/big-pickle`, cost 0). Convenient
  scaffolding, but parker's sense data transits Anomaly's gateway until a
  provider is deliberately chosen — which is the one phase-3 decision still
  open, along with the provider-side spend cap that comes with it.

Still open:

- Local reflex model VRAM behavior under `OLLAMA_KEEP_ALIVE` with a game
  running — does eviction actually free in time.
- Which provider parker speaks through, and the hard spend cap at that
  provider (one web form, zero code).
