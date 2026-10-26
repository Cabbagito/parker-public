/** @jsxImportSource @opentui/solid */
// parker's readouts inside `parker code` — the machine on screen, always.
//
// Three slots, three densities of the same idea, so it is easy to see which
// one earns its space and drop the other two:
//
//   home_footer           a hint bar — the slash commands, on the one screen
//                         where nothing is happening yet and the useful thing
//                         to show is what you can type
//   session_prompt_right  a compact machine readout beside the input box,
//                         visible while you are actually working
//   sidebar_content       the full MACHINE block: gpu, memory, focus, media
//   sidebar_footer        the statusline — peki@parker • dir • branch • model,
//                         deliberately the same shape as the Claude Code one
//                         in ~/.claude/statusline-command.sh
//
// The statusline started under the home screen and was moved: a status line
// on a screen with no status is decoration. It belongs next to the machine
// readout, behind the same ctrl+b.
//
// `home_footer` and `sidebar_footer` are single_winner (last registrant wins)
// and the other two are additive, which is why this file registers all four
// at once rather than shipping four plugins that would fight over ordering.
//
// The numbers come from `parker sense --json` — the same reading the model
// gets through `parker mcp`, so the panel and the agent can never disagree
// about the machine. sense costs ~90 ms, so it is polled on an interval
// rather than per-render; POLL_MS is the only knob that matters here.
//
// Everything degrades to nothing: if `parker` is not on PATH, or sense fails,
// or a field is absent, the row is omitted rather than rendered as "unknown".
// That is home/parker/AGENTS.md's first rule of the house — every input
// degrades to "I don't know" — applied to pixels.
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@opencode-ai/plugin/tui"
import { execFile } from "node:child_process"
import { createSignal, onCleanup, Show, type JSX } from "solid-js"

const POLL_MS = 6000

// The slash commands defined in code.nix, shown on the home screen. Kept as
// a plain list rather than read from api.state.config.command so the order is
// the useful one rather than alphabetical — `/sense` first because it is the
// one that makes parker parker.
const VERBS = ["/sense", "/look", "/drift", "/rebuild", "/calibrate", "/verify"]

type Sense = {
  at?: string
  uptime_secs?: number
  focus?: { class?: string; title?: string; workspace_name?: string; redacted?: boolean }
  media?: { player?: string; status?: string; title?: string; artist?: string }
  machine?: {
    gpu?: { util_pct?: number; mem_used_mb?: number; temp_c?: number; watts?: number }
    mem_used_gb?: number
    mem_total_gb?: number
    disk_used_pct?: number
    failed_units?: number
    failed_user_units?: number
  }
  inferred?: { mode?: string; interruptible?: boolean; health?: string }
}

// One reading of the machine, shared by every slot below. Nothing here throws:
// a missing `parker` binary is the same as a machine with nothing to say.
function createSense() {
  const [sense, setSense] = createSignal<Sense | null>(null)

  const read = () =>
    execFile("parker", ["sense", "--json"], { timeout: 4000 }, (err, stdout) => {
      if (err) return setSense(null)
      try {
        setSense(JSON.parse(stdout) as Sense)
      } catch {
        setSense(null)
      }
    })

  read()
  const timer = setInterval(read, POLL_MS)
  onCleanup(() => clearInterval(timer))

  return sense
}

const tui: TuiPlugin = async (api: TuiPluginApi) => {
  const sense = createSense()

  // Read through api.theme.current on every render, never closed over: the
  // theme is the machine's Stylix scheme and code-theme.tsx swaps it under a
  // running session when the day/night specialisation flips.
  const t = () => api.theme.current

  const Dot = (): JSX.Element => <text fg={t().borderActive}> • </text>

  // "gaming", "ok", "degraded" — the words sense already uses, coloured by
  // what they mean rather than re-deciding at each call site.
  const health = (h?: string) => (h === "bad" ? t().error : h === "degraded" ? t().warning : t().textMuted)

  // The last path segment — the same rule the Claude statusline uses, so the
  // two read as one family.
  const dirName = () => {
    const p = api.state.path?.directory ?? api.state.path?.worktree ?? ""
    const parts = p.split("/").filter(Boolean)
    return parts[parts.length - 1] ?? ""
  }

  // "anthropic/claude-sonnet-5-5" reads as "claude-sonnet-5-5" — the provider is
  // never in question on this machine and it costs a third of the width.
  const model = () => {
    const m = (api.state.config?.model as string | undefined) ?? ""
    return m.includes("/") ? m.slice(m.indexOf("/") + 1) : m
  }

  api.slots.register({
    slots: {
      // ── the hint bar ──────────────────────────────────────────────────
      //
      // The home screen is the one moment nothing is happening yet, which is
      // exactly why a statusline was wrong here — there is no status to
      // report. What you actually want at the instant before you type is to
      // be reminded what you can type. These are the commands from code.nix;
      // if you add one there, add it here.
      home_footer() {
        // Deliberately built from the same two primitives the wordmark uses —
        // a `box` and a `text` with an `fg`. Fragments inside a map and the
        // flexbox alignment props may well work, but nothing here is worth
        // finding out the hard way in a slot that renders on every launch.
        return (
          <box flexDirection="column">
            <box flexDirection="row">
              <text fg={t().accent}>{VERBS.join("  ")}</text>
            </box>
            <box flexDirection="row">
              <text fg={t().borderActive}>tab</text>
              <text fg={t().textMuted}> ghost / wired</text>
              <text fg={t().borderActive}>{"   ctrl+b"}</text>
              <text fg={t().textMuted}> machine</text>
            </box>
          </box>
        )
      },

      // ── the working readout ───────────────────────────────────────────
      session_prompt_right() {
        const gpu = () => sense()?.machine?.gpu
        const mem = () => sense()?.machine
        return (
          <Show when={sense()}>
            <box flexDirection="row">
              <Show when={gpu()?.util_pct !== undefined}>
                <text fg={t().textMuted}>gpu </text>
                <text fg={t().text}>{`${gpu()!.util_pct}%`}</text>
              </Show>
              <Show when={gpu()?.temp_c !== undefined}>
                <text fg={t().textMuted}> </text>
                <text fg={(gpu()!.temp_c ?? 0) > 80 ? t().error : t().text}>
                  {`${gpu()!.temp_c}°`}
                </text>
              </Show>
              <Show when={mem()?.mem_used_gb !== undefined}>
                <Dot />
                <text fg={t().text}>{`${mem()!.mem_used_gb!.toFixed(1)}G`}</text>
              </Show>
              <Show when={sense()?.inferred?.mode}>
                <Dot />
                <text fg={health(sense()!.inferred!.health)}>{sense()!.inferred!.mode!}</text>
              </Show>
            </box>
          </Show>
        )
      },

      // ── the full block ────────────────────────────────────────────────
      sidebar_content() {
        const s = () => sense()
        const Row = (props: { label: string; children: JSX.Element }) => (
          <box flexDirection="row">
            <text fg={t().textMuted}>{props.label.padEnd(7)}</text>
            {props.children}
          </box>
        )
        return (
          <Show
            when={s()}
            fallback={
              <box>
                <text fg={t().textMuted}>machine — no reading</text>
              </box>
            }
          >
            <box flexDirection="column">
              <text fg={t().primary}>MACHINE</text>

              <Show when={s()!.machine?.gpu}>
                <Row label="gpu">
                  <text fg={t().text}>
                    {`${s()!.machine!.gpu!.util_pct ?? "?"}%  ${s()!.machine!.gpu!.temp_c ?? "?"}°C  ${Math.round(s()!.machine!.gpu!.watts ?? 0)}W`}
                  </text>
                </Row>
              </Show>

              <Show when={s()!.machine?.mem_used_gb !== undefined}>
                <Row label="mem">
                  <text fg={t().text}>
                    {`${s()!.machine!.mem_used_gb!.toFixed(1)} / ${s()!.machine!.mem_total_gb?.toFixed(0) ?? "?"} GB`}
                  </text>
                </Row>
              </Show>

              <Show when={s()!.machine?.disk_used_pct !== undefined}>
                <Row label="disk">
                  <text fg={t().text}>{`${s()!.machine!.disk_used_pct}%`}</text>
                </Row>
              </Show>

              {/* Failed units are the one number worth shouting about. */}
              <Show
                when={(s()!.machine?.failed_units ?? 0) + (s()!.machine?.failed_user_units ?? 0) > 0}
              >
                <Row label="units">
                  <text fg={t().error}>
                    {`${(s()!.machine!.failed_units ?? 0) + (s()!.machine!.failed_user_units ?? 0)} failed`}
                  </text>
                </Row>
              </Show>

              {/* The privacy denylist fails open (AGENTS.md, known gaps), so a
                  redacted title is honoured here rather than re-derived. */}
              <Show when={s()!.focus?.class && !s()!.focus?.redacted}>
                <Row label="focus">
                  <text fg={t().text}>{s()!.focus!.class!}</text>
                </Row>
              </Show>

              <Show when={s()!.media?.title && s()!.media?.status === "Playing"}>
                <Row label="media">
                  <text fg={t().info}>
                    {`${s()!.media!.title}${s()!.media!.artist ? ` — ${s()!.media!.artist}` : ""}`}
                  </text>
                </Row>
              </Show>

              <Show when={s()!.inferred?.mode}>
                <Row label="mode">
                  <text fg={health(s()!.inferred!.health)}>
                    {`${s()!.inferred!.mode}${s()!.inferred!.interruptible === false ? " · heads down" : ""}`}
                  </text>
                </Row>
              </Show>
            </box>
          </Show>
        )
      },

      // ── the statusline ────────────────────────────────────────────────
      //
      // This is the peki@parker line, and it lives here rather than under the
      // home screen because it answers "where am I and what am I running" —
      // a question you ask *during* work, not before it. Under the MACHINE
      // block, so ctrl+b brings up the machine and its context together.
      sidebar_footer() {
        return (
          <box flexDirection="row">
            <text fg={t().primary}>peki</text>
            <text fg={t().text}>@</text>
            <text fg={t().secondary}>parker</text>
            <Show when={dirName()}>
              <Dot />
              <text fg={t().info}>{dirName()}</text>
            </Show>
            <Show when={api.state.vcs?.branch}>
              <Dot />
              <text fg={t().warning}>{api.state.vcs!.branch!}</text>
            </Show>
            <Show when={model()}>
              <Dot />
              <text fg={t().success}>{model()}</text>
            </Show>
          </box>
        )
      },
    },
  })
}

// The id is what `plugin_enabled` keys off, so it is API: keep it stable.
const plugin: TuiPluginModule = { id: "parker.status", tui }

export default plugin
