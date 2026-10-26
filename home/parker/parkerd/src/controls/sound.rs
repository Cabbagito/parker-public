// SOUND: all of it, each app making sound, the mic, mute.
//
// Read from one `pw-dump`, written with `wpctl` (pactl is not installed).
// A level here is wpctl's: the cube root of PipeWire's linear channel
// volume, which is what every volume knob since PulseAudio has meant by a
// percent. `pw-dump` hands over the linear one, so the root is taken here,
// and the two always agree (sink at 0.857 linear = wpctl's 0.95).
//
// The mixer is one row per app with a stream open — Spotify while it
// plays, Discord during a call, a game while it runs. WirePlumber keeps
// each app's level by `application.name` (stream-restore), so a level set
// here comes back the next time the app opens a stream. An app's level
// multiplies with `all sound`.
//
// Proton games name their streams after the exe, sometimes twice — the exe
// and a pretty name, `Titanfall2.exe` and `Titanfall 2` — and launchers
// open streams of their own. `friendly` turns both of a game's into one
// row and drops the launchers.

use super::{level, run, toggle, Changed, Kind, Row, Set};
use serde_json::Value;

const SINK: &str = "@DEFAULT_AUDIO_SINK@";
const SOURCE: &str = "@DEFAULT_AUDIO_SOURCE@";

pub fn rows() -> Vec<Row> {
    let Some(dump) = dump() else {
        return Vec::new();
    };
    rows_from(&dump)
}

fn dump() -> Option<Vec<Value>> {
    let out = std::process::Command::new("pw-dump").output().ok()?;
    if !out.status.success() {
        return None;
    }
    match serde_json::from_slice(&out.stdout).ok()? {
        Value::Array(a) => Some(a),
        _ => None,
    }
}

/// One node as the rows need it.
#[derive(Debug, Clone)]
struct Node {
    id: i64,
    class: String,
    name: String,
    app: String,
    binary: String,
    nick: String,
    description: String,
    bus: String,
    level: i64,
    muted: bool,
    running: bool,
}

fn nodes(dump: &[Value]) -> Vec<Node> {
    dump.iter()
        .filter(|o| o["type"] == "PipeWire:Interface:Node")
        .filter_map(|o| {
            let props = &o["info"]["props"];
            let s = |k: &str| props[k].as_str().unwrap_or_default().to_string();
            let p = o["info"]["params"]["Props"]
                .as_array()?
                .iter()
                .find(|p| p.get("channelVolumes").is_some())?;
            let linear = p["channelVolumes"]
                .as_array()?
                .iter()
                .filter_map(Value::as_f64)
                .fold(0.0_f64, f64::max);
            Some(Node {
                id: o["id"].as_i64()?,
                class: s("media.class"),
                name: s("node.name"),
                app: s("application.name"),
                binary: s("application.process.binary"),
                nick: s("node.nick"),
                description: s("node.description"),
                bus: s("device.bus"),
                level: (linear.cbrt() * 100.0).round() as i64,
                muted: p["mute"].as_bool().unwrap_or(false),
                running: o["info"]["state"] == "running",
            })
        })
        .collect()
}

/// The default sink's and source's node names, from the `default` metadata.
fn defaults(dump: &[Value]) -> (String, String) {
    let mut sink = String::new();
    let mut source = String::new();
    for o in dump {
        if o["type"] != "PipeWire:Interface:Metadata" || o["props"]["metadata.name"] != "default" {
            continue;
        }
        for m in o["metadata"].as_array().into_iter().flatten() {
            let name = m["value"]["name"].as_str().unwrap_or_default().to_string();
            match m["key"].as_str() {
                Some("default.audio.sink") => sink = name,
                Some("default.audio.source") => source = name,
                _ => {}
            }
        }
    }
    (sink, source)
}

fn rows_from(dump: &[Value]) -> Vec<Row> {
    let all = nodes(dump);
    let (sink_name, source_name) = defaults(dump);
    let pick = |class: &str, want: &str| {
        all.iter()
            .find(|n| n.class == class && !want.is_empty() && n.name == want)
            .or_else(|| all.iter().find(|n| n.class == class))
            .cloned()
    };
    let mut out = Vec::new();
    let sink = pick("Audio/Sink", &sink_name);
    if let Some(s) = &sink {
        out.push(
            Row::new("output", "all sound", Kind::Slider, s.level.min(100))
                .sub(where_(s))
                .muted(s.muted)
                .slash("volume")
                .live(),
        );
    }

    let apps = mixer(&all);
    let playing = apps.iter().filter(|a| a.running).count();
    let hint = match (apps.len(), playing) {
        (0, _) => "nothing playing".to_string(),
        (_, 0) => format!("{} open", apps.len()),
        (_, n) => format!("{n} playing"),
    };
    out.push(
        Row::new("apps", "apps", Kind::Group, apps.len() as i64)
            .hint(hint)
            .slash("volume")
            .children(
                apps.into_iter()
                    .map(|a| {
                        Row::new(format!("app:{}", slug(&a.name)), a.name.clone(), Kind::Slider, a.level.min(100))
                            .sub(if a.running { "" } else { "quiet" })
                            .muted(a.muted)
                            .slash(format!("volume {}", a.name))
                            .live()
                    })
                    .collect(),
            ),
    );

    if let Some(m) = pick("Audio/Source", &source_name) {
        out.push(
            Row::new("input", "mic", Kind::Slider, m.level.min(100))
                .sub(where_(&m))
                .muted(m.muted)
                .slash("mic")
                .live(),
        );
    }
    if let Some(s) = &sink {
        out.push(Row::new("mute", "mute", Kind::Toggle, s.muted).slash("mute"));
    }
    out
}

/// An app in the mixer: every stream it has open, under one name.
#[derive(Debug)]
struct App {
    name: String,
    ids: Vec<i64>,
    level: i64,
    muted: bool,
    running: bool,
}

fn mixer(all: &[Node]) -> Vec<App> {
    let mut apps: Vec<App> = Vec::new();
    for n in all.iter().filter(|n| n.class == "Stream/Output/Audio") {
        let raw = if n.app.is_empty() { &n.name } else { &n.app };
        let Some(name) = friendly(raw, &n.binary) else {
            continue;
        };
        match apps.iter_mut().find(|a| a.name == name) {
            Some(a) => {
                a.ids.push(n.id);
                a.muted &= n.muted;
                a.running |= n.running;
                a.level = a.level.max(n.level);
            }
            None => apps.push(App {
                name,
                ids: vec![n.id],
                level: n.level,
                muted: n.muted,
                running: n.running,
            }),
        }
    }
    apps
}

/// Launchers and helpers that open streams of their own and are never what
/// anyone means by "the game".
const NOISE: [&str; 8] = [
    "upc",
    "eadesktop",
    "eabackgroundservice",
    "socialclubhelper",
    "steamwebhelper",
    "epicwebhelper",
    "epicgameslauncher",
    "speech-dispatcher",
];

/// What a person calls an app making sound, or None for a launcher.
///
/// `Titanfall2.exe` and `Titanfall 2` both become `titanfall 2`; a stream
/// named for its engine rather than its app (`WEBRTC VoiceEngine`, which is
/// Discord in a call) takes the process's name instead.
pub(crate) fn friendly(name: &str, binary: &str) -> Option<String> {
    let generic = name.trim().is_empty()
        || name.eq_ignore_ascii_case("WEBRTC VoiceEngine")
        || name.eq_ignore_ascii_case("Chromium")
        || name.starts_with("ALSA plug-in");
    let raw = if generic && !binary.is_empty() { binary } else { name };
    let raw = raw.trim();
    let base = raw
        .strip_suffix(".exe")
        .or_else(|| raw.strip_suffix(".EXE"))
        .unwrap_or(raw);
    if NOISE.contains(&base.to_lowercase().as_str()) {
        return None;
    }
    let known = match base.to_lowercase().as_str() {
        "google chrome" | "google-chrome" | "chrome" | "google-chrome-stable" => Some("chrome"),
        "dolphin-emu" | "dolphin" => Some("dolphin"),
        "discord" | "vesktop" | "webcord" | ".discord-wrapped" => Some("discord"),
        "spotify" => Some("spotify"),
        _ => None,
    };
    if let Some(k) = known {
        return Some(k.to_string());
    }
    // CamelCase and a number run into a word are split: EldenRing → elden
    // ring, Titanfall2 → titanfall 2.
    let mut out = String::new();
    let mut prev: Option<char> = None;
    for c in base.chars() {
        let c = if c == '_' { ' ' } else { c };
        if let Some(p) = prev {
            let split = (p.is_lowercase() && c.is_uppercase())
                || (p.is_alphabetic() && c.is_ascii_digit())
                || (p.is_ascii_digit() && c.is_alphabetic());
            if split {
                out.push(' ');
            }
        }
        out.push(c);
        prev = Some(c);
    }
    let words: Vec<String> = out.split_whitespace().map(str::to_lowercase).collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// `titanfall 2` → `titanfall-2`, for the row's id.
pub(crate) fn slug(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}

/// `monitor speakers` for HDMI or DisplayPort out, `usb` for the mic.
fn where_(n: &Node) -> String {
    let d = n.description.to_lowercase();
    if d.contains("hdmi") || d.contains("displayport") {
        "monitor speakers".into()
    } else if n.bus == "usb" {
        "usb".into()
    } else if n.bus == "bluetooth" {
        n.description.to_lowercase()
    } else {
        n.nick.to_lowercase()
    }
}

pub fn set(id: &str, value: &str) -> Set {
    match id {
        "output" => device("output", "all sound", SINK, value),
        "input" => device("input", "mic", SOURCE, value),
        "mute" => {
            let (_, was) = read(SINK)?;
            let on = toggle(value, was)?;
            stamp("output", if on { "muted" } else { "on" });
            wpctl(&["set-mute", SINK, if on { "1" } else { "0" }])?;
            Ok(Changed::new("mute", "mute", Kind::Toggle, super::onoff(was), super::onoff(on)))
        }
        _ => app(id, value),
    }
}

/// A default device: a level, or mute and unmute.
fn device(id: &str, label: &str, target: &str, value: &str) -> Set {
    let (now, muted) = read(target)?;
    match value.to_lowercase().as_str() {
        "mute" | "unmute" => {
            let on = value.eq_ignore_ascii_case("mute");
            let word = |m: bool| if m { "muted" } else { "on" };
            stamp(id, word(on));
            wpctl(&["set-mute", target, if on { "1" } else { "0" }])?;
            Ok(Changed::new(id, label, Kind::Slider, word(muted), word(on)))
        }
        _ => {
            let to = level(value, now, 0, 100)?;
            stamp(id, &to.to_string());
            // `-l 1.0` caps it at unity, as reach/media.rs does.
            wpctl(&["set-volume", "-l", "1.0", target, &format!("{to}%")])?;
            Ok(Changed::new(id, label, Kind::Slider, now.to_string(), to.to_string()))
        }
    }
}

/// One app's streams, all of them: a game with two is one level.
fn app(id: &str, value: &str) -> Set {
    let want = id.strip_prefix("app:").unwrap_or(id);
    let dump = dump().ok_or("pipewire would not answer")?;
    let apps = mixer(&nodes(&dump));
    let a = apps.iter().find(|a| slug(&a.name) == want).ok_or_else(|| {
        let open: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        if open.is_empty() {
            "no app is making sound right now".to_string()
        } else {
            format!("no app '{want}' is making sound. open: {}", open.join(", "))
        }
    })?;
    match value.to_lowercase().as_str() {
        "mute" | "unmute" => {
            let on = value.eq_ignore_ascii_case("mute");
            let word = |m: bool| if m { "muted" } else { "on" };
            stamp(id, word(on));
            for i in &a.ids {
                wpctl(&["set-mute", &i.to_string(), if on { "1" } else { "0" }])?;
            }
            Ok(Changed::new(id, &a.name, Kind::Slider, word(a.muted), word(on)))
        }
        _ => {
            let to = level(value, a.level, 0, 100)?;
            stamp(id, &to.to_string());
            for i in &a.ids {
                wpctl(&["set-volume", "-l", "1.0", &i.to_string(), &format!("{to}%")])?;
            }
            Ok(Changed::new(id, &a.name, Kind::Slider, a.level.to_string(), to.to_string()))
        }
    }
}

fn wpctl(args: &[&str]) -> Result<String, String> {
    run("wpctl", args)
}

/// A device's level and mute, as wpctl says them: `Volume: 0.45 [MUTED]`.
fn read(target: &str) -> Result<(i64, bool), String> {
    let out = wpctl(&["get-volume", target])?;
    parse_volume(&out).ok_or_else(|| format!("wpctl said something unreadable: {out}"))
}

pub(crate) fn parse_volume(s: &str) -> Option<(i64, bool)> {
    let num: f64 = s.split_whitespace().nth(1)?.parse().ok()?;
    Some(((num * 100.0).round() as i64, s.contains("MUTED")))
}

// ── whose hand ──────────────────────────────────────────────────────────────
//
// Every level set here is stamped first: ^, through door, parker's
// `controls` tool, `parker controls set` by hand, and `reach media`'s volume
// (`stamp_sink`). door's level watch (visage door's `watch_levels`) reads the
// stamps: a level that moved to what was stamped is one the transcript
// already draws, as parker's act or ^,'s own, and it says nothing. Any other
// move is the keys on the desk or an app's own slider, and gets a quiet line
// of its own.
//
// `$XDG_RUNTIME_DIR/parker/sound-hand`, one line per set:
// `<unix-seconds> <row id> <level | muted | on>`. Every set is kept, not only
// the last per row — a level edited in ^, is set many times a second, and the
// watch may read any one of them. Lines older than HAND_KEEP go at the next
// write. Best effort: a stamp that cannot be written costs a line in a
// transcript, never the change.

const HAND_KEEP: f64 = 60.0;

fn stamp(id: &str, what: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    let dir = std::path::PathBuf::from(base).join("parker");
    let path = dir.join("sound-hand");
    let tmp = dir.join(format!("sound-hand.{}.tmp", std::process::id()));
    let text = stamped(
        &std::fs::read_to_string(&path).unwrap_or_default(),
        now,
        id,
        what,
    );
    let _ = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&tmp, text))
        .and_then(|()| std::fs::rename(&tmp, &path));
}

/// The stamps with this one added and the stale ones gone.
fn stamped(old: &str, now: f64, id: &str, what: &str) -> String {
    let fresh = |l: &&str| {
        let at = l
            .split_whitespace()
            .next()
            .and_then(|t| t.parse::<f64>().ok());
        at.is_some_and(|t| now - t < HAND_KEEP)
    };
    let mut out: String = old
        .lines()
        .filter(fresh)
        .map(|l| format!("{l}\n"))
        .collect();
    out.push_str(&format!("{now:.3} {id} {what}\n"));
    out
}

/// `reach media`'s volume moves the sink by a step, so its level is known
/// only afterwards: stamped from a read-back, a few milliseconds after the
/// change and well inside the watch's settling time.
pub(crate) fn stamp_sink() {
    if let Ok((level, _)) = read(SINK) {
        stamp("output", &level.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_game_is_one_row_whatever_proton_calls_it() {
        assert_eq!(friendly("Titanfall2.exe", "").as_deref(), Some("titanfall 2"));
        assert_eq!(friendly("Titanfall 2", "").as_deref(), Some("titanfall 2"));
        assert_eq!(friendly("EldenRing.exe", "").as_deref(), Some("elden ring"));
        assert_eq!(friendly("spotify", "spotify").as_deref(), Some("spotify"));
        assert_eq!(friendly("dolphin-emu", "").as_deref(), Some("dolphin"));
        assert_eq!(friendly("Google Chrome", "chrome").as_deref(), Some("chrome"));
    }

    #[test]
    fn launchers_are_not_in_the_mixer() {
        for noise in ["upc.exe", "EADesktop.exe", "SocialClubHelper.exe", "steamwebhelper"] {
            assert_eq!(friendly(noise, ""), None, "{noise}");
        }
    }

    #[test]
    fn a_stream_named_for_its_engine_takes_the_process_name() {
        assert_eq!(friendly("WEBRTC VoiceEngine", "Discord").as_deref(), Some("discord"));
        assert_eq!(friendly("", "mpv").as_deref(), Some("mpv"));
    }

    #[test]
    fn ids_are_slugs() {
        assert_eq!(slug("titanfall 2"), "titanfall-2");
        assert_eq!(slug("elden ring!"), "elden-ring");
    }

    /// door's level watch reads these lines (visage door's `watch_levels`):
    /// the shape is the contract, and a minute is all that is kept.
    #[test]
    fn a_stamp_is_one_line_and_old_ones_go() {
        let old = "100.000 output 40\n150.500 app:spotify muted\nnonsense\n";
        assert_eq!(
            stamped(old, 170.0, "output", "45"),
            "150.500 app:spotify muted\n170.000 output 45\n"
        );
        assert_eq!(stamped("", 1.25, "input", "on"), "1.250 input on\n");
    }

    #[test]
    fn wpctl_levels_parse_with_their_mute() {
        assert_eq!(parse_volume("Volume: 0.95"), Some((95, false)));
        assert_eq!(parse_volume("Volume: 0.45 [MUTED]"), Some((45, true)));
        assert_eq!(parse_volume("nonsense"), None);
    }

    fn node(id: i64, class: &str, props: Value, linear: f64, mute: bool, state: &str) -> Value {
        let mut p = props;
        p["media.class"] = json!(class);
        json!({
            "id": id,
            "type": "PipeWire:Interface:Node",
            "info": {
                "state": state,
                "props": p,
                "params": { "Props": [ { "volume": 1.0 }, { "channelVolumes": [linear, linear], "mute": mute } ] }
            }
        })
    }

    /// The shapes `pw-dump` gave on parker, Oct 2026, trimmed.
    #[test]
    fn the_rows_come_from_one_dump() {
        let dump = vec![
            json!({
                "type": "PipeWire:Interface:Metadata",
                "props": { "metadata.name": "default" },
                "metadata": [
                    { "key": "default.audio.sink", "value": { "name": "hdmi" } },
                    { "key": "default.audio.source", "value": { "name": "mic" } }
                ]
            }),
            node(55, "Audio/Sink", json!({ "node.name": "hdmi", "node.nick": "Q27G42XNE",
                "node.description": "GA102 High Definition Audio Controller Digital Stereo (HDMI) [Q27G42XNE]" }),
                0.857367, false, "running"),
            node(56, "Audio/Source", json!({ "node.name": "mic", "node.nick": "USB PnP Sound Device",
                "node.description": "CM108 Audio Controller Mono", "device.bus": "usb" }), 1.0, false, "idle"),
            node(70, "Stream/Output/Audio", json!({ "application.name": "spotify" }), 0.088, false, "running"),
            node(71, "Stream/Output/Audio", json!({ "application.name": "Titanfall2.exe" }), 1.0, false, "running"),
            node(72, "Stream/Output/Audio", json!({ "application.name": "Titanfall 2" }), 1.0, false, "running"),
            node(73, "Stream/Output/Audio", json!({ "application.name": "upc.exe" }), 1.0, false, "running"),
        ];
        let rows = rows_from(&dump);
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["output", "apps", "input", "mute"]);
        assert_eq!(rows[0].value, json!(95));
        assert_eq!(rows[0].sub, "monitor speakers");
        assert_eq!(rows[2].sub, "usb");
        let apps: Vec<(&str, &Value)> = rows[1].children.iter().map(|c| (c.id.as_str(), &c.value)).collect();
        assert_eq!(apps, [("app:spotify", &json!(44)), ("app:titanfall-2", &json!(100))]);
        assert_eq!(rows[1].hint, "2 playing");
        assert_eq!(rows[1].children[0].slash, "volume spotify");
    }
}
