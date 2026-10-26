// The controls: every setting visage's ^, shows, defined once
// (docs/visage/controls/README.md).
//
// One list and three ways in. ^, draws it (door's `controls` and
// `set_control`, over `parker controls --json` and `parker controls set`),
// ^K's "/" is generated from it (visage's commands.js), and parker reaches
// it as the `controls` tool (mcp.rs). A setting that is not here is a
// setting nowhere, and one that is here is the same in all three.
//
// Each section reads and writes in one file, so a row is never read one way
// and written another. Everything runs as Peter, without root. A read that
// fails leaves its row out or says why it cannot be used; a value is never
// guessed.
//
// What crosses to visage is data, not looks: visage draws the cells, the
// choices and the EDIT outline; this says what a row is and what it is now.

mod bluetooth;
mod desktop;
mod network;
mod sound;

pub(crate) use sound::stamp_sink;

use serde::Serialize;
use serde_json::Value;
use std::process::Command;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A level: `value` is a number between `min` and `max` (0-100).
    Slider,
    /// On or off: `value` is a bool.
    Toggle,
    /// One of `choices`: `value` is the one on.
    Choice,
    /// Something done to a thing: `value` says what it is now
    /// (`connected`, `paired`), and doing it flips that.
    Action,
    /// Read only: `value` is a line of text.
    Info,
    /// What is playing: `value` is `playing`, `paused` or `stopped`.
    Transport,
    /// lock, sleep, reboot, off: `choices`, and nothing is on.
    Session,
    /// Rows under a row, shown when it is opened: the mixer, the notices.
    Group,
}

/// Where a row may be changed from. The porthole rides the tailnet over
/// the tower's wifi, so a change that would cut that line — or could not be
/// undone from afar — is the tower's alone.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Tower {
    /// The whole row.
    All(bool),
    /// Only these of its choices.
    Only(Vec<&'static str>),
}

#[derive(Serialize, Clone, Debug)]
pub struct Row {
    pub id: String,
    pub label: String,
    /// What it is, quieter than the label: `monitor speakers`, `usb`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sub: String,
    /// A word about now, beside the value: `58%`, `giskard`, `2 playing`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hint: String,
    pub kind: Kind,
    pub value: Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// Sound rows: whether it is muted, apart from its level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tower: Option<Tower>,
    /// Why it cannot be used right now ("bluetooth is off"), or empty.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub why: String,
    /// The slash command it answers to in ^K and in ^,'s find line:
    /// `volume`, `volume spotify`. Empty for a row ^K already has a
    /// hand-written command for (`/theme`, `/brain`, `/dance`).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub slash: String,
    /// A level that is set while it is being edited, so you hear it move.
    /// The rest are set when the edit is kept.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub live: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Row>,
}

impl Row {
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: Kind, value: impl Into<Value>) -> Row {
        Row {
            id: id.into(),
            label: label.into(),
            sub: String::new(),
            hint: String::new(),
            kind,
            value: value.into(),
            choices: Vec::new(),
            min: None,
            max: None,
            muted: None,
            tower: None,
            why: String::new(),
            slash: String::new(),
            live: false,
            children: Vec::new(),
        }
        .towered()
    }

    /// The tower rule is by id (`tower`), so it is set here once rather than
    /// remembered by every section.
    fn towered(mut self) -> Row {
        self.tower = tower(&self.id);
        self
    }

    pub fn sub(mut self, s: impl Into<String>) -> Row {
        self.sub = s.into();
        self
    }
    pub fn hint(mut self, s: impl Into<String>) -> Row {
        self.hint = s.into();
        self
    }
    pub fn choices(mut self, c: Vec<String>) -> Row {
        self.choices = c;
        self
    }
    pub fn range(mut self, min: i64, max: i64) -> Row {
        self.min = Some(min);
        self.max = Some(max);
        self
    }
    pub fn muted(mut self, m: bool) -> Row {
        self.muted = Some(m);
        self
    }
    pub fn why(mut self, s: impl Into<String>) -> Row {
        self.why = s.into();
        self
    }
    pub fn slash(mut self, s: impl Into<String>) -> Row {
        self.slash = s.into();
        self
    }
    pub fn live(mut self) -> Row {
        self.live = true;
        self
    }
    pub fn children(mut self, c: Vec<Row>) -> Row {
        self.children = c;
        self
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Section {
    pub id: &'static str,
    pub name: &'static str,
    pub rows: Vec<Row>,
}

/// The tower-only rule, by row: wifi off, tailscale, and power off. Each
/// would cut the line the porthole rides on, or could not be undone from
/// afar. Reboot is not here: the page comes back a minute later.
fn tower(id: &str) -> Option<Tower> {
    match id {
        "wifi" => Some(Tower::Only(vec!["off"])),
        "tailscale" => Some(Tower::All(true)),
        "session" => Some(Tower::Only(vec!["off"])),
        _ => None,
    }
}

fn tower_only(id: &str, value: &str) -> bool {
    match tower(id) {
        Some(Tower::All(true)) => true,
        Some(Tower::Only(list)) => list.contains(&value),
        _ => false,
    }
}

/// What a change did, said the way the ledger draws it: `spotify 45 → 20`.
/// `from` and `to` are the words either side of the arrow, so visage can
/// draw the change rather than only name it (cells for a level, a dot for a
/// connection).
#[derive(Serialize, Clone, Debug)]
pub struct Changed {
    pub id: String,
    pub label: String,
    pub kind: Kind,
    pub from: String,
    pub to: String,
    pub said: String,
}

impl Changed {
    fn new(id: &str, label: &str, kind: Kind, from: impl Into<String>, to: impl Into<String>) -> Changed {
        let (from, to) = (from.into(), to.into());
        let said = if from.is_empty() || from == to {
            format!("{label} {to}")
        } else {
            format!("{label} {from} → {to}")
        };
        Changed {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            from,
            to,
            said,
        }
    }
}

pub type Set = Result<Changed, String>;

/// How a section is read.
type Read = fn() -> Vec<Row>;

/// The sections in screen order: id, name, and how each is read.
const SECTIONS: [(&str, &str, Read); 8] = [
    ("sound", "SOUND", sound::rows),
    ("playing", "PLAYING", playing_rows),
    ("network", "NETWORK", network::rows),
    ("bluetooth", "BLUETOOTH", bluetooth::rows),
    ("look", "LOOK", desktop::look_rows),
    ("quiet", "QUIET", desktop::quiet_rows),
    ("parker", "PARKER", parker_rows),
    ("session", "SESSION", session_rows),
];

/// Everything, read now. The sections read side by side: each forks a few
/// small tools (wpctl, nmcli, theme) and none waits on another.
pub fn snapshot() -> Vec<Section> {
    std::thread::scope(|s| {
        let reads: Vec<_> = SECTIONS
            .iter()
            .map(|&(id, name, read)| (id, name, s.spawn(read)))
            .collect();
        reads
            .into_iter()
            .map(|(id, name, rows)| Section {
                id,
                name,
                rows: rows.join().unwrap_or_default(),
            })
            .collect()
    })
}

/// One section, read alone: `parker controls --only sound`, which door's
/// level watch reads on every PipeWire change — one pw-dump, where the
/// whole list would also fork nmcli and ask BlueZ, for nothing.
pub fn section(id: &str) -> Option<Section> {
    SECTIONS
        .iter()
        .find(|(i, ..)| *i == id)
        .map(|&(id, name, read)| Section {
            id,
            name,
            rows: read(),
        })
}

/// The sections' ids, for the CLI's refusal of one it does not have.
pub fn section_ids() -> Vec<&'static str> {
    SECTIONS.iter().map(|(id, ..)| *id).collect()
}

fn session_rows() -> Vec<Row> {
    vec![desktop::session_row()]
}

/// Change one row. `away` is the porthole's word that the hand is not at the
/// tower: then the tower-only rule holds (`tower`).
///
/// Values, by kind: a level `0`-`100`, `+5`, `-5`, `mute`, `unmute`; a
/// toggle `on`, `off`, `toggle`; a choice by name; an action `connect`,
/// `disconnect`, `toggle`; transport `play`, `pause`, `toggle`, `next`,
/// `previous`; the session `lock`, `sleep`, `reboot`, `off`.
pub fn set(id: &str, value: &str, away: bool) -> Set {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{id} needs a value"));
    }
    if away && tower_only(id, value) {
        return Err(format!(
            "{} is the tower's: from here it would cut the line you are on",
            match id {
                "session" => "turning parker off",
                "wifi" => "turning wifi off",
                other => other,
            }
        ));
    }
    match id {
        "output" | "input" | "mute" => sound::set(id, value),
        _ if id.starts_with("app:") => sound::set(id, value),
        "track" => playing_set(value),
        "wifi" | "tailscale" => network::set(id, value),
        "bt" => bluetooth::set(id, value),
        _ if id.starts_with("bt:") => bluetooth::set(id, value),
        "mode" | "family" | "glass" | "night" | "dnd" => desktop::set(id, value),
        "session" => desktop::session(value),
        "brain" => {
            let (from, _) = crate::brain::current()?;
            crate::brain::set(value)?;
            let (to, _) = crate::brain::current()?;
            Ok(Changed::new("brain", "brain", Kind::Choice, from.name, to.name))
        }
        "dance" => {
            let was = crate::face::state::dance::get().is_some();
            let on = toggle(value, was)?;
            if on != was {
                if on {
                    crate::reach::dance("auto", Some(0.0), false)?;
                } else {
                    crate::reach::dance("stop", None, false)?;
                }
            }
            Ok(Changed::new("dance", "dance", Kind::Toggle, onoff(was), onoff(on)))
        }
        other => Err(format!("no control '{other}'. `parker controls` lists them")),
    }
}

/// Every row whose id is fixed. The mixer's (`app:…`), the devices'
/// (`bt:…`) and the notices' are made from what is there.
const FIXED: [&str; 15] = [
    "output", "input", "mute", "track", "wifi", "tailscale", "bt", "mode", "family", "glass",
    "night", "dnd", "brain", "dance", "session",
];

/// A row by its id, or by what a person would call it: its label (`spotify`,
/// `wireless controller`) or its slash name (`volume`, `quiet`). For the CLI
/// and for parker, who say names; door hands over ids only.
pub fn resolve(name: &str) -> Option<String> {
    let want = name.trim().to_lowercase();
    // An id is taken as it is, without reading the machine: a level being
    // edited is set several times a second.
    if FIXED.contains(&want.as_str()) || want.starts_with("app:") || want.starts_with("bt:") {
        return Some(want);
    }
    let all = flat(&snapshot());
    all.iter()
        .find(|r| r.id == want)
        .or_else(|| all.iter().find(|r| r.label.to_lowercase() == want))
        .or_else(|| all.iter().find(|r| !r.slash.is_empty() && r.slash == want))
        .or_else(|| {
            all.iter()
                .find(|r| r.kind != Kind::Info && r.label.to_lowercase().starts_with(&want))
        })
        .map(|r| r.id.clone())
}

/// Every row, children included, in screen order.
pub fn flat(sections: &[Section]) -> Vec<Row> {
    let mut out = Vec::new();
    for s in sections {
        for r in &s.rows {
            out.push(r.clone());
            out.extend(r.children.iter().cloned());
        }
    }
    out
}

/// The list as text: `parker controls`, and the `controls` tool's `list`.
/// Each row ends in its id, which is what `set` takes.
pub fn text(sections: &[Section]) -> String {
    let mut out = String::new();
    for s in sections {
        if s.rows.is_empty() {
            continue;
        }
        out.push_str(s.name);
        out.push('\n');
        for r in &s.rows {
            line(&mut out, r, 2);
            for c in &r.children {
                line(&mut out, c, 4);
            }
        }
    }
    out.trim_end().to_string()
}

fn line(out: &mut String, r: &Row, indent: usize) {
    let mut value = match r.kind {
        Kind::Slider => r.value.as_i64().map(|v| v.to_string()).unwrap_or_default(),
        Kind::Toggle => onoff(r.value.as_bool().unwrap_or(false)).to_string(),
        Kind::Session => r.choices.join(" · "),
        Kind::Group => String::new(),
        _ => r.value.as_str().unwrap_or_default().to_string(),
    };
    if r.muted == Some(true) {
        value.push_str(" (muted)");
    }
    if r.kind == Kind::Choice && !r.choices.is_empty() {
        value.push_str(&format!("  [{}]", r.choices.join(", ")));
    }
    let extra: Vec<&str> = [r.sub.as_str(), r.hint.as_str(), r.why.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    let label = format!("{}{}", " ".repeat(indent), r.label);
    out.push_str(&format!("{label:<24} {value}"));
    if !extra.is_empty() {
        out.push_str(&format!("  — {}", extra.join(", ")));
    }
    if r.kind != Kind::Info {
        out.push_str(&format!("  ({})", r.id));
    }
    out.push('\n');
}

// ── the small sections ──────────────────────────────────────────────────────

/// What is playing, over MPRIS (sense/media.rs reads it, reach/media.rs moves
/// it). One row: the track, its transport.
fn playing_rows() -> Vec<Row> {
    match crate::sense::media::current() {
        Some(m) => {
            let label = m.title.clone().unwrap_or_else(|| m.player.clone());
            let sub = m.artist.unwrap_or_default();
            vec![Row::new("track", label.to_lowercase(), Kind::Transport, m.status.to_lowercase())
                .sub(sub.to_lowercase())
                .hint(player_name(&m.player))
                .slash("music")]
        }
        None => vec![Row::new("track", "nothing playing", Kind::Transport, "stopped")
            .why("nothing is playing")
            .slash("music")],
    }
}

/// `spotify`, `chromium.instance1234` → `chromium`.
fn player_name(player: &str) -> String {
    player.split('.').next().unwrap_or(player).to_lowercase()
}

fn playing_set(value: &str) -> Set {
    let action = match value.to_lowercase().as_str() {
        "prev" => "previous".to_string(),
        other => other.to_string(),
    };
    if !["play", "pause", "toggle", "next", "previous"].contains(&action.as_str()) {
        return Err(format!("the track takes play, pause, toggle, next or previous, not '{value}'"));
    }
    let said = crate::reach::media::media(&action, None, None)?;
    Ok(Changed {
        id: "track".into(),
        label: "the track".into(),
        kind: Kind::Transport,
        from: String::new(),
        to: action,
        said,
    })
}

/// parker himself: which brain, and whether he is dancing. Both have
/// hand-written commands in ^K already, so no slash name here.
fn parker_rows() -> Vec<Row> {
    let mut out = Vec::new();
    if let Ok((now, cat)) = crate::brain::current() {
        out.push(
            Row::new("brain", "brain", Kind::Choice, now.name.clone())
                .choices(cat.brains.iter().map(|b| b.name.clone()).collect())
                .hint(short_model(&now.model)),
        );
    }
    out.push(Row::new(
        "dance",
        "dance",
        Kind::Toggle,
        crate::face::state::dance::get().is_some(),
    ));
    out
}

/// `anthropic/claude-opus-5-5` → `opus 5.5`, as visage's palette says it.
fn short_model(model: &str) -> String {
    let tail = model.rsplit('/').next().unwrap_or(model);
    let tail = tail.strip_prefix("claude-").unwrap_or(tail);
    let parts: Vec<&str> = tail.split('-').collect();
    match parts.as_slice() {
        [name, major, minor] if major.parse::<u32>().is_ok() && minor.parse::<u32>().is_ok() => {
            format!("{name} {major}.{minor}")
        }
        _ => tail.to_string(),
    }
}

// ── shared ──────────────────────────────────────────────────────────────────

pub(crate) fn onoff(b: bool) -> &'static str {
    if b {
        "on"
    } else {
        "off"
    }
}

/// `on`, `off` or `toggle`, against what it is now.
pub(crate) fn toggle(value: &str, now: bool) -> Result<bool, String> {
    match value.to_lowercase().as_str() {
        "on" | "true" | "1" | "yes" => Ok(true),
        "off" | "false" | "0" | "no" => Ok(false),
        "toggle" | "flip" => Ok(!now),
        other => Err(format!("on, off or toggle, not '{other}'")),
    }
}

/// A level, against what it is now: `45`, `45%`, `+5`, `-5`.
pub(crate) fn level(value: &str, now: i64, min: i64, max: i64) -> Result<i64, String> {
    let v = value.trim().trim_end_matches('%');
    let n = if let Some(d) = v.strip_prefix('+') {
        now + d.parse::<i64>().map_err(|_| format!("not a level: {value}"))?
    } else if let Some(d) = v.strip_prefix('-') {
        now - d.parse::<i64>().map_err(|_| format!("not a level: {value}"))?
    } else {
        v.parse::<f64>()
            .map_err(|_| format!("not a level: {value}"))?
            .round() as i64
    };
    Ok(n.clamp(min, max))
}

/// Run a tool and keep what it said. A failure carries its last line of
/// stderr, which is usually the whole story.
pub(crate) fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    output(Command::new(cmd).args(args), cmd)
}

pub(crate) fn output(cmd: &mut Command, name: &str) -> Result<String, String> {
    let out = cmd.output().map_err(|e| format!("{name} is not runnable: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    Err(if last.is_empty() {
        format!("{name} exited {}", out.status)
    } else {
        format!("{name}: {last}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_read_every_way_a_person_says_one() {
        assert_eq!(level("45", 10, 0, 100), Ok(45));
        assert_eq!(level("45%", 10, 0, 100), Ok(45));
        assert_eq!(level("+5", 42, 0, 100), Ok(47));
        assert_eq!(level("-50", 42, 0, 100), Ok(0));
        assert_eq!(level("250", 42, 0, 100), Ok(100));
        assert_eq!(level("40", 80, 50, 100), Ok(50));
        assert!(level("loud", 42, 0, 100).is_err());
    }

    #[test]
    fn toggles_take_a_word_or_flip() {
        assert_eq!(toggle("on", false), Ok(true));
        assert_eq!(toggle("OFF", true), Ok(false));
        assert_eq!(toggle("toggle", true), Ok(false));
        assert!(toggle("maybe", true).is_err());
    }

    #[test]
    fn the_tower_keeps_what_would_cut_the_porthole() {
        assert!(tower_only("wifi", "off"));
        assert!(!tower_only("wifi", "PEKI"));
        assert!(tower_only("tailscale", "on"));
        assert!(tower_only("session", "off"));
        assert!(!tower_only("session", "reboot"));
        assert!(!tower_only("output", "20"));
        let refused = set("wifi", "off", true).unwrap_err();
        assert!(refused.contains("tower"), "{refused}");
        assert!(set("session", "off", true).unwrap_err().contains("parker off"));
    }

    #[test]
    fn rows_carry_their_tower_rule_from_the_id() {
        let r = Row::new("tailscale", "tailscale", Kind::Toggle, true);
        assert_eq!(r.tower, Some(Tower::All(true)));
        assert_eq!(Row::new("output", "all sound", Kind::Slider, 42).tower, None);
        let json = serde_json::to_value(Row::new("wifi", "wifi", Kind::Choice, "PEKI")).unwrap();
        assert_eq!(json["tower"], serde_json::json!(["off"]));
        assert!(json.get("muted").is_none(), "empty fields stay off the wire");
    }

    #[test]
    fn a_change_is_said_with_its_arrow() {
        let c = Changed::new("app:spotify", "spotify", Kind::Slider, "45", "20");
        assert_eq!(c.said, "spotify 45 → 20");
        assert_eq!(Changed::new("x", "dnd", Kind::Toggle, "on", "on").said, "dnd on");
    }

    #[test]
    fn models_are_named_short() {
        assert_eq!(short_model("anthropic/claude-opus-5-5"), "opus 5.5");
        assert_eq!(short_model("anthropic/claude-fable-5-1"), "fable 5.1");
        assert_eq!(short_model("openai/gpt-x"), "gpt-x");
    }

    #[test]
    fn an_unknown_control_says_where_the_list_is() {
        assert!(set("warp-drive", "on", false).unwrap_err().contains("parker controls"));
        assert!(set("output", "  ", false).is_err());
    }

    #[test]
    fn text_ends_each_row_in_its_id() {
        let s = vec![Section {
            id: "sound",
            name: "SOUND",
            rows: vec![Row::new("output", "all sound", Kind::Slider, 42)
                .sub("monitor speakers")
                .muted(false)
                .children(vec![])],
        }];
        let t = text(&s);
        assert!(t.starts_with("SOUND\n"));
        assert!(t.contains("all sound"));
        assert!(t.contains("42"));
        assert!(t.ends_with("(output)"), "{t}");
    }
}
