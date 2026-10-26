// reading — parker reads, the way eyes actually read.
//
// Two eyes over four lines of text (four strokes). Reading is saccades, not
// a glide: a few fixations per line with a flick between them, then a fast
// return sweep to the next. Sometimes he re-reads a phrase (a regression);
// when a line lands he highlights it (an accent stroke along it) or stares
// into the middle distance with it for a while. Pages turn.

use super::super::pose::hash01;
use super::*;
use Easing::*;
use Field::*;

const ROWS: [f64; 4] = [9.0, 12.5, 16.0, 19.5];
const PAGES: u64 = 24;

fn eyes() -> Look {
    set(&[
        (Gap, 9.0),
        (W, 5.0),
        (H, 4.2),
        (Round, 0.85),
        (Pupil, 1.6),
        (Ring, 0.0),
        (RingW, 0.0),
        (Y, -9.0),
    ])
}

/// A page's four lines: lengths vary, an indent sometimes, the last short.
fn lines(page: u64, slide: f64, w: f64) -> Vec<Stroke> {
    ROWS.iter()
        .enumerate()
        .map(|(i, &y)| {
            let r = hash01(page * 8 + i as u64, 0x7e47);
            let x0 = if i == 0 && r < 0.4 { -13.0 } else { -18.0 };
            let x1 = if i == 3 {
                -6.0 + r * 16.0
            } else {
                15.0 + r * 3.0
            };
            s(x0 + slide, y, x1 + slide, y, w)
        })
        .collect()
}

#[derive(Clone)]
struct Scene {
    page: u64,
    slide: f64,
    weight: f64,
    gx: f64,
    gy: f64,
    /// A highlight: which line, and how far along it (0..1).
    hl: Option<(usize, f64)>,
    eye: Look,
}

fn page(p: u64) -> Scene {
    Scene {
        page: p,
        slide: 0.0,
        weight: 1.1,
        gx: 0.0,
        gy: 0.8,
        hl: None,
        eye: vec![],
    }
}

impl Scene {
    fn gaze(mut self, gx: f64, gy: f64) -> Scene {
        self.gx = gx;
        self.gy = gy;
        self
    }
    fn hl(mut self, hl: Option<(usize, f64)>) -> Scene {
        self.hl = hl;
        self
    }
    fn slid(mut self, by: f64) -> Scene {
        self.slide = by;
        self.weight = 0.0;
        self
    }
    fn eye(mut self, e: Look) -> Scene {
        self.eye = e;
        self
    }

    fn to(&self) -> To {
        let mut marks = lines(self.page, self.slide, self.weight);
        let hl = match self.hl {
            Some((k, frac)) => {
                let l = marks[k];
                let w = if frac > 0.0 { 2.0 } else { 0.0 };
                s(
                    l.x0,
                    ROWS[k] + 0.2,
                    l.x0 + (l.x1 - l.x0) * frac,
                    ROWS[k] + 0.2,
                    w,
                )
            }
            None => s(marks[0].x0, ROWS[0], marks[0].x0, ROWS[0], 0.0),
        };
        marks.push(hl);
        marks.push(gone());
        let num = with(
            &with(&eyes(), &set(&[(GazeX, self.gx), (GazeY, self.gy)])),
            &self.eye,
        );
        let mut inks = vec![Ink::Plain; 4];
        inks.push(Ink::Accent);
        To::num(num).mark(marks).inks(inks)
    }
}

pub fn beat() -> Beat {
    let mut sections = vec![];
    sections.push(blinking(vec![
        kf(To::num(eyes()).mark(lines(0, 0.0, 0.0)), 1.0, InOut),
        kf(page(0).to(), 0.8, InOut),
        hold(page(0).gaze(-2.6, 0.6).to(), 0.8),
    ]));

    for p in 0..PAGES {
        // A highlight stays on the page once it is made.
        let mut keep: Option<(usize, f64)> = None;
        for line in 0..4usize {
            let mut frames = vec![];
            let gy = 0.5 + line as f64 * 0.65;
            let l = lines(p, 0.0, 1.1)[line];
            let span = l.x1 - l.x0;
            let n = if span > 26.0 {
                4
            } else if span > 14.0 {
                3
            } else {
                2
            };
            let r = hash01(p * 8 + line as u64, 0x2ead);
            let here = move |gx: f64| page(p).gaze(gx, gy).hl(keep);
            for f in 0..n {
                let gx = -2.6 + 5.2 * ((l.x0 + 18.0 + span * (f as f64 + 0.5) / n as f64) / 36.0);
                frames.push(kf(here(gx).to(), 0.05, Out));
                let dwell = 0.2 + hash01(p * 40 + line as u64 * 5 + f as u64, 0xf1) * 0.18;
                frames.push(hold(here(gx).to(), dwell));
                if f == n - 1 && r < 0.1 {
                    // A regression: that did not parse. Back, and again.
                    frames.push(kf(here(gx - 2.4).to(), 0.06, Out));
                    frames.push(hold(here(gx - 2.4).eye(set(&[(H, 3.6)])).to(), 0.45));
                    frames.push(kf(here(gx).to(), 0.05, Out));
                    frames.push(hold(here(gx).to(), 0.3));
                }
            }
            if r > 0.93 && keep.is_none() {
                // It lands: the eyes widen, and the line gets highlighted.
                let wide = set(&[(H, 5.2), (Pupil, 1.3)]);
                frames.push(kf(here(0.0).eye(wide.clone()).to(), 0.25, BackOut));
                frames.push(hold(here(0.0).eye(wide).to(), 0.5));
                frames.push(kf(here(-2.4).hl(Some((line, 0.0))).to(), 0.12, Out));
                frames.push(kf(here(2.4).hl(Some((line, 1.0))).to(), 0.9, InOut));
                keep = Some((line, 1.0));
                frames.push(hold(page(p).gaze(2.4, gy).hl(keep).to(), 0.5));
            } else if r > 0.86 && r <= 0.93 {
                // Off into the middle distance with it, then back to the line.
                let up = page(p).gaze(2.6, -1.5).hl(keep);
                let musing = set(&[(H, 3.4)]);
                frames.push(kf(up.to(), 0.35, InOut));
                frames.push(hold(up.to(), 2.4));
                frames.push(kf(up.clone().eye(musing.clone()).to(), 0.3, InOut));
                frames.push(hold(up.eye(musing).to(), 1.0));
            }
            // The return sweep to the start of the next line.
            frames.push(kf(page(p).gaze(-2.6, gy + 0.6).hl(keep).to(), 0.12, Out));
            sections.push(blinking(frames));
        }
        // Turn the page: this one slides off left, the next in from the right.
        sections.push(blinking(vec![
            hold(page(p).gaze(2.4, 2.6).hl(keep).to(), 0.3),
            kf(page(p).slid(-14.0).gaze(-1.5, 1.2).to(), 0.45, InOut),
            kf(page(p + 1).slid(16.0).gaze(1.5, 1.0).to(), 0.01, Linear),
            kf(page(p + 1).gaze(-2.6, 0.5).to(), 0.5, Out),
            hold(page(p + 1).gaze(-2.6, 0.5).to(), 0.4),
        ]));
    }

    // Close it: the lines fold away, a contented squint.
    let closed = with(&eyes(), &set(&[(H, 3.0), (Pupil, 0.0), (Y, -2.0)]));
    sections.push(still(vec![
        kf(
            To::num(with(&eyes(), &set(&[(GazeY, 1.0)]))).mark(lines(PAGES, 0.0, 0.0)),
            0.9,
            InOut,
        ),
        kf(To::num(closed.clone()), 0.6, InOut),
        hold(To::num(closed), 1.6),
    ]));

    build("reading", 0.8, sections)
}
