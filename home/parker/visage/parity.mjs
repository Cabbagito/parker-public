// Does the JavaScript face agree with the Rust one?
//
//   node parity.mjs
//
// `src/lib/face.js`, `beat.js` and `dance.js` are hand-ported logic. The
// catalog removes the risk of mistyping a *number*, but nothing stops a
// mistyped *formula*, and a wrong one is not loud: a splitmix64 finalizer
// written with doubles instead of BigInt still produces perfectly plausible
// eye movement, just not the same eye movement. So `parker face --catalog
// --json` writes reference answers alongside the data (and `--vignettes` for
// the long pieces), and this recomputes them here and compares.
//
// Run it after touching anything in the face stack on either side. It is
// deliberately a plain script with no test-runner dependency — visage has no
// JS test setup and this is not worth acquiring one for.

import catalog from './src/lib/catalog.json' with { type: 'json' };
import vignettes from './src/lib/vignettes.json' with { type: 'json' };
import { saccade, blink, hash01, applyMotion, EXPRESSIONS, VOLUNTARY, ZERO, MARKS } from './src/lib/face.js';
import { beatAt, motionAt, blinksAt } from './src/lib/beat.js';
import { Dance } from './src/lib/dance.js';

const EPS = 1e-9;
let checked = 0;
const failures = [];

function same(what, got, want) {
  checked++;
  const ok = typeof want === 'number' ? Math.abs(got - want) < EPS : got === want;
  if (!ok) failures.push(`${what}: js ${got} vs rust ${want}`);
}

function samePose(what, got, want) {
  for (const [field, value] of Object.entries(want)) {
    same(`${what}.${field}`, got[field], value);
  }
}

const p = catalog.parity;

for (const [n, want] of p.hash) same(`hash01(${n})`, hash01(n, 0x5eed), want);

for (const [t, x, y] of p.saccade) {
  const [gx, gy] = saccade(t);
  same(`saccade(${t}).x`, gx, x);
  same(`saccade(${t}).y`, gy, y);
}

for (const [t, want] of p.blink) same(`blink(${t})`, blink(t), want);

const base = {
  ...catalog.parity.base,
  mark: [],
  ink: 'plain',
  markInk: Array.from({ length: catalog.marks }, () => 'plain'),
};
// The base pose arrives as numbers only; marks start empty, which is what the
// idle pose actually is.
base.mark = Array.from({ length: catalog.marks }, () => ({
  x0: 0, y0: 0, x1: 0, y1: 0, w: 0,
}));

const byName = Object.fromEntries(
  catalog.idle.beats.map((b) => [b.name, b]),
);

for (const entry of p.beats) {
  const beat = byName[entry.name];
  if (!beat) {
    failures.push(`beat ${entry.name} is in parity but not in the catalog`);
    continue;
  }
  for (const { t, pose } of entry.at) {
    samePose(`${entry.name}@${t}`, beatAt(beat, base, t), pose);
  }
}

// The vignettes travel apart (`parker face --vignettes --json`), with their
// own answers: the pose, and what a vignette adds to a beat — the motion each
// section plays and whether the blink comes through.
const vignetteByName = Object.fromEntries(vignettes.beats.map((b) => [b.name, b]));
for (const entry of vignettes.parity) {
  const beat = vignetteByName[entry.name];
  if (!beat) {
    failures.push(`vignette ${entry.name} is in parity but not in vignettes.json`);
    continue;
  }
  for (const { t, pose, motion, blink } of entry.at) {
    samePose(`${entry.name}@${t}`, beatAt(beat, base, t), pose);
    same(`${entry.name}@${t}.motion`, motionAt(beat, 'drift', t), motion);
    same(`${entry.name}@${t}.blink`, blinksAt(beat, t), blink);
  }
}

// The expressions (face.js still ports them by hand) and the entrances
// (catalog data, played by beat.js). Poses arrive sparse — only what differs
// from ZERO — so they are filled back in first, or a field face.js got wrong
// where parkerd has the default would slip through.
function full(sparse) {
  const out = {};
  for (const f of catalog.fields) out[f] = sparse[f] ?? ZERO[f];
  out.mark = sparse.mark ? sparse.mark.map(([x0, y0, x1, y1, w]) => ({ x0, y0, x1, y1, w })) : ZERO.mark;
  out.ink = sparse.ink ?? ZERO.ink;
  out.markInk = sparse.markInk ?? ZERO.markInk;
  return out;
}

function sameWhole(what, got, want) {
  for (const f of catalog.fields) same(`${what}.${f}`, got[f], want[f]);
  for (let i = 0; i < MARKS; i++) {
    for (const k of ['x0', 'y0', 'x1', 'y1', 'w']) {
      same(`${what}.mark[${i}].${k}`, got.mark[i][k], want.mark[i][k]);
    }
    same(`${what}.markInk[${i}]`, got.markInk[i], want.markInk[i]);
  }
  same(`${what}.ink`, got.ink, want.ink);
}

const named = new Set();
for (const e of p.expressions) {
  named.add(e.name);
  const spec = EXPRESSIONS[e.name];
  if (!spec) {
    failures.push(`expression ${e.name} is in parkerd but not in face.js`);
    continue;
  }
  sameWhole(e.name, spec.pose, full(e.pose));
  same(`${e.name}.motion`, spec.motion, e.motion);
  same(`${e.name}.blinks`, spec.blinks !== false, e.blinks);
  same(`${e.name}.voluntary`, VOLUNTARY.includes(e.name), e.voluntary);
  for (const { phase, t, pose } of e.at ?? []) {
    sameWhole(`${e.name}~${spec.motion}@${phase}`, applyMotion(spec.motion, spec.pose, phase, t), full(pose));
  }
}
for (const name of Object.keys(EXPRESSIONS)) {
  if (!named.has(name)) failures.push(`expression ${name} is in face.js but not in parkerd`);
}

for (const entry of p.entrances) {
  const beat = catalog.entrances[entry.name];
  const spec = EXPRESSIONS[entry.name];
  if (!beat || !spec) {
    failures.push(`entrance ${entry.name} has no beat or no face`);
    continue;
  }
  for (const { t, pose, motion } of entry.at) {
    sameWhole(`entrance ${entry.name}@${t}`, beatAt(beat, spec.pose, t, EXPRESSIONS.idle.pose), full(pose));
    same(`entrance ${entry.name}@${t}.motion`, motionAt(beat, spec.motion, t), motion);
  }
}

for (const entry of p.dance) {
  const routine = Dance.byName(entry.name);
  const d = new Dance(routine, entry.bpm, 0, entry.seed);
  for (const { t, pose, mood, ink } of entry.at) {
    samePose(`dance ${entry.name}@${t}`, d.pose(base, t), pose);
    same(`dance ${entry.name}@${t}.mood`, d.mood(t), mood);
    same(`dance ${entry.name}@${t}.ink`, d.ink(t), ink);
  }
}

if (failures.length) {
  console.error(`✗ ${failures.length} of ${checked} checks disagree:\n`);
  for (const f of failures.slice(0, 40)) console.error('  ' + f);
  if (failures.length > 40) console.error(`  … and ${failures.length - 40} more`);
  process.exit(1);
}
console.log(`✓ visage agrees with parkerd on all ${checked} checks`);
