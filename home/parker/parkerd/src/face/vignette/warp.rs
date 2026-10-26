// warp — parker goes to lightspeed for a bit, somewhere in Culture space.
//
// He shrinks into the distance, ring and all, and six streaks pour past him
// from the centre out, each launched on its own clock and bearing, so the
// field never visibly repeats. A lurch halfway as he jumps up a gear, then
// he drops out: the streaks snap away, he arrives full size with a jolt, and
// has a look at where he has ended up.
//
// The streaks accelerate, which straight-line keyframes cannot, so this one
// is sampled: a keyframe every 50 ms while he is at speed.

use super::super::pose::hash01;
use super::*;
use Easing::*;
use Field::*;

const DT: f64 = 0.05;
const PERIOD: f64 = 0.55;

fn far() -> Look {
    set(&[(W, 3.2), (H, 3.2), (Ring, 6.0), (RingW, 1.2), (Pupil, 1.1)])
}

/// The six streaks at time `t`. Streak `i` launches every period (offset per
/// streak) on a bearing hashed per launch, accelerating out and stretching;
/// it fades in and out at the ends of its run, so the jump back to the
/// centre for the next launch is never seen.
fn streaks(t: f64, gear: f64) -> (Vec<Stroke>, Vec<Ink>) {
    let tau = std::f64::consts::TAU;
    let per = PERIOD / gear;
    let mut marks = vec![];
    let mut inks = vec![];
    for i in 0..6u64 {
        let local = t + i as f64 * per / 6.0;
        let n = (local / per).floor();
        let u = (local - n * per) / per;
        let a = hash01(n as u64 * 6 + i, 0x3a7) * tau;
        let r0 = 5.0 + u * u * 40.0;
        let r1 = r0 + 2.0 + u * 14.0 * gear;
        let fade = if u < 0.12 {
            u / 0.12
        } else if u > 0.85 {
            (1.0 - u) / 0.15
        } else {
            1.0
        };
        // Gone once its inner end leaves the screen — what is left of it is
        // past the edge anyway — and already gone one sample before, so the
        // straight line between two samples never carries a drawn streak
        // wholly off it.
        let off = |u: f64| {
            let r = 5.0 + u * u * 40.0;
            (a.cos() * r).abs() > 47.5 || (a.sin() * r * 0.55).abs() > 19.5
        };
        let w = if off(u) || off(u + DT / per) {
            0.0
        } else {
            (fade * (0.8 + u * 1.2)).max(0.0)
        };
        marks.push(s(
            a.cos() * r0,
            a.sin() * r0 * 0.55,
            a.cos() * r1,
            a.sin() * r1 * 0.55,
            w,
        ));
        inks.push(if i % 3 == 0 { Ink::Accent } else { Ink::Plain });
    }
    (marks, inks)
}

fn cruise(
    sections: &mut Vec<Section>,
    from: f64,
    secs: f64,
    gear: f64,
    eye: &[(Field, Op)],
) -> f64 {
    let steps = (secs / DT).round() as u64;
    let frames = (0..steps)
        .map(|k| {
            let (marks, inks) = streaks(from + k as f64 * DT, gear);
            kf(
                To::num(with(&far(), eye)).mark(marks).inks(inks),
                DT,
                Linear,
            )
        })
        .collect();
    sections.push(still(frames));
    from + secs
}

pub fn beat() -> Beat {
    let mut sections = vec![];
    let wide = set(&[(W, 10.0), (H, 7.5), (Pupil, 2.4)]);
    sections.push(still(vec![
        kf(To::num(wide.clone()), 0.5, InOut),
        hold(To::num(wide), 0.3),
        kf(To::num(far()), 0.7, Out),
    ]));

    let t = cruise(&mut sections, 0.0, 5.0, 1.0, &[]);
    // A lurch: up a gear — he flinches back, and the streaks blink out where
    // they are (rather than jumping to the faster field's) and come back
    // longer.
    let (marks, inks) = streaks(t - DT, 1.0);
    let marks = marks.into_iter().map(|m| Stroke { w: 0.0, ..m }).collect();
    sections.push(still(vec![kf(
        To::num(with(&far(), &set(&[(W, 2.2), (H, 2.2), (Ring, 4.5)])))
            .mark(marks)
            .inks(inks),
        0.12,
        Out,
    )]));
    let faster = set(&[(W, 2.4), (H, 2.4), (Ring, 4.8), (Pupil, 0.9)]);
    let t = cruise(&mut sections, t + 0.12, 6.0, 1.6, &faster);

    // Drop out: the streaks snap to nothing and he arrives with a jolt.
    let dead: Vec<Stroke> = streaks(t, 1.6)
        .0
        .iter()
        .map(|m| s(m.x0 * 0.3, m.y0 * 0.3, m.x0 * 0.3, m.y0 * 0.3, 0.0))
        .collect();
    let arrive = vec![
        (W, Op::Mul(1.25)),
        (H, Op::Mul(1.25)),
        (Ring, Op::Set(18.0)),
        (Pupil, Op::Set(2.2)),
    ];
    sections.push(still(vec![
        kf(
            To::num(with(&far(), &set(&[(W, 2.4), (H, 2.4), (Ring, 4.8)]))).mark(dead),
            0.12,
            Out,
        ),
        kf(To::num(arrive), 0.3, BackOut),
        kf(To::num(set(&[(X, -1.2)])), 0.07, Linear),
        kf(To::num(set(&[(X, 0.8)])), 0.07, Linear),
        kf(To::num(vec![]), 0.3, InOut),
    ]));

    // Where has he ended up? A look round.
    let left = set(&[(GazeX, -3.4), (GazeY, -0.6)]);
    let right = set(&[(GazeX, 3.2), (GazeY, 0.8)]);
    let home = set(&[(GazeX, 0.0), (GazeY, 0.0)]);
    sections.push(blinking(vec![
        kf(To::num(left.clone()), 0.2, Out),
        hold(To::num(left), 0.8),
        kf(To::num(right.clone()), 0.22, Out),
        hold(To::num(right), 0.8),
        kf(To::num(home.clone()), 0.2, Out),
        hold(To::num(home), 0.9),
    ]));

    build("warp", 0.5, sections)
}
