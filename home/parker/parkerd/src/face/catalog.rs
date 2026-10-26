// The idle and dance catalogs, written out as JSON.
//
// This ile exists to solve one problem, and it is the problem MOTION.md §3
// says the whole plan dies of if nobody solves it.
//
// `visage/src/lib/face.js` is a hand-maintained line-for-line port of the
// pose model, and the rule around it is blunt: "an expression changed there
// is changed here, or it is a bug". That rule survives twelve expressions. It
// does not survive fourteen idle beats and two dance routines — a hundred-odd
// numbers, hand-copied, with no build error when one of them is wrong and no
// symptom except parker glancing slightly further left on one surface than
// the other.
//
// So the beats and routines are *data* (beat.rs, idle.rs, dance.rs are all
// tables), and this writes that data out. visage reads the generated file and
// never transcribes a number. What still has to be ported by hand is the
// hundred lines of *player* — walk the keyframes, apply the drives — and a
// player that is wrong is wrong loudly, on the first frame, in a way no
// amount of staring at a catalog diff would be.
//
//   parker face --catalog --json > visage/src/lib/catalog.json
//
// Sibling to `--sheet --json`, which already exists for exactly this kind of
// consumer: "surfaces that draw dots rather than print braille".
//
// Field names are the JavaScript spellings (`gazeX`, not `gaze_x`) because
// the far side is what reads them, and a mismatch there is a silent miss
// rather than a compile error.

use super::beat::Beat;
use super::dance::{Move, Routine};
use super::idle::{self, Tier};
use super::pose::{Delta, Field, Motion, Pose, Stroke};
use super::{entrance, Expression};
use serde_json::{json, Value};

fn stroke(s: &Stroke) -> Value {
    json!([s.x0, s.y0, s.x1, s.y1, s.w])
}

fn delta(d: &Delta) -> Value {
    let num: Vec<Value> = d
        .num
        .iter()
        .map(|(f, op)| {
            let (kind, v) = op.parts();
            json!([f.name(), kind, v])
        })
        .collect();
    let mut out = json!({ "num": num });
    if let Some(m) = d.mark {
        out["mark"] = Value::Array(m.iter().map(stroke).collect());
    }
    if let Some(i) = d.ink {
        out["ink"] = json!(i.name());
    }
    if let Some(i) = d.mark_ink {
        // Always the padded six, so the JS side applies it without knowing
        // the padding rule.
        let padded = crate::face::pose::Ink::pad(i);
        out["markInk"] = Value::Array(padded.iter().map(|k| json!(k.name())).collect());
    }
    out
}

fn beat(b: &Beat) -> Value {
    json!({
        "name": b.name,
        "retSecs": b.ret_secs,
        "loopFrom": b.loop_from,
        "motion": b.motion.map(|m| m.name()),
        "frames": b.frames.iter().map(|k| {
            let mut f = json!({
                "to": delta(&k.to),
                "secs": k.secs,
                "ease": k.ease.name(),
            });
            // Only when set: most keyframes defer to the beat and keep the
            // blink off, and saying so a thousand times is weight, not data.
            if let Some(m) = k.motion {
                f["motion"] = json!(m.name());
            }
            if k.blink {
                f["blink"] = json!(true);
            }
            f
        }).collect::<Vec<_>>(),
    })
}

fn moves(ms: &[Move]) -> Value {
    Value::Array(
        ms.iter()
            .map(|m| {
                let (drive, band) = m.drive.parts();
                json!({
                    "field": m.field.name(),
                    "blend": m.blend.name(),
                    "drive": drive,
                    "band": band.map(|b| b.name()),
                    "amp": m.amp,
                    "div": m.div,
                })
            })
            .collect(),
    )
}

fn routine(r: &Routine) -> Value {
    json!({
        "name": r.name,
        "bars": r.bars,
        "sections": r.sections.iter().map(|s| json!({
            "from": s.from,
            "to": s.to,
            "mood": s.mood.name(),
            "moves": moves(s.moves),
        })).collect::<Vec<_>>(),
    })
}

fn tier(t: Tier) -> Value {
    let (lo, hi) = t.interval();
    json!({
        "name": t.name(),
        "interval": if lo.is_finite() { json!([lo, hi]) } else { Value::Null },
    })
}

/// Sample times, chosen to land in awkward places: mid-flick, mid-hold, past
/// a loop boundary, and far enough out that a 32-bit hash would have drifted.
const SAMPLES: [f64; 8] = [0.1, 0.31, 1.0, 2.7, 5.55, 13.9, 61.4, 307.25];

/// Reference values, computed here, for the JavaScript port to check itself
/// against (`visage/parity.mjs`).
///
/// The catalog already carries the *data* both sides share. This carries the
/// answers, which is what turns "we copied the numbers" into "we agree" — the
/// player, the saccade and the drives are hand-ported logic, and hand-ported
/// logic is where the divergence this whole file exists to prevent would
/// actually happen. A splitmix64 finalizer written with JavaScript's 53-bit
/// doubles instead of BigInt, for instance, is wrong in a way that still
/// looks like plausible eye movement.
fn parity() -> Value {
    let base = super::Expression::Idle.pose();

    let beats: Vec<Value> = idle::tables()
        .iter()
        .map(|e| &e.beat)
        .map(|b| {
            json!({
                "name": b.name,
                "at": SAMPLES.iter().map(|t| json!({
                    "t": t,
                    "pose": pose_of(&b.at(base, *t)),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    let dances: Vec<Value> = super::dance::ROUTINES
        .iter()
        .map(|r| {
            let d = super::dance::Dance::new(r, 120.0, 0.0, 7);
            json!({
                "name": r.name,
                "bpm": 120.0,
                "seed": 7,
                "at": SAMPLES.iter().map(|t| json!({
                    "t": t,
                    "mood": d.mood(*t).name(),
                    "ink": d.ink(*t).name(),
                    "pose": pose_of(&d.pose(base, *t)),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    json!({
        "base": pose_of(&base),
        "saccade": SAMPLES.iter().map(|t| {
            let (x, y) = super::pose::saccade(*t);
            json!([t, x, y])
        }).collect::<Vec<_>>(),
        "blink": SAMPLES.iter().map(|t| json!([t, super::pose::blink(*t)])).collect::<Vec<_>>(),
        "hash": (0..6).map(|n| json!([n, super::pose::hash01(n, 0x5EED)])).collect::<Vec<_>>(),
        "beats": beats,
        "dance": dances,
        "expressions": expressions(),
        "entrances": entrances_parity(),
    })
}

fn entry(e: &idle::Entry) -> Value {
    let mut v = beat(&e.beat);
    v["tier"] = json!(e.tier.name());
    v
}

/// The vignettes, written out on their own: `parker face --vignettes --json`.
///
/// Apart from the catalog because of their size. A vignette is built, not
/// typed (vignette.rs), and a thousand keyframes flattened out is most of a
/// megabyte — past what belongs in visage's bundle, so visage loads this
/// after first paint instead. Each carries its own reference answers, spread
/// across its whole length, and they cover what a vignette adds to a beat:
/// which motion is playing and whether the blink comes through.
pub fn vignettes() -> Value {
    let tables = idle::tables().len();
    let base = super::Expression::Idle.pose();
    let built = &idle::catalog()[tables..];
    json!({
        "generated": "parker face --vignettes --json — do not edit by hand",
        "beats": built.iter().map(entry).collect::<Vec<_>>(),
        "parity": built.iter().map(|e| {
            let b = &e.beat;
            let at: Vec<Value> = (0..24)
                .map(|i| q3(b.secs() * (i as f64 + 0.37) / 24.0))
                .map(|t| json!({
                    "t": t,
                    "pose": pose_of(&b.at(base, t)),
                    "motion": b.motion_at(super::pose::Motion::Drift, t).name(),
                    "blink": b.blinks_at(t),
                }))
                .collect();
            json!({ "name": b.name, "at": at })
        }).collect::<Vec<_>>(),
    })
}

fn q3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// A pose with nothing left out — numbers, strokes and inks — written
/// sparse: only what differs from `Pose::ZERO`, which the far side fills back
/// in before comparing (parity.mjs), so a field it gets wrong is still caught.
fn pose_full(p: &Pose) -> Value {
    let zero = Pose::ZERO;
    let mut out = serde_json::Map::new();
    for f in Field::ALL {
        if p.get(f) != zero.get(f) {
            out.insert(f.name().into(), json!(p.get(f)));
        }
    }
    if p.mark != zero.mark {
        out.insert(
            "mark".into(),
            Value::Array(p.mark.iter().map(stroke).collect()),
        );
    }
    if p.ink != zero.ink {
        out.insert("ink".into(), json!(p.ink.name()));
    }
    if p.mark_ink != zero.mark_ink {
        out.insert(
            "markInk".into(),
            Value::Array(p.mark_ink.iter().map(|k| json!(k.name())).collect()),
        );
    }
    Value::Object(out)
}

/// The motions of the faces round, sampled where each one does something: a
/// burst of the giggle, the side-eye's check, the hourglass turning over.
/// The older motions have been ported for a year; these are new formulas, and
/// several move strokes, which nothing checked before.
const MOTION_SAMPLES: [(f64, f64); 4] = [(0.07, 0.43), (0.31, 1.27), (0.53, 2.71), (0.9, 13.9)];

fn new_motion(m: Motion) -> bool {
    !matches!(
        m,
        Motion::None
            | Motion::Breathe
            | Motion::Blades
            | Motion::Wave
            | Motion::Nod
            | Motion::Shake
            | Motion::Pulse
            | Motion::Drift
    )
}

/// Every expression as both faces must draw it. `face.js` still ports the
/// expressions by hand (visage/AGENTS.md), and thirty-seven poses with up to
/// six strokes each is past what "line for line" survives unchecked.
fn expressions() -> Value {
    Value::Array(
        Expression::ALL
            .iter()
            .map(|e| {
                let mut v = json!({
                    "name": e.name(),
                    "motion": e.motion().name(),
                    "blinks": e.blinks(),
                    "voluntary": Expression::VOLUNTARY.contains(e),
                    "pose": pose_full(&e.pose()),
                });
                if new_motion(e.motion()) {
                    v["at"] = Value::Array(
                        MOTION_SAMPLES
                            .iter()
                            .map(|(phase, t)| {
                                json!({
                                    "phase": phase,
                                    "t": t,
                                    "pose": pose_full(&e.motion().apply(e.pose(), *phase, *t)),
                                })
                            })
                            .collect(),
                    );
                }
                v
            })
            .collect(),
    )
}

/// Each entrance played from the resting face, at six points through it and
/// one just past the end.
fn entrances_parity() -> Value {
    let origin = Expression::Idle.pose();
    Value::Array(
        entrance::all()
            .map(|(e, b)| {
                let times: Vec<f64> = (0..6)
                    .map(|i| q3(b.secs() * (i as f64 + 0.37) / 6.0))
                    .chain([q3(b.secs() + 0.05)])
                    .collect();
                json!({
                    "name": e.name(),
                    "at": times.iter().map(|t| json!({
                        "t": t,
                        "pose": pose_full(&b.departing(origin, e.pose(), *t)),
                        "motion": b.motion_at(e.motion(), *t).name(),
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

fn pose_of(p: &super::pose::Pose) -> Value {
    Value::Object(
        Field::ALL
            .iter()
            .map(|f| (f.name().to_string(), json!(p.get(*f))))
            .collect(),
    )
}

pub fn json() -> Value {
    json!({
        // A generated file with no note in it gets hand-edited exactly once.
        "generated": "parker face --catalog --json — do not edit by hand",
        "marks": super::pose::MARKS,
        "fields": Field::ALL.iter().map(|f| f.name()).collect::<Vec<_>>(),
        "idle": {
            "tiers": [
                { "at": 0.0, "tier": tier(Tier::Settled) },
                { "at": 45.0, "tier": tier(Tier::Alert) },
                { "at": 1800.0, "tier": tier(Tier::Vignette) },
            ],
            "beats": idle::tables().iter().map(entry).collect::<Vec<_>>(),
            // The vignette phase's draw (idle.rs): how often the long pieces
            // lead. A number both clocks use, so it crosses as data.
            "deepShare": idle::DEEP_SHARE,
            // How long a cued beat waits for quiet, and how long at most
            // (Idle::cue). The same rule on both clocks, so the same data.
            "cueSettle": idle::CUE_SETTLE,
            "cueWait": idle::CUE_WAIT,
        },
        "dance": {
            "routines": super::dance::ROUTINES.iter().map(|r| routine(r)).collect::<Vec<_>>(),
            "surprises": super::dance::SURPRISES.iter().map(|s| moves(s)).collect::<Vec<_>>(),
        },
        // How the gesture faces land (entrance.rs), keyed by expression.
        "entrances": entrance::all()
            .map(|(e, b)| (e.name().to_string(), beat(b)))
            .collect::<serde_json::Map<_, _>>(),
        "parity": parity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_carries_every_beat_and_every_routine() {
        let v = json();
        let beats = v["idle"]["beats"].as_array().unwrap();
        assert_eq!(beats.len(), idle::tables().len());
        let routines = v["dance"]["routines"].as_array().unwrap();
        assert_eq!(routines.len(), super::super::dance::ROUTINES.len());
    }

    #[test]
    fn every_exported_field_name_is_one_the_far_side_can_look_up() {
        // A typo here is a field that silently never moves in visage.
        let v = json();
        let known: std::collections::HashSet<String> =
            Field::ALL.iter().map(|f| f.name().to_string()).collect();
        let mut checked = 0;
        for b in v["idle"]["beats"].as_array().unwrap() {
            for f in b["frames"].as_array().unwrap() {
                for op in f["to"]["num"].as_array().unwrap() {
                    let name = op[0].as_str().unwrap();
                    assert!(known.contains(name), "unknown field {name}");
                    assert!(op[2].as_f64().is_some(), "{name} has no value");
                    checked += 1;
                }
            }
        }
        assert!(checked > 40, "only {checked} ops in the whole catalog");
        for r in v["dance"]["routines"].as_array().unwrap() {
            for s in r["sections"].as_array().unwrap() {
                for m in s["moves"].as_array().unwrap() {
                    let name = m["field"].as_str().unwrap();
                    assert!(known.contains(name), "unknown dance field {name}");
                }
            }
        }
    }

    #[test]
    fn marks_survive_the_crossing_with_all_six_strokes() {
        let v = json();
        let notes = v["idle"]["beats"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == "notes")
            .expect("the notes beat is in the catalog");
        let marked = notes["frames"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["to"]["mark"].is_array())
            .expect("notes draws quavers and must export them");
        let strokes = marked["to"]["mark"].as_array().unwrap();
        assert_eq!(strokes.len(), super::super::pose::MARKS);
        assert_eq!(strokes[0].as_array().unwrap().len(), 5, "x0 y0 x1 y1 w");
    }

    #[test]
    fn every_vignette_goes_out_with_its_answers() {
        let v = vignettes();
        let beats = v["beats"].as_array().unwrap();
        let built = idle::catalog().len() - idle::tables().len();
        assert_eq!(beats.len(), built);
        assert!(beats.iter().any(|b| b["name"] == "nap"));
        let frames = || beats.iter().flat_map(|b| b["frames"].as_array().unwrap());
        assert!(
            frames().any(|f| f["blink"] == true),
            "no keyframe lets a blink through"
        );
        assert!(
            frames().any(|f| f["motion"] == "breathe"),
            "nap's breathing did not cross"
        );
        for p in v["parity"].as_array().unwrap() {
            assert!(
                p["at"].as_array().unwrap().len() >= 20,
                "{} is barely checked",
                p["name"]
            );
        }
    }

    #[test]
    fn the_vignettes_stay_under_their_budget() {
        let text = serde_json::to_string(&vignettes()).unwrap();
        // Loaded after first paint, apart from the bundle — but still sent
        // to the phone (about a tenth of this, gzipped). Past this, the wire
        // format wants a table of shared strokes and inks rather than every
        // keyframe restating its six.
        assert!(text.len() < 1_500_000, "vignettes are {} bytes", text.len());
    }

    #[test]
    fn it_is_valid_json_and_not_enormous() {
        let text = serde_json::to_string(&json()).unwrap();
        assert!(serde_json::from_str::<Value>(&text).is_ok());
        // It ships inside visage's bundle; a catalog that grows past this is
        // a catalog that wants a different delivery mechanism.
        assert!(text.len() < 120_000, "catalog is {} bytes", text.len());
    }
}
