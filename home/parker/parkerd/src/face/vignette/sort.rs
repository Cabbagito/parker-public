// sort — parker bubble-sorts six bars, for fun.
//
// A small iris with a small ring (still him) over six bars, the six strokes.
// He looks at each pair (accent) and swaps it or nods it through, and every
// pass locks one more bar into place (good). Sorted, the row does a little
// wave. Then something scrambles them, he narrows at it, and sorts again —
// faster, having done this before. Like `dial`, the maths is the show.

use super::*;
use Easing::*;
use Field::*;

const SLOT: [f64; 6] = [-15.0, -9.0, -3.0, 3.0, 9.0, 15.0];
const BASE: f64 = 26.0;

fn eye() -> Look {
    set(&[
        (Gap, 0.0),
        (W, 3.8),
        (H, 3.8),
        (Round, 1.0),
        (Pupil, 1.5),
        (Ring, 7.0),
        (RingW, 1.2),
        (Y, -10.0),
    ])
}

fn height(v: usize) -> f64 {
    3.0 + v as f64 * 2.4
}

/// One bar as it is drawn: its value, and where it is mid-swap.
#[derive(Clone, Copy)]
struct Bar {
    v: usize,
    x: Option<f64>,
    lift: f64,
}

fn bars(a: &[usize]) -> Vec<Bar> {
    a.iter()
        .map(|&v| Bar {
            v,
            x: None,
            lift: 0.0,
        })
        .collect()
}

#[derive(Default)]
struct Opt {
    /// Slots being compared, in accent.
    hot: Option<[usize; 2]>,
    /// Slots from here on are sorted, in good.
    done: Option<usize>,
    /// Where the eye is looking along the row.
    look: Option<f64>,
    /// Per-slot bump for the wave.
    bump: [f64; 6],
    eye: Look,
}

/// The row. Each stroke belongs to a value, not a slot, so a swap is two
/// bars trading places rather than two bars trading heights.
fn scene(items: &[Bar], o: &Opt) -> To {
    let mut marks = vec![gone(); 6];
    let mut inks = vec![Ink::Plain; 6];
    for (i, b) in items.iter().enumerate() {
        let x = b.x.unwrap_or(SLOT[i]);
        marks[b.v] = s(
            x,
            BASE - b.lift,
            x,
            BASE - b.lift - height(b.v) - o.bump[i],
            3.2,
        );
        inks[b.v] = if o.hot.is_some_and(|h| h.contains(&i)) {
            Ink::Accent
        } else if o.done.is_some_and(|d| i >= d) {
            Ink::Good
        } else {
            Ink::Plain
        };
    }
    let gaze = match o.look {
        Some(x) => set(&[(GazeX, x / 7.0), (GazeY, 1.4)]),
        None => vec![],
    };
    To::num(with(&with(&eye(), &gaze), &o.eye))
        .mark(marks)
        .inks(inks)
}

fn sort_run(start: &[usize], pace: f64, sections: &mut Vec<Section>) -> Vec<usize> {
    let mut a = start.to_vec();
    let n = a.len();
    let mut frames = vec![];
    for pass in 0..n - 1 {
        let done = n - pass;
        let mut swapped = false;
        for i in 0..n - 1 - pass {
            let mid = (SLOT[i] + SLOT[i + 1]) / 2.0;
            let o = || Opt {
                hot: Some([i, i + 1]),
                done: Some(done),
                look: Some(mid),
                ..Opt::default()
            };
            frames.push(kf(scene(&bars(&a), &o()), 0.12 * pace, Out));
            frames.push(hold(scene(&bars(&a), &o()), 0.22 * pace));
            if a[i] > a[i + 1] {
                // Swap: the bigger one hops over the smaller.
                let mut hop = bars(&a);
                hop[i].x = Some(mid);
                hop[i].lift = 3.0;
                hop[i + 1].x = Some(mid);
                frames.push(kf(scene(&hop, &o()), 0.16 * pace, InOut));
                a.swap(i, i + 1);
                frames.push(kf(scene(&bars(&a), &o()), 0.16 * pace, Out));
                swapped = true;
            } else {
                let nod = Opt {
                    eye: set(&[(H, 3.0)]),
                    ..o()
                };
                frames.push(kf(scene(&bars(&a), &nod), 0.08 * pace, InOut));
                frames.push(kf(scene(&bars(&a), &o()), 0.08 * pace, InOut));
            }
        }
        let locked = Opt {
            done: Some(done - 1),
            look: Some(SLOT[done - 1]),
            ..Opt::default()
        };
        frames.push(kf(scene(&bars(&a), &locked), 0.25 * pace, InOut));
        if !swapped {
            // Nothing moved this pass: the rest is already in order.
            let all = Opt {
                done: Some(0),
                look: Some(0.0),
                ..Opt::default()
            };
            frames.push(kf(scene(&bars(&a), &all), 0.3 * pace, InOut));
            break;
        }
    }
    let all = Opt {
        done: Some(0),
        ..Opt::default()
    };
    frames.push(kf(scene(&bars(&a), &all), 0.3 * pace, InOut));
    sections.push(blinking(frames));
    a
}

fn wave(a: &[usize], sections: &mut Vec<Section>) {
    let mut frames = vec![];
    for k in 0..=6 {
        let mut bump = [0.0; 6];
        if k < 6 {
            bump[k] = 2.4;
        }
        let look = if k < 6 { SLOT[k] } else { 0.0 };
        let o = Opt {
            done: Some(0),
            bump,
            look: Some(look),
            ..Opt::default()
        };
        frames.push(kf(scene(&bars(a), &o), 0.13, Out));
    }
    let pleased = || Opt {
        done: Some(0),
        eye: set(&[(H, 1.4), (Pupil, 0.0), (W, 4.2)]),
        ..Opt::default()
    };
    frames.push(kf(scene(&bars(a), &pleased()), 0.3, InOut));
    frames.push(hold(scene(&bars(a), &pleased()), 1.6));
    sections.push(still(frames));
}

pub fn beat() -> Beat {
    let first = [4, 1, 5, 0, 3, 2];
    let second = [2, 5, 0, 4, 1, 3];
    let mut sections = vec![];

    let look = |x: f64| Opt {
        look: Some(x),
        ..Opt::default()
    };
    let flat: Vec<Stroke> = SLOT.iter().map(|&x| s(x, BASE, x, BASE, 0.0)).collect();
    sections.push(blinking(vec![
        kf(To::num(eye()).mark(flat), 1.0, InOut),
        kf(scene(&bars(&first), &look(-15.0)), 0.8, InOut),
        kf(scene(&bars(&first), &look(15.0)), 0.9, InOut),
        hold(scene(&bars(&first), &look(0.0)), 0.7),
    ]));

    let sorted = sort_run(&first, 1.0, &mut sections);
    wave(&sorted, &mut sections);

    // Scrambled — he did not do that. Narrow at it, then sort again, faster.
    let narrow = || Opt {
        look: Some(18.0),
        eye: set(&[(H, 2.2), (Pupil, 1.2)]),
        ..Opt::default()
    };
    sections.push(blinking(vec![
        kf(scene(&bars(&second), &look(0.0)), 0.35, BackOut),
        kf(scene(&bars(&second), &narrow()), 0.3, Out),
        hold(scene(&bars(&second), &narrow()), 1.2),
        kf(scene(&bars(&second), &look(0.0)), 0.3, InOut),
    ]));
    let again = sort_run(&second, 0.55, &mut sections);
    wave(&again, &mut sections);

    build("sort", 0.7, sections)
}
