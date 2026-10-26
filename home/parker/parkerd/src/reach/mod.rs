// The output layer: what parker can change.
//
// Admission test, from PARKER.md: can it be described to a person in one
// clause? "Tell me something." "Take me there." Not "spawn a floating window
// at these coordinates."
//
// The verbs have one thing in common: none of them need state. Like sense, they turned out to be one-shot, which is why this
// arrives before the daemon rather than after it.
//
// On gating: PARKER.md has `notify` gated on `interruptible`. That gate
// belongs to parker's *initiative*, not to its obedience — suppressing a
// notification the user just asked for would be a bug, not manners. Every
// call here is user-initiated (a model acting on their behalf), so nothing
// is gated. The gate arrives with the daemon, where unprompted action does.

pub mod look;
pub mod media;
pub mod show;
pub mod visage;

use std::collections::HashMap;
use std::process::Command;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::Value;

/// Verbs report back in words, because their caller is a language model and
/// their result is going into a conversation. `Err` is a plain sentence too.
pub type Done = Result<String, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

impl Urgency {
    pub fn parse(s: &str) -> Urgency {
        match s.to_lowercase().as_str() {
            "low" => Urgency::Low,
            "critical" | "urgent" => Urgency::Critical,
            _ => Urgency::Normal,
        }
    }

    fn hint(self) -> u8 {
        match self {
            Urgency::Low => 0,
            Urgency::Normal => 1,
            Urgency::Critical => 2,
        }
    }
}

/// Tell the user something. Goes to whatever owns
/// org.freedesktop.Notifications — noctalia, on this machine — over the same
/// session bus sense already talks to, so no new dependency and no shelling
/// out to notify-send.
pub fn notify(text: &str, urgency: Urgency) -> Done {
    let text = text.trim();
    if text.is_empty() {
        return Err("nothing to say".into());
    }

    // Two sentences is parker's voice constraint (PARKER.md: constraints,
    // not adjectives). Enforced at the surface rather than trusted to the
    // prompt, because a mechanical check is the whole point of that idea.
    let (summary, body) = match text.split_once(". ") {
        Some((first, rest)) => (format!("{first}."), rest.to_string()),
        None => (text.to_string(), String::new()),
    };

    let conn = Connection::session().map_err(|e| format!("no session bus: {e}"))?;
    let proxy = Proxy::new(
        &conn,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .map_err(|e| format!("no notification daemon: {e}"))?;

    let mut hints: HashMap<&str, Value> = HashMap::new();
    hints.insert("urgency", Value::U8(urgency.hint()));

    let id: u32 = proxy
        .call(
            "Notify",
            &(
                "parker",
                0u32,
                "",
                summary.as_str(),
                body.as_str(),
                Vec::<&str>::new(),
                hints,
                -1i32,
            ),
        )
        .map_err(|e| format!("notification refused: {e}"))?;

    Ok(format!("notified (id {id})"))
}

/// Wear an expression.
///
/// `hold` in seconds; None takes the expression's own default, 0 means until
/// something changes it. Every expression is wearable, `trouble` included —
/// the face is parker's and the surfaces infer nothing (DECISIONS.md) — so
/// this call is how the face moves at all, worn per beat. Nothing else moves
/// it: if the face changed, parker changed it.
///
/// Sound is opt-in per call, never coupled by default: audio has no
/// peripheral-vision equivalent, so it cannot be half-ignored the way a face
/// can. Nothing ships a sound pack yet, and saying so is better than a silent
/// no-op.
pub fn face(name: &str, hold: Option<f64>, sound: bool) -> Done {
    // Letting go is a real thing to want: drop whatever is worn and fall back
    // to whatever the machine says parker is doing.
    if matches!(
        name.trim().to_lowercase().as_str(),
        "none" | "clear" | "release"
    ) {
        crate::face::state::clear().map_err(|e| format!("could not clear face: {e}"))?;
        return Ok("let the face go".into());
    }

    let unknown = || {
        let known: Vec<&str> = crate::face::Expression::WEARABLE
            .iter()
            .map(|e| e.name())
            .collect();
        format!("no wearable expression '{name}'. try: {}", known.join(", "))
    };
    let expr = crate::face::Expression::parse(name)
        .filter(|e| crate::face::Expression::WEARABLE.contains(e))
        .ok_or_else(unknown)?;

    crate::face::state::set(expr, hold).map_err(|e| format!("could not set face: {e}"))?;

    let mut said = format!("wearing {}", expr.name());
    match hold {
        None if expr.is_posture() => said.push_str(&format!(
            " until something changes it (at most {:.0} min)",
            expr.default_hold() / 60.0
        )),
        _ => match hold.unwrap_or_else(|| expr.default_hold()) {
            h if h > 0.0 => said.push_str(&format!(" for {h:.1}s")),
            _ => said.push_str(" until something changes it"),
        },
    }
    if sound {
        said.push_str(&format!(" ({})", play(expr.name())));
    }
    Ok(said)
}

/// The turn is over; take off a work posture if parker left one on.
///
/// Not a face parker chooses, so it is not in the MCP tool list: it is the
/// turn's end saying what is no longer true. A work posture means "mid-work"
/// (`Expression::is_work`), which stops being true when the work stops, and
/// parker reliably forgets to say so. Everything else he wears is left alone.
pub fn settle() -> Done {
    match crate::face::state::settle().map_err(|e| format!("could not settle face: {e}"))? {
        Some(e) => Ok(format!("took off {}", e.name())),
        None => Ok("nothing to take off".into()),
    }
}

/// Dance, or stop dancing.
///
/// A mode rather than an expression, and therefore its own verb rather than
/// another name in `face`: an expression is a pose you interpolate to, and a
/// mode is not. Putting it in the expression slot would mean Esc could not
/// stop a dance without also undressing him, and every surface that reads a
/// face would need one special case for the word that is not a pose.
///
/// Named routines are accepted but `auto` is the interesting one: the surface
/// picks, seeded by whatever is playing, so the same track dances the same way
/// twice. There is no BPM anywhere in sense and there is not going to be — the
/// tempo is parker's own (dance.rs).
///
/// It is a verb, so it is an MCP tool, so parker can start this himself. That
/// is deliberate and it is the point: PARKER.md wants an assistant "capable of
/// opening a YouTube window unprompted because it was funny", and a song
/// coming on that parker likes is exactly that shape.
pub fn dance(routine: &str, hold: Option<f64>, show: bool) -> Done {
    let name = routine.trim().to_lowercase();

    if matches!(name.as_str(), "stop" | "none" | "off" | "clear") {
        crate::face::state::dance::clear().map_err(|e| format!("could not stop dancing: {e}"))?;
        return Ok("stopped dancing".into());
    }

    let known: Vec<&str> = crate::face::dance::ROUTINES
        .iter()
        .map(|r| r.name)
        .collect();
    let chosen = match name.as_str() {
        "" | "auto" => None,
        other if known.contains(&other) => Some(other),
        other => {
            return Err(format!(
                "no routine '{other}'. try: auto, {}",
                known.join(", ")
            ))
        }
    };

    crate::face::state::dance::set(chosen, hold)
        .map_err(|e| format!("could not start dancing: {e}"))?;

    let what = chosen.unwrap_or("auto");
    let mut said = match hold.unwrap_or(crate::face::state::dance::DEFAULT_HOLD) {
        h if h > 0.0 => format!("dancing ({what}) for {h:.0}s"),
        _ => format!("dancing ({what}) until something stops it"),
    };

    // The "friends are over, put it on" half of the gesture. Best-effort: a
    // dance that started is worth more than an error about the window it
    // would have played on, and the dance state is written either way.
    if show {
        match visage::reveal() {
            Ok(msg) => said.push_str(&format!(" — {msg}")),
            Err(e) => said.push_str(&format!(" (but {e})")),
        }
    }
    Ok(said)
}

/// Flip the dance: on if it is off, off if it is on.
///
/// The human-sized gesture, and deliberately the same shape `parker visage`
/// has — one word, no flags, and saying it again changes which way it went.
/// You should not have to remember a routine name, a duration, or which of
/// `stop`/`none`/`off` this verb happens to accept, and you should never be
/// able to strand parker dancing because the word for "stop" would not come
/// to mind. (It happened, within an hour of shipping: `Ctrl+Space` sets no
/// deadline, so a dance toggled on and then forgotten ran until somebody
/// remembered the exact spelling of the verb that ends it.)
///
/// `reach dance` keeps the knobs — a named routine, a duration, `--show` —
/// and stays the one a model calls, the same split `parker face` (the
/// surface) and `reach face` (the choice) already make. This is not a second
/// implementation of it: the start path below *is* that verb, called with
/// the arguments a person would otherwise have had to type.
pub fn dance_toggle() -> Done {
    if crate::face::state::dance::get().is_some() {
        return dance("stop", None, false);
    }
    // No deadline, because the thing that ends it is you saying this again.
    // A timer here would stop the dance mid-song.
    dance("auto", Some(0.0), false)
}

/// Plays `<data>/parker/sounds/<name>.wav` if it is there.
fn play(name: &str) -> String {
    let base = std::env::var("XDG_DATA_HOME")
        .unwrap_or_else(|_| format!("{}/.local/share", std::env::var("HOME").unwrap_or_default()));
    let path = format!("{base}/parker/sounds/{name}.wav");
    if !std::path::Path::new(&path).exists() {
        return "no sound installed for it".into();
    }
    match Command::new("pw-play").arg(&path).spawn() {
        Ok(_) => "with sound".into(),
        Err(e) => format!("sound failed: {e}"),
    }
}

/// Take the user to a workspace.
pub fn goto(space: i64) -> Done {
    if !(1..=10).contains(&space) {
        return Err(format!("workspace {space} is out of range (1-10)"));
    }
    // Where it came from rides along so visage can draw the move (the space
    // numbers, the new one lit) rather than only name it.
    let from = crate::sense::hypr::workspaces()
        .into_iter()
        .find(|w| w.active)
        .map(|w| w.id);
    dispatch(&format!("hl.dsp.focus({{ workspace = {space} }})"))?;
    Ok(match from {
        Some(f) if f != space && f > 0 => format!("moved to workspace {space} (from {f})"),
        _ => format!("moved to workspace {space}"),
    })
}

/// Put an application in front of the user. Matched on window class by
/// substring, then dispatched by address — more robust than handing Hyprland
/// a regex and hoping.
pub fn focus(app: &str) -> Done {
    let needle = app.trim().to_lowercase();
    if needle.is_empty() {
        return Err("no application named".into());
    }

    let clients = crate::sense::hypr::clients();
    let hit = clients
        .iter()
        .find(|c| c.class.to_lowercase() == needle)
        .or_else(|| {
            clients
                .iter()
                .find(|c| c.class.to_lowercase().contains(&needle))
        })
        .or_else(|| {
            clients
                .iter()
                .find(|c| c.title.to_lowercase().contains(&needle))
        })
        .ok_or_else(|| {
            let open: Vec<&str> = clients.iter().map(|c| c.class.as_str()).collect();
            format!(
                "nothing matching '{app}' is open. open: {}",
                open.join(", ")
            )
        })?;

    dispatch(&format!(
        "hl.dsp.focus({{ window = {} }})",
        lua_str(&format!("address:{}", hit.address))
    ))?;
    Ok(format!(
        "focused {} (workspace {})",
        hit.class, hit.workspace
    ))
}

/// Run one Hyprland dispatcher, written as a Lua expression.
///
/// Since the Lua config (home/desktop/hyprland.nix, Oct 2026) `hyprctl
/// dispatch X` is evaluated as `return hl.dispatch(X)`, so the argument is
/// Lua — `hl.dsp.focus({ workspace = 3 })`, not `workspace 3`. The old words
/// are not deprecated, they are a Lua syntax error, and every verb that
/// moved anything (goto, focus, visage) failed at the first boot on the new
/// config. Forms verified live on 0.56.2: `exec_cmd("…")`, `focus({
/// workspace = n })`, `focus({ window = "address:0x…" })`,
/// `workspace.toggle_special("name")`, `cursor.move({ x = n, y = n })`.
///
/// One argument, not split on spaces: it is an expression now, and strings
/// inside it go through `lua_str`.
fn dispatch(lua: &str) -> Result<(), String> {
    let his = crate::sense::hypr::instance_signature()
        .ok_or_else(|| "no Hyprland session to talk to".to_string())?;
    let out = Command::new("hyprctl")
        .env("HYPRLAND_INSTANCE_SIGNATURE", his)
        .arg("dispatch")
        .arg(lua)
        .output()
        .map_err(|e| format!("hyprctl not runnable: {e}"))?;

    let reply = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !reply.trim().eq_ignore_ascii_case("ok") {
        return Err(format!("hyprland refused: {}", reply.trim()));
    }
    Ok(())
}

/// A Lua string literal, for text spliced into a `dispatch` expression.
fn lua_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgency_parses_loosely_and_defaults_to_normal() {
        assert_eq!(Urgency::parse("low"), Urgency::Low);
        assert_eq!(Urgency::parse("CRITICAL"), Urgency::Critical);
        assert_eq!(Urgency::parse("urgent"), Urgency::Critical);
        assert_eq!(Urgency::parse("whatever"), Urgency::Normal);
    }

    #[test]
    fn lua_strings_cannot_break_out_of_their_quotes() {
        assert_eq!(lua_str("visage"), r#""visage""#);
        assert_eq!(lua_str(r#"a "b" \c"#), r#""a \"b\" \\c""#);
        assert_eq!(lua_str("two\nlines"), r#""two\nlines""#);
    }

    #[test]
    fn empty_notification_is_refused_before_it_touches_the_bus() {
        assert!(notify("   ", Urgency::Normal).is_err());
    }

    #[test]
    fn out_of_range_workspace_is_refused_before_it_touches_hyprland() {
        assert!(goto(0).is_err());
        assert!(goto(99).is_err());
    }

    #[test]
    fn focus_with_no_name_is_refused() {
        assert!(focus("  ").is_err());
    }

    /// The dance slot is one file per machine and cargo runs tests in
    /// threads, so everything that writes it has to take a turn. Poisoning is
    /// ignored deliberately: one failing test must not turn every other one
    /// into a lock panic instead of its own result.
    static DANCING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn dance_guard() -> std::sync::MutexGuard<'static, ()> {
        DANCING.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn an_unknown_routine_is_refused_and_says_what_there_is() {
        let err = dance("breakdance", None, false).unwrap_err();
        assert!(err.contains("auto"), "the error did not list the options");
        assert!(err.contains("strut"));
    }

    #[test]
    fn the_toggle_always_gets_you_back_out() {
        // The property that makes it the human-sized gesture: whatever state
        // parker is in, saying it once more ends the dance. No spelling to
        // remember, and no way to strand him — which is exactly how a dance
        // got left running indefinitely on the first day.
        let _g = dance_guard();
        crate::face::state::dance::clear().unwrap();

        dance_toggle().unwrap();
        assert!(
            crate::face::state::dance::get().is_some(),
            "the first toggle did not start anything"
        );

        dance_toggle().unwrap();
        assert_eq!(
            crate::face::state::dance::get(),
            None,
            "the second toggle did not stop it"
        );

        // And it stops a dance somebody else started with the full verb,
        // however they spelled it.
        dance("circuit", Some(0.0), false).unwrap();
        dance_toggle().unwrap();
        assert_eq!(crate::face::state::dance::get(), None);
    }

    #[test]
    fn the_toggle_leaves_no_deadline_on_a_dance_it_starts() {
        // A timer here would end the dance mid-song. What ends it is the
        // user saying the word again.
        let _g = dance_guard();
        crate::face::state::dance::clear().unwrap();
        let msg = dance_toggle().unwrap();
        assert!(
            msg.contains("until something stops it"),
            "toggled dance reported: {msg}"
        );
        crate::face::state::dance::clear().unwrap();
    }

    #[test]
    fn stopping_dancing_has_several_spellings_because_a_model_will_pick_one() {
        let _g = dance_guard();
        for word in ["stop", "none", "off", "clear", "  STOP "] {
            assert!(
                dance(word, None, false).is_ok(),
                "{word} was not understood"
            );
        }
        assert_eq!(crate::face::state::dance::get(), None);
    }
}
