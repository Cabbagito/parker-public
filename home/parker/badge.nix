# The badge — the pump LCD as parker's heart, in the face's design
# language: every frame is a 30x30 grid of square cells (little pixels,
# like parker's idle animation), scenes are vector fields drawn thick —
# thin lines vanish on the small glass — and sampled into cells. Five
# scenes in a seamless 200-frame loop — atom orbits, the thinking spin, a
# circular waveform (polar, so it can never hit the display wall), a
# rotating wireframe pyramid, binary rain — dissolving into each other,
# inside a tick ring that never dissolves.
#
# The palette is FIXED, COLD, and deliberately not the theme's (July
# 2026), arrived at in three steps, each one a thing seen on the glass:
#
#   1. The glass is small, dark and tinted. It flatters exactly one look
#      and the softer schemes went muddy behind it, so the badge stopped
#      following the theme.
#   2. Then the badge was redline red — and disappeared. The case is
#      white, the fans and the RAM and the GPU are all red, and a red
#      badge inside a red-lit case is camouflage. **The badge is the one
#      thing in this case that must not match the lighting.** So it went
#      cold: cyan, the direct complement of red, the most separated a hue
#      can be from everything else in there — and the right story for a
#      pump block besides.
#   3. Then cyan led, and white still won on visibility. So the roles
#      swapped: **white is the subject, the hue is what falls away from
#      it.** No bias term does this — the ramp's own knee does. A stroke's
#      solid cells land in the white plateau (>= 0.88) while the partial
#      cells along its edges fall through ice into cyan, so every shape
#      gets a cyan fringe for free. The two 3D scenes then shade by actual
#      depth — the far half of each atom orbit and the back edges of the
#      tetrahedron are drawn dimmer, which the ramp turns into cyan
#      behind, white in front.
#
# The rules that hold it together:
#   - the field is true black — the circular glass covers a square panel,
#     so anything not pure black at the edges shows the panel's corners
#   - one hue at full saturation; two hues on 30x30 dots read as noise
#   - intensity IS temperature, so the brightest thing on the glass is
#     always white and always the thing that is moving or nearest. This
#     is a CONTRAST rule, not a brightness rule: pushing the whole ramp up
#     just fuses the dots into a blob and throws the hue away (tried, July
#     2026 — dot radius must stay under ~4.1 at PITCH 8 or the grid stops
#     reading as a grid). Brightness is bought by lifting the floor of the
#     dim majority, never by flattening the range.
#   - the ring is a frame, not a subject: it rests in the hue band and
#     only its comet head is allowed into the white.
#
# `mono` (steel white, the safest — never competes with anything) and
# `acid` (lime) are kept as alternates; PALETTE picks one. Case lighting
# (home/theming/halo.nix) still follows the theme; the badge does not.
#
# Composition: content lives inside r=86, the ring owns 94..102, and
# nothing is drawn past r=112 — the depth illusion dies the moment a pixel
# reaches a corner.
#
# What drives it: Kraken 2023 firmware loops nothing itself (no GIF on
# fw 2.x — PARKER.md), so permanence is this service streaming baked
# frames to CoolerControl forever at ~5.6 fps (wire-verified July 2026:
# 60 back-to-back PUTs, zero failures, 0.14–0.19s each — the floor is
# CC/device apply time, not TLS; connection reuse bought only 6%, so
# the loop stays one curl per frame and lets PUT latency pace it).
# The cc_ write token is minted in the CC UI (Access Protection), not by
# anything here, so it is dropped by hand into
# $XDG_STATE_HOME/parker/coolercontrol.token and never kept in the repo.
# The loop rereads it every pass: a re-minted token is a file edit, not a
# restart, and a missing one parks the loop instead of killing it.
# State-reactive variants (calm, busy, alarmed) are parkerd's to conduct
# later.
{ pkgs, ... }:
let
  render = pkgs.writeText "badge-render.py" ''
    import math, sys
    from PIL import Image, ImageDraw

    outdir = sys.argv[1]

    # ---- palette: fixed, cold, theme-independent -------------------------
    # PALETTE names the ramp. See the header for why it is cold and why it is
    # not the theme's; swapping the name here is the whole knob. Every ramp
    # has the same shape: a black floor, a saturated middle, and a WHITE
    # PLATEAU from 0.88 up — see the header on why white is the subject.
    PALETTE = "ice"

    BLACK = (0, 0, 0)
    RAMPS = {
        "ice": [
            (0.00, (0, 0, 0)),
            (0.18, (0, 40, 80)),
            (0.38, (0, 140, 215)),
            (0.58, (90, 220, 255)),
            (0.74, (200, 245, 255)),
            (0.88, (255, 255, 255)),
            (1.00, (255, 255, 255)),
        ],
        "mono": [
            (0.00, (0, 0, 0)),
            (0.18, (18, 22, 30)),
            (0.38, (70, 84, 102)),
            (0.58, (140, 158, 178)),
            (0.74, (205, 218, 232)),
            (0.88, (255, 255, 255)),
            (1.00, (255, 255, 255)),
        ],
        "acid": [
            (0.00, (0, 0, 0)),
            (0.18, (6, 46, 4)),
            (0.38, (48, 160, 26)),
            (0.58, (130, 235, 55)),
            (0.74, (205, 252, 145)),
            (0.88, (255, 255, 255)),
            (1.00, (255, 255, 255)),
        ],
    }
    RAMP = RAMPS[PALETTE]

    def heat(i):
        i = max(0.0, min(1.0, i))
        for k in range(len(RAMP) - 1):
            t0, c0 = RAMP[k]
            t1, c1 = RAMP[k + 1]
            if i <= t1:
                f = (i - t0) / (t1 - t0)
                return tuple(int(c0[j] + (c1[j] - c0[j]) * f) for j in range(3))
        return RAMP[-1][1]

    N = 200           # total frames
    SCENE = 40        # frames per scene
    GRID = 30         # dots per side
    PITCH = 8
    R_MAX = 112       # circle containment
    R_FEATHER = 104   # dots dim smoothly from here to R_MAX
    R_RIM = 100       # the always-on tick ring

    def clamp(v):
        return max(0.0, min(1.0, v))

    def layers():
        p = Image.new("L", (240, 240), 0)
        a = Image.new("L", (240, 240), 0)
        return p, ImageDraw.Draw(p), a, ImageDraw.Draw(a)

    def scene_atom(t):
        # The orbits are drawn segment by segment so the half of each ring
        # swinging away from the viewer is dimmer than the half swinging
        # toward it — which the ramp renders as cyan behind, white in front.
        # This is the depth the flat ellipses never had.
        p, dp, a, da = layers()
        for rot in (0, 60, 120):
            c, s = math.cos(math.radians(rot)), math.sin(math.radians(rot))
            def at(th):
                x, y = 88 * math.cos(th), 30 * math.sin(th)
                return (120 + x * c - y * s, 120 + x * s + y * c)
            for i in range(72):
                th0, th1 = math.radians(i * 5), math.radians((i + 1) * 5)
                near = clamp(0.5 + 0.5 * math.sin((th0 + th1) / 2))
                dp.line([at(th0), at(th1)], fill=int(110 + 145 * near), width=12)
            th = 2 * math.pi * (t + rot / 360)
            near = clamp(0.5 + 0.5 * math.sin(th))
            ex, ey = at(th)
            er = 9 + 4 * near
            da.ellipse([ex - er, ey - er, ex + er, ey + er],
                       fill=int(150 + 105 * near))
        for i in range(3):
            th = 2 * math.pi * (t + i / 3)
            nx, ny = 120 + 10 * math.cos(th), 120 + 10 * math.sin(th)
            da.ellipse([nx - 10, ny - 10, nx + 10, ny + 10], fill=255)
        return p, a

    def scene_spin(t):
        p, dp, a, da = layers()
        rot = t * 360
        for i in range(3):
            s = rot + i * 120
            da.arc([120 - 84, 120 - 84, 120 + 84, 120 + 84],
                   start=s, end=s + 74, fill=255, width=17)
        for i in range(4):
            s = -2 * rot + i * 90
            dp.arc([120 - 56, 120 - 56, 120 + 56, 120 + 56],
                   start=s, end=s + 42, fill=225, width=13)
        pulse = 0.5 + 0.5 * math.sin(t * 2 * math.pi * 2)
        r = 16 + 8 * pulse
        da.ellipse([120 - r, 120 - r, 120 + r, 120 + r], fill=255)
        return p, a

    def scene_polarwave(t):
        p, dp, a, da = layers()
        pts = []
        for i in range(181):
            th = math.radians(i * 2)
            w = math.sin(8 * th + t * 4 * math.pi) * (0.65 + 0.35 * math.sin(3 * th - t * 2 * math.pi))
            r = 72 + 15 * w
            pts.append((120 + r * math.cos(th), 120 + r * math.sin(th)))
        da.line(pts + [pts[0]], fill=255, width=14)
        pts = []
        for i in range(181):
            th = math.radians(i * 2)
            r = 40 + 8 * math.sin(8 * th - t * 4 * math.pi)
            pts.append((120 + r * math.cos(th), 120 + r * math.sin(th)))
        dp.line(pts + [pts[0]], fill=215, width=11)
        pulse = 0.5 + 0.5 * math.sin(t * 4 * math.pi)
        r = 9 + 6 * pulse
        da.ellipse([120 - r, 120 - r, 120 + r, 120 + r], fill=255)
        return p, a

    def scene_pyramid(t):
        p, dp, a, da = layers()
        v = [(1, 1, 1), (1, -1, -1), (-1, 1, -1), (-1, -1, 1)]
        ay = t * 2 * math.pi
        ax = 0.45 * math.sin(t * 2 * math.pi)
        proj, depth = [], []
        for x, y, z in v:
            x, z = x * math.cos(ay) + z * math.sin(ay), -x * math.sin(ay) + z * math.cos(ay)
            y, z = y * math.cos(ax) - z * math.sin(ax), y * math.sin(ax) + z * math.cos(ax)
            s = 60 * (1.6 / (1.6 + z * 0.5))
            proj.append((120 + x * s, 120 + y * s))
            depth.append(z)
        for i, j in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]:
            # near edges white, far edges cyan — same trick as the orbits
            near = clamp(0.5 - (depth[i] + depth[j]) / 3)
            dp.line([proj[i], proj[j]], fill=int(105 + 150 * near), width=13)
        for n, (x, y) in enumerate(proj):
            near = clamp(0.5 - depth[n] / 1.5)
            r = 9 + 4 * near
            da.ellipse([x - r, y - r, x + r, y + r], fill=int(155 + 100 * near))
        return p, a

    GLYPH0 = ["111", "101", "101", "101", "111"]
    GLYPH1 = ["010", "110", "010", "010", "111"]

    def scene_rain(t):
        cols = []
        for c in range(7):
            gx = 1 + c * 4
            speed = 6 + (c * 7) % 5
            off = (c * 13) % 11
            cols.append((gx, speed, off))
        cells = {}
        for gx, speed, off in cols:
            drop = t * speed
            for k in range(6):
                gy = int((drop + k * 6 + off)) % 36 - 5
                bit = (gx * 31 + k * 17 + off) % 2
                glyph = GLYPH1 if bit else GLYPH0
                head = k == 0
                for row in range(5):
                    for col in range(3):
                        if glyph[row][col] == "1":
                            cells[(gx + col, gy + row)] = 255 if head else 150
        return cells

    SCENES = [scene_atom, scene_spin, scene_polarwave, scene_pyramid, scene_rain]

    def envelope(lt):
        # Floors at 0.32: a scene dissolves into embers, it never blinks to black.
        e = 1.0
        if lt < 4: e = 0.32 + 0.68 * (lt / 4)
        if lt > SCENE - 5: e = 0.32 + 0.68 * ((SCENE - 1 - lt) / 4)
        return max(0.32, min(1.0, e))

    def ring_cells():
        # Built in cell space, not drawn and sampled: one dot wide, evenly
        # spaced, no aliasing. Ordered by angle so a head can sweep it.
        out = []
        for cy in range(GRID):
            for cx in range(GRID):
                dx, dy = cx - (GRID - 1) / 2, cy - (GRID - 1) / 2
                d = math.hypot(dx, dy)
                if 11.7 <= d <= 12.7:
                    out.append((math.atan2(dy, dx) % (2 * math.pi), cx, cy))
        out.sort()
        return [(cx, cy) for _, cx, cy in out]

    RING = ring_cells()

    def rim_cells(k):
        # The one thing that never dissolves: the outer ring, with a bright
        # comet head sweeping it once per scene. Owns the widest band of the
        # disc and carries a pulse across the scene transitions.
        n = len(RING)
        head = (k % SCENE) / SCENE * n
        tail = n / 5
        out = {}
        for i, cell in enumerate(RING):
            d = (head - i) % n
            v = 0.38 if d > tail else 0.38 + 0.62 * (1 - d / tail) ** 1.6
            out[cell] = v
        return out

    def rim(x, y):
        d = math.hypot(x - 120, y - 120)
        if d > R_MAX:
            return 0.0
        if d <= R_FEATHER:
            return 1.0
        return 1.0 - (d - R_FEATHER) / (R_MAX - R_FEATHER)

    def cells_for(k):
        """(cx, cy) -> (intensity 0..1, hot 0..1). hot biases the heat ramp up."""
        s = (k // SCENE) % len(SCENES)
        lt = k % SCENE
        t = lt / SCENE
        env = envelope(lt)
        fn = SCENES[s]
        out = {}
        def put(cx, cy, i, hot):
            if not (0 <= cx < GRID and 0 <= cy < GRID):
                return
            prev = out.get((cx, cy))
            if prev is None or i > prev[0]:
                out[(cx, cy)] = (i, hot)
        if fn is scene_rain:
            for (cx, cy), val in fn(t).items():
                put(cx, cy, (val / 255) * env, 1.0 if val == 255 else 0.0)
        else:
            plain, accent_l = fn(t)
            sp = plain.resize((GRID, GRID), Image.BOX)
            sa = accent_l.resize((GRID, GRID), Image.BOX)
            for cy in range(GRID):
                for cx in range(GRID):
                    ip = sp.getpixel((cx, cy)) / 255 * env
                    ia = sa.getpixel((cx, cy)) / 255 * env
                    put(cx, cy, max(ip, ia), 1.0 if ia > ip else 0.0)
        for (cx, cy), v in rim_cells(k).items():
            put(cx, cy, v, 1.0 if v > 0.7 else 0.0)
        return out

    def render(k, path):
        img = Image.new("RGB", (240, 240), BLACK)
        d = ImageDraw.Draw(img)
        for (cx, cy), (i, hot) in cells_for(k).items():
            x, y = cx * PITCH + PITCH // 2, cy * PITCH + PITCH // 2
            f = rim(x, y)
            if f <= 0: continue
            i *= f
            if i < 0.08: continue
            r = 1.8 + 2.3 * (i ** 0.5)
            # energy runs white, structure runs red
            temp = 0.62 + 0.38 * i if hot else 0.30 + 0.56 * i
            d.rectangle([x - r, y - r, x + r, y + r], fill=heat(temp))
        img.save(path)

    for k in range(N):
        render(k, f"{outdir}/b{k:03d}.png")
  '';

  frames = pkgs.runCommand "badge-frames" {
    nativeBuildInputs = [ (pkgs.python3.withPackages (p: [ p.pillow ])) ];
  } ''
    mkdir -p $out
    python3 ${render} $out
  '';

  # Device UID and endpoint shape: PARKER.md, wire-verified. The channel
  # name doubles the lcd segment; log=false exists for exactly this loop.
  krakenUid = "b308185de6658c811067b6e74271d1607b5340c8e2f9bdb93d3ca518c403796b";

  loop = pkgs.writeShellScript "badge-loop" ''
    url="https://127.0.0.1:11987/devices/${krakenUid}/settings/lcd/lcd/images?log=false"
    tokenFile="''${XDG_STATE_HOME:-$HOME/.local/state}/parker/coolercontrol.token"
    # Never exits non-zero: if CoolerControl is down the pushes no-op and
    # the badge holds its last frame until the daemon returns. No sleep in
    # the hot path — the ~0.18s PUT latency is the metronome; -f routes
    # HTTP errors (not just connection failures) to the backoff so the
    # loop can never spin faster than the wire.
    while :; do
      token=$(cat "$tokenFile" 2>/dev/null) || token=
      if [ -z "$token" ]; then
        sleep 30
        continue
      fi
      for f in ${frames}/b*.png; do
        ${pkgs.curl}/bin/curl -skf -m 10 -X PUT \
          -H "Authorization: Bearer $token" \
          -F mode=image -F brightness=100 \
          -F "images[]=@$f;type=image/png" \
          "$url" >/dev/null 2>&1 || sleep 5
      done
    done
  '';
in
{
  systemd.user.services.badge = {
    Unit.Description = "The badge — dot-matrix loop on the Kraken pump LCD";
    Service = {
      ExecStart = "${loop}";
      Restart = "always";
      RestartSec = 30;
    };
    Install.WantedBy = [ "default.target" ];
  };
}
