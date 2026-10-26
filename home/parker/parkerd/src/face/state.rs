// Where a chosen expression lives between the process that sets it and the
// process that draws it.
//
// One line in the runtime dir: `<expression> <unix-deadline-or-0>`. That is
// the whole mechanism. It is deliberately a file and not a socket, because
// the daemon does not exist yet and this needs to work today — `parker reach
// face curious` from an MCP call has to reach a `parker face --animate`
// running in another terminal. When the daemon arrives it owns this state in
// memory and this file goes away; nothing above it changes.
//
// Runtime dir, not state dir: a face parker was wearing yesterday is not
// something to restore at boot. It is something to look back at, though, so
// every change written here is also written to the diary (log.rs).

use super::Expression;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn path() -> PathBuf {
    slot("face")
}

fn slot(name: &str) -> PathBuf {
    let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(base).join("parker").join(name)
}

fn write(file: PathBuf, line: String) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(file, line)
}

/// A `<word> <deadline>` line, if it is there and has not expired. The shape
/// every slot in the runtime dir uses, and the reason expiry costs nothing:
/// it is evaluated on read, so no timer has to be running for a thing to end.
fn read(file: PathBuf) -> Option<String> {
    let raw = std::fs::read_to_string(file).ok()?;
    let mut parts = raw.split_whitespace();
    let word = parts.next()?.to_string();
    let deadline: f64 = parts.next()?.parse().ok()?;
    if deadline > 0.0 && now() >= deadline {
        return None;
    }
    Some(word)
}

fn remove(file: PathBuf) -> io::Result<()> {
    match std::fs::remove_file(file) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Wear an expression. `hold` in seconds; `None` uses the expression's own
/// default, and a hold of 0 means "until something changes it".
pub fn set(expr: Expression, hold: Option<f64>) -> io::Result<()> {
    // A negative hold is nonsense a model could easily emit; clamping it to
    // "until changed" is the least surprising reading, and stops it meaning
    // "already expired" by accident.
    let hold = hold.unwrap_or_else(|| expr.default_hold()).max(0.0);
    let t = now();
    let deadline = if hold > 0.0 { t + hold } else { 0.0 };
    write(path(), format!("{} {:.3}\n", expr.name(), deadline))?;
    super::log::append(Some(expr.name()), deadline, super::log::Via::Wear, t);
    Ok(())
}

/// The currently worn expression, if one is set and has not expired.
/// Expiry is evaluated on read rather than by a timer — nothing needs to be
/// running for a beat to end.
pub fn get() -> Option<Expression> {
    Expression::parse(&read(path())?)
}

/// Let go of whatever is worn.
pub fn clear() -> io::Result<()> {
    remove(path())?;
    super::log::append(None, 0.0, super::log::Via::Clear, now());
    Ok(())
}

/// The turn is over: take off a work posture if one is still worn, and leave
/// anything else alone. Returns what came off.
///
/// A reaction is left to expire on its own clock — a smug set as the last
/// word of a reply should get its twenty seconds — and the state postures
/// (listening, away, …) are allowed to outlast a turn. Only the faces that
/// mean "mid-work" are false once the work stops (`Expression::is_work`).
///
/// Logged only when something came off. A settle that found nothing is not
/// a change, and a line for it would end whatever reaction or state posture
/// is still legitimately on.
pub fn settle() -> io::Result<Option<Expression>> {
    match get() {
        Some(e) if e.is_work() => {
            remove(path())?;
            super::log::append(None, 0.0, super::log::Via::Settle, now());
            Ok(Some(e))
        }
        _ => Ok(None),
    }
}

/// Whether parker is dancing, and to what.
///
/// A second line in the same runtime dir, on purpose: dance is a *mode*, not
/// an expression, and putting it in the expression slot would make it a
/// name in `reach face` — so Esc could not end a dance without also
/// undressing him, and every surface reading a face would have to special-
/// case one word that is not a pose. Separate slot, same mechanism, same
/// expiry-on-read.
///
/// The value is a routine name, or `auto` to let the surface choose (seeded
/// by whatever is playing, so the same track dances the same way twice).
pub mod dance {
    use super::{now, read, remove, slot, write};
    use std::io;

    /// How long a dance runs when nobody said. Long enough to be a
    /// performance rather than a twitch, short enough that a model deciding
    /// to dance unprompted cannot leave parker doing it all afternoon.
    pub const DEFAULT_HOLD: f64 = 60.0;

    pub fn set(routine: Option<&str>, hold: Option<f64>) -> io::Result<()> {
        let hold = hold.unwrap_or(DEFAULT_HOLD).max(0.0);
        let deadline = if hold > 0.0 { now() + hold } else { 0.0 };
        let name = routine
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .unwrap_or("auto");
        write(slot("dance"), format!("{name} {deadline:.3}\n"))
    }

    /// The routine parker is dancing to, if he is.
    pub fn get() -> Option<String> {
        read(slot("dance"))
    }

    pub fn clear() -> io::Result<()> {
        remove(slot("dance"))
    }
}

/// "Play this beat, now, on every face."
///
/// A third slot beside `face` and `dance`, and unlike both it is a cue rather
/// than a state: nothing is being worn and no mode is on. It exists so a beat
/// can be seen on demand — `parker face --play glance-left` — instead of only
/// when the idle clock happens to roll it, which is the difference between
/// iterating on an animation and waiting twenty minutes for one.
///
/// Surfaces key change-detection on the whole raw line rather than the name:
/// the deadline differs per write, so the same beat cued twice running plays
/// twice.
pub mod beat {
    use super::{now, remove, slot, write};
    use std::io;

    /// Short on purpose. A cue is "now"; one that outlived its moment would
    /// replay on the next surface to start up and read.
    const HOLD: f64 = 4.0;

    pub fn set(name: &str) -> io::Result<()> {
        write(slot("beat"), format!("{name} {:.3}\n", now() + HOLD))
    }

    /// The raw line, deadline included, if it has not expired.
    pub fn get_raw() -> Option<String> {
        let raw = std::fs::read_to_string(slot("beat")).ok()?;
        let deadline: f64 = raw.split_whitespace().nth(1)?.parse().ok()?;
        if now() >= deadline {
            return None;
        }
        Some(raw.trim().to_string())
    }

    /// Nothing needs this at runtime — a cue ends by expiring, not by being
    /// taken down — but a slot you can write and never remove is half an API.
    #[allow(dead_code)]
    pub fn clear() -> io::Result<()> {
        remove(slot("beat"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// These share one path per process, so they must not run concurrently.
    /// Rust runs tests in threads by default, hence the guard. Poisoning is
    /// ignored deliberately — one failing test must not cascade into every
    /// other test in this module reporting a lock panic instead of its own
    /// result.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn a_worn_expression_comes_back() {
        let _g = guard();
        set(Expression::Curious, Some(60.0)).unwrap();
        assert_eq!(get(), Some(Expression::Curious));
        clear().unwrap();
    }

    #[test]
    fn a_beat_expires_on_its_own() {
        let _g = guard();
        // Written with a deadline already in the past. Nothing has to be
        // running for this to end — expiry is evaluated on read.
        std::fs::create_dir_all(path().parent().unwrap()).unwrap();
        std::fs::write(path(), format!("wince {:.3}\n", now() - 1.0)).unwrap();
        assert_eq!(get(), None, "an elapsed beat should have let go");
        clear().unwrap();
    }

    #[test]
    fn a_beat_cue_reads_back_raw_and_clears() {
        let _g = guard();
        beat::set("glance-left").unwrap();
        let raw = beat::get_raw().expect("a fresh cue reads back");
        assert!(raw.starts_with("glance-left "), "raw line keeps the name");
        beat::clear().unwrap();
        assert_eq!(beat::get_raw(), None);
    }

    #[test]
    fn a_negative_hold_does_not_secretly_mean_expired() {
        let _g = guard();
        set(Expression::Wince, Some(-5.0)).unwrap();
        assert_eq!(get(), Some(Expression::Wince), "clamped to 'until changed'");
        clear().unwrap();
    }

    #[test]
    fn a_zero_hold_persists() {
        let _g = guard();
        set(Expression::Amused, Some(0.0)).unwrap();
        assert_eq!(get(), Some(Expression::Amused));
        clear().unwrap();
    }

    #[test]
    fn clearing_is_idempotent_and_leaves_nothing_worn() {
        let _g = guard();
        set(Expression::Yes, Some(60.0)).unwrap();
        clear().unwrap();
        clear().unwrap();
        assert_eq!(get(), None);
    }

    #[test]
    fn a_beat_defaults_to_letting_go() {
        let _g = guard();
        set(Expression::Surprised, None).unwrap();
        let raw = std::fs::read_to_string(path()).unwrap();
        let deadline: f64 = raw.split_whitespace().nth(1).unwrap().parse().unwrap();
        assert!(deadline > 0.0, "a beat with no duration must still expire");
        clear().unwrap();
    }

    #[test]
    fn settling_takes_off_work_and_nothing_else() {
        let _g = guard();
        set(Expression::Pointing, None).unwrap();
        assert_eq!(settle().unwrap(), Some(Expression::Pointing));
        assert_eq!(get(), None, "a work posture should not outlast the turn");

        for kept in [Expression::Listening, Expression::Away, Expression::Smug] {
            set(kept, None).unwrap();
            assert_eq!(settle().unwrap(), None);
            assert_eq!(get(), Some(kept), "{} should survive settling", kept.name());
        }

        clear().unwrap();
        assert_eq!(settle().unwrap(), None, "settling nothing is fine");
    }

    #[test]
    fn a_posture_with_no_duration_is_capped_not_forever() {
        let _g = guard();
        set(Expression::Digging, None).unwrap();
        let raw = std::fs::read_to_string(path()).unwrap();
        let deadline: f64 = raw.split_whitespace().nth(1).unwrap().parse().unwrap();
        assert!(
            deadline > 0.0,
            "a forgotten posture must still let go eventually"
        );
        assert!(deadline - now() <= super::super::POSTURE_CAP + 1.0);
        clear().unwrap();
    }

    #[test]
    fn garbage_reads_as_nothing_worn() {
        let _g = guard();
        std::fs::create_dir_all(path().parent().unwrap()).unwrap();
        std::fs::write(path(), "not-an-expression 0\n").unwrap();
        assert_eq!(get(), None);
        clear().unwrap();
    }
}
