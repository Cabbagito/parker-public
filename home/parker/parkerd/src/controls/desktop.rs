// LOOK, QUIET and SESSION: the desktop around visage.
//
// The machine's look goes through the `theme` CLI (home/theming), the one
// implementation: light, dark and auto flip a specialisation in seconds;
// a family and the glass are rebuilds, ~30 s. The shell's own state —
// night light, do not disturb, its notices, the lock screen — is
// noctalia's, over `noctalia msg` and its state dir. The session's power is
// logind's, which lets the active session's user suspend, reboot and power
// off.
//
// visage's own look ("this screen") is not here. It is per screen and the
// page's alone (visage/src/lib/themes.js); visage adds that row itself.

use super::{level, output, run, toggle, Changed, Kind, Row, Set};
use std::process::Command;

// ── LOOK ────────────────────────────────────────────────────────────────────

pub fn look_rows() -> Vec<Row> {
    let mut out = Vec::new();
    let status = run("theme", &["status"]).ok().map(|s| Status::parse(&s));
    let families = run("theme", &["list"]).map(|s| families(&s)).unwrap_or_default();
    if let Some(st) = &status {
        let mut mode = Row::new("mode", "desktop", Kind::Choice, st.mode.clone())
            .choices(vec!["light".into(), "dark".into(), "auto".into()])
            .hint(st.hint());
        if st.mode == "pinned" {
            mode = mode.why("one preset, so no day and night");
        }
        out.push(mode);
        let family = families
            .iter()
            .find(|(_, d, n)| *d == st.day && *n == st.night)
            .map(|(f, _, _)| f.clone())
            .unwrap_or_default();
        out.push(
            Row::new("family", "family", Kind::Choice, family)
                .choices(families.iter().map(|(f, _, _)| f.clone()).collect())
                .hint(format!("{} / {}", st.day, st.night)),
        );
    }
    if let Some(v) = run("theme", &["transparency"]).ok().and_then(|s| s.parse::<f64>().ok()) {
        out.push(
            Row::new("glass", "glass", Kind::Slider, (v * 100.0).round() as i64)
                .sub("unfocused windows")
                .range(50, 100)
                .slash("glass"),
        );
    }
    out.push(Row::new("night", "night light", Kind::Toggle, night()).slash("night"));
    out
}

/// What `theme status` says, as rows need it.
#[derive(Debug, PartialEq)]
pub(crate) struct Status {
    day: String,
    night: String,
    /// `day` or `night`: what is showing.
    now: String,
    /// `light`, `dark`, `auto` or `pinned`.
    mode: String,
    /// The schedule's boundaries, when it is the one deciding.
    at_day: String,
    at_night: String,
}

impl Status {
    pub(crate) fn parse(s: &str) -> Status {
        let field = |k: &str| {
            s.lines()
                .find_map(|l| l.strip_prefix(k))
                .map(|v| v.trim().to_string())
                .unwrap_or_default()
        };
        let mode_line = field("mode:");
        let (mode, at_day, at_night) = if mode_line.starts_with("manual day") {
            ("light", "", "")
        } else if mode_line.starts_with("manual night") {
            ("dark", "", "")
        } else if mode_line.starts_with("pinned") {
            ("pinned", "", "")
        } else {
            // `scheduled (day 08:00, night 18:30)`
            let inner = mode_line
                .split_once('(')
                .and_then(|(_, r)| r.strip_suffix(')'))
                .unwrap_or("");
            let mut d = "";
            let mut n = "";
            for part in inner.split(',') {
                let part = part.trim();
                if let Some(t) = part.strip_prefix("day ") {
                    d = t;
                } else if let Some(t) = part.strip_prefix("night ") {
                    n = t;
                }
            }
            return Status {
                day: field("day:"),
                night: field("night:"),
                now: field("now:"),
                mode: "auto".into(),
                at_day: d.into(),
                at_night: n.into(),
            };
        };
        Status {
            day: field("day:"),
            night: field("night:"),
            now: field("now:"),
            mode: mode.into(),
            at_day: at_day.into(),
            at_night: at_night.into(),
        }
    }

    /// `hangar at 08:00` while the clock decides; else what is showing.
    fn hint(&self) -> String {
        let showing = if self.now == "day" { &self.day } else { &self.night };
        match (self.mode.as_str(), self.now.as_str()) {
            ("auto", "night") if !self.at_day.is_empty() => format!("{} at {}", self.day, self.at_day),
            ("auto", "day") if !self.at_night.is_empty() => format!("{} at {}", self.night, self.at_night),
            _ => showing.clone(),
        }
    }
}

/// `theme list`'s families: `forge = hangar / redline` → (forge, hangar, redline).
pub(crate) fn families(s: &str) -> Vec<(String, String, String)> {
    s.lines()
        .take_while(|l| !l.starts_with("presets:"))
        .filter_map(|l| {
            let (name, pair) = l.trim().split_once(" = ")?;
            let (d, n) = pair.split_once(" / ")?;
            Some((name.trim().into(), d.trim().into(), n.trim().into()))
        })
        .collect()
}

// Night light has no status to read: noctalia's IPC can force it on or off
// but not say which it is. So parker remembers what he last set, in the
// runtime dir — a reboot, or a noctalia restart, starts it off again, which
// is also what the file then says.
fn night_file() -> std::path::PathBuf {
    let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    std::path::Path::new(&base).join("parker").join("night")
}

fn night() -> bool {
    std::fs::read_to_string(night_file())
        .map(|s| s.trim() == "on")
        .unwrap_or(false)
}

// ── QUIET ───────────────────────────────────────────────────────────────────

pub fn quiet_rows() -> Vec<Row> {
    let mut out = Vec::new();
    if let Ok(s) = noctalia(&["notification-dnd-status"]) {
        out.push(Row::new("dnd", "do not disturb", Kind::Toggle, s.trim() == "on").slash("quiet"));
    }
    if let Some(r) = notices() {
        out.push(r);
    }
    out
}

/// The latest notices, from noctalia's history, as rows to open under the
/// group: read here, so they are read in visage rather than in a panel
/// thrown over it. Only read: what is seen is noctalia's to say.
fn notices() -> Option<Row> {
    let base = std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| {
        format!("{}/.local/state", std::env::var("HOME").unwrap_or_default())
    });
    let raw = std::fs::read_to_string(format!("{base}/noctalia/notification_history.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let entries = v["entries"].as_array()?;
    let unseen = entries.iter().filter(|e| !e["seen"].as_bool().unwrap_or(true)).count();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let children: Vec<Row> = entries
        .iter()
        .rev()
        .take(6)
        .enumerate()
        .map(|(i, e)| {
            let n = &e["notification"];
            let s = |k: &str| n[k].as_str().unwrap_or_default().trim().to_string();
            let (app, summary, body) = (s("app_name"), s("summary"), s("body"));
            let label = if app.is_empty() { summary.to_lowercase() } else { app.to_lowercase() };
            let said = match (summary.is_empty(), body.is_empty()) {
                (false, false) if !app.is_empty() => format!("{summary}: {body}"),
                (_, false) => body,
                _ => summary,
            };
            let ago = n["received_wall_ms"].as_i64().map(|t| ago(now - t)).unwrap_or_default();
            Row::new(format!("notice:{i}"), label, Kind::Info, clip(&said, 64)).hint(ago)
        })
        .collect();
    let hint = match (entries.len(), unseen) {
        (0, _) => "none".to_string(),
        (_, 0) => format!("{} kept", entries.len()),
        (_, n) => format!("{n} new"),
    };
    Some(
        Row::new("notices", "notices", Kind::Group, children.len() as i64)
            .hint(hint)
            .children(children),
    )
}

fn clip(s: &str, n: usize) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= n {
        return one;
    }
    format!("{}…", one.chars().take(n - 1).collect::<String>().trim_end())
}

/// `4m`, `3h`, `2d`.
fn ago(ms: i64) -> String {
    let s = (ms / 1000).max(0);
    match s {
        s if s < 60 => "now".into(),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86400),
    }
}

/// `noctalia msg …`. Its socket is named for the Wayland display, so a
/// caller that has none in its environment (a service started early, an
/// SSH session) is pointed at the one noctalia is listening on.
fn noctalia(args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("noctalia");
    cmd.arg("msg").args(args);
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        if let Some(d) = noctalia_display() {
            cmd.env("WAYLAND_DISPLAY", d);
        }
    }
    output(&mut cmd, "noctalia")
}

fn noctalia_display() -> Option<String> {
    let dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
    std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).find_map(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        name.strip_prefix("noctalia-")?
            .strip_suffix(".sock")
            .filter(|d| d.starts_with("wayland-"))
            .map(str::to_string)
    })
}

// ── setting them ────────────────────────────────────────────────────────────

pub fn set(id: &str, value: &str) -> Set {
    match id {
        "mode" => {
            let st = Status::parse(&run("theme", &["status"])?);
            let to = value.to_lowercase();
            if !["light", "dark", "auto"].contains(&to.as_str()) {
                return Err(format!("the desktop is light, dark or auto, not '{value}'"));
            }
            run("theme", &[to.as_str()])?;
            Ok(Changed::new("mode", "desktop", Kind::Choice, st.mode, to))
        }
        "family" => {
            let fams = families(&run("theme", &["list"])?);
            let st = Status::parse(&run("theme", &["status"])?);
            let from = fams
                .iter()
                .find(|(_, d, n)| *d == st.day && *n == st.night)
                .map(|(f, _, _)| f.clone())
                .unwrap_or_default();
            let to = fams
                .iter()
                .find(|(f, _, _)| f.eq_ignore_ascii_case(value))
                .map(|(f, _, _)| f.clone())
                .ok_or_else(|| {
                    let names: Vec<&str> = fams.iter().map(|(f, _, _)| f.as_str()).collect();
                    format!("no family '{value}'. there are {}", names.join(", "))
                })?;
            run("theme", &["set", &to])?;
            Ok(Changed::new("family", "family", Kind::Choice, from, to))
        }
        "glass" => {
            let now = run("theme", &["transparency"])?
                .parse::<f64>()
                .map(|v| (v * 100.0).round() as i64)
                .unwrap_or(80);
            let to = level(value, now, 50, 100)?;
            let arg = if to == 100 { "1".to_string() } else { format!("0.{to:02}") };
            run("theme", &["transparency", &arg])?;
            Ok(Changed::new("glass", "glass", Kind::Slider, now.to_string(), to.to_string()))
        }
        "night" => {
            let was = night();
            let on = toggle(value, was)?;
            if on != was {
                noctalia(&["nightlight-force-toggle"])?;
                let f = night_file();
                if let Some(d) = f.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                std::fs::write(&f, super::onoff(on)).map_err(|e| format!("could not remember it: {e}"))?;
            }
            Ok(Changed::new("night", "night light", Kind::Toggle, super::onoff(was), super::onoff(on)))
        }
        "dnd" => {
            let was = noctalia(&["notification-dnd-status"])?.trim() == "on";
            let on = toggle(value, was)?;
            noctalia(&["notification-dnd-set", super::onoff(on)])?;
            Ok(Changed::new("dnd", "do not disturb", Kind::Toggle, super::onoff(was), super::onoff(on)))
        }
        other => Err(format!("no control '{other}'")),
    }
}

// ── SESSION ─────────────────────────────────────────────────────────────────

pub fn session_row() -> Row {
    Row::new("session", "session", Kind::Session, "")
        .choices(["lock", "sleep", "reboot", "off"].map(String::from).to_vec())
}

/// Lock is noctalia's lock screen; the rest are logind's, through systemctl.
pub fn session(value: &str) -> Set {
    let what = value.to_lowercase();
    let (said, done) = match what.as_str() {
        "lock" => ("locked", noctalia(&["session", "lock"]).or_else(|_| run("loginctl", &["lock-session"]))),
        "sleep" | "suspend" => ("sleeping", run("systemctl", &["suspend"])),
        "reboot" => ("rebooting", run("systemctl", &["reboot"])),
        "off" | "poweroff" | "shutdown" => ("turning off", run("systemctl", &["poweroff"])),
        other => return Err(format!("the session is lock, sleep, reboot or off, not '{other}'")),
    };
    done?;
    Ok(Changed::new("session", "session", Kind::Session, "", said))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_status_reads_as_a_mode() {
        let manual = "day:   hangar\nnight: redline\nnow:   night\nmode:  manual night — 'theme auto' resumes the schedule\n";
        let st = Status::parse(manual);
        assert_eq!((st.mode.as_str(), st.day.as_str(), st.now.as_str()), ("dark", "hangar", "night"));
        assert_eq!(st.hint(), "redline");

        let auto = "day:   hangar\nnight: redline\nnow:   night\nmode:  scheduled (day 08:00, night 18:30)\n";
        let st = Status::parse(auto);
        assert_eq!(st.mode, "auto");
        assert_eq!(st.hint(), "hangar at 08:00");

        let pinned = "day:   macchiato\nnight: macchiato\nnow:   night\nmode:  pinned (single preset, schedule idle)\n";
        assert_eq!(Status::parse(pinned).mode, "pinned");
    }

    #[test]
    fn families_come_from_theme_list() {
        let list = "families (day/night):\n  drift = glacier / trench\n  forge = hangar / redline\npresets:\n  blush\n";
        let f = families(list);
        assert_eq!(f.len(), 2);
        assert_eq!(f[1], ("forge".into(), "hangar".into(), "redline".into()));
    }

    #[test]
    fn a_notice_is_one_short_line() {
        assert_eq!(clip("all   good\nthanks", 64), "all good thanks");
        assert_eq!(clip("abcdefghij", 5), "abcd…");
        assert_eq!(ago(30_000), "now");
        assert_eq!(ago(5 * 60_000), "5m");
        assert_eq!(ago(3 * 3_600_000), "3h");
    }

    #[test]
    fn the_session_says_what_it_takes() {
        assert!(session("hibernate").unwrap_err().contains("lock, sleep, reboot or off"));
    }
}
