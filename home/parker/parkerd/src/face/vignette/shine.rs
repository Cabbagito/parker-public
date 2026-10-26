// shine — Melee, the Friday-night way.
//
// A hexagon is six strokes, and a hexagon is Fox's shine. It pops in beside
// parker again and again, faster and faster, and he flinches at every one.
// Then he has had enough: the eye narrows and goes red, a ring of fire winds
// up for the longest second in the game, and Captain Falcon connects. The
// shine's six edges become the impact burst. He enjoys that a lot.

use super::*;
use std::f64::consts::PI;
use Easing::*;
use Field::*;

const SPOTS: [(f64, f64); 8] = [
    (15.0, 0.0),
    (13.0, -6.0),
    (17.0, 4.0),
    (14.0, -2.0),
    (16.0, 6.0),
    (12.0, -5.0),
    (15.0, 2.0),
    (14.0, 0.0),
];
/// Where the last shine stands when he answers it.
const AT: (f64, f64) = (15.0, 0.0);

fn eye() -> Look {
    set(&[
        (Gap, 0.0),
        (W, 6.0),
        (H, 6.0),
        (Round, 1.0),
        (Pupil, 2.2),
        (Ring, 0.0),
        (RingW, 0.0),
    ])
}

/// A shine at (hx, hy) in the world, drawn relative to wherever the face has
/// moved (`x`), so it stays put while he lunges.
fn hex(hx: f64, hy: f64, r: f64, w: f64, x: f64) -> Vec<Stroke> {
    poly(6, r, w, hx - x, hy, PI / 6.0)
}

fn shot(more: &[(Field, Op)], marks: Vec<Stroke>) -> To {
    To::num(with(&eye(), more))
        .mark(marks)
        .inks(vec![Ink::Accent; 6])
}

/// The burst where the shine was: six rays flying out from it.
fn burst(r0: f64, r1: f64, w: f64, x: f64) -> Vec<Stroke> {
    (0..6)
        .map(|i| {
            let a = i as f64 * PI / 3.0 + 0.3;
            let (cx, cy) = (AT.0 - x, AT.1);
            s(
                cx + a.cos() * r0,
                cy + a.sin() * r0 * 0.8,
                cx + a.cos() * r1,
                cy + a.sin() * r1 * 0.8,
                w,
            )
        })
        .collect()
}

pub fn beat() -> Beat {
    // Shine, shine, shine — each gap shorter than the last.
    let mut frames = vec![
        kf(To::num(eye()), 0.6, InOut),
        hold(To::num(with(&eye(), &set(&[(GazeX, 2.4)]))), 0.5),
    ];
    let mut gap = 0.9;
    for (i, &(hx, hy)) in SPOTS.iter().enumerate() {
        let look = set(&[(GazeX, 2.6), (GazeY, hy / 5.0)]);
        let flinch = with(&look, &set(&[(H, 3.6), (X, -1.2), (Pupil, 1.6)]));
        frames.push(kf(shot(&look, hex(hx, hy, 0.5, 0.0, 0.0)), 0.01, Linear));
        frames.push(kf(
            shot(&flinch, hex(hx, hy, 6.5, 1.8, -1.2)),
            0.06,
            BackOut,
        ));
        frames.push(hold(shot(&flinch, hex(hx, hy, 6.5, 1.8, -1.2)), 0.1));
        frames.push(kf(shot(&look, hex(hx, hy, 7.5, 0.0, 0.0)), 0.07, InOut));
        frames.push(hold(shot(&look, hex(hx, hy, 7.5, 0.0, 0.0)), gap));
        gap *= if i < 4 { 0.6 } else { 0.8 };
    }
    let shine = blinking(frames);

    // Enough. Narrow at it.
    let narrow = set(&[(GazeX, 2.8), (H, 2.8), (Pupil, 1.4)]);
    let enough = still(vec![
        kf(shot(&narrow, hex(AT.0, AT.1, 6.5, 1.8, 0.0)), 0.25, InOut),
        hold(shot(&narrow, hex(AT.0, AT.1, 6.5, 1.8, 0.0)), 1.1),
    ]);

    // FALCON... the windup: pull back, go red, the ring of fire grows.
    let wind = set(&[
        (X, -9.0),
        (W, 6.5),
        (H, 3.2),
        (Pupil, 1.4),
        (GazeX, 3.0),
        (Ring, 9.0),
        (RingW, 2.0),
    ]);
    let windup = |more: &[(Field, Op)], secs: f64| {
        kf(
            shot(&with(&wind, more), hex(AT.0, AT.1, 6.5, 1.8, -9.0)).ink(Ink::Bad),
            secs,
            InOut,
        )
    };
    let falcon = still(vec![
        windup(&set(&[(Ring, 2.0)]), 0.35),
        windup(&[], 0.6),
        windup(&set(&[(Ring, 12.0), (RingW, 3.0)]), 0.5),
    ]);

    // PUNCH. The lunge, and the shine's edges become the burst.
    let hit = set(&[
        (X, 4.0),
        (W, 7.5),
        (H, 3.6),
        (Pupil, 1.2),
        (GazeX, 3.0),
        (Ring, 13.0),
        (RingW, 3.0),
    ]);
    let red = vec![Ink::Bad; 6];
    let strike = |more: &[(Field, Op)], marks: Vec<Stroke>, secs: f64, ease: Easing| {
        kf(
            To::num(with(&eye(), &with(&hit, more)))
                .mark(marks)
                .inks(red.clone())
                .ink(Ink::Bad),
            secs,
            ease,
        )
    };
    let punch = still(vec![
        strike(&[], burst(2.0, 5.0, 2.4, 4.0), 0.08, BackOut),
        strike(&[], burst(6.0, 14.0, 2.0, 4.0), 0.22, Out),
        strike(
            &set(&[(Ring, 15.0), (RingW, 1.0)]),
            burst(14.0, 22.0, 0.0, 4.0),
            0.35,
            Out,
        ),
        hold(
            To::num(with(
                &eye(),
                &set(&[(X, 4.0), (W, 7.0), (H, 4.0), (Pupil, 1.6), (GazeX, 3.0)]),
            )),
            0.5,
        ),
    ]);

    // He enjoys it. A lot.
    let smug = set(&[
        (Gap, 18.0),
        (W, 7.5),
        (H, 6.0),
        (Round, 0.85),
        (Pupil, 2.8),
        (Squint, 0.42),
        (Tilt, -2.0),
        (Y, -1.0),
        (GazeX, 1.6),
        (Ring, 0.0),
        (RingW, 0.0),
    ]);
    let after = Section {
        frames: vec![
            kf(To::num(smug.clone()), 0.35, BackOut),
            hold(To::num(smug), 2.2),
        ],
        motion: Motion::Breathe,
        blink: false,
    };

    build("shine", 0.5, vec![shine, enough, falcon, punch, after])
}
