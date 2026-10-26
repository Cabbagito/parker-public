// visage's own themes — apart from the machine's (DECISIONS.md, Oct 2026).
//
// `theme set` / `theme light|dark` theme the desktop through Stylix; none of
// that reaches here, and nothing here reaches it. Six of these share a name
// and a look with themes/ (copied, not linked: retuning a system preset does
// not move visage, and the other way round) — except glacier, which wears
// the light twin's colours ("paper" on the theme board) under the drift's
// name. midnight is where visage started, the old placeholder palette.
//
// A theme is six colours. Everything a component reads is resolved from them
// at runtime (`resolve`):
//
//   fg-NN, accent-NN, good-NN, bad-NN — the ladder. Components never mix
//     their own colours; they name a rung (`var(--fg-45)`). A rung is
//     defined by *contrast*, not alpha: each is solved to sit at the
//     contrast it has on midnight, whatever the ground. Fixed alphas were
//     tuned on a dark ground and fade on a light one — grey on white loses
//     contrast faster than grey on black, and a light theme's accent can
//     never be as bright as midnight's blue. On midnight itself the solve
//     gives back the plain alphas, so its rung names stay true there.
//   accent, good, bad — held at 4.5:1 against the ground by moving their
//     OKLab lightness only, so blush's pink deepens to a rose that reads
//     and hangar's red still says red.
//   raised, palette-lift — the one floating box, the palette (^K). On a
//     dark ground: fg 4% into bg and a black lift. On a light one: paper
//     white and a soft ink shadow, because a box darker than its ground
//     reads as sunk, not raised.
//
// The choice is per screen (localStorage), not parker's: the tower and the
// phone can wear different ones, the way a lamp is per room.

export const THEMES = [
  { name: 'midnight', bg: '#101014', fg: '#c9c7cd', accent: '#7aa2f7', good: '#9ece6a', bad: '#f7768e' },
  { name: 'redline', world: 'forge', bg: '#0c0b0d', fg: '#f2eff0', accent: '#ff1524', good: '#8fb088', bad: '#e04b4b' },
  { name: 'hangar', world: 'forge', bg: '#ffffff', fg: '#2e2a2c', accent: '#e03e4b', good: '#8fb088', bad: '#e04b4b' },
  { name: 'trench', world: 'drift', bg: '#1c1b2e', fg: '#cbd8f0', accent: '#6fa8ff', good: '#8fb088', bad: '#e04b4b' },
  { name: 'glacier', world: 'drift', bg: '#f6f6f8', fg: '#26252e', accent: '#2f62d9', good: '#4c7f22', bad: '#c8304f' },
  { name: 'velour', world: 'garden', bg: '#201a1e', fg: '#d9cbd2', accent: '#e394b0', good: '#8fb088', bad: '#e04b4b' },
  { name: 'blush', world: 'garden', bg: '#faf6f1', fg: '#57504a', accent: '#d98ba6', good: '#8fb088', bad: '#e04b4b' },
  { name: 'newsprint', bg: '#eeece6', fg: '#1d1c1a', accent: '#c8401c', good: '#4f7a2e', bad: '#a3263a' },
  { name: 'overcast', bg: '#dcdde2', fg: '#1e1e25', accent: '#2a5bd0', good: '#3f6f1c', bad: '#b52a47' },
  { name: 'blueprint', bg: '#eaf0fb', fg: '#17233d', accent: '#9a5a00', good: '#2f6f3a', bad: '#b3263e' },
];

export const DEFAULT_THEME = 'midnight';

const KEY = 'visage.theme';

/// The rungs components use. Adding one is adding it here, then naming it.
const RUNGS = {
  fg: [8, 10, 16, 18, 20, 22, 25, 28, 30, 32, 35, 42, 45, 55, 58, 60, 72, 85],
  accent: [9, 14, 20, 35, 40, 50, 55, 60, 70, 75, 80, 90],
  good: [55, 75],
  bad: [45, 65],
};

/// The ladder's reference: each rung's contrast is the one it has here.
const REF = THEMES[0];

// ── colour arithmetic ──────────────────────────────────────────────────────

function rgb(hex) {
  const n = parseInt(hex.slice(1, 7), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
function hexOf(c) {
  return '#' + c.map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('');
}
const lin = (c) => ((c /= 255) <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const delin = (c) => 255 * (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);
const lum = (c) => 0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2]);
function contrast(a, b) {
  const [x, y] = [lum(a), lum(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
const over = (fg, bg, a) => fg.map((v, i) => v * a + bg[i] * (1 - a));
const rgba = (c, a) => `rgba(${c.map(Math.round).join(', ')}, ${+a.toFixed(3)})`;

/// The alpha at which `fg` over `bg` reaches `target`:1 — 1 if it never does.
function solve(fg, bg, target) {
  if (contrast(fg, bg) <= target) return 1;
  let [lo, hi] = [0, 1];
  for (let k = 0; k < 32; k++) {
    const m = (lo + hi) / 2;
    if (contrast(over(fg, bg, m), bg) < target) lo = m;
    else hi = m;
  }
  return hi;
}

function toLab(c) {
  const [r, g, b] = c.map(lin);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}
function fromLab(L, A, B) {
  const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s = (L - 0.0894841775 * A - 1.291485548 * B) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ].map(delin);
}
const inGamut = (c) => c.every((v) => v > -0.5 && v < 255.5);

/// `c` held at `min`:1 on `bg`: only lightness moves, away from the ground;
/// chroma eases off where the colour would leave sRGB.
function hold(c, bg, min) {
  if (contrast(c, bg) >= min) return c;
  const [L0, A, B] = toLab(c);
  const dir = lum(bg) > 0.18 ? -1 : 1;
  for (let L = L0 + dir * 0.004; L > 0 && L < 1; L += dir * 0.004) {
    let k = 1;
    let out = fromLab(L, A, B);
    while (!inGamut(out) && k > 0) {
      k -= 0.05;
      out = fromLab(L, A * k, B * k);
    }
    if (contrast(out, bg) >= min) return out.map(Math.round);
  }
  return dir < 0 ? [0, 0, 0] : [255, 255, 255];
}

// ── resolve and apply ──────────────────────────────────────────────────────

/// 'light' or 'dark', by how bright the ground is.
export const polarity = (theme) => (lum(rgb(theme.bg)) > 0.4 ? 'light' : 'dark');

/// A theme's six colours → every custom property visage reads.
export function resolve(theme) {
  const bg = rgb(theme.bg);
  const fg = rgb(theme.fg);
  const light = polarity(theme) === 'light';
  const ink = {
    fg,
    accent: hold(rgb(theme.accent), bg, 4.5),
    good: hold(rgb(theme.good), bg, 4.5),
    bad: hold(rgb(theme.bad), bg, 4.5),
  };
  const vars = {
    '--bg': theme.bg,
    '--fg': theme.fg,
    '--face-fg': theme.fg,
    '--accent': hexOf(ink.accent),
    '--good': hexOf(ink.good),
    '--bad': hexOf(ink.bad),
    '--bg-55': rgba(bg, 0.55),
    '--raised': light ? '#ffffff' : hexOf(over(fg, bg, 0.04)),
    '--palette-lift': light
      ? `0 40px 110px ${rgba(fg, 0.16)}, 0 2px 8px ${rgba(fg, 0.06)}`
      : '0 48px 120px rgba(0, 0, 0, 0.6)',
  };
  const refBg = rgb(REF.bg);
  for (const [name, steps] of Object.entries(RUNGS)) {
    const ref = rgb(REF[name]);
    for (const s of steps) {
      const target = contrast(over(ref, refBg, s / 100), refBg);
      vars[`--${name}-${s}`] = rgba(ink[name], solve(ink[name], bg, target));
    }
  }
  return { vars, light };
}

export const themeNamed = (name) => THEMES.find((t) => t.name === name);

/// The theme this screen wore last, or the default.
export function savedTheme() {
  try {
    const name = localStorage.getItem(KEY);
    if (themeNamed(name)) return name;
  } catch {
    /* no storage (private window): the default, every time */
  }
  return DEFAULT_THEME;
}

/// Put a theme on: the properties on <html>, which beat app.css's defaults,
/// plus the two things outside CSS — the browser chrome's colour and the
/// native widgets' polarity. Face.svelte reads its inks every frame, so the
/// face changes with the rest. Returns the name worn.
export function applyTheme(name, { save = true } = {}) {
  const theme = themeNamed(name) ?? themeNamed(DEFAULT_THEME);
  const { vars, light } = resolve(theme);
  const root = document.documentElement;
  for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
  root.style.colorScheme = light ? 'light' : 'dark';
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', theme.bg);
  if (save) {
    try {
      localStorage.setItem(KEY, theme.name);
    } catch {
      /* worn for this visit only */
    }
  }
  return theme.name;
}
