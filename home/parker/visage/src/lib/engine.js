// The webview's only door to the engine: commands in, events out. The far
// side is ../door (visage-door), hung in one of two frames — the Tauri
// window or porthole — and shell.js picks the transport. All HTTP to the
// engine and the password stay on the far side either way.

import { invoke, listen } from './shell.js';

export const engine = {
  createSession: () => invoke('create_session'),
  getMessages: (sessionId) => invoke('get_messages', { sessionId }),
  listSessions: () => invoke('list_sessions'),
  sendMessage: (sessionId, text, model) =>
    invoke('send_message', { sessionId, text, model: model ?? null }),
  interrupt: (sessionId) => invoke('interrupt_session', { sessionId }),
  /// One line describing a run of acts, from the engine's `ledger` agent.
  summarize: (text) => invoke('summarize', { text }),
  /// The cards door keeps for the palette, by thread id — stale ones included.
  digests: (ids) => invoke('digests', { ids }),
  /// Write one thread's card from the `digest` agent; door also gives the
  /// thread the card's title. Resolves with the card.
  digest: (sessionId, text, facts) => invoke('digest', { sessionId, text, facts }),
  /// The marks on these threads, by id — `{ state, by, at, updated, turns,
  /// note? }`, holding or not (a mark holds while its `updated` is the
  /// thread's).
  marks: (ids) => invoke('marks', { ids }),
  /// ^X: your word on a thread — 'done', 'open', or 'clear' to let its card
  /// speak again — through `parker recall mark`. Resolves with `{ id,
  /// updated, mark }`, `mark` null when cleared.
  mark: (sessionId, state) => invoke('mark', { sessionId, state }),
  dismiss: () => invoke('dismiss'),
  /// What parker is wearing and whether he is dancing — the two runtime
  /// files parkerd's `reach face` and `reach dance` write.
  faceState: () => invoke('face_state'),
  /// Start or stop a dance, through the one verb that owns that state.
  dance: (routine, seconds) => invoke('dance', { routine, seconds: seconds ?? null }),
  /// The palette's `/theme` — light, dark, auto, or `set <family>`. The shell
  /// allowlists the arguments; this resolves with the command's last line.
  theme: (args) => invoke('theme', { args }),
  /// Which brain parker is on — `{ name, model, default, brains: [{ name,
  /// model }] }`, or null — or, given a name from that list, switch to it.
  /// Every message goes out on it without the page naming a model: door
  /// asks for the brain itself on each send.
  brain: (name) => invoke('brain', { name: name ?? null }),
  /// One reading of the machine (`parker sense --json`), for the corner.
  sense: () => invoke('sense_snapshot'),
  /// ^,: every setting as a row — `{ sections: [{ id, name, rows }] }`, or
  /// null (parkerd's `parker controls --json`).
  controls: () => invoke('controls'),
  /// ^,: one change, by row id and value — resolves with `{ id, label,
  /// kind, from, to, said }`. From the porthole the tower's rows refuse.
  setControl: (id, value) => invoke('set_control', { id, value }),
  /// The numbered spaces and the one in front, `{ spaces: [{ id, windows }],
  /// active }`, for the corner; `onSpaces` hears every change.
  spaces: () => invoke('spaces'),
  onSpaces: (cb) => listen('spaces', (e) => cb(e.payload)),
  /// A level moved by something other than parker or ^, — the keys on the
  /// desk, an app's own slider: `{ changes: [{ id, label, kind, from, to }],
  /// keys }` (door's `watch_levels`). Only as it happens; never replayed.
  onLevels: (cb) => listen('levels', (e) => cb(e.payload)),
  onEvent: (cb) => listen('engine-event', (e) => cb(e.payload)),
  onStatus: (cb) => listen('engine-status', (e) => cb(e.payload)),
};
