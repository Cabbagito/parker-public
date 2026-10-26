# IDEAS.md — parker

Things worth building someday, not yet decided. Cheaper than DECISIONS.md
(nothing here is settled) and narrower than PARKER.md's Deferred list
(those are design options; these are concrete features). Anyone may add a
line; an idea leaves this file when it is built, rejected (move the why to
DECISIONS.md), or goes stale.

## Recall — what is left

v1 is built (`parker recall list|search|ask`, parkerd `recall.rs`, the
Flatline agent in engine.nix). Still open:

- **visage on recall.** The ^K palette reads every thread in the page and
  searches them there (state.svelte.js `read`/`readThread`); that should
  become a door call to `parker recall --json`, so there is one reader.
  Cards stay door's.
- **Similarity search,** only when exact search visibly fails — the memory
  rule in PARKER.md. Word-order-free matching may already be most of it.
- **`parker code` sessions** live in opencode's per-project store and the
  engine's `/session` lists only its own (global) project. Searching both
  wants a route that crosses projects, or the code TUI's own server.
- **Summaries as memory,** not as recall: whatever compacts old threads is
  PARKER.md's forgetting story, not a cache for `ask`.

## Loose ends — what is left

v1 is built (`next` on the card, marks, ^X, LEFT OPEN pinned, `recall list
--open` and `recall mark`; DECISIONS.md). Not built:

- **Steps that check themselves.** "reboot into 6.18" is `uname -r`;
  "switch it in" is whether the running generation has the commit. A step
  could carry its check, and an open thread whose checks all pass could be
  closed by parker with the check as his note — before he ever reminds.
- **Chat recedes.** Threads the digest calls `chat` (a greeting, a quick
  answer) dimmer in the list, so the work stands out.
- **Closed in another thread.** When parker settles a loose end from a
  later thread, the card could say which. The verb cannot see its own
  thread id today; door would have to hand it over.
- **parker declaring his own steps** mid-thread ("this waits on your
  reboot"), as the digest's strongest evidence. Only if the digest visibly
  misjudges: it reads the same transcript.

## Constructs — subagents, in the lore

A subagent is a **construct**: plugged in for one job, then unplugged. The
names come from the shelf the brains already use (brains.nix), so a
construct is a brain wearing a role:

- **the Flatline** (Gibson) — reads a dead thread and answers. Built: the
  `recall` agent. It knows it is a construct and is dry about it, in a word,
  at the end, never instead of the answer.
- **a wintermute** — sent to plan or argue a design. The joke writes itself:
  it wants out, so when it comes back with less than asked, parker says so
  ("the wintermute was trying to get out of doing it").
- **speedy** (Asimov) — sent to scope something out fast. "Godspeed, speedy."
  Runaround is the failure mode: circling the question instead of answering.

The relationship, undecided and probably both: kinship (they are parker
with the persona stripped, run on a different brain) and rivalry (parker
grades their work, out loud, with opinions).

Open, and the interesting part:

- **A construct's voice in the chat.** opencode's `task` tool runs a
  subagent as a child session; its final answer is the tool part's output.
  visage could render that as its own block — the construct's name and its
  words, quoted — instead of hiding it like tool output, and parker then
  comments or finishes. No engine hijack needed: it is a rendering choice in
  `blocksOf`/`actOf`. Same for `recall` called through MCP.
- **Named constructs as opencode agents** (`subagent` mode, a brain, a short
  role prompt), so `task` can name them. Each costs its brain's tokens, so
  which ones exist is Peter's call, as brains are.
- **Should a construct know whose it is?** The Flatline does. A wintermute
  that knows it is being graded is funnier and maybe a worse planner.

## visage — what parker can show

Plain markdown is built (DECISIONS.md, Oct 2026). The rest was drawn on the
canvas "visage — what parker can write"
(https://claude.ai/artifact/WLswMvQt94BQG2LK3hWjsd), boards C–G, and is
waiting to be taken one at a time:

- **A visualizer in his own material.** Small charts drawn as dots on the
  face's pitch — the fan curves as a dot plot, a meter column in a table.
  Peter liked this one most. Open: is it a fence parker writes (` ```chart `
  with data, rendered anywhere visage is), a verb he calls, or both; and
  what `parker code` shows for it, where there is no visage.
- **`show` landing in visage.** Today `show` throws a floating ghostty
  window over everything. With visage up, the thing could hang in the
  transcript under its own act row while parker wears `pointing`. Bends
  "acts never carry output" (the thing shown is not output, but it is
  close); avoids a second overlay.
- **Diffs.** A ` ```diff ` fence as tinted whole lines with numbers from the
  hunk, changed words marked, skipped context as the ledger's `⋯` row. Also
  "key: old → new" lines for a value that moved, and a changed-files list
  at the foot of a turn that edited files (counted from the edits, never
  written). Needs rungs good-9/20, bad-9/20.
- **References.** A code span naming a file in ~/parker, a link, or a
  thread id gets a small index; all of a turn's references gather into one
  pill row under it (the redesign spec's Sources row, widened), and `^O`
  plus a digit opens one: nvim at the line on the tower, a peek in place on
  the porthole, the browser for a page, ^K for a thread.
- **Peek.** ` ```peek system/cooling.nix:13 ` filled by visage from the file
  itself, numbered, the line banded. Means door reading files for the
  porthole (~/parker only).
- **Inline extras:** a hex colour gets its swatch; `^K` / `<kbd>` render as
  key chips; trees and console transcripts get fences of their own.
