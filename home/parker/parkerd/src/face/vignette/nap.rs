// nap — parker dozes off, sleeps a couple of minutes, wakes on his own.
//
// Not the sleep state that was removed in August 2026. That one was a place
// parker went and stayed until something woke him — a face pretending to be
// switched off. This is a bit he does and finishes: he yawns, nods off twice
// and catches himself, the iris splits into two closed lids, a Z rises with
// every breath, he dreams (the lids ripple), rolls over, and wakes up with
// the ring drawing itself back on like a boot. Any key wakes him early, the
// way any key ends any beat.

use super::*;
use Easing::*;
use Field::*;

/// Two closed lids, low and calm.
fn lids(h: f64, more: &[(Field, Op)]) -> Look {
    with(
        &set(&[
            (Gap, 7.0),
            (W, 5.0),
            (H, h),
            (Round, 1.0),
            (Pupil, 0.0),
            (Ring, 0.0),
            (RingW, 0.0),
            (Y, 2.0),
        ]),
        more,
    )
}

/// A Z along its rise: `u` 0 just off his brow, 1 gone.
fn z_at(u: f64) -> [Stroke; 3] {
    let w = if u < 0.15 {
        u / 0.15 * 1.5
    } else if u > 0.7 {
        ((1.0 - u) / 0.3 * 1.6).max(0.0)
    } else {
        1.6
    };
    zee(9.0 + u * 12.0, -3.0 - u * 13.0, 1.2 + u * 2.4, w)
}

fn z_hidden() -> [Stroke; 3] {
    zee(9.0, -3.0, 1.2, 0.0)
}

fn marks(a: [Stroke; 3], b: [Stroke; 3]) -> Vec<Stroke> {
    a.into_iter().chain(b).collect()
}

const BREATHS: u64 = 30;

pub fn beat() -> Beat {
    let mut sections = vec![];

    let tall = set(&[(W, 9.5), (H, 12.5), (Pupil, 2.2), (Ring, 18.5)]);
    let squeezed = set(&[
        (W, 11.5),
        (H, 2.0),
        (Round, 0.5),
        (Pupil, 0.0),
        (Ring, 17.0),
    ]);
    let heavy = set(&[(H, 6.5), (Pupil, 2.6), (GazeY, 1.2)]);
    sections.push(still(vec![
        kf(To::num(tall.clone()), 1.3, InOut),
        hold(To::num(tall), 0.5),
        kf(To::num(squeezed.clone()), 0.45, Out),
        hold(To::num(squeezed), 0.9),
        kf(To::num(heavy.clone()), 0.9, InOut),
        hold(To::num(heavy), 1.2),
    ]));

    // Nodding off: two he catches, one he does not.
    let sink1 = set(&[(H, 3.8), (Pupil, 2.2), (GazeY, 1.8), (Y, 1.2)]);
    let catch1 = set(&[(H, 6.2), (Pupil, 2.6), (GazeY, 0.8)]);
    let sink2 = set(&[(H, 2.4), (Pupil, 1.6), (GazeY, 2.0), (Y, 2.0), (Ring, 15.0)]);
    let catch2 = set(&[(H, 6.8), (Pupil, 2.6), (Y, -0.6), (Ring, 16.5)]);
    sections.push(still(vec![
        kf(To::num(sink1.clone()), 1.8, InOut),
        hold(To::num(sink1), 0.9),
        kf(
            To::num(set(&[(H, 8.0), (Pupil, 2.8), (Y, -0.8)])),
            0.12,
            Out,
        ),
        kf(To::num(catch1.clone()), 0.5, InOut),
        hold(To::num(catch1), 1.6),
        kf(To::num(sink2.clone()), 2.2, InOut),
        hold(To::num(sink2), 1.4),
        kf(To::num(catch2.clone()), 0.15, Out),
        hold(To::num(catch2), 0.8),
        // The last one he does not catch: the ring loosens and goes, and the
        // iris splits into two closed lids.
        kf(
            To::num(set(&[
                (H, 2.0),
                (W, 9.5),
                (Pupil, 0.0),
                (Y, 1.5),
                (Ring, 13.0),
                (RingW, 1.4),
            ])),
            2.4,
            InOut,
        ),
        kf(
            To::num(lids(1.1, &[])).mark(marks(z_hidden(), [gone(); 3])),
            1.6,
            InOut,
        ),
    ]));

    // Asleep: one breath per cycle and one Z per breath. Two Zs are on screen
    // at once — A and B take turns, each finishing its rise during the other
    // one's breath — and every Z travels at width 0 when it is out of sight,
    // so none of them ever streaks back down.
    for i in 0..BREATHS {
        let flavour = hash01(i, 0x5137);
        let mine_is_a = i % 2 == 0;
        let dream = i > 4 && flavour < 0.14;
        let turned = (15..23).contains(&i);
        let deep = flavour > 0.86;
        let rolled: Look = if turned {
            set(&[(X, 3.5), (Tilt, 0.9)])
        } else {
            vec![]
        };
        let pair = |mine: Option<f64>, other: Option<f64>| -> Vec<Stroke> {
            let a = mine.map_or(z_hidden(), z_at);
            let b = if i == 0 {
                z_hidden()
            } else {
                other.map_or(z_hidden(), z_at)
            };
            if mine_is_a {
                marks(a, b)
            } else {
                marks(b, a)
            }
        };
        let rest = lids(1.1, &rolled);
        let inhale = lids(
            if deep { 1.7 } else { 1.4 },
            &with(&rolled, &set(&[(Y, 1.5)])),
        );
        let exhale = lids(1.0, &rolled);
        let mut frames = vec![
            kf(To::num(rest).mark(pair(None, Some(0.5))), 0.01, Linear),
            kf(
                To::num(inhale).mark(pair(Some(0.22), Some(0.9))),
                if deep { 2.8 } else { 2.2 },
                InOut,
            ),
            kf(
                To::num(exhale.clone()).mark(pair(Some(0.5), Some(1.0))),
                2.8,
                InOut,
            ),
        ];
        if dream {
            let rem = lids(1.0, &with(&rolled, &set(&[(Amp, 0.5), (Freq, 1.6)])));
            frames.push(kf(
                To::num(rem.clone()).mark(pair(Some(0.5), Some(1.0))),
                0.4,
                InOut,
            ));
            frames.push(hold(To::num(rem).mark(pair(Some(0.5), Some(1.0))), 2.6));
            frames.push(kf(
                To::num(exhale).mark(pair(Some(0.5), Some(1.0))),
                0.6,
                InOut,
            ));
        }
        sections.push(Section {
            frames,
            motion: Motion::Breathe,
            blink: false,
        });
    }

    // Waking: the last Z floats off, the lids crack, close, open; back into
    // one iris, the ring draws itself on, a stretch, a look round the room.
    let parted = set(&[
        (Gap, 7.0),
        (W, 5.5),
        (H, 4.5),
        (Round, 1.0),
        (Pupil, 1.8),
        (Ring, 0.0),
        (RingW, 0.0),
        (Y, 1.0),
    ]);
    let stretch = vec![
        (W, Op::Mul(1.28)),
        (H, Op::Mul(1.18)),
        (Ring, Op::Set(18.5)),
    ];
    let left = set(&[(GazeX, -3.4)]);
    let right = set(&[(GazeX, 3.4), (GazeY, -0.5)]);
    sections.push(blinking(vec![
        kf(
            To::num(lids(1.1, &[]))
                .mark(marks(zee(15.0, -9.0, 2.8, 0.0), zee(20.0, -15.0, 3.6, 0.0))),
            1.2,
            InOut,
        ),
        kf(To::num(lids(2.6, &set(&[(Pupil, 1.2)]))), 0.5, InOut),
        kf(To::num(lids(0.9, &[])), 0.3, InOut),
        hold(To::num(lids(0.9, &[])), 0.7),
        kf(To::num(parted.clone()), 0.6, InOut),
        hold(To::num(parted), 0.8),
        kf(
            To::num(set(&[
                (Gap, 0.0),
                (W, 8.5),
                (H, 8.5),
                (Pupil, 2.6),
                (Ring, 17.0),
                (RingW, 2.0),
                (Sweep, 0.0),
            ])),
            0.7,
            InOut,
        ),
        kf(To::num(set(&[(Sweep, 1.0), (Pupil, 2.6)])), 1.4, Out),
        kf(To::num(stretch.clone()), 0.5, BackOut),
        hold(To::num(stretch), 0.5),
        kf(To::num(left.clone()), 0.25, Out),
        hold(To::num(left), 0.7),
        kf(To::num(right.clone()), 0.2, Out),
        hold(To::num(right), 0.8),
    ]));

    build("nap", 0.6, sections)
}

use super::super::pose::hash01;
