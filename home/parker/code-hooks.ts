// parker's hooks into the agent loop — the half of opencode that is not UI.
//
// code-status.tsx and code-theme.tsx are *TUI* plugins: they run in the
// terminal front-end and may only draw. This is a **server** plugin — same
// `plugin` config key, different entrypoint, and it runs in the agent loop
// where the messages and tool calls actually are. docs/OPENCODE.md said parker
// reached this layer "through MCP instead"; that is still true for the verbs,
// but MCP can only offer tools the model chooses to call. These four hooks
// fire whether or not the model thinks to ask, which is the whole point.
//
//   chat.message         before parker answers  — inject the machine reading
//   shell.env            before any bash runs   — hand the shell parker's env
//   tool.execute.before  before any tool runs   — enforce the house rules
//   event                after anything at all  — put it on parker's face
//
// The last one is the interesting one: it is the first time `parker code`
// reaches *out* of the terminal on its own. When a session goes idle, parker
// wears an expression and — only if you are not looking — says so out loud.
//
// Gating follows AGENTS.md exactly: "gating is about initiative, not
// obedience." Nothing the user asked for is gated. The idle notification is
// parker's own idea, so it is gated on `interruptible`, and a `parker sense`
// that cannot answer means no notification rather than a guessed one.
import type { Plugin } from "@opencode-ai/plugin"
import { execFile } from "node:child_process"

// Fire-and-forget. Every one of these is a nicety; none of them may ever take
// the session down, so nothing here rejects and nothing here is awaited by a
// hook that the agent loop is blocked on.
function parker(args: string[]): Promise<string | null> {
  return new Promise((resolve) =>
    execFile("parker", args, { timeout: 5000 }, (err, stdout) =>
      resolve(err ? null : stdout),
    ),
  )
}

async function sense(): Promise<any | null> {
  const raw = await parker(["sense", "--json"])
  if (!raw) return null
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}

// The text block, not the JSON: `parker sense` without --json is the form
// written for a model to read, and AGENTS.md records that the JSON of the
// same content costs ~3x the tokens.
async function senseBlock(): Promise<string | null> {
  const raw = await parker(["sense"])
  return raw?.trim() ? raw.trim() : null
}

// ── the house rules, enforced rather than documented ────────────────────
//
// AGENTS.md: "Nothing but Cargo inputs inside parkerd/. A stray file there is
// part of the derivation's source hash, so touching it rebuilds all 102
// dependency crates." That rule has been a comment that a model has to
// remember. Here it is a wall it cannot walk through.
const CARGO_OK = /\.(rs|toml|lock)$/
const PARKERD = /home\/parker\/parkerd\//

function guardPath(file: string): string | null {
  if (PARKERD.test(file) && !CARGO_OK.test(file)) {
    return (
      `Refused: ${file} is inside parkerd/, which takes Cargo inputs only. ` +
      `A stray file there is part of the derivation's source hash and rebuilds ` +
      `all 102 dependency crates. New source directories go in package.nix's fileset.`
    )
  }
  return null
}

export const ParkerHooks: Plugin = async () => {
  // One session's first message is the only one that gets the machine block —
  // re-injecting it per turn would pay for the same ~200 tokens on every
  // exchange to tell parker something that changes on the order of minutes.
  const greeted = new Set<string>()

  return {
    // ── input injection ───────────────────────────────────────────────
    // parker's prompt says "check sense before saying anything about the
    // present". This makes the first reading free and unprompted, so the
    // opening answer of a session is already grounded in the machine.
    "chat.message": async (_input: any, output: any) => {
      try {
        const id = output?.message?.sessionID ?? output?.message?.id
        if (!id || greeted.has(id)) return
        if (!Array.isArray(output?.parts)) return
        greeted.add(id)

        const block = await senseBlock()
        if (!block) return

        output.parts.push({
          type: "text",
          text: `<machine>\n${block}\n</machine>`,
          synthetic: true,
        })
      } catch {
        // A failed injection means parker calls `sense` itself, as before.
      }
    },

    // ── the shell parker hands you ────────────────────────────────────
    // Every bash tool call gets these. PARKER_* is so a script can tell it is
    // running inside parker code rather than a bare terminal; the flake path
    // is here because "rebuild" is the single most common thing typed in this
    // repo and it should never depend on cwd.
    "shell.env": async (input: any, output: any) => {
      try {
        output.env.PARKER = "code"
        output.env.PARKER_FLAKE = "/home/peki/parker#parker"
        output.env.PARKER_CWD = input?.cwd ?? ""
        // What parker commits carries his name (PARKER.md: `git log
        // --author=parker` is the audit trail); Peter stays the committer.
        // The engine sets the same pair in its service environment.
        output.env.GIT_AUTHOR_NAME = "parker"
        output.env.GIT_AUTHOR_EMAIL = "parker@parker.local"
        // Stop pagers dead. A tool call that opens `less` hangs the loop
        // until the timeout, and there is no terminal to press q in.
        output.env.PAGER = "cat"
        output.env.GIT_PAGER = "cat"
        output.env.SYSTEMD_PAGER = "cat"
      } catch {
        /* env is a convenience; never fatal */
      }
    },

    // ── the wall ──────────────────────────────────────────────────────
    // Throwing is how a before-hook denies: the tool call fails and the model
    // reads the message. Both rules below are repo invariants that were
    // previously enforced only by a human noticing.
    "tool.execute.before": async (input: any, output: any) => {
      const file: string | undefined = output?.args?.filePath ?? output?.args?.path
      if ((input?.tool === "write" || input?.tool === "edit") && file) {
        const refusal = guardPath(file)
        if (refusal) throw new Error(refusal)
      }

      if (input?.tool === "bash") {
        const cmd: string = output?.args?.command ?? ""

        // hyprctl, not .socket.sock — a client holding the request socket open
        // freezes the compositor for a 5 s timeout. AGENTS.md, rules of the
        // house. Cheap to type by accident, expensive to do.
        if (/\.socket\.sock/.test(cmd)) {
          throw new Error(
            "Refused: talking to Hyprland's request socket directly freezes the " +
              "compositor until a 5 s timeout. Use `hyprctl`.",
          )
        }

        // `nixos-rebuild switch` is the deploy, and it is the user's call, not
        // the model's. `build` is always allowed — that is the test gate.
        if (/nixos-rebuild\s+(switch|boot)/.test(cmd)) {
          throw new Error(
            "Refused: `nixos-rebuild switch` is Peter's to run. Use " +
              "`nixos-rebuild build --flake $PARKER_FLAKE` to prove it compiles, " +
              "then say it is ready.",
          )
        }
      }
    },

    // ── notifications, from inside the loop ───────────────────────────
    //
    // This used to move parker's face too — `idle` when a session went quiet,
    // `trouble` on an error, `blocked` on a permission prompt. It does not
    // anymore (Aug 2026, DECISIONS.md): nothing but parker puts a face on
    // parker. Session events are the loop's news about itself, and the face
    // is not a status bar for them.
    //
    // The `trouble` line had also been dead for a while on its own — `reach
    // face` filters on `Expression::WEARABLE`, which excluded `trouble` right
    // up until this same change, so the call always exited 1 into a `catch`
    // that swallowed it.
    //
    // Notifications stay. A notification is a thing you can miss and then go
    // read; a face is a thing you are already looking at.
    event: async ({ event }: any) => {
      try {
        if (event?.type === "session.idle") {
          // Gate it on the same `interruptible` the daemon will use, and stay
          // quiet if sense could not answer.
          const s = await sense()
          if (s?.inferred?.interruptible === true) {
            void parker(["reach", "notify", "parker code is done thinking."])
          }
          return
        }

        if (event?.type === "permission.asked") {
          // Something needs a human. This one is obedience, not initiative —
          // parker was asked to do the thing that hit the permission — so it
          // is not gated on interruptible.
          void parker(["reach", "notify", "parker code needs a decision.", "--urgency", "critical"])
        }
      } catch {
        /* notifying is never in the critical path */
      }
    },
  }
}

export default ParkerHooks
