# OPENCODE — what parker can change about `parker code`, and how

Capability reference for modifying opencode's UI without forking it.
Everything here was verified against opencode 1.18.4 on parker (July 2026);
version-specific claims say so. The design record for *why* the plugin route
beat the patch route lives in DECISIONS.md ("branding opencode without
forking it"); the config that applies it is `home/parker/code.nix`.

## The shape of opencode

opencode is a bun/TypeScript monorepo, shipped as one compiled single-file
binary. nixpkgs builds it entirely from source (`bun build --single`), so a
source patch is genuinely available — it is just rarely the better option.

Two facts drive everything below:

- **The TUI is Solid, not Go.** `packages/tui/` renders with `@opentui/solid`
  — real components, flexbox layout, a theme context. UI is data and JSX, not
  a hand-rolled framebuffer.
- **The TUI is itself built out of plugins.** opencode's own tips panel,
  sidebar sections and footer are internal TUI plugins loaded through the
  same code path external ones use (`packages/tui/src/feature-plugins/`).
  Extending it is not a bolted-on API; it is the mechanism upstream builds
  with.

## Two config files, two env vars

opencode 1.18 splits its config. Getting these confused is the most likely
way to waste an hour:

| File | Reached by | Owns |
|---|---|---|
| `config.json` | `OPENCODE_CONFIG` | agent: providers, MCP, sharing, permissions |
| `tui.json` | `OPENCODE_TUI_CONFIG` | terminal UI: theme, keybinds, TUI plugins |

`parker code` sets both (`parkerd/src/main.rs`, `run_code`), pointing at
`~/.config/parker/{code,tui}.json`. The engine (`engine.nix`) sets neither of
these — it runs headless and never loads a TUI, which is why `parker code`
and `parker-engine` can share one binary with opposite configs.

`tui.json` resolution order, lowest precedence first: global
`~/.config/opencode/tui.json` → `OPENCODE_TUI_CONFIG` → project files walking
up from cwd → `.opencode/` directories. **A project's `tui.json` outranks
parker's**, which is correct — a repo asking for its own theme should win.

### The legacy `theme` key

`theme` in `config.json` is deprecated but still honored at runtime (verified:
it applies even when its migration is blocked). On first run opencode also
*migrates* it — writing a `tui.json` next to the config file and stripping the
key from the original. For parker that wrote an untracked
`~/.config/parker/tui.json` plus a `.tui-migration.bak`, files home-manager
did not own, in a directory it does. Declaring `theme` in parker's own
`tui.json` is the same setting with none of that; `code.nix` has a one-shot
activation entry that removes the strays.

## What a TUI plugin can reach

A TUI plugin is one `.tsx` file exporting `{ id, tui }`. `tui(api)` gets:

- `api.slots.register(...)` — fill named slots in the host layout
- `api.route.register/navigate` — whole new screens
- `api.keymap.registerLayer(...)` — commands, keybinds, slash commands
- `api.ui.Dialog` / `DialogSelect` / `DialogPrompt` / `toast` — modals
- `api.theme.current` — live theme colors as RGBA
- `api.state.*` — sessions, providers, models
- `api.event.on(...)` — the same event stream the server emits
- `api.kv` — persisted per-plugin state
- `api.attention.notify(...)` — desktop notification / sound

The host slots, from `TuiHostSlotMap` in `@opencode-ai/plugin/tui`:

| Slot | Mode | Notes |
|---|---|---|
| `home_logo` | `replace` | the wordmark — what parker uses |
| `home_prompt`, `session_prompt` | `replace` | the input box itself |
| `home_footer`, `sidebar_title`, `sidebar_footer` | `single_winner` | last registrant wins |
| `app`, `app_bottom`, `home_bottom`, `home_prompt_right`, `session_prompt_right`, `sidebar_content` | default | additive, ordered by `order` |

`mode="replace"` is the important one: it means the host's own child is
*dropped* when a plugin fills the slot. The logo is replaceable by design.

Plugins can also define their own slots and render them from plugin UI, which
is how a plugin becomes extensible by another plugin.

## Server plugins — the other half

**Server plugins are a separate target**, and the distinction is not cosmetic:
a TUI plugin runs in the terminal front-end and may only draw; a server plugin
runs in the agent loop, where the messages and the tool calls actually are.

They are declared in a `plugin` array too — but in **`config.json`**, not
`tui.json`. Same key name, two different files. Putting a server plugin in
`tui.json` loads nothing and reports nothing.

The shape is a *named export* returning a hooks object, not the `{ id, tui }`
default export TUI plugins use:

```ts
import type { Plugin } from "@opencode-ai/plugin"
export const MyHooks: Plugin = async ({ project, client, $, directory, worktree }) => ({
  "chat.message": async (input, output) => {},
})
```

The hooks parker uses, all verified present in 1.18.4:

| Hook | Fires | parker uses it for |
|---|---|---|
| `chat.message` | before the model sees a message | pushing the sense block into a session's first turn |
| `shell.env` | before every bash tool call | `PARKER_*`, and killing pagers |
| `tool.execute.before` | before any tool | throwing to *deny* — the parkerd/ and request-socket rules |
| `event` | everything | `session.idle` / `session.error` / `permission.asked` → parker's face |

Throwing from a `before` hook is how you deny: the tool call fails and the
model reads the thrown message, so the message is the whole UX.

`event` is the one that reaches outside the terminal. It is how `parker code`
got the ability to change parker's face and send a notification without the
model choosing to — MCP can only offer a verb the model may decline to call.

**Testing them is harder than it looks.** `opencode agent list` honours
`OPENCODE_CONFIG` and validates agents, commands and permissions — but it does
*not* load server plugins, and neither does `opencode serve` until a session
exists. A deliberately broken plugin passes both silently. What does catch a
syntax error is transpiling the module directly:

```bash
nix run nixpkgs#bun -- build code-hooks.ts --target=bun --external '*' --outdir /tmp/x
```

That is a parse check, not a behaviour check. Nothing short of an interactive
session proves a hook actually fires.

## The rest of the config surface

Verified against 1.18.4's own shipped schemas, which are the authority and are
right there in the store:

```bash
jq -r '.["$defs"].Config.properties | keys[]' $(dirname $(readlink -f $(which opencode)))/../share/opencode/config.json
jq -r '.properties | keys[]' .../share/opencode/tui.json
```

- **Agents are replaceable, including the built-ins.** `agent.plan.disable =
  true` genuinely removes it — `opencode agent list` stops listing it. So
  "renaming" plan and build is really disabling them and defining your own.
  `default_agent` must then name a live primary: opencode falls back to
  `build` when it is invalid, and a disabled `build` is invalid.
- **`permission`** takes either an action (`ask`/`allow`/`deny`) or a
  glob→action map, per tool, and per agent. `bash = { "*" = "allow"; "sudo *"
  = "ask"; }` is the useful shape.
- **`command`** entries are prompt templates. `` !`cmd` `` pastes the command's
  *output* into the prompt before the model sees it, `@file` inlines a file,
  `$ARGUMENTS`/`$1` take what was typed. `subtask = true` runs it in a
  subagent so its reasoning never enters the main thread.
- **`lsp` and `formatter`** take explicit `command` arrays — store paths, so
  opencode cannot be running a different rust-analyzer than the dev shell.
- **`username`**, **`subagent_depth`**, **`skills`**, **`references`** exist
  and are cheap wins.

### Thinking mode

There are two separate knobs and they are easy to confuse:

- `display_thinking` (tui.json keybind) — whether thinking blocks are *drawn*.
- `variant_cycle` / `variant_list` (tui.json keybinds) and the `variant` field
  on an agent or command — the model's **reasoning variant**. This is
  opencode's actual thinking-effort control. `reasoningEffort`,
  `thinkingBudget` and `interleaved-thinking` all appear in the 1.18.4 binary
  behind it.

### tui.json, in full

`attention`, `diff_style`, `keybinds`, `leader_timeout`, `mouse`, `plugin`,
`plugin_enabled`, `prompt`, `scroll_acceleration`, `scroll_speed`, `theme`.

`attention.sounds` takes per-event sound paths (`default`, `question`,
`permission`, `error`, `done`, `subagent_done`) — a parker soundboard is plain
config, no plugin required.

## The nix trap that isn't

The docs say local plugins get `@opencode-ai/plugin` installed into a
*writable* `.opencode` dir — which would rule out store paths entirely. It
does not apply to a single-file plugin: the `@opencode-ai/plugin/tui` import
is types-only, and the runtime JSX import (`@opentui/solid`) resolves out of
the compiled binary's own bundle.

**Verified on 1.18.4:** a plugin loads from a `chmod a-w` directory with no
`node_modules` anywhere, referenced by absolute path. That is what makes this
compatible with the repo's "all configs compiled" rule.

Use an absolute store path in `tui.json`, not `./relative.tsx` — relative
specs resolve against the config file that declared them, and parker's
`tui.json` lives in the store beside nothing.

## The glyph font

`packages/tui/src/logo.ts` is the whole opencode wordmark: two arrays of four
strings. Each text row is two vertical pixels — `█` both on, `▀` top, `▄`
bottom, space neither. Four marks are not characters but instructions to the
renderer: `_` and `^` paint a cell whose background is tinted 25% from the
theme background toward the foreground (the inner shadow), `~` and `,` paint
shadow-only half-blocks.

Rendering is two-tone: `left` in `textMuted`, `right` in `text` + bold. That
is the `open|code` split; parker's falls between `par` and `ker`.

Letters are 4 cells wide with 1-cell gaps — except `r`, which parker draws 3
wide. At 4 the lowercase `r` leaves three dead columns and the word reads as
"par ker". The glyph table and the reasoning live in `home/parker/code-logo.tsx`.

## When to patch instead

Patching stays available and is cheaper than it looks — `overrideAttrs` with a
`substituteInPlace` in `postPatch`, and the `node_modules` fixed-output
derivation is keyed on `src` so it does not re-fetch. Costs: a local bun
bundle on every opencode bump, and a `--replace-fail` that breaks the build
whenever upstream reformats the targeted file.

Reach for it only when there is no slot — chrome outside the slot map, or
behavior the plugin API does not expose.

## Verify after an opencode bump

- The `home_logo` slot still exists and is still `mode="replace"`
  (`packages/tui/src/routes/home.tsx`).
- `api.theme.current` still carries `text`, `textMuted`, `background`.
- `OPENCODE_TUI_CONFIG` is still read (`packages/opencode/src/config/tui.ts`).
- No `tui.json` or `.tui-migration.bak` reappears in `~/.config/parker/`.
