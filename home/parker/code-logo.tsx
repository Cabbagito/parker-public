/** @jsxImportSource @opentui/solid */
// parker's wordmark on `parker code`'s home screen.
//
// opencode renders its own logo inside a plugin slot declared `mode="replace"`
// (packages/tui/src/routes/home.tsx), so registering `home_logo` swaps the
// wordmark with no fork and no build-time patch. That is the whole reason this
// file exists rather than a nixpkgs override: the seam is upstream's, published
// and documented (packages/opencode/specs/tui-plugins.md), so an opencode bump
// carries it forward for free.
//
// The glyphs are opencode's own font, read out of packages/tui/src/logo.ts.
// Each text row is two vertical pixels: `█` is both on, `▀` the top one, `▄`
// the bottom one. `p` and `e` are lifted verbatim; `a`, `r` and `k` are drawn
// to match. `r` is three cells wide where the others are four — a four-wide
// `r` leaves three dead columns and the word reads as "par ker".
//
// `_` and `^` are marks, not characters: they paint a cell whose background is
// tinted a quarter of the way from the theme background toward the foreground,
// which is what gives the letters their inner shadow. Blank cells inside a
// glyph use a real space instead, so no shadow hangs in open air.
//
// Left half is the theme's primary (redline's pure red), right half is bright
// foreground and bold — the same two-tone split opencode gives "open|code",
// here falling between "par" and "ker". Upstream mutes its left half; parker
// leads with the accent instead, so the wordmark carries the scheme's colour
// rather than reading as two shades of gray.
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@opencode-ai/plugin/tui"
import { RGBA, TextAttributes } from "@opentui/core"
import { For, type JSX } from "solid-js"

//        p      a     r          k      e     r
const logo = {
  left: ["             ", "█▀▀█ ▀▀▀█ █▀▀", "█__█ █^^█ █  ", "█▀▀▀ ▀▀▀▀ ▀  "],
  right: ["▄            ", "█ ▄▀ █▀▀█ █▀▀", "█▀▄  █^^^ █  ", "▀  ▀ ▀▀▀▀ ▀  "],
}

function tint(background: RGBA, foreground: RGBA, amount: number) {
  const mix = (a: number, b: number) => a + (b - a) * amount
  return RGBA.fromValues(
    mix(background.r, foreground.r),
    mix(background.g, foreground.g),
    mix(background.b, foreground.b),
    1,
  )
}

const tui: TuiPlugin = async (api: TuiPluginApi) => {
  api.slots.register({
    slots: {
      home_logo() {
        // Read through api.theme.current on every render rather than closing
        // over it: the theme is the machine's Stylix scheme, and `theme set`
        // can change it under a running session.
        const row = (line: string, fg: RGBA, bold: boolean): JSX.Element[] => {
          const shadow = tint(api.theme.current.background, fg, 0.25)
          const attributes = bold ? TextAttributes.BOLD : undefined
          return Array.from(line).map((char) => {
            if (char === "_")
              return (
                <text fg={fg} bg={shadow} attributes={attributes} selectable={false}>
                  {" "}
                </text>
              )
            if (char === "^")
              return (
                <text fg={fg} bg={shadow} attributes={attributes} selectable={false}>
                  ▀
                </text>
              )
            return (
              <text fg={fg} attributes={attributes} selectable={false}>
                {char}
              </text>
            )
          })
        }

        return (
          <box>
            <For each={logo.left}>
              {(line, index) => (
                <box flexDirection="row" gap={1}>
                  <box flexDirection="row">{row(line, api.theme.current.primary, false)}</box>
                  <box flexDirection="row">{row(logo.right[index()], api.theme.current.text, true)}</box>
                </box>
              )}
            </For>
          </box>
        )
      },
    },
  })
}

// The id is what `plugin_enabled` keys off, so it is API: keep it stable.
const plugin: TuiPluginModule = { id: "parker.logo", tui }

export default plugin
