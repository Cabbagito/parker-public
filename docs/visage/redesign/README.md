# Handoff: visage — the redesign

## Overview

A pass over `home/parker/visage`: three views instead of four, a TUI grammar
with no chat boxes, work reported rather than transcribed, a hybrid streaming
model (blocks land whole, prose types), eleven new expressions, and a travel
between views that flows instead of sliding.

The face is unchanged as a *thing* — same 96×40 grid, same pose numbers, same
lerp. What changes is what surrounds it, what he is allowed to wear, and how
he gets from one view to the next.

## About the design files

The files in this bundle are **design references written in HTML** — running
prototypes of the intended look and behaviour, not code to paste in. visage is
Tauri 2 + Svelte 5; the task is to recreate these designs there, in the
project's own idioms (`$state` runes, one store in `state.svelte.js`, Svelte
components, the pose/placement split), not to import anything from here.

`parker-face.js` is the exception worth reading closely: it is a working port
of `src/lib/face.js` plus the new travel algorithm. The travel maths in it is
meant to be transcribed into `src/lib/lens.js` / `Stage.svelte`, not shipped.

**Fidelity: high.** Colours, type scale, spacing, motion constants and copy are
final unless flagged as open at the bottom.

**Scale.** The design files draw each screen at 1280×720. The real surface is
2560×1440, so **every pixel value in this README is already doubled** for the
real screen; the design files hold exactly half of each. Type in the files is
deliberately larger relative to the frame than today's `font-size: 15px` on
2560 — that increase is intentional and is part of the design.

---

## What was decided

| decision | value |
|---|---|
| palette | today's placeholder palette, polished — no theme switch yet |
| views | three: `default`, `session`, `deep` (today's `home` and `prompt` merge) |
| input | no border, no background: a `›` and a block caret sitting in space |
| your turn | a tinted band with an accent left rule |
| work | a ledger: one line per act, magnitude on the right, **no tool output** |
| the corner | combined: time+day, his sentence, two meters, in vs out temps |
| WEARS | large, same position and size in every view |
| view indicator | horizontal hairline, same position in every view |
| keybinds | one row, same content, same place in every view |
| streaming | hybrid: blocks land whole, prose types behind a caret |
| pose transition | unchanged — the existing lerp, spring on voluntary |
| travel | `linear` (today) or `river`, a config flag; both implemented |
| expressions | parker wears all of them; only `trouble` stays an override |

---

## Design tokens

### Colour

Unchanged from `src/app.css` — these are the existing variables, and nothing
new is introduced:

```
--bg        #101014
--fg        #c9c7cd
--face-fg   #c9c7cd
--accent    #7aa2f7
--good      #9ece6a
--bad       #f7768e
```

Derived values, all expressed as alpha over `--fg` or `--accent` so a future
`theme set` keeps working:

| role | value |
|---|---|
| prose | `--fg` |
| secondary prose | `color-mix(in srgb, var(--fg) 55%, transparent)` |
| ledger row | `color-mix(in srgb, var(--fg) 45%, transparent)` |
| ledger target (the filename/command) | `color-mix(in srgb, var(--fg) 72%, transparent)` |
| ledger magnitude | `color-mix(in srgb, var(--fg) 28%, transparent)` |
| hairline / rules | `color-mix(in srgb, var(--fg) 8%, transparent)` |
| corner text | `color-mix(in srgb, var(--fg) 28%, transparent)` |
| corner sentence | `color-mix(in srgb, var(--fg) 42%, transparent)`, italic |
| keybind row | `color-mix(in srgb, var(--fg) 20%, transparent)` |
| view indicator | `color-mix(in srgb, var(--fg) 16%, transparent)`, current stop `--accent` |
| your turn band | bg `color-mix(in srgb, var(--accent) 9%, transparent)`, rule `color-mix(in srgb, var(--accent) 55%, transparent)` |
| inline code chip | bg `color-mix(in srgb, var(--fg) 8%, transparent)` |
| caret (live) | `color-mix(in srgb, var(--accent) 80%, transparent)` |
| caret (parked) | `color-mix(in srgb, var(--accent) 35%, transparent)` |
| error / stop | `--bad` at 45% |

### Type

One family, monospace throughout. The design files use **JetBrains Mono**,
which the repo already hands into the wallpaper sandbox
(`system/theming.nix`); if visage should not carry a webfont, the current
generic `monospace` stack holds the design — nothing here depends on the face.

Values for the 2560-wide screen:

| role | size / line-height / tracking |
|---|---|
| prose, answers | 28px / 1.6 |
| your turn | 24px / 1.55 |
| code block | 24px / 1.8 |
| ledger row | 23px / 1.55 |
| ledger magnitude, meta | 20px |
| corner, meters | 20px / 1.9 |
| corner sentence | 21px italic / 1.7 |
| WEARS value | 52px / 1.1, tracking −0.01em |
| WEARS label | 19px, tracking 0.18em, uppercase |
| block labels (`NIX · PATH`) | 19px, tracking 0.14em, uppercase |
| view indicator, keybind row | 20px, tracking 0.1–0.12em, uppercase |

### Spacing and shape

| token | value |
|---|---|
| transcript column max-width | 1560px, centred in its region |
| gap between blocks | 30px |
| hairline | 1px |
| band padding | 14px 24px |
| band radius | 3px |
| code block indent | 28px, with a 1px accent-35% left rule |
| ledger glyph column | 24px wide |
| caret | 16×32px block |
| screen padding (transcript region) | 80px horizontal |

No rounded cards anywhere. The only radii left are the 3px on the turn band
and the inline code chip; everything else is rules and space.

---

## The face

### Placement

Three views. `placement()` in `src/lib/views.js` keeps its shape; the numbers
become:

| view | centre x | centre y | box passed to `faceBox` |
|---|---|---|---|
| `default` | `vw * 0.5` | `vh * 0.30` | `vw * 0.62`, `vh * 0.58` |
| `session` | `vw * 0.165` | `vh * 0.5` | `vw * 0.28`, `vh * 0.52` |
| `deep` | `vw * 0.081` | `vh * 0.5` | `vw * 0.109`, `vh * 0.169` |

These are box fractions, fitted the way `faceBox` already fits them: pitch is
`min(boxW / 46, boxH / 40)`, so whichever axis runs out first binds. On 16:9
`default` binds vertically and lands a face about 950px wide — do not treat
the width fraction as a face width, which is a mistake that clips the top of
the iris.

`default` is the merged `home` + `prompt`: parker over the top third, the
transcript's tail beneath him, the input line low. It does not lean back when
a conversation starts — the previous `conversing` special case is dropped, so
every view is now fixed and layout only changes when you ask.

`deep` keeps parker on the left but the transcript starts at `x = 560` and
runs to `vw - 112`, so the right third is no longer stranded.

### Expressions

Today's twelve stay exactly as they are. Eleven are added. All are built from
the existing primitives in `pose.rs` — `iris(w, ring)`, `eyes(w, h, pupil)`,
`line(w, amp, freq)`, `stroke(...)` — and no new pose machinery is needed:

| name | pose | motion |
|---|---|---|
| `listening` | `eyes(7.5, 10, 3.2)`, `gazeY 0.6`, `squint 0.92` | breathe |
| `skimming` | `iris(4, 12)`, `ringW 1.6` | blades |
| `digging` | `iris(3, 16)`, `ringW 2.8` | blades |
| `smug` | `eyes(7.5, 6, 2.8)`, `squint 0.42`, `tilt −2`, `y −1`, `gazeX 1.6` | breathe |
| `sheepish` | `eyes(7, 7, 3)`, `squint 0.78`, `gazeX −2.6`, `gazeY 2.4`, `tilt 2` | none |
| `deadpan` | `eyes(9, 1.6, 0)`, `round 0` | none |
| `fond` | `eyes(7, 4.2, 0)`, `y −1`, ink `accent` | breathe |
| `melancholy` | `eyes(7.5, 5, 3)`, `squint 0.86`, `gazeY 2`, `tilt 2` | breathe |
| `conspiratorial` | `eyes(7.5, 8.5, 3.2)`, `squint 0.34`, `gazeX 3`, `tilt −1` | drift |
| `delighted` | `eyes(8, 4, 0)`, `y −1`, `ring 17`, `ringW 2`, ink `accent` | pulse |
| `suspicious` | `eyes(8, 9, 3.4)`, `squint 0.44`, `gazeX −2.8`, `tilt −1.5` | none |

Watch the grid bound: the ring's outer edge is `ring + ringW/2` and the grid
is 40 rows, so anything past 20 clips. `digging` was 19/3.2 in an early draft
and clipped; 16/2.8 is the corrected value.

**All eleven are voluntary**, plus `thinking` and `speaking` become voluntary:
parker wears every expression, and the app infers none. Concretely, in
`state.svelte.js`:

```js
export function expression() {
  if (app.engineStatus === 'disconnected') return 'trouble';  // a machine fact, still wins
  return app.worn ?? 'idle';                                   // everything else is his
}
```

The `busy → thinking` and `lastEventAt → speaking` inferences are deleted.
Progress lives in the transcript (the ledger rows and the caret), never on the
face. The prompt should ask him for a face **per beat** rather than per reply:
wear something before you speak, and change it when the beat changes.

Order of work, per the repo's own rule: add the poses to
`parkerd/src/face/{mod,pose}.rs` first, regenerate
`parker face --catalog --json > visage/src/lib/catalog.json`, port the same
numbers into `visage/src/lib/face.js`, then run `node parity.mjs`.

### The pose transition

Unchanged. 550ms, `backOut` for voluntary, `inOut` otherwise. Three
alternatives were tried (a scatter puff, a blink-cut) and rejected.

---

## The travel

Two options, one config flag. `linear` is today's behaviour and stays
supported; `river` is the new default.

### linear

Exactly what `Stage.svelte` does now: the box eases toward the target
placement, a fraction of the remaining distance per second, integrated per
frame so it stays interruptible.

### river

A dot-level travel. The reference implementation is `travelSwarm` in
`parker-face.js` (search for `SWARMS`); this is what it does and why.

The move is **three phases with one shared middle**, so the crossing looks the
same regardless of which face is on:

1. **fall** (0 → 0.22 of a dot's own run) — the dot moves from where it sits to
   its slot on the path. Its offset from the face's centre is *squeezed*, not
   released: the along-path component keeps 35% of its magnitude, the
   perpendicular component keeps `comp` (0.12 for river). The face collapses
   into a narrow ribbon; it must never bloom outward, which is what the
   earlier scatter version got wrong.
2. **run** (0.22 → 0.8) — the ribbon runs a quadratic Bézier from its entry
   slot to its exit slot. One control point, shared by every dot, offset
   perpendicular to the chord by `arc × pathLength` (0.20) and always bowing
   downward on screen, so the stream sags like something poured. Each dot
   varies its own bow by ±18% for body. A slow perpendicular sine (`wob`, 1.5
   dot widths) rides on top and tapers to zero at both ends. Per-dot speed
   variance is ±6%.
3. **reform** (0.8 → 1) — the dot lerps from its exit slot to its exact
   destination, so the face is whole the frame the travel ends. There is no
   separate reassembly animation to get wrong.

Launch order is the whole trick: a dot's departure time is
`(s * 0.86 + hash * 0.14) * stag`, where `s` is its position **measured along
the direction of travel**, normalised over the dots that are actually lit
(not over the 96-column grid — an iris only occupies the middle ~40 columns,
and normalising over the grid compresses the pour into a third of the travel).
The edge facing the destination leaves first, so heading left means the left
edge dissolves and runs left, and the trailing edge is still a whole face
while the front is already crossing.

A dot that has not launched yet **stays exactly where it is**. Dropping
unlaunched dots blanks the face for the first frames — the bug worth not
reproducing.

River constants:

```
duration   780ms          (linear stays at ~1.1s of easing)
stag       0.50           fraction of the travel spent launching
comp       0.12           perpendicular squeeze onto the path
arc        0.20           control-point offset, as a fraction of path length
wob        1.5            braid amplitude, in dot widths
trails     2              past samples per dot, drawn only while running
shrink     0.18           dot radius reduction while in the stream
speed      0.12           per-dot speed variance across the run
```

Trails are the same dot resampled 26ms earlier, at alpha `0.18 × (1 − i/(n+1))`,
drawn only during the run phase.

`grain` (a tighter, drier variant: `comp 0.05`, `arc 0.10`, `wob 0.5`, four
trails, 1200ms) also exists in the reference and can be dropped unless a
summon animation wants it later.

### Where it belongs

All of it is **placement**, not pose: it is `(dot position, time) → screen
position`, which is precisely the test `views.js` sets. parkerd is never told
the face came apart. Implementation goes in `lib/lens.js` (the per-dot
function) and `Stage.svelte` / `Face.svelte`'s draw loop. Nothing in
`parkerd/` changes for the travel.

One consequence for `Face.svelte`: during a travel the canvas must cover the
whole view, not a box that is being moved. Today the stage is a sized,
translated box; the travel needs the face's screen position computed per dot
inside a full-viewport canvas. That is the one structural change.

---

## The views

Chrome that is identical in all three — same position, same size, same style.
Values for 2560×1440:

| element | position |
|---|---|
| corner | top 52px, right 68–80px, right-aligned |
| WEARS | left 120px, top 1080px, width 380px |
| view indicator | left 44px, bottom 24px |
| input line | directly under a full-width hairline, 32px below it |
| keybind row | 32px under the input line |

### `default`

- Face: centred, top third.
- Beneath him, bottom-anchored to the input: the tail of the current exchange,
  in a 1400px column, masked with `linear-gradient(to bottom, transparent, #000 80px)`
  so what runs off the top dissolves.
- Hairline, then the input line: `›` in accent-55%, then the caret, then
  `talk to parker` in fg-22%.
- Typing anywhere opens the transcript with that character already in the box
  (existing behaviour, keep it).

### `session`

- Face on the left rail, WEARS beneath him.
- Transcript region starts at `x = 870`, 80px horizontal padding, column
  max-width 1560px, bottom-anchored.
- Same hairline + input line + keybind row.

### `deep`

- Face a marble on the left.
- Transcript from `x = 560` to `vw − 112`.
- Same chrome, same positions.

---

## Blocks

The transcript is a sequence of blocks with a 30px gap. Each fades in once
(`opacity 0 → 1`, `translateY(7px) → 0`, 340ms `ease-out`) and never
re-renders.

### Your turn

A band: `background: accent 9%`, `border-left: 2px solid accent 55%`, padding
14px 24px, radius 3px, full column width, text at `--fg`.

### Prose

Plain text at 28px/1.6. **Types in**, roughly 36 characters/second, with the
caret at the head. When the line finishes the caret dims to accent-35% and
parks at its end; it does not blink once parked. His expression changes on the
sentence boundary, not on the token.

### Work — the ledger

One line per act. No output, ever. A row is: glyph column (24px), the verb and
its target, then magnitude right-aligned.

| act | glyph | line | magnitude |
|---|---|---|---|
| bash | `$` (accent 60%) | `ran nixos-rebuild dry-activate` | `4.2s`, and `ok` / `exit 1` |
| read | `▤` (fg 30%) | `read system/theming.nix` | `58 lines` |
| edit / write | `±` (good 55%) | `edited hyprland.nix` | `+3` (good), `−1` (bad) |
| search | `⌕` (fg 30%) | `searched the web for stylix border_size` | `8 hits` |
| fetch | `↗` (fg 30%) | `fetched wiki.hypr.land` | domain |
| sense / look / media | `⊙` (fg 30%) | `sense`, `look (window)`, `media: pause` | — |

A failed act turns the whole row `--bad` at 65% and appends `— failed`.
While an act is in flight, a 1px indeterminate bar sits under the row: a 36%-wide
accent-70% segment sweeping `translateX(−100%) → translateX(280%)` over 1.4s.

Two behaviours already agreed but **not yet designed** — leave hooks:
old ledger rows fold, as the conversation moves past them, into a single
summary row (`▤ $ ± · four acts · 2 files read, 1 edited, 1 rebuild · 48s ⌄`)
that reopens on a key.

### Code

No card. A 1px accent-35% left rule, 28px indent, a small uppercase label above
(`NIX · HOME/DESKTOP/HYPRLAND.NIX`), then the code at 24px/1.8. Tokens: keys
`--accent`, values `--good`, comments fg-30%.

### Gesture

Italic, fg-42%, one line, no parentheses convention beyond what he writes. It
is now expressive rather than mechanical — `(already had the diff written,
which is either efficiency or presumption)` rather than `(wears curious)`.
The face he is wearing is reported by the WEARS block, so the gesture line
should never restate it.

### Sources

Pill row: 1px fg-14% border, radius 20px, padding 4px 18px, the index in
accent, the domain in fg-55%.

---

## The corner

Right-aligned, four lines, all of it about his body and his room. Never a
count of threads, never disk usage or fan speed.

```
19:31 · thursday
warm, fans idling, mildly annoyed at stylix        ← italic, fg 42%
cpu ▇▅▃▂▁▁▁▁ 22% · gpu ▇▇▇▅▃▂▁▁ 38%                ← filled cells accent 75%, empty accent 20%
51°C in · 4°C out                                  ← in --good 75%, out --accent 75%
```

The sentence is his, not the app's: rewritten whenever `sense` moves enough to
be worth a sentence. Everything else comes straight from `sense`.

---

## WEARS

```
WEARS                     ← 19px, tracking 0.18em, fg 25%
digging                   ← 52px, accent 90%
────────                  ← 240px × 1px, accent 35%
```

Same size and position in every view. It exists because the face is now
entirely his choice: it names what he picked, and it is the only place the
expression is named.

---

## The view indicator and keys

```
DEFAULT ──── SESSION ──── DEEP        current stop in --accent, others fg 16%
```

Horizontal, hairline delimiters 40px long, bottom-left corner of the screen,
identical in all three views.

```
ESC STOP   ^K COMMANDS   ^Y EARLIER   ^SPACE DANCE   ^± DEPTH
```

`ESC STOP` in `--bad` 45%, the rest fg-20%, 32px gaps, sitting under the input
line. Identical in all three views.

---

## Keys

Unchanged from today except:

- `^+` / `^-` now traverse three stops instead of four.
- `^K` opens a command palette: threads, message search across threads, and
  commands (`/theme set forge`, `/dance auto`, `/new`). Navigation is
  **`^N` / `^P`**, not `j`/`k` — the box has focus and `j` must type a `j`.
  `Enter` opens, `Esc` closes. Designed in `Visage.dc.html` (option `1e`);
  the older `^Y` list stays for now.
- Mouse: keyboard-first stays, but what already exists on screen should be
  clickable (a thread row, a folded ledger row, a palette result). The
  `cursor: none` rule in `app.css` needs revisiting for that.

---

## State

Additions to the store in `state.svelte.js`:

| state | meaning |
|---|---|
| `app.view` | `'default' | 'session' | 'deep'` — `home`/`prompt` merge |
| `app.worn` | already exists; becomes the *only* source of the expression |
| `blocks[].kind` | `text` \| `act` \| `code` \| `gesture` — the transcript's block types |
| `blocks[].streaming` | whether this text block owns the caret |
| `act.magnitude` | lines / duration / `+n −m` / hit count, per the table above |
| `act.status` | `running` \| `ok` \| `failed` |
| `travel` | config: `'linear'` \| `'river'` |

Streaming is per block: a text block appends characters; an act or code block
appears complete. The existing refetch-on-engine-chatter path can drive this
as-is, but the intended version is real deltas — the `Message.svelte` header
comment already describes the commit-per-block plan, and this design is what
it should commit *into*.

---

## Files in this bundle

| file | what it is |
|---|---|
| `Visage Bed.dc.html` | the three views, locked. One conversation, one clock, side by side. The reference for layout, chrome and blocks. |
| `Visage Motion.dc.html` | the travel (linear / river / grain) and the hybrid streaming model, live. |
| `Visage.dc.html` | the earlier exploration board: today's UI recreated, the 23-expression sheet, the `^K` palette, tool-card and ambient studies. Reference material, superseded where it disagrees with the other two. |
| `parker-face.js` | working port of `face.js` + the travel algorithm. Read `SWARMS`, `geomFor`, and the swarm branch of `loop()`. |

Open each in a browser; they need no build.

## Still undesigned

- Interrupt mid-beat: what `Esc` looks like three acts in.
- Engine gone: `trouble` ink, and whether the transcript greys.
- Work folding, live.
- The summon: `SUPER+P`, from nothing to fullscreen (`grain` is a candidate).
- The ambient parker — a puck, an edge strip, or a bubble anchored to what he
  is pointing at. Three sketches are in `Visage.dc.html` (`1h`–`1j`).
- Dance: built, never designed for.
