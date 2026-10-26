// `look` — hand the caller a picture of what is on screen.
//
// This is a screenshot and nothing more. Models read images natively, so
// there is no vision pipeline here, no OCR, no resolve-the-document
// cleverness. grim writes a PNG to stdout and we pass it along.
//
// The one rule it enforces: a window whose *class* is on the privacy
// denylist is never captured. That list is short and specific (password
// managers), so this refuses rarely and for a good reason. Note the
// asymmetry with sense — sense also redacts on title keywords, which fail
// open; look only checks class, because the user pointing at a screen and
// saying "look" is an explicit act and should not be second-guessed on a
// keyword match.

use crate::sense::{hypr, policy};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The focused window only. The default — it is what "look at this"
    /// means, and it keeps everything else on screen out of the picture.
    Window,
    /// Everything on the monitor.
    Screen,
}

impl Target {
    pub fn parse(s: &str) -> Target {
        match s.to_lowercase().as_str() {
            "screen" | "all" | "monitor" | "desktop" => Target::Screen,
            _ => Target::Window,
        }
    }
}

pub struct Shot {
    pub png: Vec<u8>,
    /// What was captured, in words — this goes back to the caller alongside
    /// the image so it knows what it is looking at.
    pub what: String,
}

/// Longest edge we bother sending. Vision models downscale past roughly this
/// anyway, so anything larger is bytes spent to be thrown away — on this
/// monitor it is the difference between a 1.1 MB screenshot and a 300 KB one,
/// per look, with no loss of anything the model can see.
const MAX_EDGE: i64 = 1568;

pub fn look(target: Target) -> Result<Shot, String> {
    let (geometry, what, edge) = match target {
        Target::Screen => {
            let (w, h) = hypr::monitor_size().unwrap_or((MAX_EDGE, MAX_EDGE));
            (None, "the whole screen".to_string(), w.max(h))
        }
        Target::Window => {
            let w = hypr::active_geometry().ok_or("nothing is focused")?;
            if policy::class_denied(&w.class) {
                return Err(format!(
                    "declined: {} is on the privacy denylist and is never captured",
                    w.class
                ));
            }
            let what = format!("{} — \"{}\"", w.class, w.title);
            (
                Some(format!("{},{} {}x{}", w.x, w.y, w.width, w.height)),
                what,
                w.width.max(w.height),
            )
        }
    };

    let mut cmd = Command::new("grim");
    cmd.arg("-t").arg("png");
    if let Some(g) = &geometry {
        cmd.arg("-g").arg(g);
    }
    if let Some(scale) = downscale(edge) {
        cmd.arg("-s").arg(format!("{scale:.4}"));
    }
    cmd.arg("-"); // stdout

    let out = cmd
        .output()
        .map_err(|e| format!("grim not runnable: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "grim failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    if out.stdout.is_empty() {
        return Err("grim produced nothing".into());
    }

    Ok(Shot {
        png: out.stdout,
        what,
    })
}

/// Scale factor to bring `edge` down to MAX_EDGE, or None when it already
/// fits. Never upscales — a small window stays small.
fn downscale(edge: i64) -> Option<f64> {
    (edge > MAX_EDGE).then(|| MAX_EDGE as f64 / edge as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_windows_are_never_upscaled() {
        assert_eq!(downscale(800), None);
        assert_eq!(downscale(MAX_EDGE), None);
    }

    #[test]
    fn oversized_captures_are_scaled_to_fit() {
        // This monitor: 2560 wide.
        let s = downscale(2560).expect("2560 needs scaling");
        assert!((s - 0.6125).abs() < 1e-6, "got {s}");
        assert!(((2560.0 * s).round() as i64) <= MAX_EDGE);
    }

    #[test]
    fn scaling_always_lands_within_the_budget() {
        for edge in [1569, 1920, 2316, 2560, 3840, 7680] {
            let s = downscale(edge).unwrap();
            assert!(
                ((edge as f64 * s).round() as i64) <= MAX_EDGE,
                "edge {edge}"
            );
        }
    }

    #[test]
    fn target_defaults_to_the_focused_window() {
        assert_eq!(Target::parse("window"), Target::Window);
        assert_eq!(Target::parse(""), Target::Window);
        assert_eq!(Target::parse("nonsense"), Target::Window);
    }

    #[test]
    fn screen_has_the_synonyms_a_model_will_reach_for() {
        for s in ["screen", "all", "monitor", "desktop", "SCREEN"] {
            assert_eq!(Target::parse(s), Target::Screen, "{s}");
        }
    }
}
