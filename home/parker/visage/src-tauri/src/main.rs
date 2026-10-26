// visage — parker's face, the fullscreen surface you talk to.
//
// Division of labour: the webview renders and nothing else — it never sees
// the engine's password and never speaks HTTP. Frontend calls arrive as
// tauri commands and engine events leave as one tauri event stream
// ("engine-event"), mirroring PARKER.md's "one merged event stream".
//
// Every command body lives in ../door (visage-door), because this window is
// one of two frames on that door: the web server in ../porthole serves the same
// calls to the tailnet. What stays here is what only a window on parker's
// own screen has — `dismiss`, the GTK and WebKit repairs, and the window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use visage_door::{Engine, Event};

#[tauri::command]
async fn create_session(engine: State<'_, Engine>) -> Result<Value, String> {
    engine.create_session().await
}

#[tauri::command]
async fn get_messages(engine: State<'_, Engine>, session_id: String) -> Result<Value, String> {
    engine.get_messages(&session_id).await
}

/// This window is on the tower's own screen, so it says so with every
/// message (door's `at_the_tower`).
#[tauri::command]
async fn send_message(
    engine: State<'_, Engine>,
    session_id: String,
    text: String,
    model: Option<String>,
) -> Result<Value, String> {
    let here = visage_door::at_the_tower();
    engine
        .send_message(&session_id, &text, model.as_deref(), Some(&here))
        .await
}

#[tauri::command]
async fn list_sessions(engine: State<'_, Engine>) -> Result<Value, String> {
    engine.list_sessions().await
}

#[tauri::command]
async fn interrupt_session(engine: State<'_, Engine>, session_id: String) -> Result<Value, String> {
    engine.interrupt_session(&session_id).await
}

#[tauri::command]
async fn summarize(engine: State<'_, Engine>, text: String) -> Result<String, String> {
    engine.summarize(&text).await
}

#[tauri::command]
fn digests(ids: Vec<String>) -> Value {
    visage_door::digests(&ids)
}

#[tauri::command]
async fn digest(
    engine: State<'_, Engine>,
    session_id: String,
    text: String,
    facts: Value,
) -> Result<Value, String> {
    engine.digest(&session_id, &text, facts).await
}

#[tauri::command]
fn marks(ids: Vec<String>) -> Value {
    visage_door::marks(&ids)
}

/// ^X in the palette: your word on a thread, through `parker recall mark`.
#[tauri::command]
async fn mark(session_id: String, state: String) -> Result<Value, String> {
    visage_door::mark(session_id, state).await
}

#[tauri::command]
fn face_state() -> Value {
    visage_door::face_state()
}

/// Put visage away — Esc's last tier.
///
/// It runs `parker visage` rather than dispatching to Hyprland here, because
/// `parker visage` *is* the summon path (reach/visage.rs): the keybind script
/// and parker-greet both go through it, and a second implementation living in
/// the app would be one that drifts. Best-effort and detached — a window that
/// will not hide is not worth an error, and parker is never in the critical
/// path, least of all of himself.
///
/// The wait runs on its own thread only to reap the child; `parker visage`
/// returns as soon as the workspace has flipped.
#[tauri::command]
fn dismiss() {
    visage_door::parker(&["visage".into()]);
}

#[tauri::command]
fn dance(routine: String, seconds: Option<f64>) {
    visage_door::dance(routine, seconds);
}

#[tauri::command]
async fn theme(args: Vec<String>) -> Result<String, String> {
    visage_door::theme(args).await
}

/// Which brain parker is on, or — with a name — switch to it (door's
/// `brain` / `set_brain`).
#[tauri::command]
async fn brain(name: Option<String>) -> Result<Value, String> {
    match name {
        Some(n) => visage_door::set_brain(n).await,
        None => Ok(visage_door::brain().await),
    }
}

#[tauri::command]
async fn sense_snapshot() -> Value {
    visage_door::sense_snapshot().await
}

/// ^,: every setting as a row (door's `controls`).
#[tauri::command]
async fn controls() -> Value {
    visage_door::controls().await
}

/// ^,: one change. This window is at the tower, so nothing is the tower's
/// alone here (door's `set_control`, `away` false).
#[tauri::command]
async fn set_control(id: String, value: String) -> Result<Value, String> {
    visage_door::set_control(id, value, false).await
}

/// The corner's spaces line, read once; `spaces` events follow it.
#[tauri::command]
fn spaces() -> Value {
    visage_door::spaces()
}

/// Tail the engine's bus forever (door's `pump`), re-emitting every event to
/// the webview.
async fn pump_events(app: AppHandle) {
    let engine = app.state::<Engine>();
    engine
        .pump(|event| {
            let _ = match event {
                Event::Status(s) => app.emit("engine-status", s),
                Event::Engine(v) => app.emit("engine-event", v),
            };
        })
        .await;
}

/// Give GTK a screen resolution if nothing else has.
///
/// `gdk_screen_get_resolution()` returns -1 when unset, and WebKitGTK divides
/// that by 96 to get the device pixel ratio — so the webview comes up at
/// dpr = -1/96 and *every CSS length goes negative*. A 2560x1440 window
/// reports `innerWidth` -245760, `100vh` resolves below zero, and the whole
/// document lays out at zero size: a window that is up, focused, fullscreen
/// and completely blank. Measured in exactly that state, July 2026.
///
/// It is unset here because nothing fills it in: there is no XSETTINGS daemon
/// under Hyprland, and the generated `~/.config/gtk-3.0/settings.ini` carries
/// theme, cursor and font but no `gtk-xft-dpi`. Most GTK apps never notice —
/// they treat -1 as "use 96" — which is why this surfaces only in the webview.
///
/// 96 dpi (`96 * 1024`, the units GTK stores it in) is the standard default,
/// so this restores what everything else already assumes. Anything that *has*
/// set a resolution wins: a real HiDPI setting is left alone.
///
/// The alternative fix lives in the machine rather than the app —
/// `gtk-xft-dpi` in home-manager's gtk config — and would help any other
/// WebKit app that trips on this. Kept here because visage already carries
/// its own WebKit workaround, and because a face that renders should not
/// depend on a system rebuild.
fn sane_dpi() {
    use gtk::prelude::*;
    if gtk::init().is_err() {
        return; // No display: nothing to render on either.
    }
    if let Some(settings) = gtk::Settings::default() {
        if settings.gtk_xft_dpi() <= 0 {
            settings.set_gtk_xft_dpi(96 * 1024);
        }
    }
}

fn main() {
    // WebKitGTK's dmabuf renderer dies with Wayland "Error 71 (Protocol
    // error)" on this NVIDIA + Hyprland combination (observed at first
    // launch, July 2026 — QUIRKS.md). Must be set before GTK initializes;
    // respect an explicit override from the environment.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // Before tao builds the window, and so before the webview reads it.
    sane_dpi();

    tauri::Builder::default()
        .setup(|app| {
            match Engine::from_env() {
                Ok(engine) => {
                    app.manage(engine);
                    let handle = app.handle().clone();
                    tauri::async_runtime::spawn(pump_events(handle));
                }
                Err(e) => {
                    // Still start: the face renders and reports, it just
                    // cannot speak. Never in the critical path, even of itself.
                    eprintln!("visage: engine unavailable: {e}");
                }
            }
            // The corner's spaces, as Hyprland moves them (door's `watch_spaces`).
            let spaces_to = app.handle().clone();
            visage_door::watch_spaces(move |v| {
                let _ = spaces_to.emit("spaces", v);
            });
            // A level moved by something other than parker or ^, — the
            // keys on the desk — for a quiet line in the transcript (door's
            // `watch_levels`).
            let levels_to = app.handle().clone();
            visage_door::watch_levels(move |v| {
                let _ = levels_to.emit("levels", v);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_session,
            get_messages,
            list_sessions,
            send_message,
            interrupt_session,
            summarize,
            digests,
            digest,
            marks,
            mark,
            dismiss,
            face_state,
            dance,
            theme,
            brain,
            sense_snapshot,
            controls,
            set_control,
            spaces,
        ])
        .run(tauri::generate_context!())
        .expect("visage failed to start");
}
