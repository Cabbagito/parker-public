// parker.
//
// Two halves that touch the world, and only two:
//
//   sense  — where parker gets information
//   reach  — what parker can change
//
// Plus `mcp`, a doorway that hands both to a model, `face`, which is the
// surface parker is seen on, and `brain`, which model he thinks with. Still
// no daemon: sense, reach and the face are
// all one-shot or file-backed, so nothing here needs a resident process yet.
// The daemon arrives when parker needs to notice things nobody asked about.

mod brain;
mod controls;
mod face;
mod mcp;
mod reach;
mod recall;
mod render;
mod sense;

use clap::{Parser, Subcommand};
use face::paint::Palette;
use face::pose::{ease, Motion, Pose};
use face::Expression;
use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(
    name = "parker",
    version,
    about = "parker — this machine, with senses and reach"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Read everything parker can currently perceive.
    Sense {
        /// Emit the full structure as JSON instead of the compact text form.
        #[arg(long)]
        json: bool,
        /// Re-read every SECS seconds until interrupted (default 2).
        #[arg(long, value_name = "SECS", num_args = 0..=1, default_missing_value = "2")]
        watch: Option<u64>,
    },

    /// Change something about the machine.
    Reach {
        #[command(subcommand)]
        verb: Verb,
    },

    /// Search the web through the machine's own engine (SearXNG, localhost).
    Search {
        /// The query. Several words need no quotes.
        #[arg(required = true, trailing_var_arg = true)]
        query: Vec<String>,
    },

    /// Read back past conversations: list them, search what was said, or ask
    /// the Flatline a question about one.
    Recall {
        #[command(subcommand)]
        what: Recall,
    },

    /// The settings visage's ^, shows: sound and each app's level, what is
    /// playing, wifi, tailscale, bluetooth, the desktop's look, do not
    /// disturb, parker, the session. No subcommand reads them all; `set`
    /// changes one (docs/visage/controls/README.md).
    Controls {
        #[command(subcommand)]
        what: Option<ControlsCmd>,
        /// As JSON, for visage.
        #[arg(long, global = true)]
        json: bool,
        /// One section alone (`sound`, `network`, …): what door's level
        /// watch reads on every PipeWire change.
        #[arg(long)]
        only: Option<String>,
    },

    /// Serve sense and reach to a model as MCP tools, over stdio.
    Mcp,

    /// Open opencode's coding TUI as parker: full coding tools, parker's
    /// MCP verbs, the machine's theme. Extra args pass through to opencode.
    Code {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show visage — parker's face as an app — or put it away again. The same
    /// gesture as SUPER+P, and it starts visage first if it is not running.
    /// Typed rather than bound: this is how you reach the face over SSH.
    Visage,

    /// Start dancing, or stop. One word, no flags, and saying it again
    /// changes which — the same shape as `parker visage`. Every face parker
    /// has reflects it; this does not open or close a window.
    Dance,

    /// Which brain parker thinks with. No name says which; a name — or
    /// enough of one: `jrti`, `opus` — picks it, from the next message on.
    Brain {
        /// The brain to switch to. Several words need no quotes.
        name: Vec<String>,
        /// Every brain, the one in use marked.
        #[arg(long, conflicts_with_all = ["model", "json"])]
        list: bool,
        /// Print only the provider/model string, for scripts.
        #[arg(long, conflicts_with = "json")]
        model: bool,
        /// The brain in use, the default and the set, as JSON (for visage).
        #[arg(long)]
        json: bool,
    },

    /// Draw parker's face.
    Face {
        /// Draw a named expression instead of the one parker is wearing.
        #[arg(long)]
        expression: Option<String>,
        /// Where in the idle loop to draw, 0.0-1.0.
        #[arg(long, default_value_t = 0.5)]
        phase: f64,
        /// Live: follow what parker is doing and what it is wearing, with
        /// transitions. This is the face — run it in a terminal and leave it.
        #[arg(long)]
        animate: bool,
        /// Print every expression, one after another.
        #[arg(long)]
        sheet: bool,
        /// With --sheet, emit the whole set as JSON: base64 dot masks per
        /// frame, for surfaces that draw dots rather than print braille.
        /// With --catalog, emit the idle and dance tables.
        #[arg(long)]
        json: bool,
        /// Emit the idle-beat and dance-routine catalogs as JSON, for visage
        /// to read instead of transcribing them (face/catalog.rs). Implies
        /// --json; there is no text form worth printing.
        #[arg(long)]
        catalog: bool,
        /// Emit the vignettes — the long idle pieces, built rather than
        /// written out — as JSON for visage (face/catalog.rs). Apart from
        /// --catalog because of their size: visage loads them after first
        /// paint rather than inside its bundle.
        #[arg(long)]
        vignettes: bool,
        /// Cue an idle beat by name on every live face: it plays once the face
        /// is resting and has been quiet a few seconds, at once if it already
        /// is. The iteration loop for the flair catalog: seeing `droop` should
        /// not take twenty minutes of being ignored. `--play ?` lists the names.
        #[arg(long, value_name = "BEAT")]
        play: Option<String>,
        /// What parker has worn: every face, how long it stayed and what
        /// ended it (face/log.rs). The last 20 unless --since or -n says.
        /// With --json, the raw diary lines.
        #[arg(long)]
        log: bool,
        /// With --log: only faces put on within this long — 90s, 30m, 1h, 2d.
        #[arg(long, value_name = "AGO", requires = "log")]
        since: Option<String>,
        /// With --log: only the last N faces.
        #[arg(short = 'n', value_name = "N", requires = "log")]
        last: Option<usize>,
    },
}

#[derive(Subcommand)]
enum Recall {
    /// Every thread, newest first: when, title, turns, size, the visage
    /// card's line when visage has written one, and where it stands.
    List {
        /// Only threads whose title holds this.
        #[arg(long)]
        title: Option<String>,
        /// Only threads left open — still waiting on someone — each with
        /// its next steps.
        #[arg(long)]
        open: bool,
        /// Only threads touched within this long — 90m, 12h, 7d.
        #[arg(long, value_name = "AGO")]
        since: Option<String>,
        /// At most N threads.
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Lines that were said holding every word of the query, any order.
    Search {
        #[arg(required = true)]
        query: Vec<String>,
        /// Match the query as one phrase instead.
        #[arg(long)]
        exact: bool,
        #[arg(long, value_name = "AGO")]
        since: Option<String>,
        /// At most N threads.
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Ask the Flatline one question about one thread. "summarise it" is a
    /// question.
    Ask {
        /// The thread's id, or any piece of it that names only one.
        thread: String,
        #[arg(required = true)]
        question: Vec<String>,
    },
    /// Your word on a thread: done, still open, or clear to let its card
    /// speak again. Holds until the thread moves. visage's ^X runs this.
    Mark {
        /// The thread's id, or any piece of it that names only one.
        thread: String,
        #[arg(value_parser = ["done", "open", "clear"])]
        state: String,
        /// Why, in a line — shown under the thread.
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ControlsCmd {
    /// Change one: `set output 40`, `set app:spotify -10`, `set bt on`,
    /// `set wifi off`. The row's id, or its name (`spotify`, `quiet`).
    Set {
        id: String,
        /// The value: a level (45, +5, -5, mute, unmute), on/off/toggle, a
        /// choice, connect/disconnect, play/pause/next/previous, or
        /// lock/sleep/reboot/off. Several words need no quotes.
        #[arg(required = true, allow_negative_numbers = true)]
        value: Vec<String>,
        /// The hand is not at the tower (the porthole): refuse what would
        /// cut the line it is on — wifi off, tailscale, power off.
        #[arg(long)]
        away: bool,
    },
}

#[derive(Subcommand)]
enum Verb {
    /// Send a desktop notification. First sentence becomes the title.
    Notify {
        text: String,
        #[arg(long, default_value = "normal")]
        urgency: String,
    },
    /// Screenshot the focused window (or the whole screen) as PNG.
    Look {
        #[arg(long, default_value = "window")]
        target: String,
    },
    /// Switch to a workspace.
    Goto { space: i64 },
    /// Put a running application in front of you.
    Focus { app: String },
    /// Wear an expression.
    Face {
        expression: String,
        /// Seconds to hold it. Omitted uses the expression's own length;
        /// 0 holds until something changes it.
        #[arg(long = "for", value_name = "SECS")]
        hold: Option<f64>,
        /// Play the matching sound, if one is installed.
        #[arg(long)]
        sound: bool,
    },
    /// The turn is over: take off a work posture if one is still worn.
    /// Called by whatever knows a turn ended (visage's door), not by parker.
    Settle,
    /// Control the music: play, pause, toggle, next, previous — or the
    /// loudness: louder, quieter, volume --percent N.
    Media {
        action: String,
        /// Prefer a player by name (substring: 'spotify', 'mpv').
        #[arg(long)]
        player: Option<String>,
        /// With `volume`: the target, 0-100.
        #[arg(long)]
        percent: Option<f64>,
    },
    /// Put something in front of you: a URL, a file path, or text.
    Show { what: String },
    /// Dance. `auto` picks a routine, `stop` puts it away.
    Dance {
        #[arg(default_value = "auto")]
        routine: String,
        /// Seconds to keep dancing; 0 means until something stops it.
        #[arg(long = "for", value_name = "SECS")]
        hold: Option<f64>,
        /// Bring visage up first if it is away. The "put it on" gesture;
        /// SUPER+ALT+P is bound to exactly this.
        #[arg(long)]
        show: bool,
    },
}

fn main() {
    let code = match Cli::parse().cmd {
        Cmd::Sense { json, watch } => match watch {
            None => {
                println!("{}", render_one(&sense::gather(None), json));
                0
            }
            Some(secs) => {
                watch_loop(secs.max(1), json);
                0
            }
        },
        Cmd::Reach { verb } => run_verb(verb),
        Cmd::Search { query } => report(sense::web::search(&query.join(" "))),
        Cmd::Recall { what } => run_recall(what),
        Cmd::Controls { what, json, only } => run_controls(what, json, only.as_deref()),
        Cmd::Mcp => {
            mcp::serve();
            0
        }
        Cmd::Code { args } => run_code(&args),
        Cmd::Visage => report(reach::visage::summon()),
        Cmd::Dance => report(reach::dance_toggle()),
        Cmd::Brain {
            name,
            list,
            model,
            json,
        } => report(if !name.is_empty() {
            brain::set(&name.join(" "))
        } else if list {
            brain::list()
        } else if model {
            brain::model()
        } else if json {
            brain::json()
        } else {
            brain::show()
        }),
        Cmd::Face {
            log: true,
            since,
            last,
            json,
            ..
        } => show_log(since.as_deref(), last, json),
        Cmd::Face {
            expression,
            phase,
            animate,
            sheet,
            json,
            catalog,
            vignettes,
            play,
            ..
        } => run_face(
            expression.as_deref(),
            phase,
            animate,
            sheet,
            json,
            catalog,
            vignettes,
            play.as_deref(),
        ),
    };
    std::process::exit(code);
}

fn run_recall(what: Recall) -> i32 {
    let filter = |since: Option<String>, limit: usize| -> Result<recall::Filter, String> {
        let since = match since {
            Some(s) => Some(parse_ago(&s).ok_or("--since wants a span like 90m, 12h or 7d")?),
            None => None,
        };
        Ok(recall::Filter { since, limit })
    };
    report(match what {
        Recall::List {
            title,
            open,
            since,
            limit,
            json,
        } => filter(since, limit).and_then(|f| recall::list(&f, title.as_deref(), open, json)),
        Recall::Search {
            query,
            exact,
            since,
            limit,
            json,
        } => filter(since, limit).and_then(|f| recall::search(&query.join(" "), exact, &f, json)),
        Recall::Ask { thread, question } => recall::ask(&thread, &question.join(" ")),
        Recall::Mark {
            thread,
            state,
            note,
            json,
        } => recall::mark(&thread, &state, note.as_deref(), recall::By::Peter, json),
    })
}

/// `parker controls`: the list, or one change. A name is resolved to its
/// row's id here, so `set spotify 20` works by hand; door hands over ids.
fn run_controls(what: Option<ControlsCmd>, json: bool, only: Option<&str>) -> i32 {
    match what {
        None => {
            let sections = match only {
                None => controls::snapshot(),
                Some(id) => match controls::section(id) {
                    Some(s) => vec![s],
                    None => {
                        eprintln!(
                            "no section '{id}'. sections: {}",
                            controls::section_ids().join(", ")
                        );
                        return 2;
                    }
                },
            };
            if json {
                println!("{}", serde_json::json!({ "sections": sections }));
            } else {
                println!("{}", controls::text(&sections));
            }
            0
        }
        Some(ControlsCmd::Set { id, value, away }) => {
            let value = value.join(" ");
            let id = controls::resolve(&id).unwrap_or(id);
            match controls::set(&id, &value, away) {
                Ok(c) if json => {
                    println!("{}", serde_json::to_string(&c).expect("Changed is serializable"));
                    0
                }
                Ok(c) => report(Ok(c.said)),
                Err(e) => report(Err(e)),
            }
        }
    }
}

/// Exec into opencode wearing parker's coding config (home/parker/code.nix
/// writes it). opencode splits its config in two — the agent's half behind
/// OPENCODE_CONFIG, the TUI's half (theme, keybinds, the home-screen wordmark
/// plugin) behind OPENCODE_TUI_CONFIG — so parker points at both. An explicit
/// value from the environment wins; a missing file means launch plain opencode
/// rather than fail — parker is never in the critical path, including of a
/// coding session.
fn run_code(args: &[String]) -> i32 {
    use std::os::unix::process::CommandExt;

    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")));

    let mut cmd = std::process::Command::new("opencode");
    for (var, file) in [
        ("OPENCODE_CONFIG", "parker/code.json"),
        ("OPENCODE_TUI_CONFIG", "parker/tui.json"),
    ] {
        if std::env::var_os(var).is_some() {
            continue;
        }
        let path = config_home.as_ref().map(|c| c.join(file));
        if let Some(path) = path.filter(|p| p.exists()) {
            cmd.env(var, path);
        }
    }
    // The session starts on whichever brain parker is on (brain.rs). No
    // brain to be had means opencode's own default, code.json's model.
    if wants_brain(args) {
        if let Ok((brain, _)) = brain::current() {
            cmd.args(["--model", &brain.model]);
        }
    }
    let err = cmd.args(args).exec();
    eprintln!("parker code: cannot launch opencode: {err}");
    1
}

/// Whether `parker code` should hand opencode the current brain. Not when
/// the call already names a model, and not past a subcommand: `parker code
/// auth login` must not grow a flag it never asked for. A bare word that is
/// a directory is the TUI's project argument, not a subcommand.
fn wants_brain(args: &[String]) -> bool {
    let names_model = args
        .iter()
        .any(|a| a == "-m" || a == "--model" || a.starts_with("--model=") || a.starts_with("-m="));
    let subcommand = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .is_some_and(|a| !std::path::Path::new(a).is_dir());
    !names_model && !subcommand
}

fn run_verb(verb: Verb) -> i32 {
    let told = match verb {
        Verb::Notify { text, urgency } => reach::notify(&text, reach::Urgency::parse(&urgency)),
        Verb::Goto { space } => reach::goto(space),
        Verb::Focus { app } => reach::focus(&app),
        Verb::Face {
            expression,
            hold,
            sound,
        } => reach::face(&expression, hold, sound),
        Verb::Settle => reach::settle(),
        Verb::Media {
            action,
            player,
            percent,
        } => reach::media::media(&action, player.as_deref(), percent),
        Verb::Show { what } => reach::show::show(&what),
        Verb::Dance {
            routine,
            hold,
            show,
        } => reach::dance(&routine, hold, show),
        Verb::Look { target } => return run_look(&target),
    };
    report(told)
}

// ── the face ────────────────────────────────────────────────────────────────

/// Frames per idle loop when exporting.
const SHEET_FRAMES: usize = 20;
/// Redraw interval. 25fps is plenty for a braille grid and keeps transitions
/// from stepping visibly.
const TICK: Duration = Duration::from_millis(40);
/// Sense forks several processes, so it is polled on its own slow clock
/// rather than every frame.
const SENSE_EVERY: Duration = Duration::from_millis(2000);
/// One full turn of an idle animation.
const LOOP_SECS: f64 = 4.0;

#[allow(clippy::too_many_arguments)]
fn run_face(
    expression: Option<&str>,
    phase: f64,
    animate: bool,
    sheet: bool,
    json: bool,
    catalog: bool,
    vignettes: bool,
    play: Option<&str>,
) -> i32 {
    if let Some(name) = play {
        return cue_beat(name);
    }

    if vignettes {
        // Compact: a thousand keyframes pretty-printed is megabytes of
        // whitespace. `jq .` reads it.
        println!(
            "{}",
            serde_json::to_string(&face::catalog::vignettes()).unwrap()
        );
        return 0;
    }

    if catalog {
        println!(
            "{}",
            serde_json::to_string_pretty(&face::catalog::json()).unwrap()
        );
        return 0;
    }

    if sheet {
        return if json { sheet_json() } else { sheet_text() };
    }

    if animate {
        animate_face();
        return 0;
    }

    let expr = match expression {
        Some(name) => match Expression::parse(name) {
            Some(e) => e,
            None => {
                eprintln!("no expression '{name}'");
                return 1;
            }
        },
        None => current_expression(),
    };
    for line in face::paint::ansi(
        &face::draw(expr.pose(), expr.motion(), phase, phase * LOOP_SECS),
        &Palette::from_env(),
    ) {
        println!("{line}");
    }
    0
}

/// `parker face --log`: the diary as a timeline. Text for reading, one
/// line per face; `--json` hands back the raw entries for anything that
/// wants to draw its own.
fn show_log(since: Option<&str>, last: Option<usize>, json: bool) -> i32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let cutoff = match since.map(parse_ago) {
        Some(Some(secs)) => Some(now - secs),
        Some(None) => {
            eprintln!("--since wants a span like 90s, 30m, 1h or 2d");
            return 1;
        }
        None => None,
    };
    let limit = last.unwrap_or(if cutoff.is_some() { usize::MAX } else { 20 });
    let entries = face::log::entries();

    if json {
        let mut keep: Vec<_> = entries
            .into_iter()
            .filter(|e| cutoff.is_none_or(|c| e["t"].as_f64().unwrap_or(0.0) >= c))
            .collect();
        keep.drain(..keep.len().saturating_sub(limit));
        println!("{}", serde_json::Value::from(keep));
        return 0;
    }

    let mut spans: Vec<_> = face::log::spans(&entries, now)
        .into_iter()
        .filter(|s| cutoff.is_none_or(|c| s.start >= c))
        .collect();
    spans.drain(..spans.len().saturating_sub(limit));
    if spans.is_empty() {
        println!(
            "no faces logged{}",
            since
                .map(|s| format!(" in the last {s}"))
                .unwrap_or_default()
        );
        return 0;
    }
    for s in spans {
        println!(
            "{}  {:<14} {:>7}  {}",
            clock(s.start),
            s.face,
            span_len(s.secs),
            s.end
        );
    }
    0
}

/// `90s`, `30m`, `1h`, `2d` → seconds. A bare number is seconds.
fn parse_ago(s: &str) -> Option<f64> {
    let s = s.trim();
    let (num, unit) = s.split_at(s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len()));
    let n: f64 = num.parse().ok().filter(|n: &f64| *n >= 0.0)?;
    let mult = match unit {
        "" | "s" => 1.0,
        "m" => 60.0,
        "h" => 3600.0,
        "d" => 86400.0,
        _ => return None,
    };
    Some(n * mult)
}

fn span_len(secs: f64) -> String {
    match secs {
        s if s < 60.0 => format!("{s:.1}s"),
        s if s < 3600.0 => format!("{}m{:02}s", (s / 60.0) as u64, (s % 60.0) as u64),
        s => format!(
            "{}h{:02}m",
            (s / 3600.0) as u64,
            ((s % 3600.0) / 60.0) as u64
        ),
    }
}

/// Local wall-clock time for a unix timestamp. The zone offset comes from
/// one `date +%z`, asked once — cheaper than a time crate for a diary
/// readout, and cheaper than a fork per line. A diary spanning a DST change
/// is off by an hour on one side of it; accepted. No `date` means UTC.
fn clock(t: f64) -> String {
    static OFFSET: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
    let offset = *OFFSET.get_or_init(|| {
        let out = std::process::Command::new("date").arg("+%z").output().ok();
        let z = out
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        let z = z.trim();
        let sign = if z.starts_with('-') { -1 } else { 1 };
        let digits = z.trim_start_matches(['+', '-']);
        match (
            digits.get(..2).and_then(|h| h.parse::<i64>().ok()),
            digits.get(2..4).and_then(|m| m.parse::<i64>().ok()),
        ) {
            (Some(h), Some(m)) => sign * (h * 3600 + m * 60),
            _ => 0,
        }
    });
    let local = t as i64 + offset;
    let days = local.div_euclid(86400);
    let secs = local.rem_euclid(86400);
    // 1970-01-01 was a Thursday.
    let day = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][days.rem_euclid(7) as usize];
    format!(
        "{day} {:02}:{:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// Write the beat slot, after checking the name is real — a typo'd cue that
/// silently does nothing on a face six feet away is indistinguishable from
/// the plumbing being broken.
fn cue_beat(name: &str) -> i32 {
    let known = |n: &str| face::idle::catalog().iter().any(|e| e.beat.name == n);
    if !known(name) {
        let names: Vec<&str> = face::idle::catalog().iter().map(|e| e.beat.name).collect();
        if name != "?" {
            eprintln!("no beat '{name}'");
        }
        eprintln!("beats: {}", names.join(", "));
        return if name == "?" { 0 } else { 1 };
    }
    match face::state::beat::set(name) {
        Ok(()) => {
            println!("cued {name} on every live face");
            0
        }
        Err(e) => {
            eprintln!("could not cue: {e}");
            1
        }
    }
}

/// What parker should be showing: whatever he chose to wear, else nothing in
/// particular. There is no second track anymore — the machine used to supply
/// `Trouble`/`Away` here when nothing was worn, and that is exactly the
/// automatic switching that got removed (DECISIONS.md). `Idle` is not a
/// verdict about parker, it is the absence of one.
fn current_expression() -> Expression {
    face::state::get().unwrap_or(Expression::Idle)
}

/// Terminal size in (cols, rows), via `stty`. Shelled out rather than pulling
/// in a crate for one ioctl; it is read once a second, not per frame.
/// Falls back to the face's own size, which centres it at the origin — right
/// when the output is being piped or captured rather than displayed.
fn term_size() -> (i32, i32) {
    let out = std::process::Command::new("stty")
        .arg("size")
        .stdin(std::process::Stdio::inherit())
        .output();
    let parsed = out.ok().and_then(|o| {
        let s = String::from_utf8_lossy(&o.stdout);
        let mut it = s.split_whitespace();
        let rows: i32 = it.next()?.parse().ok()?;
        let cols: i32 = it.next()?.parse().ok()?;
        Some((cols, rows))
    });
    parsed.unwrap_or((face::COLS, face::ROWS))
}

/// Centre the face in the terminal. A 48x10 face pinned to the top-left of a
/// fullscreen window reads as a stray process; centred, it reads as a face in
/// a space.
fn centred(lines: &[String], cols: i32, rows: i32) -> String {
    let pad_x = ((cols - face::COLS) / 2).max(0) as usize;
    let pad_y = ((rows - face::ROWS) / 2).max(0) as usize;
    let indent = " ".repeat(pad_x);
    let mut out = "\n".repeat(pad_y);
    out.push_str(
        &lines
            .iter()
            .map(|l| format!("{indent}{l}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    out
}

/// How often to re-read the dance slot. Cheap (one small file), but there is
/// no reason to stat it twenty-five times a second.
const DANCE_EVERY: Duration = Duration::from_millis(200);

/// Whether this pose has anything a blink could close. A waveform and a check
/// do not, and squashing them looks like a glitch rather than an eyelid.
fn blinkable(p: &Pose) -> bool {
    p.pupil > 0.3 || (p.gap > 0.6 && p.h > 2.0)
}

/// Only decorrelates two faces running side by side; nothing depends on it
/// being unpredictable.
fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0)
}

/// The live face.
///
/// Transitions interpolate poses rather than cross-fading frames, so an eye
/// genuinely squashes and an iris genuinely slides. Chosen expressions arrive
/// with a spring; derived ones ease, because a state change should not
/// announce itself as hard as a reaction does.
///
/// Three layers stack here, and the order is the whole of it:
///
///   1. the transition — where the face is between two expressions, or the
///      entrance it lands with (face/entrance.rs) while that plays;
///   2. the tier droop — how long parker has been left alone (idle::rest);
///   3. one beat, or the dance — a deviation with its own timeline.
///
/// Then a blink on top, but only when nothing else owns the eye. Dance
/// replaces 2 and 3 outright rather than layering over them: choreography
/// that a droop was quietly scaling would not land on the beat.
fn animate_face() {
    let palette = Palette::from_env();
    let started = Instant::now();
    let mut sensed = sense::gather(None);
    let mut sensed_at = Instant::now();
    let (mut cols, mut rows) = term_size();

    let mut showing = current_expression();
    let mut from = showing.pose();
    let mut began = Instant::now();
    let mut span = 0.0_f64;
    let mut springy = false;
    // The entrance the face being put on lands with, if it has one. It
    // replaces the spring: it departs from `from` and ends on the pose.
    let mut entering: Option<&'static face::beat::Beat> = None;

    let mut idle = face::idle::Idle::new(seed());
    // When anything last happened to parker. Not the user's idle time —
    // `sense` cannot see that and does not need to (face/idle.rs).
    let mut quiet_since = 0.0_f64;
    let mut dancer: Option<face::dance::Dance> = None;
    let mut dance_checked = Instant::now() - DANCE_EVERY;
    // The last beat cue seen, keyed on the raw line — the deadline differs
    // per write, so the same beat cued twice running still plays twice.
    let mut beat_seen: Option<String> = None;

    print!("\x1b[2J\x1b[?25l");
    loop {
        let elapsed = started.elapsed().as_secs_f64();

        if sensed_at.elapsed() >= SENSE_EVERY {
            sensed = sense::gather(None);
            // Re-read on the same slow clock so a resize is picked up without
            // forking `stty` twenty-five times a second.
            (cols, rows) = term_size();
            sensed_at = Instant::now();
        }

        if dance_checked.elapsed() >= DANCE_EVERY {
            dance_checked = Instant::now();
            match (face::state::dance::get(), dancer.is_some()) {
                (Some(want), false) => {
                    let routine = if want == "auto" {
                        // Seeded by the track, so the same song dances the
                        // same way twice.
                        let track = sensed.media.as_ref().and_then(|m| m.title.as_deref());
                        face::dance::Dance::routine_for(track, seed())
                    } else {
                        face::dance::ROUTINES
                            .iter()
                            .find(|r| r.name == want)
                            .copied()
                            .unwrap_or(face::dance::ROUTINES[0])
                    };
                    idle.interrupt(elapsed);
                    dancer = Some(face::dance::Dance::new(routine, 120.0, elapsed, seed()));
                }
                (None, true) => {
                    dancer = None;
                    // Do not walk out of a dance straight into a nap.
                    quiet_since = elapsed;
                }
                _ => {}
            }

            // A cued beat (face/state.rs beat), handed to the idle clock to
            // play at the next rest (Idle::cue). The dance owns the face while
            // it is on, so a cue during one is dropped rather than queued.
            let cue = face::state::beat::get_raw();
            if cue != beat_seen {
                if let (Some(raw), None) = (&cue, &dancer) {
                    if let Some(name) = raw.split_whitespace().next() {
                        idle.cue(name, elapsed);
                    }
                }
                beat_seen = cue;
            }
        }

        let want = current_expression();
        if want != showing {
            // Depart from wherever the face actually is right now, not from
            // the pose it was nominally holding — otherwise interrupting a
            // transition snaps. A beat being drawn is part of where it is:
            // read it before the interrupt below drops it, or a pyramid
            // vanishes a frame before the next face starts. Not the dance,
            // which keeps playing over the next face.
            from = pose_now(showing, from, began, span, entering);
            if showing == Expression::Idle && dancer.is_none() {
                from = idle.pose(from, elapsed);
            }
            springy = Expression::VOLUNTARY.contains(&want);
            span = if springy { 0.22 } else { 0.36 };
            entering = want.entrance();
            began = Instant::now();
            showing = want;
            quiet_since = elapsed;
            idle.interrupt(elapsed);
        }

        let phase = (elapsed / LOOP_SECS) % 1.0;
        let into = began.elapsed().as_secs_f64();
        let landing = entering.filter(|b| into < b.secs());
        let mut p = if let Some(b) = landing {
            b.departing(from, showing.pose(), into)
        } else if span > 0.0 {
            let t = (began.elapsed().as_secs_f64() / span).min(1.0);
            let eased = if springy {
                ease::back_out(t)
            } else {
                ease::in_out(t)
            };
            Pose::lerp(from, showing.pose(), eased)
        } else {
            showing.pose()
        };
        let mut motion = landing.map_or(showing.motion(), |b| b.motion_at(showing.motion(), into));

        if let Some(d) = &dancer {
            p = d.pose(p, elapsed);
            motion = d.motion();
        } else {
            // Flair only over the resting face. Parker glancing about while
            // he is thinking, or while he is waiting on an answer from you,
            // would make the involuntary track a liar.
            if showing == Expression::Idle {
                let quiet = elapsed - quiet_since;
                idle.tick(elapsed, quiet);
                p = idle.pose(p, elapsed);
                motion = idle.motion(motion, elapsed);
            }
            // Not mid-entrance (it steers the eye), and not on a face that
            // must not blink (Expression::blinks).
            if landing.is_none() && showing.blinks() && idle.blinkable(elapsed) && blinkable(&p) {
                p.h *= face::pose::blink(elapsed);
            }
        }

        let drawn = face::draw(p, motion, phase, elapsed);
        print!(
            "\x1b[H{}",
            centred(&face::paint::ansi(&drawn, &palette), cols, rows)
        );
        let _ = std::io::stdout().flush();
        std::thread::sleep(TICK);
    }
}

/// Where the face is mid-transition, so an interruption can depart from it —
/// mid-entrance included, or a face cut off while it lands would snap.
fn pose_now(
    expr: Expression,
    from: Pose,
    began: Instant,
    span: f64,
    entering: Option<&face::beat::Beat>,
) -> Pose {
    let into = began.elapsed().as_secs_f64();
    if let Some(b) = entering.filter(|b| into < b.secs()) {
        return b.departing(from, expr.pose(), into);
    }
    if span <= 0.0 {
        return expr.pose();
    }
    let t = (into / span).min(1.0);
    Pose::lerp(from, expr.pose(), ease::in_out(t))
}

fn sheet_text() -> i32 {
    for e in Expression::ALL {
        println!("\n── {} ──", e.name());
        for line in face::paint::ansi(
            &face::draw(e.pose(), e.motion(), 0.5, 0.5 * LOOP_SECS),
            &Palette::from_env(),
        ) {
            println!("{line}");
        }
    }
    0
}

fn sheet_json() -> i32 {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    // Dots and inks, both base64, so a surface that draws its own dots can
    // also colour them the way the terminal does.
    let shot = |p: Pose, m: Motion, phase: f64| -> (String, String) {
        let f = face::draw(p, m, phase, phase * LOOP_SECS);
        let masks: Vec<u8> = f
            .rows
            .iter()
            .flat_map(|l| l.chars().map(|c| (c as u32 - 0x2800) as u8))
            .collect();
        (STANDARD.encode(masks), STANDARD.encode(&f.inks))
    };

    let expressions: serde_json::Map<String, serde_json::Value> = Expression::ALL
        .iter()
        .map(|e| {
            let shots: Vec<(String, String)> = (0..SHEET_FRAMES)
                .map(|i| shot(e.pose(), e.motion(), i as f64 / SHEET_FRAMES as f64))
                .collect();
            (
                e.name().to_string(),
                serde_json::json!({
                    "frames": shots.iter().map(|(m, _)| m).collect::<Vec<_>>(),
                    "inks": shots.iter().map(|(_, k)| k).collect::<Vec<_>>(),
                    "voluntary": Expression::VOLUNTARY.contains(e),
                    "wearable": Expression::WEARABLE.contains(e),
                    "hold": e.default_hold(),
                }),
            )
        })
        .collect();

    // Every transition, sampled — this is what makes the morphs inspectable
    // without a terminal.
    let morphs: Vec<serde_json::Value> = [
        // Chosen to show the range: a face into a glyph, a face into a voice,
        // and the two extremes of size.
        (Expression::Idle, Expression::Yes),
        (Expression::Blocked, Expression::No),
        (Expression::Idle, Expression::Speaking),
        (Expression::Surprised, Expression::Trouble),
    ]
    .iter()
    .map(|(a, b)| {
        let steps: Vec<(String, String)> = (0..SHEET_FRAMES)
            .map(|i| {
                let t = ease::in_out(i as f64 / (SHEET_FRAMES - 1) as f64);
                shot(Pose::lerp(a.pose(), b.pose(), t), Motion::None, 0.5)
            })
            .collect();
        serde_json::json!({
            "from": a.name(), "to": b.name(),
            "steps": steps.iter().map(|(m, _)| m).collect::<Vec<_>>(),
            "inks": steps.iter().map(|(_, k)| k).collect::<Vec<_>>(),
        })
    })
    .collect();

    println!(
        "{}",
        serde_json::json!({
            "cols": face::COLS, "rows": face::ROWS, "frames": SHEET_FRAMES,
            "expressions": expressions, "morphs": morphs,
        })
    );
    0
}

// ── the rest ────────────────────────────────────────────────────────────────

/// A screenshot on a terminal is unreadable, so `look` writes PNG bytes to
/// stdout when it is piped and saves a file when it is not.
fn run_look(target: &str) -> i32 {
    let shot = match reach::look::look(reach::look::Target::parse(target)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    if std::io::stdout().is_terminal() {
        let path = std::env::temp_dir().join(format!("parker-look-{}.png", std::process::id()));
        match std::fs::write(&path, &shot.png) {
            Ok(()) => {
                println!("{} → {}", shot.what, path.display());
                0
            }
            Err(e) => {
                eprintln!("could not write {}: {e}", path.display());
                1
            }
        }
    } else {
        let mut out = std::io::stdout();
        match out.write_all(&shot.png).and_then(|()| out.flush()) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("could not write image: {e}");
                1
            }
        }
    }
}

fn report(told: reach::Done) -> i32 {
    match told {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn render_one(s: &sense::Sense, json: bool) -> String {
    if json {
        serde_json::to_string_pretty(s).expect("Sense is always serializable")
    } else {
        render::text(s)
    }
}

/// Watch mode. Also the cheapest preview of why the daemon has to exist: it
/// tracks how long the current focus has been held, which is the one thing a
/// single reading structurally cannot know.
fn watch_loop(secs: u64, json: bool) {
    let mut held: Option<(String, Instant)> = None;

    loop {
        let key = sense::gather(None)
            .focus
            .as_ref()
            .map(sense::Focus::identity);

        held = match (held, key) {
            (Some((prev, since)), Some(now)) if prev == now => Some((prev, since)),
            (_, Some(now)) => Some((now, Instant::now())),
            (_, None) => None,
        };

        let focus_secs = held.as_ref().map(|(_, since)| since.elapsed().as_secs());
        let s = sense::gather(focus_secs);

        print!("\x1b[2J\x1b[H{}", render_one(&s, json));
        let _ = std::io::stdout().flush();

        std::thread::sleep(Duration::from_secs(secs));
    }
}

#[cfg(test)]
mod tests {
    use super::wants_brain;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn parker_code_gets_the_brain_unless_told_otherwise() {
        assert!(wants_brain(&args(&[])));
        assert!(wants_brain(&args(&["--continue"])));
        assert!(
            wants_brain(&args(&["/tmp"])),
            "a project directory is not a subcommand"
        );

        assert!(!wants_brain(&args(&["-m", "anthropic/claude-haiku-5-5"])));
        assert!(!wants_brain(&args(&["--model=anthropic/claude-haiku-5-5"])));
        assert!(!wants_brain(&args(&["auth", "login"])));
        assert!(!wants_brain(&args(&["run", "hello"])));
    }
}
