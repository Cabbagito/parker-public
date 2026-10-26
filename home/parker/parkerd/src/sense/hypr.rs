// Hyprland, via `hyprctl -j`.
//
// Deliberately shelling out rather than opening `.socket.sock` directly:
// Hyprland evaluates request-socket connections *synchronously*, so a client
// that holds one open freezes the whole compositor until a 5s timeout.
// hyprctl does open/write/read/close correctly, and a fork costs a few ms in
// a CLI. When the daemon wants the event stream it will connect to
// `.socket2.sock` — a different socket with none of that hazard.
//
// Parsed through serde_json::Value rather than typed structs on purpose:
// Hyprland changes its JSON between minor releases without saying so, and a
// missing field should cost us one number, not the whole read.

use super::{Focus, Workspace};
use serde_json::Value;
use std::process::Command;

/// Hyprland addresses its sockets by instance signature. Under UWSM the env
/// var is exported into the user manager, but fall back to the newest
/// instance directory so this still works from a bare shell or an SSH
/// session — which is exactly where it gets tested.
pub fn instance_signature() -> Option<String> {
    if let Ok(his) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        if !his.is_empty() {
            return Some(his);
        }
    }
    let mut instances: Vec<_> = std::fs::read_dir(format!("{}/hypr", runtime_dir()?))
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let t = e.metadata().ok()?.modified().ok()?;
            Some((t, e.file_name().to_string_lossy().into_owned()))
        })
        .collect();
    instances.sort_by_key(|(t, _)| *t);
    instances.pop().map(|(_, name)| name)
}

/// `XDG_RUNTIME_DIR` when the session set one, else `/run/user/<uid>` — which
/// is what it would have been. `ssh parker parker visage` runs a command, not
/// a login shell, and does not always get the variable; the couch is the
/// whole point of typing the verb, so it degrades to the derivable answer
/// rather than to "no Hyprland session". The uid comes from `/proc/self`
/// because that is one syscall and no new dependency.
fn runtime_dir() -> Option<String> {
    use std::os::unix::fs::MetadataExt;

    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !dir.is_empty() {
            return Some(dir);
        }
    }
    let uid = std::fs::metadata("/proc/self").ok()?.uid();
    let guess = format!("/run/user/{uid}");
    std::path::Path::new(&guess).is_dir().then_some(guess)
}

fn hyprctl(args: &[&str]) -> Option<Value> {
    let his = instance_signature()?;
    let out = Command::new("hyprctl")
        .env("HYPRLAND_INSTANCE_SIGNATURE", his)
        .args(["-j"])
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

pub fn active_window() -> Option<Focus> {
    let v = hyprctl(&["activewindow"])?;

    // With nothing focused, hyprctl returns `{}` rather than an error.
    let class = v.get("class")?.as_str()?.to_string();
    if class.is_empty() {
        return None;
    }

    Some(Focus {
        class,
        title: v
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        workspace: v
            .pointer("/workspace/id")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        workspace_name: v
            .pointer("/workspace/name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        fullscreen: v.get("fullscreen").and_then(Value::as_i64).unwrap_or(0),
        floating: v.get("floating").and_then(Value::as_bool).unwrap_or(false),
        redacted: false,
        held_secs: None,
    })
}

/// Enough of a window to point at it. Not part of Sense — geometry is
/// reach's business (grim needs a rectangle), and putting pixel coordinates
/// in the block parker reads every turn would be noise.
pub struct Window {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace: i64,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

fn window_from(v: &Value) -> Option<Window> {
    let at = v.get("at")?.as_array()?;
    let size = v.get("size")?.as_array()?;
    Some(Window {
        address: v.get("address")?.as_str()?.to_string(),
        class: v
            .get("class")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: v
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        workspace: v
            .pointer("/workspace/id")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        x: at.first()?.as_i64()?,
        y: at.get(1)?.as_i64()?,
        width: size.first()?.as_i64()?,
        height: size.get(1)?.as_i64()?,
    })
}

/// Every mapped window. Used by `reach::focus` to resolve a name to an
/// address rather than handing Hyprland a regex and hoping.
pub fn clients() -> Vec<Window> {
    let Some(Value::Array(list)) = hyprctl(&["clients"]) else {
        return Vec::new();
    };
    list.iter()
        .filter(|c| c.get("mapped").and_then(Value::as_bool).unwrap_or(true))
        .filter_map(window_from)
        .filter(|w| !w.class.is_empty())
        .collect()
}

pub fn active_geometry() -> Option<Window> {
    window_from(&hyprctl(&["activewindow"])?).filter(|w| !w.class.is_empty())
}

/// Logical size of the focused monitor, for sizing a full-screen capture.
pub fn monitor_size() -> Option<(i64, i64)> {
    let Value::Array(monitors) = hyprctl(&["monitors"])? else {
        return None;
    };
    let m = monitors
        .iter()
        .find(|m| m.get("focused").and_then(Value::as_bool).unwrap_or(false))
        .or_else(|| monitors.first())?;
    Some((m.get("width")?.as_i64()?, m.get("height")?.as_i64()?))
}

/// The special workspace currently revealed on the focused monitor, without
/// the `special:` prefix. `None` means none is showing — which is also what a
/// monitor that does not report the field means, because a face that might be
/// up is worth less than a face we can honestly say is not.
pub fn special_workspace() -> Option<String> {
    let Value::Array(monitors) = hyprctl(&["monitors"])? else {
        return None;
    };
    let m = monitors
        .iter()
        .find(|m| m.get("focused").and_then(Value::as_bool).unwrap_or(false))
        .or_else(|| monitors.first())?;
    let name = m
        .get("specialWorkspace")
        .and_then(|w| w.get("name"))
        .and_then(Value::as_str)?;
    let name = name.strip_prefix("special:").unwrap_or(name);
    (!name.is_empty()).then(|| name.to_string())
}

pub fn workspaces() -> Vec<Workspace> {
    // The active workspace is a separate call — `workspaces` does not mark it.
    let active = hyprctl(&["activeworkspace"])
        .and_then(|v| v.get("id").and_then(Value::as_i64))
        .unwrap_or(-1);

    let Some(Value::Array(list)) = hyprctl(&["workspaces"]) else {
        return Vec::new();
    };

    let mut out: Vec<Workspace> = list
        .iter()
        .filter_map(|w| {
            let id = w.get("id")?.as_i64()?;
            Some(Workspace {
                id,
                name: w
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                windows: w.get("windows").and_then(Value::as_i64).unwrap_or(0),
                active: id == active,
            })
        })
        .collect();

    // Hyprland returns them in creation order; humans read them in order.
    // Special workspaces carry negative ids and sort ahead, which is fine.
    out.sort_by_key(|w| w.id);
    out
}
