// What "/" lists in the palette (Palette.svelte): the verbs, by what they
// touch, each with what it is set to now, the choices behind it, and where
// it lands. The palette renders them and App runs them (`pickPalette`), so a
// command and its key go down the same path and can never disagree.
//
// A verb with choices opens them (⏎, ⇥, or a space after its name); a verb
// without runs. A new command is one entry here. Its card is every other
// command's card — what it does, what is on, where it lands — unless it has
// something of its own to show, and `body` names which: `inks` (a look's
// colours, resolved) or `lore` (what a brain is named for, from brains.nix).
//
// Called inside the palette's $derived, so what it reads from `app` is
// tracked: the brain door hands over and the look this screen wears stay
// current while the palette is open.
//
// THE MACHINE's commands past `/theme` are not written here: they are made
// from ^,'s list (parkerd's `parker controls`, controls.svelte.js), one
// verb per slash name — `/volume` with all sound and every app making
// sound, `/wifi`, `/bluetooth`, `/connect`, `/quiet` — so a setting
// exists once and ^K and ^, can never disagree. A pick runs as ^, runs it
// (`runSlash`). The session is not among them: ^, has it behind ⏎ ⏎, and
// one ⏎ in a list is too few for off.

import { app } from './state.svelte.js';
import { THEMES, polarity } from './themes.js';
import { immersive } from './shell.js';
import { isDancing } from './motion.svelte.js';
import { ctl, allRows } from './controls.svelte.js';

/// What a command touches, in the order the palette lists them.
export const DOMAINS = ['PARKER', 'THIS SCREEN', 'THE MACHINE'];

/// `anthropic/claude-haiku-5-5` → `haiku 5.5`.
export const shortModel = (model) =>
  String(model ?? '')
    .split('/')
    .at(-1)
    .replace(/^claude-/, '')
    .replace(/-(\d+)-(\d+)$/, ' $1.$2');

export function commands() {
  const brain = app.brain;
  return [
    {
      id: 'new',
      domain: 'PARKER',
      what: 'a new thread',
      key: '^N',
      lands: ['the next thing you say starts a thread.', 'this one stays in earlier.'],
    },
    {
      id: 'dance',
      domain: 'PARKER',
      what: 'dance, or stop dancing',
      key: '^SPACE',
      now: isDancing() ? 'dancing' : '',
      choices: [
        { arg: 'auto', hint: 'until you stop it' },
        { arg: 'stop', hint: 'ESC stops it too' },
      ],
      lands: ["parker's face, on every screen, until you stop it."],
    },
    {
      id: 'brain',
      domain: 'PARKER',
      what: 'what parker thinks with',
      now: brain?.name ?? '',
      body: 'lore',
      choicesLabel: 'BRAINS',
      // Not hard-coded: the set is brains.nix, handed over by door.
      choices: (brain?.brains ?? []).map((b) => ({
        arg: b.name,
        hint: shortModel(b.model),
        model: b.model,
        lore: b.lore ?? '',
        now: b.name === brain.name,
      })),
      lands: ['every thread, from your next message.', 'one brain for all of parker, never one per thread.'],
    },
    {
      id: 'view',
      domain: 'THIS SCREEN',
      what: 'default ⇄ session',
      key: '^TAB',
      now: app.view,
      lands: ['this screen, at once. the face and the transcript move; nothing else does.'],
    },
    {
      id: 'visage',
      domain: 'THIS SCREEN',
      what: 'how this screen looks',
      now: app.theme,
      body: 'inks',
      choicesLabel: 'LOOKS',
      choices: THEMES.map((t) => ({
        arg: t.name,
        hint: polarity(t),
        theme: t,
        group: t.world ?? 'own',
        now: t.name === app.theme,
      })),
      groups: [
        ['own', "VISAGE'S OWN"],
        ['forge', 'THE FORGE'],
        ['drift', 'THE DRIFT'],
        ['garden', 'THE GARDEN'],
      ],
      lands: ['only this screen. the tower and the phone can differ.', 'never the desktop: that is /theme.'],
    },
    {
      id: 'controls',
      domain: 'THIS SCREEN',
      what: 'every setting, in vim keys',
      key: '^,',
      now: app.controls ? 'up' : '',
      lands: ["the transcript's place, parker on the rail.", '^, again puts them away.'],
    },
    // Only where there is such a thing (shell.js) — never on the tower.
    ...(immersive
      ? [
          {
            id: 'fullscreen',
            domain: 'THIS SCREEN',
            what: "real fullscreen, the browser's keys handed over",
            key: '^F',
            lands: ['this window. hold ESC to leave.'],
          },
        ]
      : []),
    {
      id: 'theme',
      domain: 'THE MACHINE',
      what: 'desktop, terminal, shell',
      // The `theme` CLI through door's allowlist: these and nothing else.
      choices: [
        { arg: 'dark', hint: 'seconds', group: 'when' },
        { arg: 'light', hint: 'seconds', group: 'when' },
        { arg: 'auto', hint: 'follow the clock', group: 'when' },
        { arg: 'set forge', hint: 'rebuild ~30s', group: 'family' },
        { arg: 'set drift', hint: 'rebuild ~30s', group: 'family' },
        { arg: 'set garden', hint: 'rebuild ~30s', group: 'family' },
      ],
      groups: [
        ['when', 'DAY AND NIGHT'],
        ['family', 'FAMILY'],
      ],
      lands: ['the whole desktop: windows, terminal, shell.', 'never visage, which wears its own.'],
    },
    ...machine(),
  ];
}

/// What a verb is about, by the rows it reaches.
const WHAT = {
  volume: 'all sound, or one app',
  mic: 'the mic',
  mute: 'all sound, silent or not',
  music: 'what is playing',
  wifi: 'which network, or none',
  tailscale: 'the tailnet',
  bluetooth: 'the radio',
  connect: 'a paired device',
  glass: 'unfocused windows, see-through',
  night: 'night light',
  quiet: 'do not disturb',
};

/// ^,'s rows as ^K commands, one per slash verb.
function machine() {
  ctl.sections; // tracked: the commands follow the list as it is read
  const by = new Map();
  for (const { r } of allRows()) {
    if (!r.slash || r.local) continue;
    const [verb, ...rest] = r.slash.split(' ');
    if (!by.has(verb)) by.set(verb, []);
    by.get(verb).push({ r, target: rest.join(' ') });
  }
  return [...by].map(([verb, rows]) => {
    const head = rows[0].r;
    const groups = rows.length > 1 ? rows.map(({ r, target }) => [r.id, (target || r.label).toUpperCase()]) : undefined;
    return {
      id: verb,
      domain: 'THE MACHINE',
      machine: true,
      what: WHAT[verb] ?? head.label,
      now: nowOf(head),
      choices: rows.flatMap(({ r, target }) => choicesOf(r, target).map((c) => ({ ...c, group: groups ? r.id : undefined }))),
      groups,
      // A level's card says where each row is now, not twenty-odd levels.
      card: head.kind === 'slider' ? rows.map(({ r, target }) => ({ arg: `${target || r.label} · ${nowOf(r)}` })) : undefined,
      lands: [verb === 'glass' ? 'the desktop, through a rebuild: about half a minute.' : 'the machine, at once.', '^, has it too.'],
    };
  });
}

function nowOf(r) {
  if (r.kind === 'slider') return `${r.value}${r.muted ? ' · muted' : ''}`;
  if (r.kind === 'toggle') return r.value ? 'on' : 'off';
  return String(r.value ?? '');
}

/// A row's choices as ^K offers them: levels by tens, on and off, its
/// choices, or the one thing a device does. The one on now is `now`, which
/// the palette marks itself.
function choicesOf(r, target) {
  const arg = (v) => (target ? `${target} ${v}` : `${v}`);
  switch (r.kind) {
    case 'slider': {
      const lo = r.min ?? 0;
      const hi = r.max ?? 100;
      const step = hi - lo <= 50 ? 5 : 10;
      const out = [];
      const near = Math.round((r.value - lo) / step) * step + lo;
      for (let v = hi; v >= lo; v -= step) out.push({ arg: arg(v), hint: '', now: v === near });
      if (r.muted != null) out.push({ arg: arg(r.muted ? 'unmute' : 'mute'), hint: r.muted ? 'muted' : '' });
      return out;
    }
    case 'toggle':
      return ['on', 'off'].map((w) => ({ arg: arg(w), hint: '', now: (w === 'on') === !!r.value }));
    case 'choice':
      return (r.choices ?? []).map((c) => ({ arg: arg(c), hint: '', now: c === r.value }));
    case 'action':
      return [{ arg: target || r.label, hint: r.value === 'connected' ? 'disconnect' : 'connect' }];
    case 'transport':
      return [
        { arg: arg('toggle'), hint: r.value === 'playing' ? 'pause' : 'play' },
        { arg: arg('next'), hint: '' },
        { arg: arg('previous'), hint: '' },
      ];
    default:
      return [];
  }
}
