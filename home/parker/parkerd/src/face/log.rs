// A diary of the face: every change to what parker is wearing, appended as it
// happens.
//
// The runtime slot (state.rs) knows only the present, and the present is the
// one thing nobody needs to ask about — it is on the screen. What a slot cannot
// answer is "what was he wearing", "for how long", and above all "why is that
// still on", which is the question that started this file: a work posture left
// over from a turn that should have settled it, and no way to tell whether the
// settle never ran, ran and found nothing, or was overwritten after.
//
// One JSON line per change: `{"t":…, "face":"digging"|null, "until":…, "via":…}`.
// `until` is the deadline the slot was written with (0 = until changed), so
// nothing is ever logged for an *end*: a face ends at the next line or at its
// deadline, whichever comes first, and both are already here. Expiry stays
// what it is everywhere else — evaluated on read.
//
// State dir, not runtime dir — the opposite call from the slot, for the
// opposite reason. A face worn yesterday is not something to restore, but it
// is something to look back at, and the runtime dir is wiped at logout.
//
// Every write is best effort. A diary that cannot be written costs the diary,
// never the face: `set` must work on a full disk.

use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;

/// Past this size the file is trimmed to its newest `KEEP` lines. At ~70
/// bytes a line that is a few thousand changes — days of faces — and the
/// check is one `metadata` call per write.
const TRIM_AT: u64 = 256 * 1024;
const KEEP: usize = 2000;

/// How a change came about. The difference between `clear` and `settle` is
/// the whole point: parker letting go and the turn's end taking a work
/// posture off are the same write and very different events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    /// parker chose a face (`reach face <name>`).
    Wear,
    /// parker let go (`reach face none`).
    Clear,
    /// The turn ended and a work posture came off (`reach settle`).
    Settle,
}

impl Via {
    fn name(self) -> &'static str {
        match self {
            Via::Wear => "wear",
            Via::Clear => "clear",
            Via::Settle => "settle",
        }
    }
}

fn path() -> PathBuf {
    // Tests must never write Peter's real diary, and the build sandbox has
    // no state dir to write to anyway. One diary per test thread, because
    // state.rs's tests append here too and run alongside these.
    if cfg!(test) {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("main").replace("::", "-");
        return std::env::temp_dir()
            .join(format!("parker-test-{}", std::process::id()))
            .join(format!("{name}.log"));
    }
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("parker").join("face.log")
}

/// Write one line. `face` is None when the change took the face off.
pub fn append(face: Option<&str>, until: f64, via: Via, t: f64) {
    let line = json!({ "t": round(t), "face": face, "until": round(until), "via": via.name() });
    let file = path();
    let _ = (|| -> std::io::Result<()> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)?;
        writeln!(f, "{line}")?;
        if f.metadata()?.len() > TRIM_AT {
            trim(&file)?;
        }
        Ok(())
    })();
}

fn trim(file: &PathBuf) -> std::io::Result<()> {
    let raw = std::fs::read_to_string(file)?;
    let lines: Vec<&str> = raw.lines().collect();
    let tail = &lines[lines.len().saturating_sub(KEEP)..];
    // Written beside and renamed over, so a reader never sees half a file.
    let tmp = file.with_extension("log.tmp");
    std::fs::write(&tmp, tail.join("\n") + "\n")?;
    std::fs::rename(tmp, file)
}

fn round(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// One face as it was actually worn: when it went on, how long it stayed,
/// and what ended it.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: f64,
    pub face: String,
    pub via: String,
    /// Seconds it was on. For a face still worn, up to `now`.
    pub secs: f64,
    /// What ended it: `expired`, `→ <next face>`, `let go`, `settled`, or
    /// `worn` if it is still on.
    pub end: String,
}

/// Every entry in the diary, oldest first. Lines that do not parse are
/// skipped rather than fatal: a diary with one torn line is still a diary.
pub fn entries() -> Vec<Value> {
    std::fs::read_to_string(path())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v["t"].is_f64() || v["t"].is_u64())
        .collect()
}

/// Turn the diary into what was worn. The derivation is the reason the log
/// stores no end times: a face ends at its deadline or at the next change,
/// whichever is first, and a clear or settle is an end, not a span.
pub fn spans(entries: &[Value], now: f64) -> Vec<Span> {
    let mut out = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let Some(face) = e["face"].as_str() else {
            continue;
        };
        let start = e["t"].as_f64().unwrap_or(0.0);
        let until = e["until"].as_f64().unwrap_or(0.0);
        let next = entries.get(i + 1);
        let next_t = next.and_then(|n| n["t"].as_f64());

        let (stop, end) = match next_t {
            Some(nt) if until > 0.0 && until <= nt => (until, "expired".to_string()),
            Some(nt) => {
                let n = next.unwrap();
                let end = match (n["face"].as_str(), n["via"].as_str()) {
                    (Some(f), _) => format!("→ {f}"),
                    (None, Some("settle")) => "settled".into(),
                    (None, _) => "let go".into(),
                };
                (nt, end)
            }
            None if until > 0.0 && until <= now => (until, "expired".to_string()),
            None => (now, "worn".to_string()),
        };
        out.push(Span {
            start,
            face: face.to_string(),
            via: e["via"].as_str().unwrap_or("?").to_string(),
            secs: (stop - start).max(0.0),
            end,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(t: f64, face: Option<&str>, until: f64, via: &str) -> Value {
        json!({ "t": t, "face": face, "until": until, "via": via })
    }

    #[test]
    fn a_face_ends_at_the_next_change() {
        let log = [
            e(100.0, Some("digging"), 700.0, "wear"),
            e(130.0, Some("smug"), 150.0, "wear"),
        ];
        let s = spans(&log, 1000.0);
        assert_eq!(s[0].secs, 30.0);
        assert_eq!(s[0].end, "→ smug");
    }

    #[test]
    fn a_face_ends_at_its_deadline_if_that_comes_first() {
        let log = [
            e(100.0, Some("wince"), 115.0, "wear"),
            e(200.0, Some("yes"), 215.0, "wear"),
        ];
        let s = spans(&log, 1000.0);
        assert_eq!(s[0].secs, 15.0);
        assert_eq!(s[0].end, "expired");
        assert_eq!(s[1].end, "expired", "the last face expires too");
    }

    #[test]
    fn a_settle_and_a_clear_end_a_face_and_are_not_faces() {
        let log = [
            e(100.0, Some("pointing"), 700.0, "wear"),
            e(160.0, None, 0.0, "settle"),
            e(200.0, Some("listening"), 0.0, "wear"),
            e(260.0, None, 0.0, "clear"),
        ];
        let s = spans(&log, 1000.0);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].end, "settled");
        assert_eq!(s[1].end, "let go");
    }

    #[test]
    fn a_face_still_on_runs_until_now() {
        let log = [e(100.0, Some("listening"), 0.0, "wear")];
        let s = spans(&log, 400.0);
        assert_eq!(s[0].secs, 300.0);
        assert_eq!(s[0].end, "worn");
    }

    #[test]
    fn the_diary_round_trips_and_trims() {
        let file = path();
        let _ = std::fs::remove_file(&file);
        append(Some("curious"), 120.0, Via::Wear, 100.0);
        append(None, 0.0, Via::Settle, 110.0);
        let got = entries();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0]["face"], "curious");
        assert_eq!(got[1]["via"], "settle");

        // Torn lines are skipped, not fatal.
        std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap()
            .write_all(b"{not json\n")
            .unwrap();
        assert_eq!(entries().len(), 2);

        let big: String = (0..KEEP + 500)
            .map(|i| format!("{{\"t\":{i},\"face\":\"yes\",\"until\":0,\"via\":\"wear\"}}\n"))
            .collect();
        std::fs::write(&file, big.repeat(3)).unwrap();
        append(Some("smug"), 0.0, Via::Wear, 9e9);
        let got = entries();
        assert_eq!(got.len(), KEEP);
        assert_eq!(
            got.last().unwrap()["face"],
            "smug",
            "the newest line survives a trim"
        );
        let _ = std::fs::remove_file(&file);
    }
}
