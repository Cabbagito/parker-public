# MOTION — idle flair and dance mode

**Status: built, July 2026.** This is the design record — it says *why*, the
code says *what*. The plan below is kept in its original shape rather than
rewritten into a description, because the arguments are the point; where the
build diverged from the plan, the divergence is marked inline and says why.

Two features that look unrelated and are the same feature: parker's face
does something on its own, on a clock that is not the user's. One is quiet
and ambient, one is not.

## What shipped

| where | what |
|---|---|
| `parkerd/src/face/beat.rs` | the third layer: keyframes, the player |
| `parkerd/src/face/idle.rs` | the clock: 3 tiers, 5 short beats, the pool |
| `parkerd/src/face/vignette/` | 9 vignettes, built: Part Three |
| `parkerd/src/face/entrance.rs` | how 7 reactions land: Part Four |
| `parkerd/src/face/dance.rs` | `BeatClock`, drives, 2 routines, 4 surprises |
| `parkerd/src/face/catalog.rs` | the JSON both surfaces read |
| `visage/src/lib/{beat,idle,dance,lens}.js` | the port, and the camera |
| `visage/parity.mjs` | 12000+ checks that the two agree |

*(The catalog this table first described — fourteen beats, four tiers,
sleep and waking — was replaced after it was watched on a real screen; see
the batch notes in `face/idle.rs` and the August restructure under §1.1.)*

Reach it with `parker dance` (a plain toggle), `Ctrl+Space` inside visage, or
`SUPER+ALT+P`. `parker reach dance [routine] [--for SECS] [--show]` has the
knobs and is the MCP tool, so parker can start one himself.

**Dance is a mode, not a window.** One line in `$XDG_RUNTIME_DIR/parker/dance`
that every face reads. It is orthogonal to `parker visage`, which only says
whether the app is on screen — and Hyprland shows one special workspace per
monitor, so `special:parker` (the terminal face) and `special:visage` swap
rather than stack. Both windows stay alive either way; you just see one.

The toggle was added the day after shipping, for a reason worth recording:
`Ctrl+Space` starts a dance with no deadline, so one toggled on and forgotten
ran indefinitely, and stopping it meant remembering `parker reach dance stop`.
A gesture you can enter but not obviously leave is a bad gesture.

### Decisions taken during the build

- **`mark` went to six strokes, not four** (§1.3 asked). Two Zs at once is
  six exactly, and `zZZ` wants two on screen.
- **`saccade-sweep` was cut** from the catalog. Once `Drift` itself became
  saccadic (§1.4) the beat was a louder copy of the resting face. `track`
  and `unfold` cover the same ground with something to look at.
- **`tilt-and-wait` became `lean-in`.** `tilt` and `squint` only apply to
  the *right cell*, and the resting face is a single merged aperture — so
  the beat did nothing at all until it opened the gap first, at which point
  it was `unfold` with worse timing.
- **`droop` is weighted, not uniform.** Drawn evenly from fourteen it came
  up about once an hour, so the two nods sleep is gated behind were
  unreachable and parker never slept. Caught by the test, not by reading.
- **The Zs were redrawn twice.** First pass: a letter five dots across
  drawn with a three-dot brush, which rasterizes to a filled square.
- **`!` moved off the face.** Marks draw last and on top, so a centred bang
  landed inside the aperture and read as a smudge in parker's eye.
- **`SUPER+SHIFT+P` was already the terminal face**, so the dance gesture is
  `SUPER+ALT+P`.
- **`reach dance --show` exists; `parker visage --show` does not.** The
  summon verb stays one word with no flags. `--show` lives on `dance`
  because a gesture that starts a performance has to be idempotent, and
  `parker visage` is a toggle.
- **Sleep does not auto-summon, and neither does an MCP dance.** Gating is
  about initiative, not obedience: a dance parker decided on must not throw
  a fullscreen window over what you are doing.

---

## 0. The one idea

The animation system today has exactly two layers:

| layer | what it is | who owns it |
|---|---|---|
| **pose** | 16 numbers + 2 strokes + 2 inks. A target. | `Expression::pose()` |
| **motion** | a periodic function of `phase` applied on top | `Motion::apply()` |

A transition is a lerp between poses. That is the whole system, and it is
good — an eye genuinely squashes, an iris genuinely slides.

What neither layer can express is **a thing that happens once and ends**.
Motion is periodic and eternal; a pose is a place you go and stay. Idle
flair and dance are both sequences with a beginning, a middle and an end.

So: a third layer.

| layer | what it is | who owns it |
|---|---|---|
| **beat** | a short scripted deviation with its own timeline, played over the current pose and dying back into it | new |

A beat is a list of keyframes. A keyframe is a partial pose (a delta), a
duration, and an easing. The player lerps between them exactly the way
`animate_face()` already lerps between expressions, and the last keyframe
is always "back to where we came from". That is the entire mechanism.

**Why this and not more expressions.** `face/mod.rs` says twelve
deliberately, and "the set grows back from here, one at a time, when
something is actually missing". Twenty flair moves as twenty `Expression`
variants would blow that up, would each need a name, a `default_hold`, a
`motion`, a row in `VOLUNTARY`, and a duplicate in `face.js`. A beat needs
none of those. It is data, it has no name anyone types, and it cannot be
worn.

**Why beats are not expressions is also load-bearing.** An expression is
worn — chosen, named, held. A beat is neither chosen nor held — it is
*idling visibly*, a third thing, and it must never be mistaken for a face
parker picked. A glance does not mean parker is Curious. Keeping beats off
the expression enum is what keeps WEARS honest, now that WEARS is the one
place the expression is named.

---

# PART ONE — IDLE FLAIR

## 1.1 The clock

Videogame idle flair is a solved problem and the solutions are all pacing,
not animation. The rules that make it read as alive rather than as a bug:

1. **A settling period.** Nothing fires for the first ~45s. Flair
   immediately after you stop typing reads as a reaction to you, which is
   worse than nothing.
2. **Never metronomic.** Interval is a range, jittered per fire.
3. **Never twice in a row**, and a per-beat cooldown so the pool rotates.
4. **Escalation.** The longer the idle, the deeper the beat. This is the
   thing that turns "random animations" into "time passing".
5. **Instantly interruptible.** Any state change kills the beat mid-flight
   and departs from wherever the face actually is. Not for free, as this
   once said: `pose_now` is only the transition, so until October 2026
   both surfaces dropped the beat a frame before departing and a pyramid
   vanished rather than morphed. The departure reads the beat first —
   `animate_face()` in main.rs, `depart()` in visage's motion.svelte.js.

### What "idle for N minutes" is measured from

`sense` has no seat-idle signal. `focus.held_secs` exists but is
watch-loop-only and needs the daemon. **Do not invent a sense input for
this.** The correct clock is local and already available on both surfaces:

- **visage** — time since the last keystroke in the window, the last
  engine event *and* the last change of face. The third is not redundant:
  a face worn from Claude Code, `parker code` or another thread reaches
  visage with no keystroke and no event for its thread. Leaving it out
  (until October 2026) fired a beat the moment such a face let go. It
  counts by name, whether or not this build can draw the face: a visage
  older than parkerd draws an unknown face as a still `idle`. Counting
  only the faces it knew let a vignette play on under the faces round's
  new names for six days.
- **terminal face** — time since `current_expression()` last changed.

Both mean the same thing: *nothing has happened to parker*. That is the
right trigger and it costs nothing.

### Tiers

| tier | idle for | interval | pool |
|---|---|---|---|
| `settled` | 0–45s | — | nothing at all |
| `alert` | 45s–5m | 20–45s | glances, blinks, small checks |
| `bored` | 5–20m | 40–90s | character beats — the ones with opinions |
| `drowsy` | 20m+ | 90–180s | droop, nod-off, and eventually sleep |

*(Restructured, August 2026: the four tiers collapsed to three —
`settled` / `alert` (45s–30m, the short beats) / `vignette` (30m+, the
long deliberate pieces, with the short pool still eligible). Sleep, droop
and wake are gone entirely, and with them the wake note: parker is an AI
that is literally on, and a face that pretends otherwise is a worse
wallpaper. Deep idle now reads as parker doing his own thing. The original
tables below stay as the design record; the catalog itself was also
replaced wholesale after being watched on a real screen — see the batch
notes at the top of `face/idle.rs`.)*

Two refinements worth having:

- **The resting pose drifts with the tier.** At `drowsy` the lids sit
  lower (`h` scaled ~0.85) and the drift slows, *before* any beat fires.
  So the escalation reads even between beats. This is a lerp on the base
  pose, not a beat.
- **Sleep is terminal, not a beat.** Once parker is asleep he stays
  asleep, and the exit is `wake-with-a-start`, fired by whatever woke him.
  Every other beat returns to rest; this one does not. It is the only
  exception and it is the one that makes the screensaver a screensaver.

## 1.2 The catalog

Grouped by tier. `→` is a keyframe. Numbers are dot units on the 96×40
grid, relative to the idle pose (`iris(9,17)`, `pupil 3`, drift).

### `alert` — small, cheap, high frequency

**glance** · 800ms
: `gaze_x → ±4.5` over 90ms (fast out), hold 500ms, return over 220ms.
The single highest value-per-line beat in the catalog. Eyes that look at
something are alive; eyes that sine-wave are a screensaver.

**double-take** · 1.1s
: glance to `+4.5`, hold 200ms, then snap to `-1` with `back_out` in
120ms while `pupil 3→2.2` and `w 9→10.5`. Overshoot is the joke.

**blink-stutter** · 400ms
: three fast blinks instead of one, the middle one incomplete. Reads as
a hitch. Optional: `ink → accent` for a single frame on the second blink,
which reads as a machine having a moment rather than a creature.

**self-check** · 1.6s
: `sweep 1 → 0` over 300ms, `→ 1` over 900ms with the ring in `accent`,
`pupil 3 → 1.2` and back. parker running its own diagnostic. This is the
one that most says *this is an AI and not a cartoon animal*, and it uses
only parameters that already exist.

**stretch** · 900ms
: `w,h ×1.14`, `ring ×1.08` out with `back_out`, settle. A yawn without a
mouth.

### `bored` — character beats

**unfold** · 2.2s
: `gap 0 → 18`, `round 1 → 0.85`, `h 9 → 11`, `ring → 0` — parker opens
into two eyes, does a **glance**, then folds back. This is the prettiest
transition the system owns (iris↔eyes is the whole reason `gap` exists)
and it is currently only ever seen in passing. Making it a deliberate beat
is free showmanship.

**saccade sweep** · 2.6s
: four discrete fixations — `gaze_x` steps to `-4, +3, -1, 0` with 40ms
flicks and 500ms holds. Real eyes flick and hold; they do not glide. See
§1.4 — this deserves to become the *base* idle motion, not just a beat.

**track** · 3s
: `gaze_x` traverses `-5 → +5` slowly and `x` (the whole face) drags
~1.2 dots behind, then catches up. Follow-through. Something crossed the
room.

**thought** · 2.4s
: `ring 17 → 22` with `sweep → 0.6` and `y → -1.5` — the ring detaches
upward like a bubble leaving, then snaps back. Read: an idea floated off.

**hum** · 3s
: lerp most of the way to the `speaking` pose (`line(46,10,3)` + wave) and
back, at half amplitude. parker humming to itself with nothing to say.
Costs nothing — it is an existing expression used as a beat — and it is
funny precisely because `speaking` normally means words.

**tilt-and-wait** · 1.8s
: `tilt → -3`, `squint → 0.7`, hold, return. A one-second Curious that
never becomes Curious.

**the shrug** · 1.4s
: both marks appear as short angled brows (`stroke(-9,-6,-4,-8)` and
mirrored), pop with `back_out`, hold 400ms, `w → 0`. Marks as expression.
The mark channel is currently used by exactly two glyphs; it is the most
underused thing in the pose model.

### `drowsy` — the long tail

**droop** · 2s
: `h 9 → 5.5`, `gaze_y → +2`, hold, and *snap back* over 120ms with a
tiny `y` kick. Catching yourself nodding off. Fires two or three times,
lower each time, before sleep is allowed.

**sleep** · terminal, loops
: pose goes to `w 11, h 1.6, round 0.4, ring 0, pupil 0` — a closed,
slightly curved lid, which is `away`'s flat line with a smile in it. Then
the Zs: three of them, each rising from `(+7,-3)` to `(+15,-12)` while
`w: 3 → 0`, on a 1.2s stagger, looping forever. Slow `breathe` underneath.

**wake-with-a-start** · 700ms
: the exit. `w,h` overshoot to `×1.5` with `back_out`, `pupil → 1.5`,
`x` shakes ±2 for 200ms, settle into whatever the new state is. Fires
*into* the transition rather than before it, so waking up and starting to
think are one motion.

**tick** · ambient, not a beat — *shipped, then removed (July 2026)*
: at `drowsy`, the ring advances one notch per second — `sweep` quantized
to 1/60ths. A second hand. This is the actual screensaver: something that
is quietly, visibly counting while you are gone.
: *Removal, after living with it: on a resting face the stepping gap read
as a defect — a metronomic hitch — not as counting. If parker ever shows
the time it will be a deliberate beat (become a clock, then stop), not a
permanent tremor on the idle face.*

### `!` — the comic-book channel

Not a tier. A **pop** overlay usable by any beat: two marks form an
exclamation (a thick vertical `stroke(0,-9,0,-1)` plus a short dot-stroke
at `(0,3)`), spring in with `back_out` over 140ms, hold 400ms, `w → 0`.
Speed lines are the same trick — two short diagonals flanking the face,
popping outward on a hit.

This is where "comic book vibes" actually lives, and it needs one change
to the pose model to be worth anything (§1.3).

## 1.3 The two changes to the pose model

The entire plan — both features — needs exactly two new numbers. That is
the strongest argument that the pose model was designed right.

**1. `mark: [Stroke; 2]` → `[Stroke; 4]`.**
The comment says "two segments cover every glyph worth having at this
size", which was true when the glyphs were a check and a cross. A `Z` is
three. `!` is two but wants a third for a companion. Speed lines want
four. Widening is mechanical: `lerp` already maps over the array, the
tests already assert marks interpolate, and `ZERO` grows two entries.
Cost is real but one-time and shallow. Payoff is a glyph alphabet:
`Z ! ? ~ ↑ ⚡ ×` and speed lines.

**2. `spin: f64` — the ring's start angle.**
Today a partial ring always starts at `-TAU/4`. One angle offset makes the
sweep *rotate*, which unlocks the radar scan, the turntable, and the
scratch. It lerps like everything else, it defaults to 0, and it is
already half-implemented — `Motion::Blades` rotates arcs from `phase`, it
just cannot be beat-locked or interpolated.

Everything else in this document is composition of numbers that already
exist.

*(A third change arrived with the flair redesign, July 2026: `mark_ink`
widened from one ink to one per stroke. The first multi-subject beats hit
the single ink immediately — three quavers in the house colours, a white
scan line beside a red contact. Same shape as the mark widening: `Ink::pad`
mirrors `Stroke::pad`, colour still never interpolates.)*

## 1.4 The change I would make first, before any of this

**Replace `Motion::Drift` with saccades.**

The resting face currently drifts its gaze on a sine. Real eyes do not do
that — they flick (30–50ms) and fixate (200–600ms), and that specific
rhythm is most of what separates "a thing that is looking" from "a thing
that is animated". It is one function, it changes no interface, it applies
to `Idle` and `Curious` on every surface at once, and I think it will do
more for aliveness than the entire flair catalog.

Do it first, look at it for a day, then decide how much flair is actually
needed on top. It may be less than you think.

---

# PART TWO — DANCE

## 2.1 What makes it different from flair

Flair is beats on a lazy clock. Dance is beats on **someone else's clock**,
and it is the first thing that genuinely needs both halves of the
pose/placement split at once:

- **choreography of poses** — parkerd. Same beat player, different data.
- **choreography of the camera** — visage. And this is where it gets to be
  spectacular, because `views.js` already licenses it: *"anything
  expressible as (dot position, time) → screen position is placement"*.

The terminal face gets the pose half and nothing else, which is correct —
it is the over-SSH face, not the showcase.

## 2.2 The beat clock — a ladder, not a choice

There is no BPM anywhere in `sense`, and there is not going to be. So:

| rung | source | cost | what you get |
|---|---|---|---|
| **v0** | internal metronome, fixed 120 BPM, tap-to-set with a key | ~0 | dancing. It always works, it works with no music, and it is genuinely most of the effect. |
| **v1** | `sense.media` gates it | ~20 lines | starts on Play, freezes on Pause, changes routine on track change, and can pick a routine seeded by the track title so the same song dances the same way twice. |
| **v2** | real audio — PipeWire monitor capture → FFT → onset detection | a weekend | it is actually dancing to the music. Bass envelope → scale, onset → pose hit, treble → gaze and `freq`. |

**The load-bearing decision is that the choreographer takes a `BeatClock`
trait and nothing else**: `bar()`, `beat()`, `phase_in_beat()`,
`energy(band)`. A metronome and an onset detector implement the same four
methods. Build v0 against that interface and v2 is a new impl, not a
rewrite — the same move the engine seam makes in PARKER.md.

Do not skip to v2. v0 is testable, deterministic, and will expose every
choreography problem there is.

## 2.3 Pose moves (parkerd)

Quantization is the whole trick. Sine motion reads as drift; **stepped**
motion reads as dance. Same parameters, different scheduler.

**the split** — `gap 0 ↔ 18` on eighths. The signature move, because
iris-into-eyes is the best transition the system has and it snaps
beautifully at tempo.

**the hit** — on every kick: `w ×1.35, h ×0.6` for 60ms, then `back_out`
home. Squash and stretch. This one is not optional; it is what makes
everything else land.

**headbang** — `y` on the beat, `back_out` on the down so it *arrives*
rather than oscillates.

**sidestep** — `x` steps in quantized jumps of ±6, holding between. Never
interpolate this one smoothly or it becomes drift.

**lean** — `tilt` rocks ±4 on the bar. Cheap, reads as body language.

**visualizer** — become `speaking`: `amp` rides the bass envelope, `freq`
rides the treble. The face *is* the waveform. On v0 this is a fake
envelope and still looks great.

**turntable** — `spin` advances one revolution per bar with `sweep 0.35`.
On a drop, `spin` runs backwards fast: a scratch.

**strobe** — `ink` cycles `plain → accent → good → bad` on bar
boundaries. Note that `Ink` is deliberately *not* interpolated ("a colour
crossfade at 1-bit is mud") — which means it snaps, which is exactly what
a strobe is. An existing constraint becomes a feature for free.

**blades at tempo** — the `thinking` arcs at 4× speed, strobing.

**wink** — `squint → 0` for one beat.

**speed lines** — the mark channel, popping outward on hits. Comic book.

## 2.4 Camera moves (visage) — the actual unlock

`Face.svelte` currently draws each lit dot at `(ox + x·pitch, oy +
y·pitch)`. Insert one function there:

```js
lens(x, y, t) -> { dx, dy, scale, ink }
```

That single hook is a lens between the grid and the screen, it is a pure
function of dot position and time, and it is therefore **placement by the
existing definition** — no pose is invented, parkerd learns nothing. It
costs about fifteen lines and it is the difference between "the face
moves" and "the face is a music video".

What it buys:

**scatter / reform** — dots fly outward on a per-dot random vector and
reform on the next bar. The `Stage.svelte` comment already names this as
an anticipated camera ("a dissolve-and-reform"). Highest impact single
effect available.

**vortex** — angular displacement proportional to radius. The face twists
into a whirlpool and untwists on the bar.

**ripple** — `dy += A·sin(x·k − t)`. A wave crossing the face.

**shear** — alternating rows slide opposite directions on the beat.
Split-flap.

**echo** — keep the last N grids, draw them at falling alpha and slight
offset. Trails. Cheap, enormous payoff, and it makes every other move look
more expensive than it is.

**chromatic split** — draw the grid three times in accent/good/bad at
±1px offsets. RGB glitch. Very comic book, and it is theme-derived colour
so it stays coherent under `theme set`.

**the dive** — on a drop, the placement box scales past the screen: dots
become enormous discs and the face is abstract for two bars.

**orbit / whip-pan** — the box traverses the screen. `Stage.svelte`'s
integrator already makes this interruptible for free.

**confetti** — dots that detach on a drop and fall out of frame, and the
grid reforms without them for a bar.

## 2.5 Routine structure

Not one hardcoded timeline. A routine is a **stack of moves with bar
ranges**, so routines are data and adding one is a table entry:

```
intro   bars 1–2   headbang + lean
build   bars 3–4   + split on eighths, echo camera
drop    bars 5–6   hit + strobe + scatter/reform, dive on bar 6
outro   bars 7–8   decay: moves drop out one per bar, settle to idle
```

Two things worth building in from the start:

- **A surprise slot.** One bar in ~16, ignore the routine and do something
  stupid — a wink, a single `!`, the face walking off-frame and back. The
  thing that makes it feel authored rather than generated is that it is
  occasionally *wrong*.
- **Phrase awareness.** Moves change on bar boundaries, never mid-bar.
  Everything looks choreographed if it changes on the 1.

## 2.6 How you get into it

### The verb

You suggested `parker visage --dance`. I would not do that one, and the
reason is already written down: `reach/visage.rs` says *"one word, no
flags… a `--hide` would be a second way to say a thing the first way
already says"*. A `--dance` flag reopens exactly that.

**Recommendation: `parker reach dance [--for SECS]`**, mirroring `reach
face` exactly. It is the same kind of thing — wearing a mode — so it gets
the same mechanism: one line in `face/state.rs`'s runtime file, expiring
on read, no daemon needed. Both surfaces pick it up on their next frame,
so it works over SSH and it works on the terminal face.

And because every `reach` verb is an MCP tool, this means **parker can
decide to dance**. That is free, and PARKER.md explicitly wants it —
*"capable of opening a YouTube window unprompted because it was funny"*.
A song comes on, parker likes it, parker dances. Nobody asked. That is the
feature.

`parker dance` as a bare top-level alias is tempting for typing comfort,
and it is exactly the second-way-to-say-it the visage comment warns
against. I would skip it. (Weak opinion — if the couch ergonomics matter
more than the principle, take the alias.)

### The keybind, inside visage

The existing map is Ctrl-based, and every plain letter at `home` is taken
(typing opens `prompt`). So it must be a chord, and it must not fight the
depth axis, because dance is a *mode*, not a view.

**Recommendation: `Ctrl+Space`.** Nothing uses it, space↔playback is the
universal music idiom, and it is a big key for a big mode. Exit on `Esc`,
which needs no new rule at all: Esc already means "cancel whatever is in
front of you, in whatever order things are cancellable", and dance is
simply the frontmost cancellable thing. It tiers in above interrupt.

`Ctrl+K` and `Ctrl+.` are the alternatives and both are arbitrary.

### Optional: `SUPER+SHIFT+P`

Summon visage straight into dance. One line in `hyprland.nix`, and it is
the "put the thing on" gesture — you press it when friends are over,
not when you want to talk to parker.

### Optional: the easter egg

A hidden trigger — a typed word at `default` that never reaches the input,
or a key sequence. Fun, zero architectural weight, and easy to add later.
Not worth designing now.

---

# PART THREE — VIGNETTES (Oct 2026)

The vignette phase had a slot and nothing in it: every shipped beat was a
short `alert` one, so thirty minutes of quiet only slowed the clock down.
Vignettes fill it — pieces that run for minutes and are parker getting on
with something, not fidgeting. They were designed on a canvas ("visage —
vignettes"), each one running live on visage's own renderer, and only the
ones Peter picked from it were built.

| vignette | pool | ≈ length | what |
|---|---|---|---|
| `nap` | vignette | 3 min | yawns, nods off, sleeps with a Z per breath, dreams, wakes on his own |
| `chores` | vignette | 2 min | dusts the inside of his porthole; the dust makes him sneeze |
| `bonsai` | vignette | 4½ min | becomes the pot and grows a tree, then prunes it |
| `reading` | vignette | 4 min | reads in saccades, re-reads, highlights the line that lands |
| `pong` | vignette | 1 min | five points against someone off-screen; loses on a double-take |
| `sort` | alert | 30 s | bubble-sorts six bars, then again, faster |
| `shine` | alert | 12 s | Fox's shine (a hexagon is six strokes), then a Falcon Punch |
| `power-cycle` | alert | 11 s | an old CRT switching off and on again |
| `warp` | alert | 17 s | lightspeed streaks, a gear change, drops out somewhere new |

How they are made, and why:

- **Nothing new in the pose.** Every one is drawn from the parts the idle
  face already has — two cells, the ring, the pupil, the waveform, six
  strokes, four inks. The six-stroke budget kept turning out to be the
  design: a tetrahedron, a hexagon and a duster are all exactly six.
- **Built, not typed.** A vignette is code that writes the keyframe table
  (`face/vignette/`): sections — yawn, thirty breaths, waking — flattened
  once, on first use, into an ordinary `Beat`. The player learned two
  things per keyframe, set per section: the motion under it (a nap
  breathes, a sweep holds still) and whether the ordinary blink comes
  through. Minutes of an open eye that never blinks read as a frozen screen.
- **Delivered on their own.** Flattened, they are about a megabyte, past
  what belongs in visage's bundle (`catalog.rs` keeps the catalog under
  120 kB on purpose). `parker face --vignettes --json` writes
  `vignettes.json`, which visage loads after first paint. Still data, still
  with its own parity answers — §3's answer holds, by a second file.
- **Deep idle leads with them.** In the vignette phase one roll picks the
  pool first: the long pieces 70% of the time (`DEEP_SHARE`), the short
  beats otherwise, so they are sprinkled between rather than outnumbering.
- **`nap` is not the sleep state back.** Sleep was a place parker went and
  stayed until something woke him; it went in August for pretending he was
  switched off. A nap is a bit he does and finishes, and any key ends it
  like any beat. If the old wake note ("you caught me napping") is ever
  wanted, `motion.svelte.js::stir` is where it goes.

Cut on the canvas: a twelve-bar blues with six strokes as six strings. The
strings flashed and shivered on the beat, and it still did not read as
music. Music on this face probably needs the waveform to carry it.

---

# PART FOUR — ENTRANCES (Oct 2026)

The faces round (DECISIONS.md) added gestures to the vocabulary: faces whose
meaning is in how they land. An eye-roll held is a look at the ceiling; the
roll is the point. A spring from the last face into the pose cannot say that.

An entrance is a beat. `face/entrance.rs` writes each one as a few whole
poses, turned into deltas against the face it lands in, and both faces play
it with `Beat::departing(origin, base, t)`: the first keyframe leaves from
wherever the face was (`origin`), the last ends exactly on the pose
(`base`), and nothing returns home because home is where it ends. While it
plays it replaces the spring, holds the face's own motion off and keeps the
blink out; a change of face mid-entrance departs from mid-entrance, the same
as any transition.

| face | ≈ length | what |
|---|---|---|
| `smug` | 1.8 s | the eyes look up, the shades come down, bump, settle |
| `eye-roll` | 1.4 s | left, up and over, the lids drop with a sigh |
| `table-flip` | 1.5 s | grab, throw, the table turns over in the air, legs up |
| `double-take` | 1.1 s | a glance, a look away, a beat, the snap back |
| `shrug` | 0.9 s | the arms spring up from the sides, dip, settle |
| `conspiratorial` | 0.9 s | both eyes on you, then one snaps shut into a ^ |
| `dead` | 0.7 s | eyes pop wide, knocked back, then crossed out |

They cross to visage as data (`catalog.json` → `entrances`), with parity
answers for each, so no keyframe is transcribed. The new motions of the same
round — twelve, most written for one face (`Tip` rocks a balance, `Sand`
runs an hourglass) — are hand-ported like every motion, and the catalog
carries samples of each for `parity.mjs`.

---

## 3. The duplication problem, honestly

`visage/AGENTS.md`: *"`lib/face.js` is a line-for-line port of
`pose,mod.rs` — an expression changed there is changed here, or it is a
bug."*

Twenty flair beats plus a dance catalog written twice is where this plan
dies if nothing is done about it. Three ways out, in increasing order of
correctness:

**(a) Keep beats as pure data, port the table.** A beat is keyframes; a
keyframe is numbers. The *player* is written twice (~60 lines each), the
*catalog* is a table copied verbatim. Divergence risk drops from
twenty-odd hand-ports to one data blob. Acceptable, still manual.

**(b) Generate the JS catalog from the Rust.** There is already precedent
and machinery: `parker face --sheet --json` exists and emits frames for
"surfaces that draw dots rather than print braille". A sibling
`parker face --catalog --json` emitting the beat table, baked into
visage's nix build, makes divergence structurally impossible. This is
cheap and I think it is the right answer.

**(c) parkerd decides, visage plays.** parkerd owns the idle clock and
pushes *"play beat X now"*; visage plays it from the shared catalog. Not
pose data at 25Hz — just the trigger. This is the architecturally correct
version, it keeps parkerd genuinely owning the pose, and it is a forcing
function for the pose channel that `state.svelte.js` is openly waiting
for (*"when there is a channel to subscribe to, this body becomes a read
and nothing else in the app moves"*).

**Recommendation: (b) now, (c) when the channel lands.** They compose —
(c) is a trigger on top of the catalog that (b) already shares.

For a first cut, visage running its own idle clock is *the same debt that
already exists* in `expression()`, not new debt. It is fine to start there
and say so in the comment.

---

## 4. Phasing

Each phase is separately shippable and separately enjoyable.

| phase | what | why here |
|---|---|---|
| **0** | saccades replace drift (§1.4) | biggest aliveness-per-line in the document, changes no interface, tells you how much flair you actually need |
| **1** | the beat player + `alert` tier + 4 beats, visage only | proves the third layer on the surface that shows it best |
| **2** | `mark → [Stroke; 4]`, `spin`, the comic-book pop | the only two pose-model changes anything needs |
| **3** | `bored` + `drowsy` tiers, sleep, wake-with-a-start | the ambient screensaver is complete |
| **4** | `--catalog --json` + terminal face plays beats | kills the divergence risk, SSH face catches up |
| **5** | `reach dance`, metronome clock, pose moves, one routine | it dances |
| **6** | the `lens()` hook + echo + scatter/reform | it is a music video |
| **7** | media gating; then audio capture → onset detection | it is dancing to the actual music |

Phases 0–3 are the ambient feature and stand alone. 5–7 are the loud one.
4 is the bridge and is the phase most likely to get skipped and most
likely to be regretted.

---

## 5. The questions, and how they were answered

1. **Does sleep survive a summon?** **Yes — you catch him napping.** He
   stays asleep until you actually type, and then the message that woke him
   carries a note saying so, stripped from the transcript but not from what
   the engine sees. That last part was not in the proposal and is the better
   idea: parker is an AI, and an AI that was asleep should be able to *say*
   he was asleep. `motion.svelte.js::wakeNote`.
2. **Does flair fire on the terminal face?** **Yes.** Both surfaces run the
   same catalog off the same clock. It was nearly free once the catalog was
   data, and the alternative was the SSH face being the only one that never
   blinks.
3. **Is `drowsy` gated on the user being away?** **No, and deliberately.**
   The clock measures "nothing has happened to parker", and since visage is
   a scratchpad you summon, that is almost always true. Being asleep when
   you open him is the normal case, not a failure of the gate. Revisit if
   it turns out to be annoying rather than charming — the gate would be
   `sense.inferred.mode` and it is one condition.
4. **How loud is dance allowed to be?** **Full send**, as asked: ink
   strobing on bar boundaries, RGB split, scatter-and-reform, the dive. The
   one ceiling that exists is `sane()` in dance.rs, which is about not
   drawing something impossible rather than about taste.
5. **Does dance survive a message?** **Yes** — it is a mode, not a view, so
   it keeps going while you type and the depth axis makes parker smaller as
   it always did. Esc stops it as the new first tier.
6. **How many marks?** **Six.** Four covers one Z; `zZZ` wants two.

### Still open

- **Audio.** Everything is written against `BeatClock`, and only the
  metronome implements it. Media gating (`sense.media` already has
  play state and title) is small; real onset detection needs parkerd to
  push energy at frame rate, which needs the channel §3 keeps pointing at.
- **`currentTrack()` in visage returns null.** The `auto` routine is seeded
  per session there rather than per track, because visage cannot see MPRIS
  and the dance slot carries a routine name but not a title. parkerd's own
  face already seeds from the real track.
- **The involuntary track is gone everywhere** (2026-08 redesign, then the
  `trouble` removal that finished it — DECISIONS.md): nothing is inferred on
  any surface, the worn read (`face_state`) is the whole face, and progress
  lives in the transcript. The terminal face used to keep deriving
  Trouble/Away from sense via `from_sense`, on the argument that those were
  machine facts rather than activity claims. The argument held and the
  feature still had to go: a machine fact is a true thing to say, but the
  face is not where it gets said.
