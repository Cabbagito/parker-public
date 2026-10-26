// visage — parker's face as an app, summoned.
//
// The same summon-or-reveal shape SUPER+P has always had, moved out of the
// `parker-visage` shell script and into the binary so there is one
// implementation of it: the keybind runs `parker visage`, and so does a hand
// on the couch over SSH. That is the whole reason this exists as a verb —
// from SSH there is no keybind to press, and every piece of the old script
// (find the instance signature, dispatch, poll) was already here in Rust.
//
// One word, no flags. `parker visage` is the same gesture as SUPER+P: it is
// up, or it is away, and saying it again changes which. A `--hide` would be a
// second way to say a thing the first way already says, and the point of
// typing this from the couch is that it is one word.

use super::{dispatch, lua_str, Done};
use std::time::{Duration, Instant};

/// The live Wayland app_id, not the tauri identifier `com.parker.visage`
/// (visage/AGENTS.md). The Hyprland window rule in home/desktop/hyprland.nix
/// and the special workspace name are keyed on this same string.
const CLASS: &str = "visage";

/// A cold Tauri start during the login stampede takes seconds. Toggling a
/// special workspace whose window has not mapped yet reveals an empty screen
/// and the window then arrives `silent` and stays hidden — which looks
/// exactly like visage failing to launch, so wait for it instead.
const SPAWN_WAIT: Duration = Duration::from_secs(12);
const POLL: Duration = Duration::from_millis(200);

/// Flip visage: up if it is away, away if it is up.
///
/// The state is read rather than assumed only so the answer can say which way
/// it went — `toggle_special` would flip it either way.
/// Bring visage up if it is not already up, and do nothing if it is.
///
/// Not a `--show` flag on `parker visage`, which stays one word with no flags
/// for the reason its header gives: a flag there would be a second way to say
/// the toggle. This says something the toggle *cannot* — "make sure it is
/// up" — and it exists because a gesture that puts on a performance has to be
/// idempotent. Pressing the dance bind twice should not put the face away
/// halfway through the dance it just started.
///
/// Deliberately not called by `dance` itself. A dance parker decides on his
/// own must not throw a fullscreen window over whatever you are doing:
/// gating is about initiative, not obedience (reach/mod.rs), and this is the
/// initiative side. It is reached only when a human asks for it.
pub fn reveal() -> Done {
    if showing() {
        return Ok("visage is already up".into());
    }
    summon()
}

fn showing() -> bool {
    crate::sense::hypr::special_workspace().as_deref() == Some(CLASS)
}

pub fn summon() -> Done {
    let showing = showing();

    // Reveal-or-spawn. Normally the window is already there — visage is
    // started at login and simply lives on its workspace — so this is a
    // reveal, and the face you summon is mid-animation rather than cold.
    if !running() {
        let bin = binary();
        // `exec_cmd` hands its string to a shell, so a path with a space would
        // arrive as two words. Neither candidate path has one; say so rather
        // than producing a baffling hyprland error if that ever changes.
        if bin.contains(' ') {
            return Err(format!("visage's path has a space in it: {bin}"));
        }
        dispatch(&format!("hl.dsp.exec_cmd({})", lua_str(&bin)))?;
        let began = Instant::now();
        while !running() {
            if began.elapsed() >= SPAWN_WAIT {
                return Err(format!(
                    "visage did not come up within {}s ({bin}). \
                     Is it built? cargo tauri build --no-bundle",
                    SPAWN_WAIT.as_secs()
                ));
            }
            std::thread::sleep(POLL);
        }
        toggle()?;
        nudge_pointer();
        return Ok("started visage and brought it up".into());
    }

    toggle()?;
    if !showing {
        nudge_pointer();
    }
    Ok(if showing {
        "put visage away".into()
    } else {
        "brought visage up".into()
    })
}

/// Move the pointer one pixel and back, after a reveal.
///
/// visage hides the mouse cursor with `cursor: none` (visage/src/app.css),
/// but a Wayland client only gets to say what the cursor looks like when the
/// pointer hands it an event. Revealing a workspace under a pointer nobody
/// has touched is not such an event, so without this you press SUPER+P and
/// the arrow from whatever was underneath sits frozen on parker's face until
/// you happen to jog the mouse. Verified both ways by screenshot with
/// `grim -c`, July 2026: stale arrow without the nudge, no cursor with it.
///
/// One pixel out and one back is a real motion event that leaves the pointer
/// exactly where it was — a warp to some fixed spot would hide the cursor
/// just as well and move the user's mouse to do it.
///
/// Only on the way up. Dismissing visage needs no counterpart: the window
/// underneath sets its own cursor the next time the pointer speaks to it, and
/// nothing about being hidden makes visage's `none` outlive its surface
/// (checked with the same screenshots).
///
/// Best-effort by design. A face with a stray arrow on it is worth far more
/// than no face, so a failure here is swallowed rather than failing the
/// summon — PARKER.md invariant 1, parker is never in the critical path.
/// `movecursorrelative` does not exist any more.
///
/// It did when this was written, and the swallowed errors meant its removal
/// was completely silent — the two dispatches simply failed and the stale
/// arrow came back. Confirmed live on Hyprland 0.56.0: `hyprctl dispatch
/// movecursorrelative 1 0` answers "Invalid dispatcher", and `movecursor` is
/// the only one left, taking absolute coordinates. (Under the Lua config it
/// is spelled `hl.dsp.cursor.move({ x, y })`; same absolute semantics.)
///
/// So the position is read first and restored explicitly. That keeps the
/// original intent — the header rejects a warp to a fixed spot precisely
/// because it would move the user's mouse — while using the dispatcher that
/// still exists. Net movement is one pixel out and one back, as before.
fn nudge_pointer() {
    // The reveal is animated: land the pointer on visage's surface rather
    // than on the one still sliding out from under it.
    std::thread::sleep(POLL);
    let Some((x, y)) = cursor_pos() else { return };
    let _ = dispatch(&format!("hl.dsp.cursor.move({{ x = {}, y = {y} }})", x + 1));
    let _ = dispatch(&format!("hl.dsp.cursor.move({{ x = {x}, y = {y} }})"));
}

/// Show or hide visage's special workspace.
fn toggle() -> Result<(), String> {
    dispatch(&format!(
        "hl.dsp.workspace.toggle_special({})",
        lua_str(CLASS)
    ))
}

/// Where the pointer is, as `hyprctl cursorpos` reports it: `"1234, 567"`.
/// Plain text rather than JSON because that is all this route emits.
fn cursor_pos() -> Option<(i64, i64)> {
    let his = crate::sense::hypr::instance_signature()?;
    let out = std::process::Command::new("hyprctl")
        .env("HYPRLAND_INSTANCE_SIGNATURE", his)
        .arg("cursorpos")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (x, y) = text.trim().split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn running() -> bool {
    crate::sense::hypr::clients()
        .iter()
        .any(|c| c.class == CLASS)
}

/// Where visage's binary is, most explicit first:
///
/// 1. `PARKER_VISAGE_BIN`, for anyone pointing at a build of their own;
/// 2. the dev-loop build under the repo checkout — scaffolding, and the one
///    place that path is written now that the shell script is gone. The
///    visage package derivation replaces this branch when it lands;
/// 3. bare `visage`, resolved by Hyprland's PATH, which is what will be
///    correct once it is a nix artifact.
fn binary() -> String {
    choose(
        std::env::var_os("PARKER_VISAGE_BIN").map(|b| b.to_string_lossy().into_owned()),
        std::env::var_os("HOME").map(std::path::PathBuf::from),
    )
}

/// Split out from the environment so it can be tested without touching it —
/// `set_var` is a process-global write and the test runner is threaded.
fn choose(explicit: Option<String>, home: Option<std::path::PathBuf>) -> String {
    if let Some(explicit) = explicit.filter(|e| !e.is_empty()) {
        return explicit;
    }
    if let Some(dev) = home
        .map(|h| h.join("parker/home/parker/visage/src-tauri/target/release/visage"))
        .filter(|d| d.exists())
    {
        return dev.to_string_lossy().into_owned();
    }
    CLASS.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_binary_wins_over_everything() {
        let chosen = choose(Some("/somewhere/visage".into()), Some("/home/peki".into()));
        assert_eq!(chosen, "/somewhere/visage");
    }

    #[test]
    fn with_no_dev_build_and_no_override_it_falls_back_to_the_path() {
        assert_eq!(choose(None, Some("/nonexistent".into())), "visage");
        assert_eq!(choose(Some(String::new()), None), "visage");
    }
}
