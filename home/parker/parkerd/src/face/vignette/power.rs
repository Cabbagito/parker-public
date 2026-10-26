// power cycle — parker switches himself off and on again, like an old CRT.
//
// Shape only, so it reads from across the room: the picture collapses to a
// bright line, the line to a dot, the dot hangs and fades, and the screen
// stays dark long enough to make you look. Then the dot, the line snapping
// wide, the iris overshooting back, the ring drawing itself on, a degauss
// shudder and one roll bar down the face — and a look at you, as if to ask
// whether that fixed it.

use super::*;
use Easing::*;
use Field::*;

fn dark() -> Look {
    set(&[(Ring, 0.0), (RingW, 0.0), (Pupil, 0.0)])
}

fn shape(pairs: &[(Field, f64)]) -> To {
    To::num(with(&dark(), &set(pairs)))
}

fn roll(y: f64, w: f64) -> Vec<Stroke> {
    vec![s(-24.0, y, 24.0, y, w)]
}

pub fn beat() -> Beat {
    let line = [(W, 28.0), (H, 0.45), (Round, 0.0)];
    let spot = [(W, 0.9), (H, 0.9), (Round, 1.0)];
    let off = still(vec![
        hold(To::num(vec![]), 0.6),
        kf(shape(&line), 0.16, Out),
        hold(shape(&line), 0.1),
        kf(shape(&spot), 0.18, Out),
        hold(shape(&spot), 0.7),
        kf(shape(&[(W, 0.4), (H, 0.4)]), 0.8, InOut),
    ]);
    let gone = [(W, 0.0), (H, 0.0)];
    let blank = still(vec![kf(shape(&gone), 0.2, InOut), hold(shape(&gone), 2.4)]);

    let pupilless = set(&[(Pupil, 0.0)]);
    let shudder = |x: f64, secs: f64| kf(To::num(with(&pupilless, &set(&[(X, x)]))), secs, Linear);
    let on = blinking(vec![
        kf(shape(&[(W, 0.8), (H, 0.8)]), 0.06, Out),
        hold(shape(&[(W, 0.8), (H, 0.8)]), 0.45),
        kf(shape(&[(W, 30.0), (H, 0.4), (Round, 0.0)]), 0.12, Out),
        kf(shape(&[(W, 9.0), (H, 9.0)]), 0.4, BackOut),
        kf(
            To::num(set(&[
                (Ring, 17.0),
                (RingW, 2.0),
                (Sweep, 0.0),
                (Pupil, 0.0),
            ])),
            0.15,
            InOut,
        ),
        kf(To::num(pupilless.clone()), 0.55, Out),
        // Degauss: the picture shudders sideways and settles.
        shudder(1.8, 0.05),
        shudder(-1.5, 0.05),
        shudder(1.1, 0.05),
        shudder(-0.6, 0.06),
        kf(To::num(pupilless), 0.08, InOut),
        kf(To::num(set(&[(Pupil, 3.0)])), 0.3, Out),
        // One roll bar, top to bottom.
        kf(To::num(vec![]).mark(roll(-20.0, 0.0)), 0.01, Linear),
        kf(To::num(vec![]).mark(roll(-17.0, 1.2)), 0.12, Linear),
        kf(To::num(vec![]).mark(roll(17.0, 1.2)), 0.75, Linear),
        kf(To::num(vec![]).mark(roll(20.0, 0.0)), 0.12, Linear),
        hold(To::num(vec![]), 0.4),
    ]);

    let fixed = set(&[(GazeX, 0.0), (GazeY, 0.0), (W, 9.6), (H, 9.6), (Pupil, 2.6)]);
    let check = blinking(vec![
        kf(To::num(fixed.clone()), 0.3, InOut),
        hold(To::num(fixed), 1.6),
    ]);

    build("power-cycle", 0.5, vec![off, blank, on, check])
}
