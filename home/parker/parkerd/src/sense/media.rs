// Media, over MPRIS on the session bus.
//
// Every failure path here returns None. A missing D-Bus, a player that
// disappeared mid-read, a metadata field with a type nobody expected — none
// of that is worth a diagnostic, because "nothing is playing" and "I couldn't
// tell what is playing" are the same fact from parker's side.
//
// Note there are usually two candidates on this machine: the real player and
// `dev.noctalia.Mpris`, which the shell exposes as a proxy. Only well-known
// names under org.mpris.MediaPlayer2.* are considered, so the proxy is
// skipped for free.

use super::Media;
use std::collections::HashMap;
use zbus::blocking::{fdo::DBusProxy, Connection, Proxy};
use zbus::zvariant::OwnedValue;

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const IFACE: &str = "org.mpris.MediaPlayer2.Player";

pub fn current() -> Option<Media> {
    let conn = Connection::session().ok()?;
    let names = DBusProxy::new(&conn).ok()?.list_names().ok()?;

    let players: Vec<String> = names
        .into_iter()
        .map(|n| n.as_str().to_string())
        .filter(|n| n.starts_with(PREFIX))
        .collect();

    // With several players up, the one actually making noise is the one that
    // matters. Otherwise take whatever is there — a paused Spotify is still
    // worth knowing about.
    let mut fallback = None;
    for name in players {
        match read(&conn, &name) {
            Some(m) if m.status == "Playing" => return Some(m),
            Some(m) if fallback.is_none() => fallback = Some(m),
            _ => {}
        }
    }
    fallback
}

/// Shared with reach/media.rs, which reads the player back after acting on
/// it — one reader, so the report and the sense block can never disagree.
pub(crate) fn read(conn: &Connection, name: &str) -> Option<Media> {
    let proxy = Proxy::new(conn, name, PATH, IFACE).ok()?;
    let status: String = proxy.get_property("PlaybackStatus").ok()?;
    let meta: HashMap<String, OwnedValue> = proxy.get_property("Metadata").unwrap_or_default();

    Some(Media {
        player: name.trim_start_matches(PREFIX).to_string(),
        status,
        title: meta.get("xesam:title").and_then(as_text),
        artist: meta.get("xesam:artist").and_then(as_text),
    })
}

/// MPRIS is loose about types: `xesam:title` is a string, `xesam:artist` is
/// an array of them, and players disagree about the rest. Try both shapes and
/// give up quietly.
fn as_text(v: &OwnedValue) -> Option<String> {
    if let Ok(s) = String::try_from(v.clone()) {
        return non_empty(s);
    }
    if let Ok(list) = Vec::<String>::try_from(v.clone()) {
        return list.into_iter().find_map(non_empty);
    }
    None
}

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}
