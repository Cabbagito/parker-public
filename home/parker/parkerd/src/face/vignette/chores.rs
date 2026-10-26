// chores — parker dusts the inside of his own porthole.
//
// The eye shrinks to a small iris and holds a feather duster: a handle and
// three feathers, four strokes. Two motes of dust, the last two strokes, puff
// off at every swish. He works his way round the rim, stops to inspect a
// spot, doubles back for one he missed, and every so often the dust gets
// him. It ends on the one spot that actually sparkles.

use super::super::pose::hash01;
use super::*;
use Easing::*;
use Field::*;

fn eye() -> Look {
    set(&[
        (Gap, 0.0),
        (W, 4.6),
        (H, 4.6),
        (Round, 1.0),
        (Ring, 0.0),
        (RingW, 0.0),
        (Pupil, 1.7),
    ])
}

/// A point on the rim at angle `a`, `reach` of the way out.
fn rim(a: f64, reach: f64) -> (f64, f64) {
    (a.cos() * 19.0 * reach, a.sin() * 13.0 * reach)
}

/// Everything on stage at one instant.
#[derive(Clone)]
struct Stage {
    /// Where on the rim the duster is working.
    a: f64,
    /// How far out the tip is: 0 tucked in, 1 on the rim.
    reach: f64,
    /// Feather swish, -1..1.
    sw: f64,
    /// The two motes' progress: 0 at the tip, 1 drifted off and gone.
    m: [f64; 2],
    /// Stroke weight; 0 puts the duster away.
    wd: f64,
    /// Anything the eye does on top of following the tip.
    eye: Look,
}

fn at(a: f64) -> Stage {
    Stage {
        a,
        reach: 1.0,
        sw: 0.0,
        m: [1.0, 1.0],
        wd: 1.7,
        eye: vec![],
    }
}

impl Stage {
    fn reach(mut self, r: f64) -> Stage {
        self.reach = r;
        self
    }
    fn sw(mut self, s: f64) -> Stage {
        self.sw = s;
        self
    }
    fn motes(mut self, a: f64, b: f64) -> Stage {
        self.m = [a, b];
        self
    }
    fn away(mut self) -> Stage {
        self.wd = 0.0;
        self
    }
    fn eye(mut self, e: Look) -> Stage {
        self.eye = e;
        self
    }

    fn to(&self) -> To {
        let tip = rim(self.a, self.reach);
        let grip = rim(self.a, 0.38 * self.reach + 0.12);
        let dir = (tip.1 - grip.1).atan2(tip.0 - grip.0);
        let sw = self.sw * 0.38;
        let mut marks = vec![s(grip.0, grip.1, tip.0, tip.1, self.wd * 0.9)];
        for k in [-0.55, 0.0, 0.55] {
            let b = dir + k + sw;
            marks.push(s(
                tip.0,
                tip.1,
                tip.0 + b.cos() * 5.5,
                tip.1 + b.sin() * 5.5,
                self.wd,
            ));
        }
        for (i, u) in self.m.iter().enumerate() {
            let out = dir + if i == 1 { 0.7 } else { -0.5 };
            let d = 3.0 + u * 7.0;
            let x = tip.0 + out.cos() * d;
            let y = tip.1 + out.sin() * d + u * u * 4.0;
            // Dust settles at the edge of the porthole rather than falling
            // out of it while still drawn.
            let (x, y) = (clamp(x, -45.0, 45.0), clamp(y, -17.0, 17.0));
            let w = if *u >= 1.0 {
                0.0
            } else if *u < 0.1 {
                u * 18.0
            } else {
                1.8 * (1.0 - u)
            };
            marks.push(dot(x, y, w));
        }
        let gaze = set(&[
            (GazeX, clamp(tip.0 / 6.0, -2.6, 2.6)),
            (GazeY, clamp(tip.1 / 6.0, -1.6, 1.6)),
        ]);
        To::num(with(&with(&eye(), &gaze), &self.eye)).mark(marks)
    }
}

fn swishes(frames: &mut Vec<Frame>, a: f64, n: u64) {
    for k in 0..n {
        let u = (k + 1) as f64 / (n + 1) as f64;
        let side = if k % 2 == 1 { -1.0 } else { 1.0 };
        frames.push(kf(at(a).sw(side).motes(u * 0.7, u * 0.5).to(), 0.16, Out));
    }
    frames.push(kf(at(a).motes(0.85, 0.75).to(), 0.35, InOut));
    frames.push(kf(at(a).motes(1.0, 1.0).to(), 0.3, InOut));
}

fn sneeze(a: f64) -> Vec<Frame> {
    let ah = set(&[(H, 6.2), (Pupil, 1.2), (Y, -1.0)]);
    let ahh = set(&[(H, 7.4), (W, 4.2), (Pupil, 1.0), (Y, -2.0)]);
    let choo = set(&[(H, 0.7), (W, 6.4), (Pupil, 0.0), (Y, 2.5), (X, -2.5)]);
    let sniff = set(&[(H, 3.6), (Pupil, 1.4)]);
    vec![
        kf(at(a).reach(0.8).eye(ah.clone()).to(), 0.5, InOut),
        hold(at(a).reach(0.8).eye(ah).to(), 0.4),
        kf(at(a).reach(0.75).eye(ahh.clone()).to(), 0.4, InOut),
        hold(at(a).reach(0.75).eye(ahh).to(), 0.7),
        // CHOO. Everything jolts and the motes blast off.
        kf(
            at(a + 0.25)
                .reach(1.1)
                .sw(1.0)
                .motes(0.6, 0.55)
                .eye(choo.clone())
                .to(),
            0.08,
            Out,
        ),
        hold(
            at(a + 0.25)
                .reach(1.1)
                .sw(1.0)
                .motes(0.75, 0.7)
                .eye(choo)
                .to(),
            0.3,
        ),
        kf(at(a).reach(0.85).eye(set(&[(X, 1.5)])).to(), 0.12, InOut),
        kf(at(a).reach(0.85).eye(set(&[(X, -1.2)])).to(), 0.12, InOut),
        kf(
            at(a).reach(0.85).eye(set(&[(X, 0.6), (H, 3.6)])).to(),
            0.12,
            InOut,
        ),
        hold(at(a).reach(0.85).eye(sniff).to(), 0.9),
        kf(at(a).to(), 0.5, InOut),
    ]
}

const STOPS: u64 = 46;

pub fn beat() -> Beat {
    let mut sections = vec![];
    let mut a = -2.3; // start top-left

    sections.push(still(vec![
        kf(
            To::num(eye()).mark(at(1.0).reach(0.2).away().to().mark.unwrap()),
            0.9,
            InOut,
        ),
        kf(at(1.0).reach(0.5).to(), 0.7, InOut),
        hold(at(1.0).reach(0.5).to(), 0.5),
        kf(at(a).to(), 1.0, InOut),
    ]));

    // Round the rim, clockwise with stops. About two minutes.
    for i in 0..STOPS {
        let r = hash01(i, 0xd057);
        let back = r < 0.12 && i > 2;
        let next = if back { a - 0.55 } else { a + 0.42 + r * 0.3 };
        let mut frames = vec![kf(at(next).to(), 0.55, InOut)];
        a = next;
        swishes(&mut frames, a, 3 + (hash01(i, 0x5115) * 4.0) as u64);
        if r > 0.82 {
            // Inspect: lean in on the spot, hold, then a doubtful extra swish.
            let lean = set(&[
                (W, 5.2),
                (H, 5.2),
                (Pupil, 1.3),
                (X, a.cos() * 2.5),
                (Y, a.sin() * 1.4),
            ]);
            frames.push(kf(at(a).reach(0.9).eye(lean.clone()).to(), 0.5, InOut));
            frames.push(hold(at(a).reach(0.9).eye(lean).to(), 1.3));
            frames.push(kf(at(a).to(), 0.4, InOut));
            swishes(&mut frames, a, 2);
        }
        sections.push(blinking(frames));
        if i == 14 || i == 33 {
            sections.push(still(sneeze(a)));
        }
    }

    // Done: the duster goes, and one spot on the rim catches the light.
    let (sx, sy) = rim(a, 1.0);
    let sparkle = |r: f64, w: f64| -> Vec<Stroke> {
        let mut m = vec![
            s(sx - r, sy, sx + r, sy, w),
            s(sx, sy - r * 0.8, sx, sy + r * 0.8, w),
        ];
        m.extend(none(4));
        m
    };
    let lit = vec![Ink::Accent, Ink::Accent];
    let look = with(&eye(), &set(&[(GazeX, sx / 7.0), (GazeY, sy / 7.0)]));
    let pleased = set(&[
        (Gap, 9.0),
        (W, 7.0),
        (H, 3.0),
        (Round, 0.85),
        (Pupil, 0.0),
        (Y, -1.0),
        (Ring, 0.0),
        (RingW, 0.0),
    ]);
    sections.push(still(vec![
        kf(
            To::num(eye()).mark(at(a).reach(0.2).away().to().mark.unwrap()),
            0.8,
            InOut,
        ),
        kf(
            To::num(look.clone())
                .mark(sparkle(0.5, 0.0))
                .inks(lit.clone()),
            0.4,
            InOut,
        ),
        kf(
            To::num(look.clone())
                .mark(sparkle(3.2, 1.4))
                .inks(lit.clone()),
            0.25,
            BackOut,
        ),
        kf(
            To::num(look.clone())
                .mark(sparkle(1.2, 0.8))
                .inks(lit.clone()),
            0.3,
            InOut,
        ),
        kf(
            To::num(look).mark(sparkle(2.6, 1.2)).inks(lit.clone()),
            0.3,
            BackOut,
        ),
        kf(
            To::num(pleased.clone()).mark(sparkle(0.4, 0.0)).inks(lit),
            0.5,
            InOut,
        ),
        hold(To::num(pleased), 1.4),
        kf(
            To::num(set(&[(Ring, 17.0), (RingW, 2.0), (Sweep, 0.0)])),
            0.5,
            InOut,
        ),
        kf(To::num(vec![]), 1.0, Out),
    ]));

    build("chores", 0.5, sections)
}
