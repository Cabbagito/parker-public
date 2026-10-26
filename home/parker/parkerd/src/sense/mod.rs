// The input layer: where parker gets information. One struct, one gather().
//
// Every field is optional-shaped or has a defined empty state, because an
// input that cannot answer must degrade to "I don't know" rather than take
// the process down with it — parker is never in the critical path
// (PARKER.md invariant 1), and that starts here.
//
// `version` is never removed and fields are never reordered. That is the
// entirety of what survived the replay invariant: anything that later wants
// to log a decision can log the Sense beside it for free.

pub mod hypr;
pub mod machine;
pub mod media;
pub mod policy;
pub mod web;

use serde::Serialize;

pub const VERSION: u32 = 1;

#[derive(Serialize, Debug, Clone)]
pub struct Sense {
    pub version: u32,
    pub at: String,
    pub uptime_secs: u64,

    pub focus: Option<Focus>,
    pub workspaces: Vec<Workspace>,
    pub media: Option<Media>,
    pub machine: Machine,

    pub inferred: Inferred,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Focus {
    pub class: String,
    pub title: String,
    pub workspace: i64,
    pub workspace_name: String,
    pub fullscreen: i64,
    pub floating: bool,
    /// True when policy replaced the title. The flag is reported rather than
    /// hidden — parker knowing it is not being told something is better than
    /// parker silently believing the window has no title.
    pub redacted: bool,
    /// Seconds this window has been focused. Only ever populated in watch
    /// mode; a single reading cannot know it. None means "not measured",
    /// never "zero".
    pub held_secs: Option<u64>,
}

impl Focus {
    /// Identity for "is this still the same thing I'm looking at". Title is
    /// excluded on purpose: a browser retitles on every tab switch, and
    /// treating that as a new focus would make held_secs useless.
    pub fn identity(&self) -> String {
        format!("{}::{}", self.class, self.workspace)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub windows: i64,
    pub active: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct Media {
    pub player: String,
    pub status: String,
    pub title: Option<String>,
    pub artist: Option<String>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct Machine {
    pub gpu: Option<Gpu>,
    pub mem_used_gb: f64,
    pub mem_total_gb: f64,
    pub disk_used_pct: Option<u8>,
    pub failed_units: usize,
    pub failed_user_units: usize,
    // Appended (fields are never reordered): the CPU, for visage's corner
    // meters as much as for the prompt.
    pub cpu: Option<Cpu>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Cpu {
    pub util_pct: u8,
    pub temp_c: Option<u8>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Gpu {
    pub util_pct: u8,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub temp_c: u8,
    pub watts: f64,
}

#[derive(Serialize, Debug, Clone)]
pub struct Inferred {
    pub mode: String,
    pub interruptible: bool,
    pub health: String,
}

/// Read everything. `held_secs` is threaded in from watch mode rather than
/// measured here, because a one-shot read has nothing to measure against.
pub fn gather(held_secs: Option<u64>) -> Sense {
    let mut focus = hypr::active_window();
    if let Some(f) = focus.as_mut() {
        policy::redact(f);
        f.held_secs = held_secs;
    }

    let workspaces = hypr::workspaces();
    let media = media::current();
    let machine = machine::read();
    let inferred = derive(focus.as_ref(), &machine);

    Sense {
        version: VERSION,
        at: machine::local_time(),
        uptime_secs: machine::uptime_secs(),
        focus,
        workspaces,
        media,
        machine,
        inferred,
    }
}

/// The derived layer, v0 edition — deliberately crude and deliberately small.
///
/// `mode` is a class lookup and `interruptible` is "are you fullscreen". Both
/// are placeholders for the real thing, which needs history: how long you have
/// been here, whether you just came back, whether the last five minutes look
/// like reading or like thrashing. That arrives with the daemon. Guessing
/// harder from a single sample would only produce confident nonsense.
fn derive(focus: Option<&Focus>, machine: &Machine) -> Inferred {
    // Parker's own face is fullscreen by design, and fullscreen is how we
    // infer "busy" — so without this, summoning parker made parker decide the
    // user was unavailable and play the away animation at them. Looking at
    // parker is the most interruptible anyone ever is.
    let at_parker = focus
        .map(|f| f.class == crate::face::FACE_CLASS)
        .unwrap_or(false);

    let mode = if at_parker {
        "parker".to_string()
    } else {
        focus
            .map(|f| mode_for(&f.class, f.fullscreen))
            .unwrap_or("idle")
            .to_string()
    };

    let interruptible = at_parker || focus.map(|f| f.fullscreen == 0).unwrap_or(true);

    let health = if machine.failed_units > 0 || machine.failed_user_units > 0 {
        "degraded"
    } else {
        "ok"
    }
    .to_string();

    Inferred {
        mode,
        interruptible,
        health,
    }
}

/// Class → mode. The citizens of this machine are known (DECISIONS.md's
/// workspace-per-app model), so a lookup beats a heuristic. An unknown class
/// held fullscreen is the one guess worth making: that is what a game looks
/// like from here.
fn mode_for(class: &str, fullscreen: i64) -> &'static str {
    match class {
        "google-chrome" | "chromium-browser" => "browsing",
        "com.anthropic.Claude" => "claude",
        "com.mitchellh.ghostty" => "terminal",
        "spotify" => "music",
        "discord" => "chat",
        "steam" | "steam_app" | "dolphin-emu" | "Slippi Dolphin" => "gaming",
        "org.pwmt.zathura" | "imv" | "mpv" => "reading",
        _ if fullscreen != 0 => "gaming",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus(class: &str, fullscreen: i64) -> Focus {
        Focus {
            class: class.into(),
            title: "t".into(),
            workspace: 1,
            workspace_name: "1".into(),
            fullscreen,
            floating: false,
            redacted: false,
            held_secs: None,
        }
    }

    #[test]
    fn known_citizens_map_to_their_mode() {
        assert_eq!(mode_for("google-chrome", 0), "browsing");
        assert_eq!(mode_for("com.mitchellh.ghostty", 0), "terminal");
        assert_eq!(mode_for("spotify", 0), "music");
    }

    #[test]
    fn unknown_fullscreen_class_reads_as_gaming() {
        assert_eq!(mode_for("some-unknown-game", 1), "gaming");
        assert_eq!(mode_for("some-unknown-game", 0), "other");
    }

    #[test]
    fn a_known_citizen_stays_itself_even_fullscreen() {
        // Fullscreen Chrome is a video, not a game.
        assert_eq!(mode_for("google-chrome", 1), "browsing");
    }

    #[test]
    fn fullscreen_means_do_not_interrupt() {
        let m = Machine::default();
        assert!(!derive(Some(&focus("anything", 1)), &m).interruptible);
        assert!(derive(Some(&focus("anything", 0)), &m).interruptible);
    }

    #[test]
    fn looking_at_parker_is_never_read_as_being_busy() {
        // The face is fullscreen by design, and fullscreen is how "busy" is
        // inferred — so summoning parker made parker conclude the user was
        // unavailable and play the away animation at them.
        let m = Machine::default();
        let i = derive(Some(&focus(crate::face::FACE_CLASS, 1)), &m);
        assert!(i.interruptible, "looking at parker is maximally available");
        assert_eq!(i.mode, "parker");
        // And it is genuinely the fullscreen exception, not a blanket one.
        assert!(!derive(Some(&focus("some-game", 1)), &m).interruptible);
    }

    #[test]
    fn no_focus_is_idle_and_interruptible() {
        let i = derive(None, &Machine::default());
        assert_eq!(i.mode, "idle");
        assert!(i.interruptible);
    }

    #[test]
    fn failed_units_degrade_health() {
        let mut m = Machine::default();
        assert_eq!(derive(None, &m).health, "ok");
        m.failed_user_units = 1;
        assert_eq!(derive(None, &m).health, "degraded");
    }

    #[test]
    fn focus_identity_ignores_title_churn() {
        // Tab switches must not reset held_secs.
        let mut a = focus("google-chrome", 0);
        let mut b = a.clone();
        a.title = "Inbox".into();
        b.title = "Some other tab".into();
        assert_eq!(a.identity(), b.identity());
    }

    #[test]
    fn focus_identity_separates_workspaces() {
        let a = focus("com.mitchellh.ghostty", 0);
        let mut b = a.clone();
        b.workspace = 4;
        assert_ne!(a.identity(), b.identity());
    }
}
