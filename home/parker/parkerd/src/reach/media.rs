// The music, as a verb: transport over MPRIS, volume over wpctl. One verb
// for both because the clause is the same — "pause it", "skip this", "turn
// it down" — and a second verb for loudness would be the third abstraction
// nobody needs.
//
// Player discovery mirrors sense/media.rs (same well-known-name filter, so
// noctalia's proxy is skipped for free) and the report re-reads the player
// after acting: the caller is a model, and what is playing *now* is half
// the sentence it is about to write.

use super::Done;
use std::process::Command;
use zbus::blocking::{fdo::DBusProxy, Connection, Proxy};

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const IFACE: &str = "org.mpris.MediaPlayer2.Player";

/// Nudge size for louder/quieter. Five percent is one notch of a volume
/// knob: audible, never a jump scare.
const STEP: &str = "5%";

pub fn media(action: &str, player: Option<&str>, percent: Option<f64>) -> Done {
    let action = action.trim().to_lowercase();
    match action.as_str() {
        "play" | "pause" | "toggle" | "next" | "previous" | "prev" => transport(&action, player),
        "louder" | "quieter" | "volume" => volume(&action, percent),
        other => Err(format!(
            "no media action '{other}'. try: play, pause, toggle, next, previous, \
             louder, quieter, volume"
        )),
    }
}

fn method_for(action: &str) -> &'static str {
    match action {
        "play" => "Play",
        "pause" => "Pause",
        "toggle" => "PlayPause",
        "next" => "Next",
        _ => "Previous",
    }
}

fn transport(action: &str, wanted: Option<&str>) -> Done {
    let conn = Connection::session().map_err(|e| format!("no session bus: {e}"))?;
    let bus = DBusProxy::new(&conn).map_err(|e| format!("bus would not answer: {e}"))?;
    let names: Vec<String> = bus
        .list_names()
        .map_err(|e| format!("bus would not answer: {e}"))?
        .into_iter()
        .map(|n| n.as_str().to_string())
        .filter(|n| n.starts_with(PREFIX))
        .collect();

    let name = pick(&conn, &names, wanted)?;
    let proxy =
        Proxy::new(&conn, name.as_str(), PATH, IFACE).map_err(|e| format!("no player: {e}"))?;
    proxy
        .call::<_, _, ()>(method_for(action), &())
        .map_err(|e| format!("{} refused: {e}", short(&name)))?;

    // Best-effort read-back; a skip can lag a beat and that is fine.
    let now = crate::sense::media::read(&conn, &name);
    let mut said = format!("{action} — {}", short(&name));
    if let Some(m) = now {
        match (m.artist, m.title) {
            (Some(a), Some(t)) => said.push_str(&format!(": {a}, \"{t}\" ({})", m.status)),
            (None, Some(t)) => said.push_str(&format!(": \"{t}\" ({})", m.status)),
            _ => said.push_str(&format!(" ({})", m.status)),
        }
    }
    Ok(said)
}

/// The player that matters: a named one if asked, else the one making
/// noise, else whoever answered first — a paused Spotify is still the thing
/// "play" means.
fn pick(conn: &Connection, names: &[String], wanted: Option<&str>) -> Result<String, String> {
    if names.is_empty() {
        return Err("nothing is open that can play anything".into());
    }
    if let Some(w) = wanted {
        let needle = w.trim().to_lowercase();
        return names
            .iter()
            .find(|n| short(n).to_lowercase().contains(&needle))
            .cloned()
            .ok_or_else(|| {
                let have: Vec<&str> = names.iter().map(|n| short(n)).collect();
                format!("no player matching '{w}'. here: {}", have.join(", "))
            });
    }
    Ok(names
        .iter()
        .find(|n| {
            crate::sense::media::read(conn, n)
                .map(|m| m.status == "Playing")
                .unwrap_or(false)
        })
        .unwrap_or(&names[0])
        .clone())
}

fn short(name: &str) -> &str {
    name.trim_start_matches(PREFIX)
}

fn volume(action: &str, percent: Option<f64>) -> Done {
    let arg = volume_arg(action, percent)?;
    let before = current_volume();
    // `-l 1.0` caps the nudges at unity gain: a model must not be able to
    // walk the sink past 100% five percent at a time.
    let out = Command::new("wpctl")
        .args(["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &arg])
        .output()
        .map_err(|e| format!("wpctl not runnable: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "wpctl refused: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    // Stamped as parker's, so door's level watch leaves it to the ledger
    // row this call already draws (controls/sound.rs, "whose hand").
    crate::controls::stamp_sink();
    // "volume 42% → 50%": the level it left rides along, so visage can draw
    // the change as cells going from one to the other.
    match (before, current_volume()) {
        (Some(was), Some(now)) => Ok(moved(&was, &now)),
        (_, Some(now)) => Ok(now),
        _ => Ok("volume changed".into()),
    }
}

/// `volume 42%` and `volume 50%` → `volume 42% → 50%`; the mute flag stays
/// on whichever side has it.
fn moved(was: &str, now: &str) -> String {
    if was == now {
        return now.to_string();
    }
    let tail = now.strip_prefix("volume ").unwrap_or(now);
    format!("{was} → {tail}")
}

fn volume_arg(action: &str, percent: Option<f64>) -> Result<String, String> {
    match action {
        "louder" => Ok(format!("{STEP}+")),
        "quieter" => Ok(format!("{STEP}-")),
        _ => {
            let p = percent.ok_or("volume needs a percent (0-100)")?;
            Ok(format!("{}%", p.clamp(0.0, 100.0).round() as u32))
        }
    }
}

fn current_volume() -> Option<String> {
    let out = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .ok()?;
    parse_volume(&String::from_utf8_lossy(&out.stdout))
}

/// "Volume: 0.45" or "Volume: 0.45 [MUTED]" → "volume 45%" (+ the mute flag,
/// because reporting 45% while the sink is muted would be a lie of omission).
fn parse_volume(s: &str) -> Option<String> {
    let num: f64 = s.split_whitespace().nth(1)?.parse().ok()?;
    let pct = (num * 100.0).round() as u32;
    Some(if s.contains("MUTED") {
        format!("volume {pct}% (muted)")
    } else {
        format!("volume {pct}%")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_action_is_refused_and_says_what_there_is() {
        let err = media("blast", None, None).unwrap_err();
        assert!(err.contains("louder"), "options missing from: {err}");
    }

    #[test]
    fn transport_actions_map_to_their_mpris_methods() {
        assert_eq!(method_for("toggle"), "PlayPause");
        assert_eq!(method_for("next"), "Next");
        assert_eq!(method_for("prev"), "Previous");
    }

    #[test]
    fn volume_needs_its_percent_and_clamps_it() {
        assert!(volume_arg("volume", None).is_err());
        assert_eq!(volume_arg("volume", Some(45.0)).unwrap(), "45%");
        assert_eq!(volume_arg("volume", Some(250.0)).unwrap(), "100%");
        assert_eq!(volume_arg("volume", Some(-5.0)).unwrap(), "0%");
    }

    #[test]
    fn nudges_are_one_notch_either_way() {
        assert_eq!(volume_arg("louder", None).unwrap(), "5%+");
        assert_eq!(volume_arg("quieter", None).unwrap(), "5%-");
    }

    #[test]
    fn a_volume_change_says_where_it_came_from() {
        assert_eq!(moved("volume 42%", "volume 50%"), "volume 42% → 50%");
        assert_eq!(moved("volume 50%", "volume 50%"), "volume 50%");
        assert_eq!(moved("volume 42% (muted)", "volume 45% (muted)"), "volume 42% (muted) → 45% (muted)");
    }

    #[test]
    fn wpctl_output_parses_including_the_mute_flag() {
        assert_eq!(parse_volume("Volume: 0.45").unwrap(), "volume 45%");
        assert_eq!(
            parse_volume("Volume: 0.45 [MUTED]").unwrap(),
            "volume 45% (muted)"
        );
        assert!(parse_volume("garbage").is_none());
    }
}
