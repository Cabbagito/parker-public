// Entrances: how a reaction lands, when the landing is the point.
//
// A face normally arrives by interpolation — a spring for a reaction, an ease
// for a posture — straight from wherever the face was into the new pose. For
// most faces that is the whole story. For a few it is not: an eye-roll is the
// roll, not the ceiling it ends on; a double-take is the look away and the
// snap back; the shades have to come down from above. Held, those poses are
// a look at the ceiling, a stare, a pair of sunglasses — the meaning was in
// the arrival.
//
// So a face may have an entrance: a short run of keyframes it plays as it
// lands, *departing from wherever the face was* and ending exactly on its
// own pose. It is an ordinary `Beat` (beat.rs), played with
// `Beat::departing` — the same player idle flair and dance use, pointed at a
// new moment — and it crosses to visage in catalog.json as data, so no
// keyframe here is ever transcribed by hand.
//
// Every step states a whole pose, the way the faces in mod.rs are written,
// and `toward` turns it into a delta against the face it lands in. Designed
// on the "visage — faces" canvas (Oct 2026), where each one played live on
// visage's renderer first.

use super::beat::{Beat, Keyframe};
use super::pose::{Delta, Easing, Field, Motion, Op, Pose, Stroke, MARKS};
use super::{eyes, s, shades, shrug_arms, Expression};
use std::f64::consts::PI;
use std::sync::LazyLock;

/// One step of an entrance: the whole pose to be in, how long to take getting
/// there, and how.
struct Step(Pose, f64, Easing);

/// Built once, kept for the life of the process — the lifetime a `static`
/// table would have had.
fn keep<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

/// Positions to a hundredth of a dot, so the export carries `0.3` and not
/// `0.30000000000000004` and both faces play exactly the numbers in it.
fn q(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// A whole pose as a keyframe against the face it lands in: every number
/// that differs is set, and the strokes and inks are given whole if they
/// differ at all. Deltas are measured from the base (beat.rs), so a field
/// left out is the face's own.
fn toward(base: &Pose, p: &Pose) -> Delta {
    let num: Vec<(Field, Op)> = Field::ALL
        .iter()
        .copied()
        .filter(|f| p.get(*f) != base.get(*f))
        .map(|f| (f, Op::Set(q(p.get(f)))))
        .collect();
    let mark = (p.mark != base.mark).then(|| {
        keep(
            p.mark
                .iter()
                .map(|m| Stroke {
                    x0: q(m.x0),
                    y0: q(m.y0),
                    x1: q(m.x1),
                    y1: q(m.y1),
                    w: q(m.w),
                })
                .collect(),
        )
    });
    Delta {
        num: keep(num),
        mark,
        ink: (p.ink != base.ink).then_some(p.ink),
        mark_ink: (p.mark_ink != base.mark_ink).then(|| keep(p.mark_ink.to_vec())),
    }
}

fn build(e: Expression, steps: Vec<Step>) -> Beat {
    let base = e.pose();
    let frames = steps
        .iter()
        .map(|Step(p, secs, ease)| Keyframe::new(toward(&base, p), *secs, *ease))
        .collect();
    Beat {
        name: e.name(),
        frames: keep(frames),
        // Every entrance ends on its own pose, stated as its last step, so
        // there is no trip home to add.
        ret_secs: 0.0,
        loop_from: None,
        // The face's own motion waits until it has landed: a shake or a
        // breath laid over the roll would blur it.
        motion: Some(Motion::None),
    }
}

/// A stroke's endpoints turned about a point, then moved: for a table in the
/// air.
fn tumble(m: Stroke, px: f64, py: f64, th: f64, dx: f64, dy: f64) -> Stroke {
    let (sn, c) = th.sin_cos();
    let at = |x: f64, y: f64| {
        (
            px + (x - px) * c - (y - py) * sn + dx,
            py + (x - px) * sn + (y - py) * c + dy,
        )
    };
    let (x0, y0) = at(m.x0, m.y0);
    let (x1, y1) = at(m.x1, m.y1);
    Stroke {
        x0,
        y0,
        x1,
        y1,
        ..m
    }
}

use Easing::{BackOut, InOut, Linear, Out};

fn smug() -> Beat {
    // The eyes look up at the shades arriving, the shades drop, bump, settle.
    let base = Expression::Smug.pose();
    let watching = |h: f64| Pose {
        gaze_y: -3.2,
        mark: shades(-30.0),
        ..eyes(6.5, h, 3.0)
    };
    let on = |y: f64| Pose {
        y: -1.0,
        mark: shades(y),
        ..eyes(6.0, 3.5, 2.0)
    };
    build(
        Expression::Smug,
        vec![
            Step(watching(8.0), 0.4, InOut),
            Step(watching(7.0), 0.3, Linear),
            Step(on(0.0), 0.75, Out),
            Step(on(-2.0), 0.12, InOut),
            Step(base, 0.18, InOut),
        ],
    )
}

fn conspiratorial() -> Beat {
    // Both eyes on you; a beat; the right one snaps shut into the ^ and the
    // cheek lifts with it.
    let base = Expression::Conspiratorial.pose();
    let open = Pose {
        gaze_x: 0.4,
        ..eyes(7.5, 8.5, 3.2)
    };
    build(
        Expression::Conspiratorial,
        vec![
            Step(open, 0.35, InOut),
            Step(open, 0.25, Linear),
            Step(Pose { y: -1.0, ..base }, 0.04, Linear),
            Step(base, 0.3, BackOut),
        ],
    )
}

fn eye_roll() -> Beat {
    // Left, up and over, then the lids come down with a sigh.
    let look = |gx: f64, gy: f64, h: f64, y: f64| Pose {
        gaze_x: gx,
        gaze_y: gy,
        y,
        ..eyes(7.5, h, 3.0)
    };
    build(
        Expression::EyeRoll,
        vec![
            Step(look(-3.2, 0.0, 8.5, 0.0), 0.2, InOut),
            Step(look(-2.4, -3.0, 8.5, 0.0), 0.16, Linear),
            Step(look(0.0, -4.2, 8.5, 0.0), 0.16, Linear),
            Step(look(2.6, -3.0, 8.5, 0.0), 0.16, Linear),
            Step(look(1.0, -3.0, 5.5, 1.5), 0.45, InOut),
            Step(Expression::EyeRoll.pose(), 0.3, InOut),
        ],
    )
}

fn double_take() -> Beat {
    // A glance over, a look away, a beat, and the snap back — wide.
    let glance = |gx: f64| Pose {
        gaze_x: gx,
        ..eyes(7.5, 10.0, 3.0)
    };
    build(
        Expression::DoubleTake,
        vec![
            Step(glance(3.0), 0.25, InOut),
            Step(glance(-1.5), 0.3, InOut),
            Step(glance(-1.5), 0.45, Linear),
            Step(Expression::DoubleTake.pose(), 0.14, BackOut),
        ],
    )
}

fn shrug() -> Beat {
    // The arms spring up from the sides, dip, and settle up.
    let base = Expression::Shrug.pose();
    let arms = |y: f64, lift: f64| Pose {
        y,
        mark: shrug_arms(lift),
        ..base
    };
    build(
        Expression::Shrug,
        vec![
            Step(arms(-1.0, 0.55), 0.3, InOut),
            Step(arms(-1.0, 0.0), 0.28, BackOut),
            Step(arms(0.5, 0.12), 0.14, InOut),
            Step(base, 0.22, BackOut),
        ],
    )
}

fn table_flip() -> Beat {
    // The table is three strokes, grabbed, thrown, turned over in the air,
    // landing legs up. The face sits 14 dots left and strokes are written
    // from its centre, so everything here is laid out on the canvas and moved
    // into the face's frame at the end.
    const FX: f64 = -14.0;
    let table = [
        s(8.0, 6.0, 30.0, 6.0, 2.6),
        s(11.0, 6.0, 11.0, 14.0, 2.0),
        s(27.0, 6.0, 27.0, 14.0, 2.0),
    ];
    let flung = |th: f64, dx: f64, dy: f64| table.map(|m| tumble(m, 19.0, 10.0, th, dx, dy));
    let calm = [
        s(-25.0, 4.0, -29.0, 12.0, 2.2),
        s(-3.0, 4.0, 1.0, 12.0, 2.2),
    ];
    let grab = [s(-25.0, 4.0, -29.0, 12.0, 2.2), s(-3.0, 4.0, 8.0, 6.0, 2.2)];
    let up = [
        s(-25.0, 2.0, -33.0, -10.0, 2.2),
        s(-3.0, 2.0, 5.0, -10.0, 2.2),
    ];
    let frame = |arms: [Stroke; 2], tab: [Stroke; 3]| -> [Stroke; MARKS] {
        let all = [arms[0], arms[1], tab[0], tab[1], tab[2]];
        Stroke::pad(&all.map(|m| Stroke {
            x0: m.x0 - FX,
            x1: m.x1 - FX,
            ..m
        }))
    };
    let face = |wide: bool, y: f64, mark: [Stroke; MARKS]| Pose {
        x: FX,
        y,
        gap: 9.0,
        w: 4.2,
        h: if wide { 5.0 } else { 4.2 },
        round: 1.0,
        pupil: if wide { 1.3 } else { 2.0 },
        mark,
        ..Pose::ZERO
    };
    build(
        Expression::TableFlip,
        vec![
            Step(face(false, 0.0, frame(calm, table)), 0.45, InOut),
            Step(face(true, 0.0, frame(grab, table)), 0.3, InOut),
            Step(face(true, 0.0, frame(up, flung(1.2, 7.0, -12.0))), 0.2, Out),
            Step(
                face(true, 0.0, frame(up, flung(2.3, 11.0, -9.0))),
                0.16,
                Linear,
            ),
            Step(
                face(true, 0.0, frame(up, flung(PI, 11.0, 0.0))),
                0.2,
                Linear,
            ),
            Step(
                face(true, 0.6, frame(up, flung(PI, 11.0, -1.2))),
                0.08,
                InOut,
            ),
            Step(Expression::TableFlip.pose(), 0.12, InOut),
        ],
    )
}

fn dead() -> Beat {
    // Lands like a hit: eyes pop wide and get knocked back, then cross out.
    let base = Expression::Dead.pose();
    let hit = Pose {
        x: 5.0,
        y: -1.0,
        ..eyes(8.0, 10.0, 1.8)
    };
    build(
        Expression::Dead,
        vec![
            Step(hit, 0.1, Out),
            Step(hit, 0.12, Linear),
            Step(
                Pose {
                    x: 2.0,
                    mark: base.mark,
                    ..Pose::ZERO
                },
                0.2,
                Out,
            ),
            Step(base, 0.3, BackOut),
        ],
    )
}

static ENTRANCES: LazyLock<Vec<(Expression, Beat)>> = LazyLock::new(|| {
    vec![
        (Expression::Smug, smug()),
        (Expression::Conspiratorial, conspiratorial()),
        (Expression::EyeRoll, eye_roll()),
        (Expression::DoubleTake, double_take()),
        (Expression::Shrug, shrug()),
        (Expression::TableFlip, table_flip()),
        (Expression::Dead, dead()),
    ]
});

/// The entrance a face lands with, if it has one.
pub fn of(e: Expression) -> Option<&'static Beat> {
    ENTRANCES.iter().find(|(x, _)| *x == e).map(|(_, b)| b)
}

/// Every entrance, in a fixed order, for the catalog.
pub fn all() -> impl Iterator<Item = (Expression, &'static Beat)> {
    ENTRANCES.iter().map(|(e, b)| (*e, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entrance_lands_exactly_on_its_face() {
        // The whole contract: wherever it departs from, it ends on the pose
        // the face is worn in, so nothing jumps when it hands over.
        let origin = Expression::Idle.pose();
        for (e, b) in all() {
            let end = b.departing(origin, e.pose(), b.secs() + 0.01);
            assert_eq!(end, e.pose(), "{} ends somewhere else", e.name());
        }
    }

    #[test]
    fn an_entrance_departs_from_wherever_the_face_was() {
        // Shape, not colour: a transition takes the target's inks at once
        // (pose.rs, `Pose::ink`), and an entrance is a transition.
        for (e, b) in all() {
            for from in [Expression::Idle, Expression::Speaking, Expression::Yes] {
                let start = b.departing(from.pose(), e.pose(), 0.0);
                let start = Pose {
                    ink: from.pose().ink,
                    mark_ink: from.pose().mark_ink,
                    ..start
                };
                assert_eq!(start, from.pose(), "{} jumps off {}", e.name(), from.name());
            }
        }
    }

    #[test]
    fn entrances_are_quick() {
        // A gesture, not a vignette: the face is answering something, and an
        // answer that takes three seconds to arrive is late.
        for (e, b) in all() {
            assert!(
                b.secs() <= 2.0,
                "{} takes {:.2}s to land",
                e.name(),
                b.secs()
            );
            assert!(!b.terminal(), "{} never lands", e.name());
        }
    }

    #[test]
    fn only_reactions_have_entrances() {
        for (e, _) in all() {
            assert!(
                Expression::VOLUNTARY.contains(&e),
                "{} is a posture with an entrance",
                e.name()
            );
        }
    }
}
