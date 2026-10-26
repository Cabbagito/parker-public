// show — "put this in front of me." The verb PARKER.md designed on day one
// and the one that makes the unprompted-YouTube-window story real.
//
// Dispatch is on the shape of `what`: a URL goes to the browser, an existing
// file goes to a viewer, and anything else is treated as text and rendered
// in a floating pane. The floating is hyprland.nix's job, keyed on the
// classes below — this file decides *what* opens, never where windows go.

use super::Done;
use std::path::Path;
use std::process::{Command, Stdio};

/// The pane's window class. MUST contain a dot: ghostty silently ignores
/// `--class` unless it is a valid GTK application id (hyprland.nix learned
/// this the hard way with parker-face).
const PANE_CLASS: &str = "com.parker.show";

pub fn show(what: &str) -> Done {
    let what = what.trim();
    if what.is_empty() {
        return Err("nothing to show".into());
    }

    if what.starts_with("http://") || what.starts_with("https://") {
        return url(what);
    }
    let path = Path::new(what);
    if path.exists() {
        return file(path);
    }
    text(what)
}

fn url(u: &str) -> Done {
    spawn("xdg-open", &[u])?;
    // xdg-open lands the tab; focus brings the window forward. Best-effort:
    // a browser that was not open arrives focused on its own, and a focus
    // that missed is not worth failing a show that worked.
    let mut said = format!("opened {u}");
    if super::focus("chrome").is_ok() {
        said.push_str(" and brought the browser forward");
    }
    Ok(said)
}

fn file(p: &Path) -> Done {
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("it")
        .to_string();
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    // Absolute, so the viewer does not depend on where parker was standing.
    let full = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let full = full.to_string_lossy();

    let (viewer, how) = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp" => ("imv", "floating"),
        "pdf" => ("zathura", "zathura"),
        _ => ("xdg-open", "default app"),
    };
    match spawn(viewer, &[&full]) {
        Ok(()) => Ok(format!("showing {name} ({how})")),
        // A missing viewer degrades to the desktop's own idea of one.
        Err(_) if viewer != "xdg-open" => {
            spawn("xdg-open", &[&full])?;
            Ok(format!("showing {name} (default app)"))
        }
        Err(e) => Err(e),
    }
}

/// Text or markdown, rendered in a floating pane over whatever is on screen.
/// One well-known file rather than temp files: a second show replaces the
/// first, which is what "show me" means, and nothing accumulates.
fn text(t: &str) -> Done {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    let dir = format!("{dir}/parker");
    std::fs::create_dir_all(&dir).map_err(|e| format!("no runtime dir: {e}"))?;
    let path = format!("{dir}/show.md");
    std::fs::write(&path, t).map_err(|e| format!("could not write {path}: {e}"))?;

    spawn(
        "ghostty",
        &[&format!("--class={PANE_CLASS}"), "-e", "glow", "-p", &path],
    )?;
    Ok("showing it (floating pane)".into())
}

/// Fire and forget. The child outlives this one-shot process (init reaps
/// it); a viewer that cannot start is a plain sentence, not a panic.
fn spawn(cmd: &str, args: &[&str]) -> Result<(), String> {
    Command::new(cmd)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{cmd} not runnable: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_refused_before_anything_opens() {
        assert!(show("   ").is_err());
    }

    #[test]
    fn the_pane_class_keeps_its_load_bearing_dot() {
        // ghostty silently drops --class without one, and the window rule
        // then matches nothing — the failure mode is invisible.
        assert!(PANE_CLASS.contains('.'));
    }
}
