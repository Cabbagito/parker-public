// bonsai — parker grows one, the way the Pi on the shelf does (cbonsai).
//
// He becomes the pot: one flat cell low on the screen with the pupil left in
// it, so the pot is the one watching. A trunk, two branches and three
// cloud-pruned foliage pads (six strokes, the pads in `good`) grow over a few
// minutes, the wind moving them. At the end one pad grows past its shape, he
// eyes it, and snips it back.

use super::super::pose::hash01;
use super::*;
use Easing::*;
use Field::*;

fn pot() -> Look {
    set(&[
        (Gap, 0.0),
        (W, 8.5),
        (H, 3.2),
        (Round, 0.3),
        (Ring, 0.0),
        (RingW, 0.0),
        (Pupil, 1.3),
        (Y, 15.0),
    ])
}

/// One part of the tree: where it runs, how heavy, and the stretch of growth
/// (0 seed → 1 grown) over which it appears. A pad grows from its centre out
/// to `half` either side.
struct Part {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    w: f64,
    from: f64,
    to: f64,
    half: f64,
}

const PARTS: [Part; 6] = [
    Part {
        x0: 0.0,
        y0: -3.0,
        x1: -2.0,
        y1: -26.0,
        w: 2.2,
        from: 0.0,
        to: 0.42,
        half: 0.0,
    },
    Part {
        x0: -1.1,
        y0: -12.0,
        x1: -13.0,
        y1: -17.0,
        w: 1.6,
        from: 0.28,
        to: 0.58,
        half: 0.0,
    },
    Part {
        x0: -1.6,
        y0: -17.0,
        x1: 11.0,
        y1: -21.0,
        w: 1.6,
        from: 0.42,
        to: 0.72,
        half: 0.0,
    },
    Part {
        x0: -14.0,
        y0: -19.0,
        x1: -14.0,
        y1: -19.0,
        w: 3.4,
        from: 0.55,
        to: 0.84,
        half: 5.0,
    },
    Part {
        x0: 11.5,
        y0: -23.0,
        x1: 11.5,
        y1: -23.0,
        w: 3.4,
        from: 0.64,
        to: 0.93,
        half: 5.5,
    },
    Part {
        x0: -2.5,
        y0: -29.0,
        x1: -2.5,
        y1: -29.0,
        w: 3.4,
        from: 0.72,
        to: 1.0,
        half: 5.5,
    },
];

/// The tree at growth `u`, swayed by the wind `wind` (-1..1). `over`: how
/// far the right-hand pad has outgrown its shape (the pruning act).
/// Returns the strokes and the growing tip, which the pot watches.
fn tree(u: f64, wind: f64, over: f64) -> (Vec<Stroke>, (f64, f64)) {
    let mut tip = (0.0, -3.0);
    let marks = PARTS
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let k = clamp((u - p.from) / (p.to - p.from), 0.0, 1.0);
            let w = if k <= 0.0 {
                0.0
            } else {
                p.w * (k * 5.0).min(1.0)
            };
            if k > 0.0 && k < 1.0 {
                tip = (p.x1, p.y1);
            }
            if i < 3 {
                let sway = wind * if i == 0 { 0.6 } else { 1.1 };
                s(
                    p.x0,
                    p.y0,
                    p.x0 + (p.x1 - p.x0) * k + sway,
                    p.y0 + (p.y1 - p.y0) * k,
                    w,
                )
            } else {
                let half = p.half * k;
                let extra = if i == 4 { over } else { 0.0 };
                let dx = wind * 1.4;
                s(p.x0 - half + dx, p.y0, p.x0 + half + extra + dx, p.y0, w)
            }
        })
        .collect();
    (marks, tip)
}

fn scene(u: f64, wind: f64, eye: &[(Field, Op)], over: f64) -> To {
    let (marks, tip) = tree(u, wind, over);
    let gaze = set(&[
        (GazeX, clamp(tip.0 / 5.0, -3.0, 3.0)),
        (GazeY, clamp(-1.0 + tip.1 / 40.0, -1.4, -0.4)),
    ]);
    let inks = vec![
        Ink::Plain,
        Ink::Plain,
        Ink::Plain,
        Ink::Good,
        Ink::Good,
        Ink::Good,
    ];
    To::num(with(&with(&pot(), &gaze), eye))
        .mark(marks)
        .inks(inks)
}

const STEPS: u64 = 58;

pub fn beat() -> Beat {
    let mut sections = vec![];

    let seed_look = set(&[(GazeX, 0.0), (GazeY, -0.8)]);
    sections.push(blinking(vec![
        kf(
            To::num(set(&[
                (W, 6.0),
                (H, 6.0),
                (Ring, 8.0),
                (RingW, 1.5),
                (Pupil, 2.0),
                (Y, 6.0),
            ])),
            1.0,
            InOut,
        ),
        kf(scene(0.0, 0.0, &seed_look, 0.0), 1.4, InOut),
        hold(scene(0.0, 0.0, &seed_look, 0.0), 1.6),
        kf(scene(0.012, 0.0, &set(&[(GazeY, -1.2)]), 0.0), 1.2, InOut),
        hold(scene(0.012, 0.0, &set(&[(GazeY, -1.2)]), 0.0), 1.5),
    ]));

    // Growing: irregular steps, pauses, the wind, the pot glancing round.
    let (mut u, mut wind) = (0.012, 0.0);
    for i in 0..STEPS {
        let r = hash01(i, 0xb0a5);
        let du = if r < 0.25 {
            0.0
        } else {
            (0.6 + r) * (1.0 / STEPS as f64) * 1.05
        };
        u = (u + du).min(1.0);
        let gust = hash01(i, 0x3d1d) * 2.0 - 1.0;
        wind = wind * 0.3 + gust * 0.7;
        let secs = 1.4 + hash01(i, 0x7) * 1.8;
        let mut frames = vec![
            kf(scene(u, wind, &[], 0.0), secs, InOut),
            kf(scene(u, -wind * 0.5, &[], 0.0), 0.9, InOut),
        ];
        if r > 0.9 {
            // He looks away, round the room, and back up at it.
            let away = set(&[(GazeX, -3.6), (GazeY, 0.4)]);
            frames.push(kf(scene(u, 0.0, &away, 0.0), 0.2, Out));
            frames.push(hold(scene(u, 0.0, &away, 0.0), 1.1));
            frames.push(kf(scene(u, 0.0, &[], 0.0), 0.25, Out));
        } else {
            frames.push(hold(scene(u, -wind * 0.5, &[], 0.0), 0.4 + r * 0.8));
        }
        sections.push(blinking(frames));
    }

    // Admire it, content: the pupil closes into a flat, happy squint.
    let content = set(&[(Pupil, 0.0), (H, 1.6)]);
    sections.push(blinking(vec![
        kf(scene(1.0, 0.6, &[], 0.0), 2.2, InOut),
        kf(scene(1.0, -0.5, &[], 0.0), 2.2, InOut),
        kf(scene(1.0, 0.3, &content, 0.0), 0.8, InOut),
        hold(scene(1.0, 0.3, &content, 0.0), 2.5),
        kf(scene(1.0, -0.2, &[], 0.0), 0.9, InOut),
        kf(scene(1.0, 0.4, &[], 0.0), 2.2, InOut),
    ]));

    // One pad keeps going. He notices, narrows, and snips.
    let narrow = set(&[(GazeX, 3.2), (GazeY, -1.3), (H, 2.2), (Pupil, 1.1)]);
    sections.push(blinking(vec![
        kf(scene(1.0, 0.0, &[], 2.5), 3.0, InOut),
        kf(scene(1.0, 0.0, &[], 5.0), 3.0, InOut),
        kf(scene(1.0, 0.0, &narrow, 5.0), 0.3, Out),
        hold(scene(1.0, 0.0, &narrow, 5.0), 1.6),
        kf(scene(1.0, 0.0, &narrow, 0.0), 0.06, Linear),
        kf(scene(1.0, 0.0, &set(&[(Y, 14.4)]), 0.0), 0.12, Out),
        hold(scene(1.0, 0.0, &[], 0.0), 0.8),
        kf(scene(1.0, 0.0, &content, 0.0), 0.6, InOut),
        hold(scene(1.0, 0.0, &content, 0.0), 2.4),
        kf(scene(1.0, 0.5, &[], 0.0), 2.0, InOut),
        kf(scene(1.0, -0.3, &[], 0.0), 2.0, InOut),
    ]));

    build("bonsai", 1.6, sections)
}
