// pong — parker plays a game against someone off to the right.
//
// Two paddles, a ball and a score bar each side (five strokes); his eyes sit
// at the net and follow every shot. Five points, the rallies getting longer
// and faster, a smash of his own, two he lets slip, and the last one goes
// past him in a double-take that ends dead-centre: looking at you.
//
// Keyframes land where something happens — each bounce off a wall, each
// paddle — plus a few waypoints for the paddles' eased chase. Between them
// everything moves in straight lines, which a ball in flight does anyway.

use super::super::pose::hash01;
use super::*;
use Easing::*;
use Field::*;

const FACE: f64 = 33.0;
const LIM: f64 = 15.5;
/// Paddle waypoints per flight, as fractions of it.
const WAYPOINTS: [f64; 6] = [1.0 / 6.0, 2.0 / 6.0, 0.5, 4.0 / 6.0, 5.0 / 6.0, 1.0];

fn eyes() -> Look {
    set(&[
        (Gap, 6.0),
        (W, 3.2),
        (H, 3.2),
        (Round, 0.85),
        (Pupil, 1.4),
        (Ring, 0.0),
        (RingW, 0.0),
    ])
}

/// Where a ball moving in a straight line is, once the walls fold it back.
fn fold(y: f64) -> f64 {
    let l = LIM;
    let p = ((y + l) % (4.0 * l) + 4.0 * l) % (4.0 * l);
    if p < 2.0 * l {
        p - l
    } else {
        3.0 * l - p
    }
}

struct Game {
    score: [u32; 2],
    left: f64,
    right: f64,
    sections: Vec<Section>,
}

impl Game {
    fn stage(&self, bx: f64, by: f64, bw: f64, eye: &[(Field, Op)], smash: bool) -> To {
        let dir = if bx < 0.0 { -1.0 } else { 1.0 };
        let gaze = set(&[
            (GazeX, clamp(bx / 14.0, -1.9, 1.9)),
            (GazeY, clamp(by / 9.0, -1.5, 1.5)),
        ]);
        let bar = |side: f64, n: u32| {
            let w = if n > 0 { 1.4 } else { 0.0 };
            s(6.0 * side, -18.0, (6.0 + 3.2 * n as f64) * side, -18.0, w)
        };
        let marks = vec![
            s(
                -FACE - 1.5,
                self.left - 3.6,
                -FACE - 1.5,
                self.left + 3.6,
                2.2,
            ),
            s(
                FACE + 1.5,
                self.right - 3.6,
                FACE + 1.5,
                self.right + 3.6,
                2.2,
            ),
            dot(bx, by, bw),
            bar(-1.0, self.score[0]),
            bar(1.0, self.score[1]),
            if smash {
                s(bx - dir * 9.0, by, bx - dir * 3.0, by, 1.4)
            } else {
                s(bx, by, bx, by, 0.0)
            },
        ];
        let inks = vec![
            Ink::Plain,
            Ink::Plain,
            Ink::Accent,
            Ink::Plain,
            Ink::Plain,
            Ink::Accent,
        ];
        To::num(with(&with(&eyes(), &gaze), eye))
            .mark(marks)
            .inks(inks)
    }

    /// One flight from x0 to x1. The receiver chases the landing point (or
    /// only `reach` of the way, when this is the one he misses); the other
    /// paddle drifts home. Returns where the ball arrives.
    #[allow(clippy::too_many_arguments)]
    fn fly(
        &mut self,
        frames: &mut Vec<Frame>,
        x0: f64,
        x1: f64,
        y0: f64,
        vy: f64,
        speed: f64,
        reach: f64,
        smash: bool,
    ) -> f64 {
        let span = (x1 - x0).abs() / speed;
        let landing = fold(y0 + vy * span);
        let toward_right = x1 > x0;
        let (start_l, start_r) = (self.left, self.right);

        // The moments worth a keyframe: every wall bounce, every waypoint.
        let mut times: Vec<f64> = WAYPOINTS.iter().map(|u| u * span).collect();
        if vy.abs() > 1e-9 {
            for k in -4..=4 {
                let t = ((2 * k + 1) as f64 * LIM - y0) / vy;
                if t > 0.0 && t < span {
                    times.push(t);
                }
            }
        }
        times.sort_by(|a, b| a.total_cmp(b));

        let mut last = 0.0;
        for t in times {
            let u = t / span;
            let chase = smooth((u * 1.35).min(1.0));
            if toward_right {
                self.right = start_r + (landing * reach - start_r) * chase;
                self.left = start_l * (1.0 - u * 0.5);
            } else {
                self.left = start_l + (landing * reach - start_l) * chase;
                self.right = start_r * (1.0 - u * 0.5);
            }
            let bx = x0 + (x1 - x0) * u;
            let by = fold(y0 + vy * t);
            frames.push(kf(
                self.stage(bx, by, 2.6, &[], smash && u < 0.6),
                t - last,
                Linear,
            ));
            last = t;
        }
        landing
    }

    /// A point: served by `left_serves`, `hits` returns, then one side misses.
    fn point(&mut self, id: u64, left_serves: bool, hits: u64, left_loses: bool, speed0: f64) {
        let mut frames = vec![];
        let mut x = if left_serves { -FACE } else { FACE };
        let mut y = if left_serves { self.left } else { self.right };
        let mut speed = speed0;
        for h in 0..=hits {
            let vy = (hash01(id * 31 + h, 0x9011) * 2.0 - 1.0) * 26.0;
            let to = -x;
            let last = h == hits;
            let misses = last && ((to < 0.0) == left_loses);
            let smash = id == 2 && last;
            if smash {
                speed *= 1.7;
            }
            y = self.fly(
                &mut frames,
                x,
                to,
                y,
                vy,
                speed,
                if misses { 0.35 } else { 1.0 },
                smash,
            );
            if misses {
                // Past the paddle and off the edge.
                let dir = if to < 0.0 { -1.0 } else { 1.0 };
                let mut bx = to;
                for k in 1..=4 {
                    bx += dir * 3.5;
                    let by = fold(y + vy * k as f64 * 0.06);
                    frames.push(kf(self.stage(bx, by, 2.6, &[], false), 0.06, Linear));
                }
                frames.push(kf(
                    self.stage(bx + dir * 0.9, y, 0.0, &[], false),
                    0.06,
                    Linear,
                ));
                break;
            }
            x = to;
            speed += 2.4;
        }
        self.sections.push(blinking(frames));
    }

    fn react(&mut self, eye: Look, secs: f64, winner: usize) {
        self.score[winner] += 1;
        self.sections.push(still(vec![
            kf(self.stage(0.0, 0.0, 0.0, &eye, false), 0.3, BackOut),
            hold(self.stage(0.0, 0.0, 0.0, &eye, false), secs),
            kf(self.stage(0.0, 0.0, 0.0, &[], false), 0.4, InOut),
        ]));
    }
}

pub fn beat() -> Beat {
    let mut g = Game {
        score: [0, 0],
        left: 0.0,
        right: 0.0,
        sections: vec![],
    };
    g.sections.push(still(vec![
        kf(To::num(eyes()), 0.9, InOut),
        kf(g.stage(-FACE + 1.0, 0.0, 0.0, &[], false), 0.6, InOut),
        hold(g.stage(-FACE + 1.0, 0.0, 2.6, &[], false), 0.6),
    ]));

    let smug = set(&[(H, 1.5), (Pupil, 0.0), (W, 3.6)]);
    let sheepish = set(&[(GazeX, -1.8), (GazeY, 1.3), (Tilt, 0.8), (Squint, 0.8)]);
    g.point(0, true, 4, false, 34.0);
    g.react(smug.clone(), 1.1, 0);
    g.point(1, false, 6, true, 36.0);
    g.react(sheepish.clone(), 1.2, 1);
    g.point(2, true, 4, false, 36.0);
    g.react(with(&smug, &set(&[(W, 4.0)])), 1.4, 0);
    g.point(3, false, 6, true, 38.0);
    g.react(sheepish, 1.0, 1);
    g.point(4, true, 11, true, 36.0);
    g.score[1] += 1;

    // The double-take: at the ball, then at you, then nothing moves.
    let at_ball = set(&[(GazeX, -1.9), (GazeY, 0.2)]);
    let at_you = set(&[(GazeX, 0.0), (GazeY, 0.0), (W, 3.8), (H, 4.2), (Pupil, 1.1)]);
    let deadpan = set(&[
        (GazeX, 0.0),
        (GazeY, 0.0),
        (H, 1.0),
        (Round, 0.0),
        (Pupil, 0.0),
        (W, 3.6),
    ]);
    let ending = vec![
        kf(g.stage(-46.0, 0.0, 0.0, &at_ball, false), 0.2, Out),
        hold(g.stage(-46.0, 0.0, 0.0, &at_ball, false), 0.5),
        kf(g.stage(0.0, 0.0, 0.0, &at_you, false), 0.12, BackOut),
        hold(g.stage(0.0, 0.0, 0.0, &at_you, false), 0.9),
        kf(g.stage(0.0, 0.0, 0.0, &deadpan, false), 0.35, InOut),
        hold(g.stage(0.0, 0.0, 0.0, &deadpan, false), 2.2),
    ];
    g.sections.push(still(ending));

    build("pong", 0.7, g.sections)
}
