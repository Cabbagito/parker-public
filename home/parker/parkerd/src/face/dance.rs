// parker dances.
//
// The loud half of MOTION.md. Idle flair is beats on a lazy clock; this is
// beats on somebody else's clock, and it is the first thing that genuinely
// needs both halves of the pose/placement split at once:
//
//   pose choreography    here. Which dots are lit, on the beat.
//   camera choreography  visage. Where the grid lands and what happens to it
//                        on the way. A section says what it *is* — intro,
//                        build, drop, outro — and visage decides what a drop
//                        looks like on this screen. Naming the drop is music
//                        structure; scattering the face is placement, and
//                        parkerd is not allowed an opinion about it.
//
// ## The clock is a trait on purpose
//
// There is no BPM anywhere in `sense` and there is not going to be. So the
// choreographer takes a `BeatClock` and nothing else — four methods that a
// metronome and a real onset detector implement identically. v0 is a fixed
// tempo, which always works and needs no audio at all; media gating and then
// PipeWire capture with an FFT are new implementations of this trait rather
// than a rewrite of everything above it. Same move as the engine seam in
// PARKER.md: commit to the simple one, keep the seam clean.
//
// ## Quantization is the whole trick
//
// Every parameter here is one the idle face already uses. What makes it read
// as dancing rather than as drifting is that the values *step* — a sidestep
// that eases between positions is a wander, and the identical numbers held
// and snapped land as choreography. Where a drive interpolates at all, it
// interpolates within one subdivision and arrives before the next.

use super::pose::{Field, Ink, Motion, Pose};

// ── the clock ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Low,
    Mid,
    High,
}

/// What the choreographer needs from the music, and the entire surface a
/// future audio-reactive implementation has to satisfy.
pub trait BeatClock {
    /// Beats since the music started, fractional. Everything else — bars,
    /// subdivisions, phase — is arithmetic on this.
    fn beats(&self, now: f64) -> f64;
    /// How loud a band is right now, 0..1.
    fn energy(&self, band: Band, now: f64) -> f64;

    fn bars(&self, now: f64) -> f64 {
        self.beats(now) / 4.0
    }
}

/// A fixed tempo, and the whole of v0.
///
/// It is worth being clear that this is not a placeholder: dancing to a
/// steady 120 is most of the effect, it is deterministic enough to test, and
/// it will expose every choreography problem there is before any audio
/// pipeline exists to blame them on.
#[derive(Clone, Copy, Debug)]
pub struct Metronome {
    pub bpm: f64,
    pub started: f64,
}

impl Metronome {
    pub fn new(bpm: f64, started: f64) -> Metronome {
        Metronome {
            bpm: bpm.clamp(40.0, 220.0),
            started,
        }
    }
}

impl BeatClock for Metronome {
    fn beats(&self, now: f64) -> f64 {
        ((now - self.started) * self.bpm / 60.0).max(0.0)
    }

    /// A synthesized envelope: kick on the odd beats, snare on the even ones,
    /// eighth-note flutter up top. It is a drum machine, not an analysis, and
    /// it exists so that every move downstream can be written against
    /// `energy()` today and still be correct when the source is real audio.
    fn energy(&self, band: Band, now: f64) -> f64 {
        let b = self.beats(now);
        let into = b.fract();
        let idx = (b.floor() as i64).rem_euclid(4);
        let decay = (1.0 - into).powi(2);
        match band {
            Band::Low => {
                if idx % 2 == 0 {
                    decay
                } else {
                    decay * 0.25
                }
            }
            Band::Mid => {
                if idx % 2 == 1 {
                    decay
                } else {
                    decay * 0.2
                }
            }
            Band::High => {
                let e = (b * 2.0).fract();
                (1.0 - e).powf(1.5) * 0.8
            }
        }
    }
}

// ── moves ───────────────────────────────────────────────────────────────────

/// How a move's value varies inside one subdivision.
///
/// Unipolar drives (`Hit`, `Pulse`, `Square`, `Ramp`, `Energy`) run 0..1;
/// bipolar ones (`Sine`, `Alt`) run -1..1. A move's `amp` scales whichever it
/// gets, so the two kinds are not interchangeable and the catalog says which
/// it means by which drive it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drive {
    /// Full on the downbeat, gone by the next one. The impact drive: this is
    /// what makes anything else land, and it is the one move a routine
    /// should never be without.
    Hit,
    /// A half-sine hump per subdivision. Softer than `Hit`, still rhythmic.
    Pulse,
    /// A full sine per subdivision. The only genuinely smooth drive, for the
    /// things that should sway rather than strike.
    Sine,
    /// 0 or 1, flipping each subdivision. Stepped, and therefore danced.
    Square,
    /// -1 or +1, flipping each subdivision.
    Alt,
    /// A sawtooth 0..1 per subdivision. One revolution of the ring per
    /// subdivision when applied to `spin`.
    Ramp,
    /// Whatever the low band is doing. On a metronome this is a kick
    /// envelope; on real audio it is the actual bass.
    Energy(Band),
}

impl Band {
    pub fn name(self) -> &'static str {
        match self {
            Band::Low => "low",
            Band::Mid => "mid",
            Band::High => "high",
        }
    }
}

impl Blend {
    pub fn name(self) -> &'static str {
        match self {
            Blend::Add => "add",
            Blend::Mul => "mul",
            Blend::Set => "set",
        }
    }
}

impl Drive {
    /// The name, and the band it reads if it reads one. Two values rather
    /// than eight names so that adding a band never invents a new drive.
    pub fn parts(self) -> (&'static str, Option<Band>) {
        match self {
            Drive::Hit => ("hit", None),
            Drive::Pulse => ("pulse", None),
            Drive::Sine => ("sine", None),
            Drive::Square => ("square", None),
            Drive::Alt => ("alt", None),
            Drive::Ramp => ("ramp", None),
            Drive::Energy(b) => ("energy", Some(b)),
        }
    }

    /// The drive's value at `beats`, given a subdivision length in beats.
    pub fn at(self, beats: f64, div: f64, clock: &dyn BeatClock, now: f64) -> f64 {
        let div = if div <= 0.0 { 1.0 } else { div };
        let u = beats / div;
        let into = u.fract();
        let step = u.floor() as i64;
        match self {
            // Cubic decay: an attack you can see land, rather than a linear
            // ramp that reads as a fade.
            Drive::Hit => (1.0 - into).powi(3),
            Drive::Pulse => (into * std::f64::consts::PI).sin(),
            Drive::Sine => (into * std::f64::consts::TAU).sin(),
            Drive::Square => (step.rem_euclid(2) == 0) as i64 as f64,
            Drive::Alt => {
                if step.rem_euclid(2) == 0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Drive::Ramp => into,
            Drive::Energy(b) => clock.energy(b, now),
        }
    }
}

/// How a move's value meets the pose already there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    /// `base + value * amp`.
    Add,
    /// `base * (1 + value * (amp - 1))` — so `amp` reads as the multiplier at
    /// full drive and the base is untouched at zero. Squash and stretch.
    Mul,
    /// `value * amp`, ignoring the base. For the fields whose resting value
    /// is not the interesting one.
    Set,
}

#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub field: Field,
    pub blend: Blend,
    pub drive: Drive,
    pub amp: f64,
    /// Subdivision, in beats. 1.0 is a beat, 0.5 eighths, 4.0 a bar.
    pub div: f64,
}

const fn mv(field: Field, blend: Blend, drive: Drive, amp: f64, div: f64) -> Move {
    Move {
        field,
        blend,
        drive,
        amp,
        div,
    }
}

// ── routines ────────────────────────────────────────────────────────────────

/// What a section *is*, musically. visage maps these to cameras; parkerd has
/// no opinion about what a drop looks like on a screen it cannot see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Intro,
    Build,
    Drop,
    Outro,
}

impl Mood {
    pub fn name(self) -> &'static str {
        match self {
            Mood::Intro => "intro",
            Mood::Build => "build",
            Mood::Drop => "drop",
            Mood::Outro => "outro",
        }
    }
}

pub struct Section {
    /// Half-open, in bars from the start of the routine.
    pub from: f64,
    pub to: f64,
    pub mood: Mood,
    pub moves: &'static [Move],
}

pub struct Routine {
    pub name: &'static str,
    pub bars: f64,
    pub sections: &'static [Section],
}

use Blend::*;
use Drive::*;
use Field::*;

/// Everything is stacked on top of `the hit`, which is on every section of
/// every routine. Without an impact on the downbeat the rest reads as motion
/// rather than as dancing — it is the floor, not a flourish.
const HIT_W: Move = mv(W, Mul, Hit, 1.32, 1.0);
const HIT_H: Move = mv(H, Mul, Hit, 0.66, 1.0);

const STRUT_INTRO: &[Move] = &[
    HIT_W,
    HIT_H,
    // Headbang. `Pulse` rather than `Sine` so it strikes downward and
    // recovers, instead of spending half the beat above centre.
    mv(Y, Add, Pulse, 3.2, 1.0),
    mv(Tilt, Add, Sine, 3.0, 4.0),
];

const STRUT_BUILD: &[Move] = &[
    HIT_W,
    HIT_H,
    mv(Y, Add, Pulse, 3.6, 1.0),
    // The signature move: the single aperture snaps open into two eyes on
    // the eighths and shut again. Iris-into-eyes is the best transition the
    // pose model owns and at tempo it is unmistakable.
    mv(Gap, Set, Square, 18.0, 0.5),
    mv(Round, Set, Square, 0.85, 0.5),
    // Quantized, never eased: a sidestep that interpolates is a wander.
    mv(X, Add, Alt, 5.0, 2.0),
    mv(Pupil, Mul, Hit, 0.55, 1.0),
];

const STRUT_DROP: &[Move] = &[
    mv(W, Mul, Hit, 1.5, 0.5),
    mv(H, Mul, Hit, 0.5, 0.5),
    mv(Y, Add, Pulse, 4.5, 0.5),
    mv(X, Add, Alt, 7.0, 1.0),
    mv(Gap, Set, Square, 20.0, 0.25),
    mv(Round, Set, Square, 0.85, 0.25),
    // The turntable: one full revolution of the ring gap per beat.
    mv(Sweep, Set, Square, 0.55, 1.0),
    mv(Spin, Add, Ramp, 1.0, 1.0),
    mv(Ring, Mul, Energy(Band::Low), 1.25, 1.0),
];

const STRUT_OUTRO: &[Move] = &[
    HIT_W,
    HIT_H,
    mv(Y, Add, Pulse, 2.0, 1.0),
    mv(Tilt, Add, Sine, 2.0, 4.0),
];

static STRUT: Routine = Routine {
    name: "strut",
    bars: 8.0,
    sections: &[
        Section {
            from: 0.0,
            to: 2.0,
            mood: Mood::Intro,
            moves: STRUT_INTRO,
        },
        Section {
            from: 2.0,
            to: 4.0,
            mood: Mood::Build,
            moves: STRUT_BUILD,
        },
        Section {
            from: 4.0,
            to: 6.0,
            mood: Mood::Drop,
            moves: STRUT_DROP,
        },
        Section {
            from: 6.0,
            to: 8.0,
            mood: Mood::Outro,
            moves: STRUT_OUTRO,
        },
    ],
};

/// The other routine, and deliberately not a variation on the first: where
/// `strut` is a body moving, this one is a machine running. parker is both.
const CIRCUIT_INTRO: &[Move] = &[
    HIT_W,
    HIT_H,
    mv(Spin, Add, Ramp, 1.0, 4.0),
    mv(Sweep, Set, Square, 0.72, 2.0),
];

const CIRCUIT_BUILD: &[Move] = &[
    HIT_W,
    HIT_H,
    mv(Spin, Add, Ramp, 1.0, 2.0),
    mv(Sweep, Set, Square, 0.45, 1.0),
    mv(Pupil, Mul, Energy(Band::Mid), 0.4, 1.0),
    mv(GazeX, Add, Alt, 3.0, 1.0),
];

/// The visualizer: parker stops having a face and becomes the waveform, with
/// amplitude on the bass and frequency on the treble. Reusing `speaking`'s
/// shape for this is most of why it costs nothing.
const CIRCUIT_DROP: &[Move] = &[
    mv(W, Set, Square, 40.0, 1.0),
    mv(H, Set, Square, 2.0, 1.0),
    mv(Round, Set, Square, 0.1, 1.0),
    mv(Pupil, Set, Square, 0.0, 1.0),
    mv(Amp, Set, Energy(Band::Low), 11.0, 1.0),
    mv(Freq, Set, Energy(Band::High), 5.0, 0.5),
    mv(Y, Add, Pulse, 2.5, 0.5),
    mv(Ring, Mul, Hit, 1.2, 1.0),
];

const CIRCUIT_OUTRO: &[Move] = &[
    HIT_W,
    HIT_H,
    mv(Spin, Add, Ramp, -1.0, 2.0),
    mv(Sweep, Set, Square, 0.8, 2.0),
];

static CIRCUIT: Routine = Routine {
    name: "circuit",
    bars: 8.0,
    sections: &[
        Section {
            from: 0.0,
            to: 2.0,
            mood: Mood::Intro,
            moves: CIRCUIT_INTRO,
        },
        Section {
            from: 2.0,
            to: 4.0,
            mood: Mood::Build,
            moves: CIRCUIT_BUILD,
        },
        Section {
            from: 4.0,
            to: 6.0,
            mood: Mood::Drop,
            moves: CIRCUIT_DROP,
        },
        Section {
            from: 6.0,
            to: 8.0,
            mood: Mood::Outro,
            moves: CIRCUIT_OUTRO,
        },
    ],
};

pub static ROUTINES: &[&Routine] = &[&STRUT, &CIRCUIT];

/// One bar in sixteen, ignore the routine and do something stupid.
///
/// The thing that makes choreography feel authored rather than generated is
/// that it is occasionally *wrong*. These are deliberately not better than
/// the routine — they are only different, and they arrive without warning.
pub static SURPRISES: &[&[Move]] = &[
    // A wink: one cell collapses for a bar.
    &[
        HIT_W,
        HIT_H,
        mv(Gap, Set, Square, 18.0, 4.0),
        mv(Squint, Set, Square, 0.06, 4.0),
        mv(Y, Add, Pulse, 2.0, 1.0),
    ],
    // Hide, then reappear on the last beat of the bar.
    &[
        mv(Y, Add, Square, 26.0, 2.0),
        mv(W, Mul, Hit, 1.4, 1.0),
        mv(H, Mul, Hit, 0.7, 1.0),
    ],
    // Spin the ring backwards, fast. A scratch.
    &[
        HIT_W,
        HIT_H,
        mv(Sweep, Set, Square, 0.3, 4.0),
        mv(Spin, Add, Ramp, -2.0, 0.5),
    ],
    // Perfectly, pointedly still, for one bar. In context this is the
    // funniest one available.
    &[mv(Pupil, Mul, Sine, 1.15, 4.0)],
];

// ── the choreographer ───────────────────────────────────────────────────────

/// Inspection, for tests and for the visage port to mirror (see the same
/// note on `beat::Player`). `mood` in particular is the one thing parkerd
/// tells visage about a dance, so it is API even where the terminal face has
/// no use for it.
#[allow(dead_code)]
impl Dance {
    pub fn name(&self) -> &'static str {
        self.routine.name
    }

    pub fn bpm(&self) -> f64 {
        self.clock.bpm
    }

    /// What this instant is, musically. visage reads it to choose a camera.
    pub fn mood(&self, now: f64) -> Mood {
        if self.surprise(now).is_some() {
            // A surprise is its own thing and the camera should not be
            // mid-drop while it happens.
            return Mood::Intro;
        }
        self.section(now).mood
    }
}

pub struct Dance {
    routine: &'static Routine,
    clock: Metronome,
    seed: u64,
}

impl Dance {
    pub fn new(routine: &'static Routine, bpm: f64, started: f64, seed: u64) -> Dance {
        Dance {
            routine,
            clock: Metronome::new(bpm, started),
            seed,
        }
    }

    /// Pick a routine. Seeded by the track when there is one, so the same
    /// song dances the same way twice — which reads as parker knowing it.
    pub fn routine_for(track: Option<&str>, seed: u64) -> &'static Routine {
        let key = match track {
            Some(t) if !t.trim().is_empty() => t
                .bytes()
                .fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)),
            _ => seed,
        };
        ROUTINES[(key as usize) % ROUTINES.len()]
    }

    /// Where we are in the routine, in bars, wrapped to its length.
    fn bar_in_routine(&self, now: f64) -> f64 {
        self.clock.bars(now) % self.routine.bars
    }

    /// Which bar of the whole performance — used for the surprise slot, which
    /// must not repeat every time the routine loops.
    fn absolute_bar(&self, now: f64) -> i64 {
        self.clock.bars(now).floor() as i64
    }

    fn surprise(&self, now: f64) -> Option<&'static [Move]> {
        let bar = self.absolute_bar(now);
        // Never in the first bar: the routine should establish itself before
        // it starts lying.
        if bar < 2 {
            return None;
        }
        let r = super::pose::hash01(bar as u64, self.seed ^ 0x5111);
        (r < 0.0625).then(|| SURPRISES[(r * 1000.0) as usize % SURPRISES.len()])
    }

    pub fn section(&self, now: f64) -> &'static Section {
        let bar = self.bar_in_routine(now);
        self.routine
            .sections
            .iter()
            .find(|s| bar >= s.from && bar < s.to)
            .unwrap_or(&self.routine.sections[0])
    }

    /// Full send: the ink snaps to a new slot every bar during a drop.
    ///
    /// Snapping is not a compromise — `Ink` is deliberately never
    /// interpolated ("a colour crossfade at 1-bit is mud", pose.rs), and a
    /// strobe is exactly a colour that does not interpolate. The existing
    /// constraint is the feature.
    pub fn ink(&self, now: f64) -> Ink {
        if self.section(now).mood != Mood::Drop {
            return Ink::Plain;
        }
        match self.absolute_bar(now).rem_euclid(4) {
            0 => Ink::Accent,
            1 => Ink::Good,
            2 => Ink::Bad,
            _ => Ink::Plain,
        }
    }

    /// Dance drives every field it cares about explicitly; leaving a resting
    /// motion running underneath would add a saccade to a face that is
    /// supposed to be hitting marks.
    pub fn motion(&self) -> Motion {
        Motion::None
    }

    pub fn pose(&self, base: Pose, now: f64) -> Pose {
        let beats = self.clock.beats(now);
        let moves = self
            .surprise(now)
            .unwrap_or_else(|| self.section(now).moves);

        let mut p = base;
        for m in moves {
            let v = m.drive.at(beats, m.div, &self.clock, now);
            let b = base.get(m.field);
            p.set(
                m.field,
                match m.blend {
                    Blend::Add => b + v * m.amp,
                    Blend::Mul => b * (1.0 + v * (m.amp - 1.0)),
                    Blend::Set => v * m.amp,
                },
            );
        }
        p.ink = self.ink(now);
        sane(p)
    }
}

/// Keep a routine from drawing something impossible.
///
/// Moves compose freely and a `Mul` on top of a `Set` can land anywhere, so
/// this is a guardrail rather than a formatter: a negative width silently
/// draws nothing, and a face that vanishes mid-drop looks exactly like a
/// crash. Clamping here means a bad routine looks wrong instead of looking
/// broken, and it is one place rather than every catalog entry.
fn sane(mut p: Pose) -> Pose {
    p.w = p.w.clamp(0.2, 46.0);
    p.h = p.h.clamp(0.2, 19.0);
    p.gap = p.gap.clamp(0.0, 24.0);
    p.round = p.round.clamp(0.0, 1.0);
    p.pupil = p.pupil.max(0.0);
    p.ring = p.ring.clamp(0.0, 24.0);
    p.ring_w = p.ring_w.max(0.0);
    p.sweep = p.sweep.clamp(0.0, 1.0);
    p.squint = p.squint.clamp(0.0, 2.0);
    p.amp = p.amp.clamp(0.0, 14.0);
    p.freq = p.freq.clamp(0.0, 8.0);
    p.x = p.x.clamp(-30.0, 30.0);
    p.y = p.y.clamp(-30.0, 30.0);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Pose {
        crate::face::Expression::Idle.pose()
    }

    fn dance() -> Dance {
        Dance::new(&STRUT, 120.0, 0.0, 0)
    }

    #[test]
    fn a_metronome_counts_beats_at_the_tempo_it_was_given() {
        let m = Metronome::new(120.0, 0.0);
        assert!(
            (m.beats(1.0) - 2.0).abs() < 1e-9,
            "120bpm is two per second"
        );
        assert!((m.bars(2.0) - 1.0).abs() < 1e-9, "four beats is one bar");
        // Nonsense tempos are clamped rather than believed.
        assert!(Metronome::new(0.0, 0.0).bpm >= 40.0);
        assert!(Metronome::new(9000.0, 0.0).bpm <= 220.0);
        // Time before the start is not negative beats.
        assert_eq!(m.beats(-5.0), 0.0);
    }

    #[test]
    fn energy_stays_in_range_and_the_kick_lands_on_the_downbeat() {
        let m = Metronome::new(120.0, 0.0);
        let mut t = 0.0;
        while t < 20.0 {
            for b in [Band::Low, Band::Mid, Band::High] {
                let e = m.energy(b, t);
                assert!((0.0..=1.0).contains(&e), "{b:?} energy {e} at {t}");
            }
            t += 0.01;
        }
        // Beat 0 is a kick, beat 1 is not.
        assert!(m.energy(Band::Low, 0.02) > m.energy(Band::Low, 0.52));
    }

    #[test]
    fn stepped_drives_actually_step() {
        // The difference between dancing and drifting. A `Square` must hold
        // its value across the subdivision and change only at the boundary.
        let m = Metronome::new(120.0, 0.0);
        let a = Drive::Square.at(0.1, 1.0, &m, 0.0);
        let b = Drive::Square.at(0.9, 1.0, &m, 0.0);
        let c = Drive::Square.at(1.1, 1.0, &m, 0.0);
        assert_eq!(a, b, "a square drive drifted inside its subdivision");
        assert_ne!(b, c, "a square drive failed to flip at the boundary");
    }

    #[test]
    fn drives_stay_in_their_declared_polarity() {
        let m = Metronome::new(120.0, 0.0);
        let mut beats = 0.0;
        while beats < 40.0 {
            for d in [Drive::Hit, Drive::Pulse, Drive::Square, Drive::Ramp] {
                let v = d.at(beats, 1.0, &m, beats / 2.0);
                assert!((0.0..=1.0).contains(&v), "{d:?} unipolar gave {v}");
            }
            for d in [Drive::Sine, Drive::Alt] {
                let v = d.at(beats, 1.0, &m, beats / 2.0);
                assert!((-1.0..=1.0).contains(&v), "{d:?} bipolar gave {v}");
            }
            beats += 0.013;
        }
    }

    #[test]
    fn a_zero_subdivision_does_not_divide_by_zero() {
        let m = Metronome::new(120.0, 0.0);
        assert!(Drive::Hit.at(1.5, 0.0, &m, 0.0).is_finite());
    }

    #[test]
    fn the_hit_is_strongest_on_the_downbeat() {
        let m = Metronome::new(120.0, 0.0);
        let on = Drive::Hit.at(0.0, 1.0, &m, 0.0);
        let off = Drive::Hit.at(0.5, 1.0, &m, 0.0);
        assert!(on > off * 3.0, "the hit did not land: {on} vs {off}");
    }

    #[test]
    fn blends_meet_the_base_the_way_they_say_they_do() {
        let p = Pose {
            w: 10.0,
            ..Pose::ZERO
        };
        // Mul at full drive is the multiplier; at zero drive it is the base.
        let full: f64 = 10.0 * (1.0 + 1.0 * (1.32 - 1.0));
        assert!((full - 13.2).abs() < 1e-9);
        assert_eq!(p.w * (1.0 + 0.0 * (1.32 - 1.0)), 10.0);
    }

    #[test]
    fn every_routine_covers_its_whole_length_without_gaps_or_overlaps() {
        for r in ROUTINES {
            let mut at = 0.0;
            for s in r.sections {
                assert!(
                    (s.from - at).abs() < 1e-9,
                    "{}: gap or overlap at bar {}",
                    r.name,
                    s.from
                );
                assert!(s.to > s.from, "{}: empty section", r.name);
                at = s.to;
            }
            assert!(
                (at - r.bars).abs() < 1e-9,
                "{} does not fill its bars",
                r.name
            );
        }
    }

    #[test]
    fn every_section_lands_a_hit_of_some_kind() {
        // Without an impact on the downbeat a section reads as motion rather
        // than as dancing.
        for r in ROUTINES {
            for s in r.sections {
                assert!(
                    s.moves
                        .iter()
                        .any(|m| matches!(m.drive, Drive::Hit | Drive::Pulse | Drive::Energy(_))),
                    "{} / {} has nothing on the beat",
                    r.name,
                    s.mood.name()
                );
            }
        }
    }

    #[test]
    fn a_routine_never_draws_something_impossible() {
        for r in ROUTINES {
            let d = Dance::new(r, 128.0, 0.0, 3);
            let mut t = 0.0;
            while t < 60.0 {
                let p = d.pose(base(), t);
                assert!(p.w > 0.0 && p.w <= 46.0, "{}: w {} at {t}", r.name, p.w);
                assert!(p.h > 0.0 && p.h <= 19.0, "{}: h {} at {t}", r.name, p.h);
                assert!(p.gap >= 0.0, "{}: negative gap", r.name);
                assert!(p.pupil >= 0.0, "{}: negative pupil", r.name);
                assert!(
                    (0.0..=1.0).contains(&p.sweep),
                    "{}: sweep {}",
                    r.name,
                    p.sweep
                );
                for f in Field::ALL {
                    assert!(p.get(f).is_finite(), "{}: {} went wild", r.name, f.name());
                }
                t += 0.008;
            }
        }
    }

    #[test]
    fn the_face_visibly_moves_on_every_beat() {
        let d = dance();
        // Sample the top of a beat and the middle of it; a routine that comes
        // out identical is not dancing.
        let mut differed = 0;
        for beat in 0..32 {
            let t = beat as f64 * 0.5;
            let a = d.pose(base(), t + 0.01);
            let b = d.pose(base(), t + 0.25);
            if (a.w - b.w).abs() > 0.1 || (a.y - b.y).abs() > 0.1 {
                differed += 1;
            }
        }
        assert!(differed > 28, "only {differed}/32 beats moved");
    }

    #[test]
    fn sections_advance_and_the_routine_loops() {
        let d = dance();
        // 120bpm: one bar is two seconds, the routine is eight bars.
        assert_eq!(d.section(0.5).mood, Mood::Intro);
        assert_eq!(d.section(4.5).mood, Mood::Build);
        assert_eq!(d.section(8.5).mood, Mood::Drop);
        assert_eq!(d.section(12.5).mood, Mood::Outro);
        assert_eq!(
            d.section(16.5).mood,
            Mood::Intro,
            "the routine did not loop"
        );
    }

    #[test]
    fn the_ink_only_strobes_in_the_drop() {
        let d = dance();
        assert_eq!(d.ink(0.5), Ink::Plain, "the intro was flashing");
        assert_eq!(d.ink(12.5), Ink::Plain, "the outro was flashing");
        let colours: std::collections::HashSet<Ink> =
            (0..8).map(|i| d.ink(8.5 + i as f64 * 2.0)).collect();
        assert!(colours.len() > 2, "the drop only used {colours:?}");
    }

    #[test]
    fn a_surprise_is_rare_and_never_in_the_opening_bars() {
        let d = Dance::new(&STRUT, 120.0, 0.0, 42);
        let mut surprises = 0;
        for bar in 0..400 {
            let t = bar as f64 * 2.0 + 0.5;
            if d.surprise(t).is_some() {
                assert!(bar >= 2, "a surprise landed in bar {bar}");
                surprises += 1;
            }
        }
        let rate = surprises as f64 / 400.0;
        assert!(
            (0.01..0.14).contains(&rate),
            "surprises fired {:.1}% of bars",
            rate * 100.0
        );
    }

    #[test]
    fn a_surprise_holds_for_its_whole_bar_and_no_longer() {
        let d = Dance::new(&STRUT, 120.0, 0.0, 42);
        let bar = (0..400).find(|b| d.surprise(*b as f64 * 2.0 + 0.5).is_some());
        let bar = bar.expect("no surprise in 400 bars") as f64;
        assert!(d.surprise(bar * 2.0 + 0.1).is_some());
        assert!(d.surprise(bar * 2.0 + 1.9).is_some());
        assert!(
            d.surprise(bar * 2.0 + 2.1).is_none(),
            "a surprise ran past its bar"
        );
    }

    #[test]
    fn the_same_track_always_gets_the_same_routine() {
        let a = Dance::routine_for(Some("Madvillain - Accordion"), 0);
        let b = Dance::routine_for(Some("Madvillain - Accordion"), 999);
        assert_eq!(a.name, b.name, "the seed leaked into a track-keyed choice");
        // And a different track can get a different one.
        let names: std::collections::HashSet<&str> = [
            "Accordion",
            "Rhinestone Cowboy",
            "All Caps",
            "Meat Grinder",
            "Curls",
            "Figaro",
        ]
        .iter()
        .map(|t| Dance::routine_for(Some(t), 0).name)
        .collect();
        assert!(names.len() > 1, "every track picked the same routine");
    }

    #[test]
    fn nothing_is_playing_underneath_the_choreography() {
        assert_eq!(dance().motion(), Motion::None);
    }

    #[test]
    fn routine_names_are_unique() {
        let names: std::collections::HashSet<&str> = ROUTINES.iter().map(|r| r.name).collect();
        assert_eq!(names.len(), ROUTINES.len());
    }
}
