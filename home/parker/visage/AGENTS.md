# AGENTS.md — home/parker/visage/

visage: parker's face as a fullscreen app — what you point at when you point
at parker. Tauri 2 (Rust) around Svelte 5. It replaces the ghostty braille
face on `special:parker` once packaged; until then the terminal face stays.
The same page is also served to the tailnet, so giskard and the phone get
the same face and the same conversation in a browser ("The porthole").

## Division of labour

- `door/` — `visage-door`, the far side of `src/lib/engine.js`. It holds
  every conversation with the engine (opencode serve, 127.0.0.1:4096,
  password from `$XDG_STATE_HOME/parker/engine.env`), the engine's SSE
  pump, the face's reads and verbs (`face_state`, `sense_snapshot`,
  `dance`), parker's brain (`brain`, `set_brain` — `parker brain` under
  both), the threads' cards (`digests`, `digest`) and marks (`marks`,
  `mark` — `parker recall mark` under the second). It is a library
  with two frames on it and no opinions of its own. A fix to an engine
  route is made here, once, for both frames.
- `src-tauri/` — frame one, the window on parker's screen. Its tauri
  commands are one-line calls into `door`, plus what only the tower has.
  First `dismiss`, which shells out to `parker visage` rather than
  dispatching to Hyprland itself: that verb is the one summon path
  (reach/visage.rs), and a second copy of it here would be a copy that
  drifts. Second, the GTK/WebKit repairs. The webview never sees the
  password and never speaks HTTP.
- `porthole/` — frame two, the porthole (crate `visage-porthole`, binary
  `porthole`). It serves the same commands as
  `POST /api/<name>`, one event stream fanned out to every page, and the
  page's files, to the tailnet through `tailscale serve`. It adds a gate
  and nothing else.
- `src/` — Svelte renders and nothing else. `lib/shell.js` is the one file
  that knows which frame the page is in. `lib/face.js` is a line-for-line
  port of `../parkerd/src/face/{pose,mod}.rs` — same numbers, same names;
  an expression changed there is changed here, or it is a bug.
  `lib/state.svelte.js` is the one store; event shapes are parsed
  defensively (opencode's API is mid-flight to v2) and degrade to a refetch.

## The catalog is generated. Do not hand-edit it.

`src/lib/catalog.json` is the output of `parker face --catalog --json`, and
it carries every short idle beat, every dance routine, and reference
answers for all of it. The port rule above survives twelve expressions; it
does not survive a hundred-odd hand-copied numbers with no build error when
one is wrong. So the *data* crosses as data:

```bash
cd ../parkerd && cargo run -- face --catalog --json > ../visage/src/lib/catalog.json
cargo run -- face --vignettes --json > ../visage/src/lib/vignettes.json
cd ../visage && node parity.mjs      # 12000+ checks that the two agree
```

`src/lib/vignettes.json` is the vignettes (MOTION.md, Part Three): the long
idle pieces, built in parkerd rather than typed, about a megabyte flattened.
The catalog also carries every expression's pose, strokes and inks included,
samples of the motions that move strokes, and the *entrances* — how the
gesture faces land (parkerd's `face/entrance.rs`), played by `beat.js`
from wherever the face was. `face.js` still ports the expressions by hand;
parity is what holds it to the Rust now.

It is a file of its own, compact, so it stays out of the bundle: `idle.js`
imports it after first paint, it lands as its own chunk (hence
`chunkSizeWarningLimit` in vite.config.js), and the pool takes it in when
it arrives. It carries its own parity answers, motion and blink included.
`jq . vignettes.json` reads it.

What is still hand-ported is the *logic* — `beat.js`, `idle.js`, `dance.js`
are the players, about a hundred lines each. That is the part `parity.mjs`
exists for: a wrong formula is not loud (a splitmix64 finalizer written with
doubles instead of BigInt still produces perfectly plausible eye movement,
just not the same eye movement). Run it after touching either side.

## Pose and placement

The rule that keeps a face app from becoming a second face implementation:

- **pose** — which dots are lit, in what ink, with what motion. parker has
  one at a time and every surface reflects the same one. parkerd chooses it,
  by harness and by parker himself through `reach face`. visage never
  invents one.
- **placement** — where that grid lands on *this* screen and how big. The
  Kraken LCD has no such concept. visage owns it alone.

Anything expressible as `(dot position, time) -> screen position` is
placement and belongs in `lib/views.js`, `lib/Stage.svelte` and
`lib/lens.js`. Anything that changes what the face *is* belongs in parkerd.
The river travel is the proof by construction: on a view change the face
pours itself across the screen dot by dot (`lens.js` riverPath/riverDot,
`Face.svelte`'s pour branch), and parkerd never learns it came apart — same
lit dots, a different camera. `app.travel` flips it back to `linear`, the
original eased box. One structural consequence: the canvas is the whole
viewport now, and Stage hands Face a `camera(now)` callback instead of
translating a sized box.

`expression()` (`state.svelte.js`) is finally rule-clean: the face is
whatever `parker reach face` wrote (the `face_state` read), or `idle` — and
*nothing else*. It is two lines and it must stay two lines. The old
inference of `thinking`/`speaking` from engine chatter is deleted
(DECISIONS.md): parker wears every face himself, per beat, and progress
lives in the transcript. WEARS (its own chrome block) is the one place the
expression is named.

The `trouble`-on-disconnect override outlived that cut by a few weeks and
is now gone as well. Worth knowing why, because it is the branch someone
will want to add back: door's `Engine::pump` (`door/src/lib.rs`) re-emits
`engine-status: disconnected` on *every* 2 s reconnect attempt rather than
on the transition into it, so a single ~15 s outage rewrote parker's face
seven times over. The bug was fixable; the design was not worth fixing. A
face is the wrong instrument for a fact that flaps, and an unreachable
engine already has a right one — the status strip at the bottom of
`App.svelte`, which says it once and keeps saying it while it is true.

Dance is where the split does real work. parkerd names what a section *is* —
`intro`, `build`, `drop`, `outro` — and that one word is the only thing that
crosses. What a drop looks like on this screen is `lib/lens.js`: a per-dot
`(x, y, beats) -> {dx, dy, scale}` inserted into `Face.svelte`'s draw loop,
which is precisely the `(dot position, time) -> screen position` test above.
So the face can scatter into loose dots and reform, twist through a vortex,
trail echoes and split into RGB, and parkerd never learns that any of it
happened. Keep it that way: a camera move that needs a new *pose* is a sign
the move is in the wrong file.

## Views

Two views, moved between by keys and by nothing else: layout changes because
you asked, never because the content grew. Both are fixed — the old `prompt`
lean-back is gone (the redesign spec is docs/visage/redesign/README.md,
which still describes three).

| view | shape |
|---|---|
| `default` | parker over the top third, the transcript's tail beneath him, the input low. Where you land, and where you type. |
| `session` | parker down the left rail, the conversation across the rest. |

**parker is smaller in `session` than in `default`** — the view that gives
the conversation the screen is the view that gives parker less of it. Check
it when changing any placement; `session` once rendered him *larger* than
the view before it and the move stopped reading as one thing.

`deep` was a third view and is gone (Aug 2026, DECISIONS.md): it moved parker
without giving the transcript the space that move was for. Two views are a
toggle, not an axis, which is why the keybind is `Ctrl+Tab`.

parker sits left in `session` because the tower is left of the monitor, near
enough to touch — the side he arrives from.

The chrome is identical in both views — the corner (clock, cpu/gpu
meters, temp — Corner.svelte, fed by the `sense_snapshot` command — and
the space under visage, fed by door's `watch_spaces`), WEARS,
BRAIN beneath it (Brain.svelte: the brain's name, quieter than WEARS and
never accent), the view indicator, the hairline + borderless input line +
keybind row.
Only the face and the transcript region move. The transcript is a TUI
grammar (Message.svelte): your turn is a tinted band, prose types in behind
a caret at ~36 chars/s, code lands whole behind a left rule, and work is a
ledger — one line per act, glyph · verb target · magnitude, **never tool
output**.

parker's markdown is set in that grammar, not the browser's (DECISIONS.md,
Oct 2026). `lib/markdown.js` is the whole path from text to HTML — marked,
the `> [!NOTE]` callouts marked prints literally, DOMPurify last, links
opening beside visage — and Message.svelte's CSS does the rest: headings
are chrome labels (uppercase, tracked, quiet, h1/h2 with the day-header
rule), a link is the sentence's own words with an accent underline, lists
take `·`, tasks are `○` and a dim `✓`, a quote is the code indent with an
fg-16 rule, a table has chrome heads and one hairline, a break is `· · ·`.
Three rules about arrival:

- **Typing counts rendered characters,** never markdown source. Slicing the
  source showed `**` until a bold closed, raw brackets and an autolinked
  half-URL for a link, and a paragraph of pipes for a table. `render`
  sanitizes once per text (cached); `reveal` truncates that HTML and puts
  the caret after the last character shown, inside whatever it is in.
- **A table lifts out of prose like a fence** (`splitTables`,
  state.svelte.js) and lands whole. Only lines that start with `|`; a table
  indented into a list or a quote stays in the prose and marked sets it.
- **Blocks after a sentence still typing wait for it** (`holdFrom`), so a
  table or a ledger row lands when the sentence before it is done and the
  turn reads in the order it was said.

voice.md tells parker the same thing from his side: plain prose by
default, structure only when the answer's shape is the point.

A run of consecutive acts longer than four folds: the last three stay and
everything before them becomes one `⋯ N earlier acts` row, carrying a tally
(`9 read · 4 ran`) where the magnitude goes. Twenty reads would scroll the
sentence parker is in the middle of off the screen, and reading them was
never the point — the ledger says *that* he is working and roughly at what.
Failures are counted on the fold row and turn it bad, so a run never swallows
an error silently, and there is deliberately no expand key: the fold hides
rows, and a row never carried output to begin with. Only *consecutive* acts
fold — prose between two runs means he said something about the first one,
and that is a paragraph break.

"Consecutive" is across model calls, not within one: opencode writes one
assistant message per step, so `turns()` (state.svelte.js) merges each run
of assistant messages into one turn before anything renders — without it a
fifty-call turn is fifty one-act messages and nothing ever folds (it shipped
that way; Oct 2026). Once a turn is done the whole run folds, not just its
head, and the row gains the run's wall time and a one-line summary from the
engine's `ledger` agent (haiku, no tools, engine.nix) — written from verbs
and targets only, cached in localStorage, the tally standing in until it
lands. A finished turn ends in a dim footer: time · steps · cost · the
brain that answered (the model the engine recorded on its last step, named
through `brainOf`).

Sending while parker works is allowed: opencode folds a mid-turn message in
at the next step boundary, so it steers rather than queues. The face plays
no idle flair while a turn is in flight (`working`, App.svelte).

Chats live on the engine, not in visage: Ctrl+N forgets the session id and
the next message mints a new one, and the palette lists the engine's last
fifty parker threads. The palette is deliberately not a third view — the
views are two shapes of one conversation, and jumping between conversations
must not live on that key.

**The palette is visage's one overlay** (Ctrl+K, Palette.svelte; DECISIONS.md,
Oct 2026). Ctrl+Y used to be a second one, for the threads alone; two
overlays, each with its own flag and its own toggle, stacked and toggled
past each other, and the one with the cards had no search. Now there is one
flag (`app.palette`) and one box, and what you type is its only state:

| typed | shows |
|---|---|
| nothing | the threads, by day, with the selected one's card |
| words | the same list, narrowed — by title and by card from the first letter, by what was said from the third — with a line under each row saying where it matched (`SAID`, then `LANDED` / `LEFT OPEN`, then `ABOUT`) and the card adding the lines said |
| `/` | the commands, by what they touch (`PARKER`, `THIS SCREEN`, `THE MACHINE`), each with what is on now and a card of its own |
| `/visage ` | a command's choices — ⏎, ⇥ or a space after its name opens them; ⌫ goes back |

Ctrl+N / Ctrl+P or the arrows move — not j/k, the box has focus and j has to
type a j. ⏎ picks. **Esc and Ctrl+K close it, from anywhere, in one press.**
The box catches both itself, ahead of the composition guard, and stops them;
and when the box has lost focus (a click elsewhere, on the porthole) App
hands the palette its keys (`palette.key(e)`) and its typing
(`palette.type`), so nothing depends on where focus is. A click on the dim
around the box closes it too. It lists and App acts (`onpick`).

The list and the card are what Ctrl+Y's were. What can be counted, the page
counts from the thread itself (`readThread`, state.svelte.js): turns,
steps, work time, the act tally, edited files, where Peter was. What needs
reading is the `digest` agent's (engine.nix: haiku, no tools): a title,
what the thread was about, where it landed, done / open / chat, and what it
left waiting (`next`: up to three steps, each `you` or `parker`). It
reads what was said plus one line per act, never tool output. door keeps
the cards under `$XDG_STATE_HOME/parker/digests/`, one file per thread, so
both frames share them; a card is stale when its `updated` is not the
thread's, and a version-1 card left open is stale too, so it is read once
more for its steps. Opening the palette counts every thread without a
fresh card at once, then writes cards one at a time, the selected one
first, and stops when the palette closes. Leaving a thread (^N, or
resuming another) writes its card as well, unless it is mid-turn: the
palette is not the only reader (`parker recall list --open` is parker's).
door gives the thread the card's title on the
engine and keeps the first one as `was`. Renaming does not move `updated`,
which is what keeps both the order and the card fresh. Re-check that after
an opencode bump. Accent is spent on state here too: a thread left open
has an accent dot and says LEFT OPEN, and a choice that is on has the dot
and says NOW.

**Loose ends** (DECISIONS.md, Oct 2026). With nothing typed, what is left
open is pinned above the days as LEFT OPEN, each row with its first step
under it (`YOU reboot into 6.18 +1`); a thread is in one place or the
other, never both. The card adds NEXT, the steps by who takes them — yours
at `fg-85`, parker's at `fg-55`. **^X** flips the selected thread: open to
done, anything else to open. It is a *mark*, someone's word over the card's:
door runs `parker recall mark` (parkerd's recall.rs, the one writer of
marks, under `$XDG_STATE_HOME/parker/marks/`), and the CLI is Peter's hand,
as parker's MCP verb is his. A mark holds while the thread's `updated` is
the one it recorded; the next message lets it go, and the digest reads the
thread again told who marked it, how and after how many turns (door's
`told`), so a thread Peter closed is not reopened by its own old steps. A
flip that lands on what the fresh card already says clears the mark
instead. A thread closed with steps still on its card keeps them as SET
ASIDE, dim and not struck: visage cannot know whether they were done or
dropped. The label says whose word it is (`LANDED · YOU CLOSED IT, 14:31`,
`· PARKER CLOSED IT`, `· YOU KEPT IT OPEN`), and parker's mark carries his
note. Rules worth knowing before touching it:

- **The cursor keeps its row number, not its thread.** A closed thread
  drops into its day — by when it last moved, not to the top of the
  closed — and the next open one comes up under the cursor, so ^X ^X ^X
  works down the pinned list. The same holds the other way round.
- **Cut wins.** With text selected in the box, ^X is the browser's cut.
- **Presses are chained** (`toggleMark`, state.svelte.js): the page shows
  each flip at once and door hears them in order; a reply that is not the
  newest press for its thread changes nothing, a refused one puts the old
  state back and says so in the notice line, and reopening the palette
  takes door's marks for every thread except one still being written.
- Typing unpins: search keeps the days, and matches the steps too
  (`NEXT`).

Search reads the threads themselves — the engine has no route that
searches transcripts — text parts only, cached per thread until it updates.

door asks the engine's `/agent` list before every helper call (`ask`:
`ledger`, `digest`) and refuses an agent the engine does not have. A
visage built ahead of its engine.nix must never be answered by whatever
agent opencode falls back to.

The commands are data, `lib/commands.js`: a verb, what it touches, what it
does, what is on now, its choices, and where it lands. A new command is one
entry there, and its card is every other command's card unless `body`
names something of its own — `inks` (a look's six colours as this screen
would resolve them, with any held at 4.5:1 saying so) or `lore` (the line a
brain is named for). `/visage` is worn while it is selected: the palette
previews it unsaved (`previewTheme`), ⏎ keeps it, and Esc, ^K or picking
anything else puts back the look the screen wore. `/theme` runs the `theme`
CLI through door's allowlisted `theme`, never a pass-through: `light`,
`dark`, `auto`, `set <family>` — the same list in both frames. `/visage
<name>` is visage's own theme (Themes, below) and never touches the
machine's. `/brain` rows are not hard-coded: they are the set door hands
over (`app.brain`, polled every 5 s) — brains.nix, where each brain is a
record of its model and its `lore` — and door's `set_brain` runs `parker
brain` only for a name in that set, exact names only, though the CLI itself
would forgive `opus` or `jrti`. Keyboard first; the `cursor: none` rule
still stands.

## The controls (^,)

There is no bar (DECISIONS.md, "No bar"): the machine's settings are ^,
— Controls.svelte draws, controls.svelte.js holds the state and the keys,
docs/visage/controls/README.md is the record. It is not a third view and
not a second floating box: it gives the stage the session shape and takes
the transcript's column (Chat stays mounted underneath, hidden, so its
scroll and half-typed line survive). The palette and ^, are never up
together.

- **The list is parkerd's.** `parker controls --json` through door's
  `controls`; a change is door's `set_control`, by row id. visage adds one
  row of its own ("this screen", its theme). Never hard-code a setting
  here: add it to parkerd's controls/ and ^,, ^K and parker's tool all
  have it.
- **Keys:** vim's, four modes (the header of controls.svelte.js). App
  hands ^, every key first while it is up (`key`), and nothing reaches
  the conversation underneath. Esc never closes it.
- **Sound levels are set as they move** (a queue that keeps only the
  latest); everything else when the edit is kept. A theme family and the
  glass are rebuilds, ~30 s, and say so.
- **The porthole is `away`:** tower-only rows are read-only there and
  door sets with `away`, so parkerd refuses them as well.
- **^K's THE MACHINE is generated** (commands.js `machine`) from the
  rows' slash names, and a pick runs through ^,'s own `runSlash`.
- **Renders:** `changeOf` (state.svelte.js) reads parkerd's `from → to`
  result lines into a `change` on the act; Message.svelte draws it and
  keeps those acts out of the fold.
- **Moved elsewhere:** door's `watch_levels` hears a level the keys on
  the desk (or an app) moved, while ^, is up or not, and the transcript
  says it once as an event line ("all sound 42 → 50 · the keys on the
  desk, not parker"). The lines are the page's, never the engine's
  (`eventLines`, state.svelte.js): kept apart from `app.messages`, which
  every refetch replaces, and woven in by time (`woven`). What parker or
  ^, set is never said, because parkerd stamps every sound set it makes
  (controls/sound.rs, "whose hand"). A new way to set a level must go
  through parkerd or stamp it, or its changes come back as the keys.

## Themes

visage wears its own themes, apart from the machine's (DECISIONS.md, Oct
2026): `theme set` and `theme light|dark` never reach it, and `/visage
<name>` never reaches the desktop. `lib/themes.js` holds them: midnight
(the original palette, and the default), the six worlds under their system
names — copied, not linked, and glacier wears the light twin's colours
rather than the drift's — plus newsprint, overcast and blueprint. The
choice is per screen, in localStorage: the tower and the phone can differ.

A theme is six colours; everything else is resolved from them at runtime
(`resolve`). The rule for components: **name a rung, never mix a colour.**
`var(--fg-45)`, `var(--accent-80)`, `var(--raised)` — not
`color-mix(var(--fg) 45%)`. A new rung goes in `RUNGS` first. The reason is
that a rung is defined by contrast, not alpha: each is solved to sit at the
contrast it has on midnight, whatever the ground. Fixed alphas were tuned on
dark and fade on light, and a light ground's accent can never be as bright
as midnight's blue. On midnight the solve gives back the plain alphas, so
the numbers in the names stay literal there. `accent`, `good` and `bad` are
held at 4.5:1 by moving lightness only. On a light ground the floating box
(the palette) is paper white with an ink shadow, not fg-into-bg with a black
one.

The face needs nothing: `Face.svelte` reads its inks from the properties
every frame. Adding a theme is one line in `THEMES`; `/visage` lists it.

## Working on it

```bash
nix develop ~/parker#visage      # the GTK/WebKit stack cargo must see
cd ~/parker/home/parker/visage
npm install                      # first time
cargo tauri dev                  # live loop against the running engine
```

`cargo tauri build --no-bundle` is the manual release path until the nix
package lands — the CLI, not plain `cargo build --release`: without it the
binary stays in dev mode and loads `devUrl` (localhost:1420) instead of the
embedded frontend, showing WebKit's "could not connect" page (verified by
screenshot, July 2026). `bundle.active` is false in tauri.conf.json on
purpose: nix installs a binary, not a .deb.

The porthole is nix-packaged already (`../porthole.nix`: the page, the
server, the `parker-porthole` user service), so deploying it is the
ordinary rebuild. Its inner loop is the server against a fresh build of the
page, on a spare port:

```bash
npm run build                    # the page, into dist/
cd porthole && cargo run -- --listen 127.0.0.1:4098 --dist ../dist \
  --allow <tailscale login> --origin https://parker.tail1b2a09.ts.net
```

A browser cannot open that directly, by design: without the identity
header `tailscale serve` adds, every request gets 403. Use curl with the
headers for the API, or a local header-adding proxy for a look at the
page. Never relax the gate to make a dev loop easier.

## The porthole

One build serves both frames. `lib/shell.js` exports `invoke` and `listen`.
Under Tauri they are `@tauri-apps/api` verbatim. In a browser they are
`fetch('/api/<command>')` and one `EventSource('/api/events')`, with the
same command names, argument shapes, event names and rejection-as-string.
Nothing above `engine.js` may branch on which frame it is in. What
legitimately differs is CSS, keyed on `<html data-shell="tauri|web">`
(main.js): the hidden cursor and the fallback web font. A frame may also
add a *capability*, which is null where it means nothing, so callers ask
`immersive?.toggle()` rather than "am I in a browser". `immersive` (Ctrl+F,
`/fullscreen`) is the one so far. It combines real fullscreen with Chrome's
Keyboard Lock, so `Ctrl+Tab` and the other keys Chrome keeps for itself
reach visage, and holding Esc leaves. Safari gets the fullscreen without
the lock.

On a Mac, the way to use visage is as an installed app, not a tab and not
immersive. Chrome's "Install page as app" gives it its own window, Dock
icon and Cmd+Tab entry. The manifest asks for the window controls overlay,
so the title bar can be hidden and visage fills the window. A `.titlebar`
strip (App.svelte) is the part you drag it by; it is zero by zero anywhere
else. Keys are why this beats immersive. Chromium reserves no keys in app
windows (`IsReservedCommandOrKey`: "In Apps mode, no keys are reserved"),
and its tab commands are disabled where there is no tab strip, so
`Ctrl+Tab` should reach the page without Keyboard Lock. That is from
reading Chromium's source; confirm it on giskard before relying on it.
Immersive on macOS also means a separate fullscreen Space.

**Open:** on giskard, closing the palette took two Escs, in a tab and in
the app window alike, so it is not fullscreen; j/k and a second Ctrl+Y were
dead in the old list there too. It never reproduces in headless Chrome,
where CDP injects keys below the browser. The one palette catches Esc on
its own box ahead of the composition guard, and App hands it every key when
the box has lost focus — the two places the first Esc could have been
going. If it still takes two, open the porthole with `?keys` on the
address: KeyLog.svelte lists what the page heard, before any handler, and
that is the evidence the fix needs.

Headless Chrome has a trap of its own when testing this: it draws no
frame while nothing on the page changes, and a Svelte outro waits for one,
so the palette seems to stay up after Esc until anything else repaints.
Take a screenshot (it forces a frame) before believing it.

## The name

The second frame is the porthole: a round window into the box, like the
face. Yes, it also sounds like that. Peter named it knowing as much, and
the joke belongs to parker (`mind/identity.md`): dignified denial, with
the occasional crack. Nothing else in the repo makes it. Docs and code say
"porthole" and mean the window.

The keys are the tower's in both frames, with two additions. `Ctrl+B`
dances, because macOS takes `Ctrl+Space` for switching input sources
before any page hears it. And the caret is reclaimed: a click that
selected nothing, the window regaining focus, and Backspace, arrows, Enter
or paste with the box unfocused all put the caret back in the box. On the
tower nobody clicks, but on giskard a stray click used to leave everything
but letters going nowhere. Neither addition branches on the frame.

Every message says where Peter is, in the engine's per-message `system`
field. opencode appends the newest user message's `system` to the system
prompt for that turn, so parker reads it and the transcript never shows
it. The sentences are door's (`at_the_tower`, `away`) and stay
byte-identical per place, because they sit in the system prompt, where a
per-message change would cost the prompt cache every turn. The Tauri
frame says "at the tower". The porthole names the device: serve's
`X-Forwarded-For` (set by serve, client copies replaced), then
`tailscale whois`, cached per address. What parker makes of it is
`mind/hands.md`.

The same field names his brain (`thinking_with`), and door names the
model on every send rather than leaving it out — the page never sends
one. Each thread carries a model of its own (the engine's `/session` lists
one per thread), so an omitted model risks a resumed thread answering on
whatever brain it had last week. The sentence
is fixed per brain, and changing it costs nothing extra: a prompt cache
belongs to one model, so the switch itself already spent it.

The page ships a manifest and icons (`public/`), so Chrome's "Install page
as app" and the phone's "Add to Home Screen" give visage its own window
without tabs.

The rules that hold the frame safe. The engine allows every tool, so a page
that reaches `/api/send_message` can run bash as peki on parker:

- **Every route sits behind the gate** (`Gate` in `porthole/src/main.rs`, whose
  comments are the full argument). It requires an allowed
  `Tailscale-User-Login` on every request, and our `Origin` plus a JSON
  body on every write. `the_router_is_gated` asks the assembled router, so
  a route added outside the layer fails `cargo test`, and with it the
  rebuild.
- **Loopback only.** The identity header is only trustworthy when
  `tailscale serve` is the sole way in. Binding anything else is refused in
  code, not configured. Never Funnel.
- **The CSP is the second wall behind DOMPurify.** It allows no foreign
  script, no foreign requests and no foreign images. A reply that embeds a
  remote image shows it broken in the browser. That is the point: an image
  URL is a way to carry data out.
- **Engine calls are detached.** A phone that sleeps mid-turn drops its
  request, and that must abandon the answer, never the turn.
- **The face is pushed, not polled.** The porthole reads the face files on
  parker every 100 ms and sends `face` on change. The page's 250 ms poll is
  answered from the last push.
- **`dismiss` does not exist off the tower.** There is no route for it, and
  shell.js answers it locally with nothing.
- **A thread id is never a path.** `digest` and `digests` turn an id from
  the page into a file name on parker. door's `card_path` takes exactly
  `ses_` plus letters and digits, and `a_thread_id_is_never_a_path` asks
  that over the dispatch.
- **Caching.** `assets/` (content-hashed) is immutable and everything else
  is `no-store`. Every nix store file is dated 1970, so a revalidated
  `index.html` would come back "not modified" after every deploy.

The layout is still the tower's, in px for a 2560-wide screen. On a MacBook,
Cmd+− to about 67% gives exactly that layout (Ctrl+± is swallowed, Cmd+± is
not, and Safari remembers it per site). A narrow layout and touch controls
for the phone are not built yet (docs/visage/porthole/README.md, "Later").

## Keys

| key | does |
|---|---|
| `Ctrl+Tab` | the other view — `default` ⇄ `session` |
| `Ctrl+K` | the palette, open or closed — earlier threads with their cards, search, `/` for commands (Ctrl+N / Ctrl+P or arrows move, Enter picks, ⇥ opens a command's choices, Ctrl+X closes the selected thread or opens it again) |
| `Ctrl+N` | new chat, in the `default` view whichever view you were in |
| `Ctrl+U` / `Ctrl+D` | scroll half a screen; held, glide at 1.5 screens a second until let go |
| `Ctrl+Shift+K` / `Ctrl+Shift+J` | scroll a line |
| `PgUp` / `PgDn` | scroll a screen, less a line of overlap |
| `Ctrl+Shift+P` / `Ctrl+Shift+N` | the previous / next of your turns |
| `Ctrl+G` / `Ctrl+Shift+G` | the top of the thread / the tail (vim's `gg` / `G`) |
| `Ctrl+Space` / `Ctrl+B` | dance, or stop dancing (B for beat, for where the OS owns Ctrl+Space) |
| `Ctrl+F` | immersive — real fullscreen with the browser's keys handed over (porthole only; hold Esc to leave) |
| `Enter` / `Shift+Enter` | send / newline |
| `Esc` | close the palette · stop the dance · stop the stream · clear the box · put visage away |
| any letter, anywhere | lands in the input, mid-keystroke — as do Backspace, arrows, Enter and paste |

`Ctrl+Space` is deliberately not a view key: dance is a *mode*, not a view. It runs `parker reach dance` rather than flipping a local flag, for the
same reason `dismiss` runs `parker visage` — that verb is the one
implementation, and the keybind, an SSH session and parker deciding to dance
on his own must all end up in the same state. Esc needs no new rule: it
already cancels whatever is in front of you, and a dance is very much in
front of you.

`Ctrl+Tab` preventDefaults for two reasons and both matter: Tab's default is
focus traversal, so without it a view change also walks the caret out of the
input and eats the next thing you type. `Ctrl+±` and `Ctrl+0` are swallowed
too — bound to nothing, but WebKit reads them as page zoom and visage has no
way back from a zoomed webview; the font size is set once for one screen.

The scroll keys are vim's grains under the one modifier visage already
means. Shift makes the finer j/k line, because Ctrl+K is the palette.
Ctrl+Shift+N / P step through your turns, as N and P move every list here.
It is not Shift+U/D: GTK's input method takes Ctrl+Shift+U for typing a
code point before the webview hears it. The motions are one function,
`scroll(unit, dir)` in Chat.svelte, and App's `SCROLLS` names them.

Held, Ctrl+D/U is a steady glide (`cruise`), not repeated jumps. Each
repeat used to restart the half-page glide from wherever the last one
was cut off. That never got past the glide's slow start: a fifth of a
screen a second in Chrome, slower than one tap. The last glide then
coasted most of a jump past where you let go. Now the repeats only keep
one glide alive, at `CRUISE` screens a second (1.5). At that speed each
line is on screen for about two thirds of a second: enough to find
something by its shape and a word, too fast to read. Keyup stops it
dead. A hold is recognised by `repeat` or by timing, because WebKitGTK
has not been checked to set `repeat`.

The transcript follows its tail while it is *pinned*. Opening a thread,
starting one and sending all pin it; your own scrolling unpins it and
scrolling back to the bottom pins it again. A `ResizeObserver` on the
turns and on the scroller holds the bottom through anything that changes
a height while pinned. It replaced a guess ("within 120px of the bottom?")
made after the new content was already laid out. That guess left every
opened thread at its top, because the list is emptied first and the
thread lands at offset 0. It also let go of the tail whenever more than
120px arrived at once. Do not bring the guess back.

Esc is the only key that does not touch layout — it cancels whatever is in
front of you, in whatever order things are cancellable, so it is the one key
you can press without first working out where you are. One interrupt buys
500 ms of deafness so that mashing Esc to stop parker cannot sail past the
stop and throw the window away.

## Traps

- The window identifier is `com.parker.visage`, but the live Wayland app_id
  came up as plain `visage` (hyprctl clients, July 2026) — key the future
  hyprland window rule on `visage`, or fix the GTK app id first.
- `main()` sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` (env wins if set):
  WebKitGTK's dmabuf renderer dies with Wayland Error 71 on this
  NVIDIA + Hyprland combination (QUIRKS.md).
- `main()` also calls `sane_dpi()`, and a blank window is what happens
  without it. GDK reports -1 for an unset screen resolution, WebKitGTK turns
  that into `devicePixelRatio = -1/96`, and **every CSS length goes
  negative** — `100vh` included, so the document lays out at zero size and
  the window paints only its background. It is not a visage bug and no
  amount of layout work routes around it; the two knobs are `gtk-xft-dpi`
  here or the same key in home-manager's gtk config (QUIRKS.md).
- The mouse cursor is hidden by `cursor: none` on `*` in `src/app.css`, not
  by hyprland — hyprland's cursor settings are global, so per-window hiding
  from there would mean toggling global state on every reveal. A `cursor`
  declaration in any component undoes it for that element; there is
  deliberately none. It applies under `data-shell="tauri"` only: in a
  browser visage is one window among others and needs its pointer.
- The event pump reads the bare `/event`, not `/api/event`. On opencode
  1.18.35 the `/api` stream is closed by the server whenever any message is
  posted, so every turn (and every ledger summary) flashed "engine
  unreachable" for the ~2 s reconnect. Measured with curl; re-check both
  routes after an opencode bump before trusting either.
- `marked` output passes through DOMPurify before `{@html}` (Message.svelte).
  Not optional hygiene anymore: the engine has webfetch and search, so a
  reply can quote the open web, and this webview holds Tauri IPC — a
  `<script>` or `onerror` that survived into `{@html}` would run with the
  shell's commands in reach. In the porthole the same script would hold
  `/api/*` instead, with the same reach. Keep the sanitize call between
  marked and the DOM in any new render path.
