// The text form — what parker actually reads.
//
// This exists instead of feeding it JSON because the sense block goes into
// every prompt, and the JSON of the same content costs roughly three times
// the tokens for information a model reads perfectly well either way. Braces
// and repeated keys are not doing any work here.
//
// Two rules that keep it cheap: a line that has nothing to say is omitted
// entirely (no `media: null`), and healthy states collapse to one word. The
// exception is anything wrong — `units` appears only when something failed,
// which means its presence is the signal.

use crate::sense::{Machine, Media, Sense, Workspace};
use std::fmt::Write;

pub fn text(s: &Sense) -> String {
    let mut o = String::new();
    let mut row = |k: &str, v: String| {
        let _ = writeln!(o, "{k:<7} {v}");
    };

    row("at", format!("{} · up {}", s.at, duration(s.uptime_secs)));

    match &s.focus {
        Some(f) => {
            let mut v = format!(
                "{} · \"{}\" · ws{}",
                short_class(&f.class),
                f.title,
                f.workspace
            );
            if f.fullscreen != 0 {
                v.push_str(" · fullscreen");
            }
            if let Some(held) = f.held_secs {
                let _ = write!(v, " · {}", duration(held));
            }
            row("focus", v);
        }
        None => row("focus", "nothing focused".into()),
    }

    if !s.workspaces.is_empty() {
        row("spaces", spaces(&s.workspaces));
    }
    if let Some(m) = &s.media {
        row("media", media(m));
    }
    if let Some(c) = &s.machine.cpu {
        let mut v = format!("{}%", c.util_pct);
        if let Some(t) = c.temp_c {
            let _ = write!(v, " · {t}°C");
        }
        row("cpu", v);
    }
    if let Some(g) = &s.machine.gpu {
        row(
            "gpu",
            format!(
                "{}% · {:.1}/{:.1}G · {}°C · {:.0}W",
                g.util_pct,
                g.mem_used_mb as f64 / 1024.0,
                g.mem_total_mb as f64 / 1024.0,
                g.temp_c,
                g.watts
            ),
        );
    }

    let mut mem = format!(
        "{:.1}/{:.1}G",
        s.machine.mem_used_gb, s.machine.mem_total_gb
    );
    if let Some(d) = s.machine.disk_used_pct {
        let _ = write!(mem, " · disk {d}%");
    }
    row("mem", mem);

    if let Some(u) = units(&s.machine) {
        row("units", u);
    }

    row(
        "mode",
        format!(
            "{} · {} · {}",
            s.inferred.mode,
            if s.inferred.interruptible {
                "interruptible"
            } else {
                "busy"
            },
            s.inferred.health
        ),
    );

    o
}

/// `com.mitchellh.ghostty` is not a word anyone says. Reverse-DNS classes
/// collapse to their last segment; everything else is already short.
fn short_class(class: &str) -> &str {
    if class.matches('.').count() >= 2 {
        class.rsplit('.').next().unwrap_or(class)
    } else {
        class
    }
}

/// `1·1 2·1 [3·2] 4·1` — space and window count, active one bracketed.
/// Special workspaces (negative ids) are skipped; they are scratchpads, not
/// places you are.
fn spaces(ws: &[Workspace]) -> String {
    ws.iter()
        .filter(|w| w.id > 0)
        .map(|w| {
            let cell = format!("{}·{}", w.id, w.windows);
            if w.active {
                format!("[{cell}]")
            } else {
                cell
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn media(m: &Media) -> String {
    let mut v = format!("{} · {}", m.player, m.status.to_lowercase());
    match (&m.title, &m.artist) {
        (Some(t), Some(a)) => {
            let _ = write!(v, " · {t} — {a}");
        }
        (Some(t), None) => {
            let _ = write!(v, " · {t}");
        }
        _ => {}
    }
    v
}

/// Only rendered when something is wrong — a missing `units` line means
/// nothing failed.
fn units(m: &Machine) -> Option<String> {
    let mut parts = Vec::new();
    if m.failed_units > 0 {
        parts.push(format!("{} failed (system)", m.failed_units));
    }
    if m.failed_user_units > 0 {
        parts.push(format!("{} failed (user)", m.failed_user_units));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn duration(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86400 => match (s / 3600, (s % 3600) / 60) {
            (h, 0) => format!("{h}h"),
            (h, m) => format!("{h}h{m}m"),
        },
        s => match (s / 86400, (s % 86400) / 3600) {
            (d, 0) => format!("{d}d"),
            (d, h) => format!("{d}d {h}h"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sense::{Cpu, Focus, Gpu, Inferred, Machine, Sense, Workspace, VERSION};

    fn sample() -> Sense {
        Sense {
            version: VERSION,
            at: "15:42:11".into(),
            uptime_secs: 273_600, // 3d 4h
            focus: Some(Focus {
                class: "com.mitchellh.ghostty".into(),
                title: "parker | ~".into(),
                workspace: 3,
                workspace_name: "󰆍".into(),
                fullscreen: 0,
                floating: false,
                redacted: false,
                held_secs: Some(738),
            }),
            workspaces: vec![
                Workspace {
                    id: 1,
                    name: "󰖟".into(),
                    windows: 1,
                    active: false,
                },
                Workspace {
                    id: 3,
                    name: "󰆍".into(),
                    windows: 2,
                    active: true,
                },
                Workspace {
                    id: -99,
                    name: "special".into(),
                    windows: 1,
                    active: false,
                },
            ],
            media: Some(Media {
                player: "spotify".into(),
                status: "Paused".into(),
                title: Some("Weightless".into()),
                artist: Some("Marconi Union".into()),
            }),
            machine: Machine {
                gpu: Some(Gpu {
                    util_pct: 4,
                    mem_used_mb: 1019,
                    mem_total_mb: 24576,
                    temp_c: 56,
                    watts: 146.06,
                }),
                mem_used_gb: 11.2,
                mem_total_gb: 62.6,
                disk_used_pct: Some(6),
                failed_units: 0,
                failed_user_units: 0,
                cpu: Some(Cpu {
                    util_pct: 23,
                    temp_c: Some(54),
                }),
            },
            inferred: Inferred {
                mode: "terminal".into(),
                interruptible: true,
                health: "ok".into(),
            },
        }
    }

    // Golden. If this changes, the shape parker reads changed — which is
    // allowed, but should be on purpose.
    #[test]
    fn renders_the_expected_block() {
        let expected = "\
at      15:42:11 · up 3d 4h
focus   ghostty · \"parker | ~\" · ws3 · 12m
spaces  1·1 [3·2]
media   spotify · paused · Weightless — Marconi Union
cpu     23% · 54°C
gpu     4% · 1.0/24.0G · 56°C · 146W
mem     11.2/62.6G · disk 6%
mode    terminal · interruptible · ok
";
        assert_eq!(text(&sample()), expected);
    }

    #[test]
    fn stays_cheaper_than_its_json() {
        let s = sample();
        let json = serde_json::to_string_pretty(&s).unwrap();
        assert!(
            text(&s).len() * 2 < json.len(),
            "text {} vs json {}",
            text(&s).len(),
            json.len()
        );
    }

    #[test]
    fn quiet_inputs_omit_their_lines_entirely() {
        let mut s = sample();
        s.media = None;
        s.machine.gpu = None;
        s.machine.cpu = None;
        s.workspaces.clear();
        let out = text(&s);
        assert!(!out.contains("media"));
        assert!(!out.contains("gpu"));
        assert!(!out.contains("cpu"));
        assert!(!out.contains("spaces"));
        assert!(!out.contains("null"));
    }

    #[test]
    fn units_line_appears_only_when_something_failed() {
        let mut s = sample();
        assert!(!text(&s).contains("units"));
        s.machine.failed_user_units = 2;
        assert!(text(&s).contains("units   2 failed (user)"));
    }

    #[test]
    fn fullscreen_is_called_out() {
        let mut s = sample();
        s.focus.as_mut().unwrap().fullscreen = 1;
        assert!(text(&s).contains("fullscreen"));
    }

    #[test]
    fn unmeasured_focus_duration_is_absent_not_zero() {
        let mut s = sample();
        s.focus.as_mut().unwrap().held_secs = None;
        let line = text(&s).lines().nth(1).unwrap().to_string();
        assert!(line.ends_with("ws3"), "got {line:?}");
    }

    #[test]
    fn reverse_dns_classes_shorten_plain_ones_do_not() {
        assert_eq!(short_class("com.mitchellh.ghostty"), "ghostty");
        assert_eq!(short_class("com.anthropic.Claude"), "Claude");
        assert_eq!(short_class("google-chrome"), "google-chrome");
        assert_eq!(short_class("spotify"), "spotify");
    }

    #[test]
    fn durations_read_like_a_human_wrote_them() {
        assert_eq!(duration(45), "45s");
        assert_eq!(duration(738), "12m");
        assert_eq!(duration(3600), "1h");
        assert_eq!(duration(3900), "1h5m");
        assert_eq!(duration(273_600), "3d 4h");
        assert_eq!(duration(172_800), "2d");
    }
}
