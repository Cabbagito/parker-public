// parker's face — a browser port of home/parker/visage/src/lib/face.js
// (itself a port of parkerd/src/face/{pose,mod}.rs). Same numbers, same
// names: geometry in dot units on the 96x40 grid.
//
// Registered as <parker-face expression="idle" fit="grid|seen">.
(function () {
  const DOTS_W = 96;
  const DOTS_H = 40;
  const TAU = Math.PI * 2;
  const MARKS = 6;

  const noStroke = () => ({ x0: 0, y0: 0, x1: 0, y1: 0, w: 0 });

  const ZERO = {
    gap: 0, w: 0, h: 0, round: 1, y: 0, x: 0, tilt: 0, squint: 1,
    pupil: 0, gazeX: 0, gazeY: 0,
    ring: 0, ringW: 0, sweep: 1, spin: 0,
    amp: 0, freq: 0,
    mark: Array.from({ length: MARKS }, noStroke),
    ink: 'plain',
    markInk: Array.from({ length: MARKS }, () => 'plain'),
  };

  const NUMERIC = ['gap','w','h','round','y','x','tilt','squint','pupil','gazeX','gazeY','ring','ringW','sweep','spin','amp','freq'];

  function padMarks(given) {
    const out = Array.from({ length: MARKS }, noStroke);
    for (let i = 0; i < Math.min(given.length, MARKS); i++) out[i] = given[i];
    return out;
  }

  function lerp(a, b, t) {
    const out = { ...ZERO };
    for (const k of NUMERIC) out[k] = a[k] + (b[k] - a[k]) * t;
    out.mark = a.mark.map((m, i) => {
      const n = b.mark[i] ?? m;
      return {
        x0: m.x0 + (n.x0 - m.x0) * t, y0: m.y0 + (n.y0 - m.y0) * t,
        x1: m.x1 + (n.x1 - m.x1) * t, y1: m.y1 + (n.y1 - m.y1) * t,
        w: m.w + (n.w - m.w) * t,
      };
    });
    out.ink = b.ink;
    out.markInk = b.markInk;
    return out;
  }

  const ease = {
    inOut(t) { t = Math.min(1, Math.max(0, t)); return t < 0.5 ? 4*t*t*t : 1 - Math.pow(-2*t+2, 3)/2; },
    backOut(t) { t = Math.min(1, Math.max(0, t)); const c1 = 1.70158, c3 = c1 + 1; return 1 + c3*Math.pow(t-1,3) + c1*Math.pow(t-1,2); },
    out(t) { t = Math.min(1, Math.max(0, t)); return 1 - Math.pow(1-t, 3); },
  };

  const M64 = (1n << 64n) - 1n;
  function hash01(n, salt) {
    let z = (BigInt(n) * 0x9e3779b97f4a7c15n) & M64;
    z ^= (BigInt(salt) * 0xbf58476d1ce4e5b9n) & M64;
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & M64;
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & M64;
    z ^= z >> 31n;
    return Number(z >> 11n) / 9007199254740992;
  }

  function saccade(t) {
    const DWELL = 0.62, FLICK = 0.045, REACH_X = 3.4, REACH_Y = 1.5;
    const anchor = (slot) => {
      let s = slot;
      for (let i = 0; i < 2; i++) { if (s > 0 && hash01(s, 0x5eed) < 0.34) s -= 1; else break; }
      return s;
    };
    const target = (slot) => {
      const s = anchor(slot);
      if (hash01(s, 0xc0ffee) < 0.22) return [0, 0];
      return [(hash01(s, 0xa11ce)*2-1)*REACH_X, (hash01(s, 0xb0b)*2-1)*REACH_Y];
    };
    const slot = Math.floor(t / DWELL);
    const into = t - slot * DWELL;
    const [fx, fy] = target(slot);
    const [px, py] = target(slot - 1);
    const k = into < FLICK ? ease.out(into / FLICK) : 1;
    const dx = Math.sin(t*1.7)*0.22 + Math.sin(t*0.53)*0.12;
    const dy = Math.cos(t*1.13)*0.14;
    return [px + (fx-px)*k + dx, py + (fy-py)*k + dy];
  }

  function blink(t) {
    const SLOT = 1.05, SHUT = 0.14, AGAIN = 0.23;
    const slot = Math.floor(t / SLOT);
    let squash = 1;
    for (const s of [slot - 1, slot]) {
      if (hash01(s, 0xb11) >= 0.34) continue;
      const begins = s*SLOT + hash01(s, 0xb12)*Math.max(0, SLOT - SHUT - AGAIN);
      const starts = [begins];
      if (hash01(s, 0xb13) < 0.18) starts.push(begins + AGAIN);
      for (const start of starts) {
        const into = (t - start) / SHUT;
        if (into >= 0 && into < 1) squash = Math.min(squash, Math.max(0.06, Math.abs(1 - into*2)));
      }
    }
    return squash;
  }

  const blinkable = (p) => p.pupil > 0.3 || (p.gap > 0.6 && p.h > 2);

  function applyMotion(motion, pose, phase, t) {
    const p = { ...pose, mark: pose.mark };
    const a = phase * TAU;
    switch (motion) {
      case 'breathe': {
        const s = 1 + Math.sin(a) * 0.06;
        p.w *= s; p.h *= s;
        if (p.pupil > 0) p.pupil *= 1 + Math.sin(a) * 0.12;
        break;
      }
      case 'wave': p.amp *= 1 + Math.sin(a * 0.5) * 0.15; break;
      case 'pulse': {
        const s = 1 - Math.max(0, Math.sin(a * 5)) * 0.22;
        p.w *= s; p.h *= s; p.ring *= 1 + Math.sin(a * 5) * 0.05;
        break;
      }
      case 'nod': p.y += Math.sin(a * 7) * 3.5; break;
      case 'shake': p.x += Math.sin(a * 9) * 3.5; break;
      case 'sway': p.x += Math.sin(a * 2) * 2.2; p.y += Math.sin(a * 4) * 0.9; break;
      case 'drift': { const [gx, gy] = saccade(t); p.gazeX += gx; p.gazeY += gy; break; }
    }
    return p;
  }

  const iris = (w, ring) => ({ ...ZERO, gap: 0, w, h: w, round: 1, ring, ringW: 2 });
  const eyes = (w, h, pupil) => ({ ...ZERO, gap: 18, w, h, round: 0.85, pupil });
  const line = (w, amp, freq) => ({ ...ZERO, gap: 0, w, h: 1.4, round: 0, amp, freq });
  const stroke = (x0, y0, x1, y1) => ({ x0, y0, x1, y1, w: 3 });
  const allInk = (name) => Array.from({ length: MARKS }, () => name);

  // The twelve that exist today (face.js), then the ones this design proposes.
  const EXPRESSIONS = {
    idle: { pose: { ...iris(9, 17), pupil: 3 }, motion: 'drift' },
    thinking: { pose: iris(4, 17), motion: 'blades' },
    speaking: { pose: line(46, 10, 3), motion: 'wave' },
    blocked: { pose: eyes(7, 11, 3.5), motion: 'breathe' },
    away: { pose: line(8, 0, 0), motion: 'none' },
    trouble: { pose: { ...iris(5, 10), pupil: 1.8, ringW: 3, ink: 'bad' }, motion: 'pulse' },
    yes: { pose: { ...ZERO, mark: padMarks([stroke(-11,1,-3,9), stroke(-3,9,13,-9)]), markInk: allInk('good') }, motion: 'nod' },
    no: { pose: { ...ZERO, mark: padMarks([stroke(-10,-10,10,10), stroke(10,-10,-10,10)]), markInk: allInk('bad') }, motion: 'shake' },
    curious: { pose: { ...eyes(7.5, 11, 3.5), squint: 0.65, tilt: -3, gazeY: -2.5 }, motion: 'drift' },
    surprised: { pose: { ...iris(15, 19), pupil: 2 }, motion: 'breathe' },
    amused: { pose: { ...eyes(8, 3, 0), y: -1 }, motion: 'breathe' },
    wince: { pose: { ...eyes(7.5, 3.5, 0), tilt: 3, squint: 0.55 }, motion: 'none' },

    // proposed — same primitives, same vocabulary
    listening: { pose: { ...eyes(7.5, 10, 3.2), gazeY: 0.6, squint: 0.92 }, motion: 'breathe' },
    skimming: { pose: { ...iris(4, 12), ringW: 1.6 }, motion: 'blades' },
    digging: { pose: { ...iris(3, 16), ringW: 2.8 }, motion: 'blades' },
    smug: { pose: { ...eyes(7.5, 6, 2.8), squint: 0.42, tilt: -2, y: -1, gazeX: 1.6 }, motion: 'breathe' },
    sheepish: { pose: { ...eyes(7, 7, 3), squint: 0.78, gazeX: -2.6, gazeY: 2.4, tilt: 2 }, motion: 'none' },
    deadpan: { pose: { ...eyes(9, 1.6, 0), round: 0 }, motion: 'none' },
    fond: { pose: { ...eyes(7, 4.2, 0), y: -1, ink: 'accent' }, motion: 'breathe' },
    melancholy: { pose: { ...eyes(7.5, 5, 3), squint: 0.86, gazeY: 2, tilt: 2 }, motion: 'breathe' },
    conspiratorial: { pose: { ...eyes(7.5, 8.5, 3.2), squint: 0.34, gazeX: 3, tilt: -1 }, motion: 'drift' },
    delighted: { pose: { ...eyes(8, 4, 0), y: -1, ring: 17, ringW: 2, ink: 'accent' }, motion: 'pulse' },
    suspicious: { pose: { ...eyes(8, 9, 3.4), squint: 0.44, gazeX: -2.8, tilt: -1.5 }, motion: 'none' },
    pointing: { pose: { ...iris(6, 14), pupil: 2.2, gazeX: 3.2, mark: padMarks([stroke(6, 0, 22, 0)]), markInk: allInk('accent') }, motion: 'none' },
  };

  const VOLUNTARY = ['yes','no','curious','surprised','amused','wince','smug','sheepish','deadpan','fond','melancholy','conspiratorial','delighted','suspicious'];

  class Grid {
    constructor(w = DOTS_W, h = DOTS_H) {
      this.w = w; this.h = h;
      this.dots = new Uint8Array(w * h);
      this.inkIdx = 1;
    }
    useInk(name) { this.inkIdx = { plain: 1, accent: 2, good: 3, bad: 4 }[name] ?? 1; }
    set(x, y) { x = Math.round(x); y = Math.round(y); if (x >= 0 && x < this.w && y >= 0 && y < this.h) this.dots[y*this.w+x] = this.inkIdx; }
    clear(x, y) { x = Math.round(x); y = Math.round(y); if (x >= 0 && x < this.w && y >= 0 && y < this.h) this.dots[y*this.w+x] = 0; }
    span(x, y0, y1) { for (let y = Math.round(y0); y <= Math.round(y1); y++) this.set(x, y); }
    clearEllipse(cx, cy, rx, ry) {
      for (let y = Math.floor(cy-ry); y <= Math.ceil(cy+ry); y++)
        for (let x = Math.floor(cx-rx); x <= Math.ceil(cx+rx); x++) {
          const nx = (x-cx)/rx, ny = (y-cy)/Math.max(ry, 0.1);
          if (nx*nx + ny*ny <= 1) this.clear(x, y);
        }
    }
    stroke(x0, y0, x1, y1, w) {
      const dx = x1-x0, dy = y1-y0;
      const len = Math.hypot(dx, dy);
      const steps = Math.max(1, Math.ceil(len*2));
      const r = w/2;
      for (let i = 0; i <= steps; i++) {
        const t = i/steps;
        const px = x0 + dx*t, py = y0 + dy*t;
        for (let oy = -Math.ceil(r); oy <= Math.ceil(r); oy++)
          for (let ox = -Math.ceil(r); ox <= Math.ceil(r); ox++)
            if (ox*ox + oy*oy <= r*r) this.set(px+ox, py+oy);
      }
    }
    ringAt(cx, cy, radius, width, sweep, spin) {
      const s = Math.min(1, Math.max(0, sweep));
      const from = -TAU/4 + spin*TAU;
      this.ringBand(cx, cy, radius, width, from, from + s*TAU);
    }
    ringBand(cx, cy, radius, width, a0, a1) {
      const steps = Math.max(24, Math.ceil(radius*TAU*2));
      for (let i = 0; i <= steps; i++) {
        const a = a0 + ((a1-a0)*i)/steps;
        for (let r = radius - width/2; r <= radius + width/2; r += 0.5) this.set(cx + Math.cos(a)*r, cy + Math.sin(a)*r);
      }
    }
  }

  function cell(g, cx, cy, w, h, round, amp, freq, phase) {
    if (w <= 0 || h <= 0) return;
    const r = Math.min(1, Math.max(0, round)) * Math.min(w, h);
    const flat = Math.max(0, w - r);
    const steps = Math.max(1, Math.ceil(w));
    for (let dx = -steps; dx <= steps; dx++) {
      const x = dx;
      if (Math.abs(x) > w) continue;
      let half;
      if (Math.abs(x) <= flat) half = h;
      else {
        const over = (Math.abs(x) - flat) / Math.max(r, 0.001);
        half = h * Math.sqrt(Math.max(0, 1 - over*over));
      }
      const dy = amp > 0 ? amp * Math.sin(TAU * (freq * (x/(2*w)) - phase)) : 0;
      g.span(cx + x, cy + dy - half, cy + dy + half);
    }
  }

  function draw(pose, motion, phase, t, grid) {
    const g = grid ?? new Grid();
    g.dots.fill(0);
    const p = applyMotion(motion, pose, phase, t);
    const cx = g.w/2 + p.x;
    const cy = g.h/2 + p.y;
    g.useInk(p.ink);
    if (p.ring > 1 && p.ringW > 0.1) g.ringAt(cx, cy, p.ring, p.ringW, p.sweep, p.spin);
    if (motion === 'blades') {
      for (let k = 0; k < 3; k++) {
        const a = phase*TAU + (k*TAU)/3;
        g.ringBand(cx, cy, Math.max(4, p.ring - 5), 2, a, a + 0.7);
      }
    }
    const centres = p.gap < 0.6
      ? [[cx, cy, 1]]
      : [[cx - p.gap, cy, 1], [cx + p.gap, cy + p.tilt, p.squint]];
    for (const [ex, ey, hscale] of centres) {
      const h = Math.max(0.5, p.h * hscale);
      if (p.w > 0.05 && h > 0.05) {
        cell(g, ex, ey, p.w, h, p.round, p.amp, p.freq, phase);
        if (p.pupil > 0.3 && p.amp === 0) g.clearEllipse(ex + p.gazeX, ey + p.gazeY, p.pupil, Math.max(0.6, p.pupil*hscale));
      }
    }
    p.mark.forEach((m, i) => {
      if (m.w > 0.05) {
        g.useInk(p.markInk[i] ?? 'plain');
        g.stroke(cx + m.x0, cy + m.y0, cx + m.x1, cy + m.y1, m.w);
      }
    });
    return g;
  }

  const LOOP_SECS = 6;
  const TRANSITION_MS = 550;
  const TRAVEL_MS = 1200;

  // Where the face sits in each view, as fractions of the stage it lives in —
  // the placement half of the split (views.js), written so one canvas can hold
  // every view and the travel between them. `wf`/`hf` are the *box* fractions
  // views.js passes to faceBox, so the fit below binds on whichever axis runs
  // out first, exactly as faceBox does.
  const PLACEMENTS = {
    default: { cx: 0.5, cy: 0.3, wf: 0.62, hf: 0.58 },
    session: { cx: 0.165, cy: 0.5, wf: 0.28, hf: 0.52 },
    deep: { cx: 0.081, cy: 0.5, wf: 0.109, hf: 0.169 },
  };

  const IDENT = { dx: 0, dy: 0, scale: 1 };

  /// Screen geometry for a placement: dot pitch and the grid's origin, in
  /// canvas pixels. Two of these are all a swarm needs — every dot has a real
  /// start and a real destination on the page, so nothing is a box being moved.
  function geomFor(p, c) {
    const pitch = Math.min((p.wf * c.width) / 46, (p.hf * c.height) / DOTS_H);
    return { pitch, ox: p.cx * c.width - (DOTS_W / 2) * pitch, oy: p.cy * c.height - (DOTS_H / 2) * pitch };
  }

  // How a travel flows. The shape of the crossing is the *same* every time —
  // dots collapse onto the path, run it, and reform — so nothing about it
  // depends on which face is on at the time.
  //
  //   stag   fraction of the travel spent launching (bigger = longer river)
  //   comp   how hard the face's own spread is squeezed onto the path
  //          (0.12 = a narrow ribbon; 1 would be no squeeze at all)
  //   arc    how far the run bows off the straight line, as a fraction of the
  //          path length — one shared curve, so it reads as a trajectory
  //   wob    perpendicular braid while running, in dot widths
  //   speed  per-dot speed variance along the run
  //   shrink how much smaller a dot gets while it is in the stream
  const SWARMS = {
    river: { ms: 780, stag: 0.5, comp: 0.12, arc: 0.2, wob: 1.5, trails: 2, shrink: 0.18, speed: 0.12 },
    grain: { ms: 1200, stag: 0.64, comp: 0.05, arc: 0.1, wob: 0.5, trails: 4, shrink: 0.34, speed: 0.3 },
  };

  // Per-dot cameras. None of these is a pose: (dot position, time) -> screen
  // position is placement, so parkerd never learns any of it happened.
  const LENSES = {
    none: () => IDENT,
    scatter(x, y, k, dir) {
      const a = hash01(y * 128 + x, 0x5ca7) * TAU;
      const d = (0.35 + hash01(y * 128 + x, 0x11e5) * 0.65) * k * 13;
      return { dx: Math.cos(a) * d + dir * k * 4, dy: Math.sin(a) * d * 0.42, scale: 1 - k * 0.4 };
    },
    vortex(x, y, k) {
      const cx = x - 48;
      const cy = (y - 20) * 2.4;
      const r = Math.hypot(cx, cy);
      if (r < 0.001) return IDENT;
      const a = Math.atan2(cy, cx) + (r / 40) * k * 2.2;
      return { dx: Math.cos(a) * r - cx, dy: (Math.sin(a) * r - cy) / 2.4, scale: 1 - k * 0.1 };
    },
    stream(x, y, k, dir) {
      const j = hash01(y * 128 + x, 0x51ea);
      return { dx: dir * (5 + j * 20) * k, dy: (j - 0.5) * k * 1.4, scale: 1 - k * 0.2 };
    },
    collapse(x, y, k) {
      return { dx: (x - 48) * k * 0.12, dy: -(y - 20) * k * 0.96, scale: 1 - k * 0.25 };
    },
  };

  class ParkerFace extends HTMLElement {
    static get observedAttributes() { return ['expression', 'stage']; }
    connectedCallback() {
      if (this.canvas) return;
      this.style.display = 'block';
      this.canvas = document.createElement('canvas');
      this.canvas.style.cssText = 'width:100%;height:100%;display:block';
      this.appendChild(this.canvas);
      this.grid = new Grid();
      this.target = this.getAttribute('expression') || 'idle';
      if (!EXPRESSIONS[this.target]) this.target = 'idle';
      this.fromPose = EXPRESSIONS[this.target].pose;
      this.transitionStart = -9999;
      this.stageFrom = PLACEMENTS[this.getAttribute('stage')] || null;
      this.stageTo = this.stageFrom;
      this.travelStart = -9999;
      this.travelDir = -1;
      this.loop = this.loop.bind(this);
      this.raf = requestAnimationFrame(this.loop);
    }
    disconnectedCallback() { cancelAnimationFrame(this.raf); this.raf = 0; }
    attributeChangedCallback(name, _old, val) {
      if (!this.canvas) return;
      if (name === 'stage') {
        const next = PLACEMENTS[val];
        if (!next || next === this.stageTo) return;
        // No current placement means this is the first landing: parker is
        // already here when the view opens, he does not fly in from nowhere.
        const from = this.currentPlacement(performance.now()) || next;
        this.stageFrom = from;
        this.stageTo = next;
        this.travelDir = Math.sign(next.cx - from.cx) || -1;
        this.travelStart = from === next ? -9999 : performance.now();
        return;
      }
      if (val === this.target || !EXPRESSIONS[val]) return;
      this.fromPose = this.currentPose(performance.now());
      this.target = val;
      this.transitionStart = performance.now();
      this.switchStart = performance.now();
    }
    /// How far through the current travel, 0..1. Each travel style sets its
    /// own duration — a river that reads well is quicker than a linear ease.
    travelT(now) {
      const swarm = SWARMS[this.getAttribute('travel')];
      const ms = swarm ? swarm.ms : TRAVEL_MS;
      return Math.min(1, Math.max(0, (now - this.travelStart) / ms));
    }
    currentPlacement(now) {
      const a = this.stageFrom;
      const b = this.stageTo;
      if (!a || !b) return b || a;
      const t = ease.inOut(this.travelT(now));
      return {
        cx: a.cx + (b.cx - a.cx) * t,
        cy: a.cy + (b.cy - a.cy) * t,
        wf: a.wf + (b.wf - a.wf) * t,
        hf: a.hf + (b.hf - a.hf) * t,
      };
    }
    currentPose(now) {
      const spec = EXPRESSIONS[this.target];
      const t = Math.min(1, (now - this.transitionStart) / TRANSITION_MS);
      const eased = VOLUNTARY.includes(this.target) ? ease.backOut(t) : ease.inOut(t);
      return t >= 1 ? spec.pose : lerp(this.fromPose, spec.pose, eased);
    }
    palette() {
      const s = getComputedStyle(this);
      const v = (n, d) => s.getPropertyValue(n).trim() || d;
      return ['transparent', v('--face-fg', '#c9c7cd'), v('--accent', '#7aa2f7'), v('--good', '#9ece6a'), v('--bad', '#f7768e')];
    }
    loop(now) {
      this.raf = requestAnimationFrame(this.loop);
      const c = this.canvas;
      const rect = c.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      if (c.width !== Math.round(rect.width*dpr) || c.height !== Math.round(rect.height*dpr)) {
        c.width = Math.round(rect.width*dpr);
        c.height = Math.round(rect.height*dpr);
      }
      const ctx = c.getContext('2d');
      const secs = now/1000;
      const phase = (secs/LOOP_SECS) % 1;
      let pose = this.currentPose(now);
      const motion = EXPRESSIONS[this.target].motion;
      if (blinkable(pose)) pose = { ...pose, h: pose.h * blink(secs) };
      draw(pose, motion, phase, secs, this.grid);

      const style = this.getAttribute('travel') || 'none';
      const tt = this.stageTo && this.travelStart > 0 ? this.travelT(now) : 1;
      const palette = this.palette();
      const dots = this.grid.dots;
      ctx.clearRect(0, 0, c.width, c.height);

      // The travel is three phases and the middle one is always the same
      // shape: dots fall onto the path, run it as a narrow stream, then reform
      // at the far end. Nothing here expands, and nothing depends on the pose.
      const swarm = SWARMS[style];
      if (swarm && tt < 1 && this.stageFrom && this.stageTo) {
        const A = geomFor(this.stageFrom, c);
        const B = geomFor(this.stageTo, c);
        const acx = A.ox + (DOTS_W / 2) * A.pitch;
        const acy = A.oy + (DOTS_H / 2) * A.pitch;
        const bcx = B.ox + (DOTS_W / 2) * B.pitch;
        const bcy = B.oy + (DOTS_H / 2) * B.pitch;
        const pathLen = Math.hypot(bcx - acx, bcy - acy);
        if (pathLen > 4) {
          const ux = (bcx - acx) / pathLen;
          const uy = (bcy - acy) / pathLen;
          const vx = -uy;
          const vy = ux;
          const midPitch = (A.pitch + B.pitch) / 2;
          // The run follows one shared curve rather than the chord: a control
          // point pushed off the midpoint, always bowing downward on screen so
          // the stream sags like something being poured.
          const bow = vy < 0 ? -1 : 1;
          const ctlOffx = vx * bow * swarm.arc * pathLen;
          const ctlOffy = vy * bow * swarm.arc * pathLen;

          // Who goes first: measured along the direction of travel, over the
          // dots that are actually lit. Heading left means the left edge
          // dissolves first and runs left — sand off a ledge, not an explosion.
          let minL = Infinity;
          let maxL = -Infinity;
          for (let y = 0; y < DOTS_H; y++) {
            for (let x = 0; x < DOTS_W; x++) {
              if (!dots[y * DOTS_W + x]) continue;
              const off = (x + 0.5 - DOTS_W / 2) * A.pitch * ux + (y + 0.5 - DOTS_H / 2) * A.pitch * uy;
              if (off < minL) minL = off;
              if (off > maxL) maxL = off;
            }
          }
          const range = Math.max(1, maxL - minL);
          const FALL = 0.22;
          const FORM = 0.8;
          const run = 1 - swarm.stag;

          for (let y = 0; y < DOTS_H; y++) {
            for (let x = 0; x < DOTS_W; x++) {
              const ink = dots[y * DOTS_W + x];
              if (!ink) continue;
              const srcx = A.ox + (x + 0.5) * A.pitch;
              const srcy = A.oy + (y + 0.5) * A.pitch;
              const dstx = B.ox + (x + 0.5) * B.pitch;
              const dsty = B.oy + (y + 0.5) * B.pitch;
              const off = (srcx - acx) * ux + (srcy - acy) * uy;
              const s = Math.min(1, Math.max(0, (maxL - off) / range));
              const jit = hash01(y * 128 + x, 0x9a17);
              const depart = (s * 0.86 + jit * 0.14) * swarm.stag;

              // Where this dot sits in the stream: its own spread squeezed
              // hard onto the path, at each end of the run.
              const sou = off;
              const sov = (srcx - acx) * vx + (srcy - acy) * vy;
              const dou = (dstx - bcx) * ux + (dsty - bcy) * uy;
              const dov = (dstx - bcx) * vx + (dsty - bcy) * vy;
              const entx = acx + ux * sou * 0.35 + vx * sov * swarm.comp;
              const enty = acy + uy * sou * 0.35 + vy * sov * swarm.comp;
              const exx = bcx + ux * dou * 0.35 + vx * dov * swarm.comp;
              const exy = bcy + uy * dou * 0.35 + vy * dov * swarm.comp;

              for (let sm = swarm.trails; sm >= 0; sm--) {
                let w = (tt - depart - sm * 0.026) / run;
                if (sm > 0 && (w <= FALL || w >= FORM)) continue;
                // Before its turn a dot simply stays where it is: the far edge
                // is still a whole face while the front is already running.
                w = Math.min(1, Math.max(0, w));
                let px;
                let py;
                let pitch;
                let thin;
                if (w < FALL) {
                  const e = ease.inOut(w / FALL);
                  px = srcx + (entx - srcx) * e;
                  py = srcy + (enty - srcy) * e;
                  pitch = A.pitch;
                  thin = e;
                } else if (w < FORM) {
                  let q = (w - FALL) / (FORM - FALL);
                  q = Math.min(1, Math.max(0, q * (1 + (jit - 0.5) * swarm.speed)));
                  // Quadratic through the shared control point. Each dot's own
                  // bow varies a little, which gives the ribbon body without
                  // letting it widen.
                  const k = 0.82 + jit * 0.36;
                  const cx2 = (entx + exx) / 2 + ctlOffx * k;
                  const cy2 = (enty + exy) / 2 + ctlOffy * k;
                  const m = 1 - q;
                  const braid = Math.sin(s * 7 + secs * 8 + jit * TAU) * swarm.wob * midPitch * Math.sin(Math.PI * q);
                  px = m * m * entx + 2 * m * q * cx2 + q * q * exx + vx * braid;
                  py = m * m * enty + 2 * m * q * cy2 + q * q * exy + vy * braid;
                  pitch = A.pitch + (B.pitch - A.pitch) * q;
                  thin = 1;
                } else {
                  const e = ease.inOut((w - FORM) / (1 - FORM));
                  px = exx + (dstx - exx) * e;
                  py = exy + (dsty - exy) * e;
                  pitch = B.pitch;
                  thin = 1 - e;
                }
                const rr = pitch * 0.34 * (1 - thin * swarm.shrink);
                if (rr <= 0.08) continue;
                ctx.globalAlpha = sm === 0 ? 1 : 0.18 * (1 - sm / (swarm.trails + 1));
                ctx.fillStyle = palette[ink];
                ctx.beginPath();
                ctx.arc(px, py, rr, 0, TAU);
                ctx.fill();
              }
            }
          }
          ctx.globalAlpha = 1;
          return;
        }
      }

      let pitch, ox, oy;
      if (this.stageTo) {
        const g = geomFor(this.currentPlacement(now), c);
        pitch = g.pitch;
        ox = g.ox;
        oy = g.oy;
      } else {
        // Fit: the padded grid (faithful to Stage.svelte) or just the lit middle.
        const seen = this.getAttribute('fit') === 'seen';
        const padW = seen ? 50 : DOTS_W + 4;
        const padH = seen ? 44 : DOTS_H + 6;
        pitch = Math.min(c.width/padW, c.height/padH);
        ox = (c.width - pitch*DOTS_W)/2;
        oy = (c.height - pitch*DOTS_H)/2;
      }
      const r = pitch * 0.34;

      let k = tt < 1 ? Math.sin(Math.PI * tt) : 0;
      let lens = k > 0.004 ? (LENSES[style] || LENSES.none) : LENSES.none;
      let dir = this.travelDir;

      // A pose change can have its own small burst — the state switch, not the
      // travel. Whichever is louder owns the dots.
      const sw = this.getAttribute('switch');
      if (sw && sw !== 'none' && this.switchStart) {
        const st = (now - this.switchStart) / 420;
        if (st >= 0 && st < 1) {
          const ks = Math.sin(Math.PI * st) * 0.6;
          if (ks > k) { k = ks; lens = LENSES[sw] || LENSES.none; dir = 1; }
        }
      }
      const echoes = k > 0.02 && (style === 'stream' || style === 'scatter' || sw === 'scatter') ? 3 : 0;

      const paint = (alpha, back) => {
        ctx.globalAlpha = alpha;
        for (let y = 0; y < DOTS_H; y++) {
          for (let x = 0; x < DOTS_W; x++) {
            const ink = dots[y*DOTS_W + x];
            if (!ink) continue;
            const d = lens(x, y, k * (1 - back * 0.28), dir);
            const rr = r * d.scale;
            if (rr <= 0.08) continue;
            ctx.fillStyle = palette[ink];
            ctx.beginPath();
            ctx.arc(ox + (x + 0.5 + d.dx)*pitch, oy + (y + 0.5 + d.dy)*pitch, rr, 0, TAU);
            ctx.fill();
          }
        }
        ctx.globalAlpha = 1;
      };
      for (let i = echoes; i >= 1; i--) paint(0.06 + 0.14 * (1 - i / (echoes + 1)), i);
      paint(1, 0);
    }
  }

  if (!window.customElements.get('parker-face')) window.customElements.define('parker-face', ParkerFace);
  window.PARKER_EXPRESSIONS = EXPRESSIONS;
})();
