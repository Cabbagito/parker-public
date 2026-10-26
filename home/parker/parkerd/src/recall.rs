// `parker recall` — the conversations parker has had, read back.
//
// Three verbs that read, cheapest first, and one that writes:
//
//   list    every thread: when, what it was called, how long it ran, the
//           ^K card's one line if visage has written one, and where it
//           stands — left open with its next steps, or settled; `--open`
//           keeps only what is still waiting on someone
//   search  plain text over what was *said* — exact, no model in the loop
//   ask     one question about one thread, answered by the Flatline: the
//           engine's `recall` agent (engine.nix), a construct that reads the
//           whole thread so parker's own context does not have to
//   mark    someone's word on whether a thread is done: Peter's (this CLI,
//           and visage's ^X through it) or parker's (the MCP verb). The one
//           writer of marks.
//
// Where a thread stands has two sources. The card says what the `digest`
// agent read in it (`state`, `next`); a mark says what Peter or parker
// decided, and beats the card for as long as it holds — until the thread
// moves, which is when the digest reads it again, told about the mark.
// Peter's mark beats parker's: parker cannot change one Peter made.
//
// Everything goes through the engine's HTTP API, never opencode's sqlite
// file: the schema is opencode's private business and moves with updates,
// and the API is the seam door already speaks (visage/door/src/lib.rs —
// the route traps recorded there apply here too; the bare `/session` routes
// are the real ones). Shelled out to curl in house style (sense/web.rs).
//
// What is read is what was said, plus one line per act — never tool output
// and never reasoning, the line visage's ledger already draws.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

const BASE: &str = "http://127.0.0.1:4096";

/// Throwaway sessions the helpers make and delete; one that survived a
/// crash is noise, not a conversation.
const HELPERS: [&str; 3] = ["ledger", "digest", "recall"];

/// A first line meant for parker and not part of what was said (visage
/// state.svelte.js `FOR_PARKER`): the old wake marker `⟦parker:woken⟧`,
/// and `⟦parker:controls⟧ …` on what Peter asked from ^, — where he was
/// and on which row.
const FOR_PARKER: &str = "⟦parker:";

/// How much of a thread the Flatline reads. ~100k tokens: past this the
/// middle goes, head and tail kept, the shape visage's digest uses.
const ASK_TOTAL: usize = 400_000;
const ASK_HEAD: usize = 120_000;

// ---------------------------------------------------------------- engine

fn state_dir() -> String {
    std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.local/state")
    })
}

fn password() -> Result<String, String> {
    let path = format!("{}/parker/engine.env", state_dir());
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {path}: {e} — is parker-engine set up?"))?;
    text.lines()
        .find_map(|l| l.strip_prefix("OPENCODE_SERVER_PASSWORD="))
        .map(|p| p.trim().to_string())
        .ok_or_else(|| format!("no OPENCODE_SERVER_PASSWORD in {path}"))
}

/// One engine call. The password goes to curl on stdin as config, never on
/// the command line where `ps` would show it; a body goes through a file
/// in the runtime dir for the same reason stdin is taken.
fn engine(method: &str, path: &str, body: Option<&Value>, max_secs: u32) -> Result<Value, String> {
    let pw = password()?;
    let body_file = match body {
        Some(b) => {
            let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
            let f = format!("{dir}/parker-recall-{}.json", std::process::id());
            std::fs::write(&f, b.to_string())
                .map_err(|e| format!("cannot stage a request: {e}"))?;
            Some(f)
        }
        None => None,
    };
    let mut cmd = Command::new("curl");
    cmd.args([
        "-sS",
        "-K",
        "-",
        "--max-time",
        &max_secs.to_string(),
        "-X",
        method,
    ])
    .args(["-w", "\n%{http_code}"]);
    if let Some(f) = &body_file {
        cmd.args(["-H", "Content-Type: application/json", "--data-binary"])
            .arg(format!("@{f}"));
    }
    cmd.arg(format!("{BASE}{path}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let run = (|| {
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("could not run curl: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = writeln!(stdin, "user = \"opencode:{pw}\"");
        }
        child.wait_with_output().map_err(|e| e.to_string())
    })();
    if let Some(f) = &body_file {
        let _ = std::fs::remove_file(f);
    }
    let out = run?;
    if !out.status.success() {
        return Err("the engine is unreachable — is parker-engine running?".into());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", &text));
    if !code.trim().starts_with('2') {
        return Err(format!(
            "engine said {}: {}",
            code.trim(),
            first_line(body, 200)
        ));
    }
    if body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(body).map_err(|e| format!("engine answered something unreadable: {e}"))
}

// --------------------------------------------------------------- threads

struct Thread {
    id: String,
    title: String,
    created: f64,
    updated: f64,
    messages: Vec<Value>,
}

/// Every conversation, newest first, helper leftovers dropped, read whole.
/// One local request per thread — all fifty-odd take about half a second
/// (measured Oct 2026), so nothing is cached.
fn threads(since: Option<f64>) -> Result<Vec<Thread>, String> {
    let list = engine("GET", "/session", None, 10)?;
    let cutoff = since.map(|s| now_ms() - s * 1000.0);
    let mut out = Vec::new();
    for s in list.as_array().into_iter().flatten() {
        let title = s["title"].as_str().unwrap_or("").to_string();
        if HELPERS.contains(&title.as_str()) || !s["parentID"].is_null() {
            continue;
        }
        let updated = s["time"]["updated"].as_f64().unwrap_or(0.0);
        if cutoff.is_some_and(|c| updated < c) {
            continue;
        }
        let id = s["id"].as_str().unwrap_or("").to_string();
        let messages = engine("GET", &format!("/session/{id}/message"), None, 10)?
            .as_array()
            .cloned()
            .unwrap_or_default();
        out.push(Thread {
            id,
            title,
            created: s["time"]["created"].as_f64().unwrap_or(0.0),
            updated,
            messages,
        });
    }
    out.sort_by(|a, b| b.updated.total_cmp(&a.updated));
    Ok(out)
}

fn role(m: &Value) -> &str {
    m["info"]["role"].as_str().unwrap_or("assistant")
}

/// What one message said: its text parts, not attachments, not the wake
/// marker.
fn prose(m: &Value) -> String {
    let text = m["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["type"] == "text" && p["synthetic"] != true)
        .filter_map(|p| p["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let text = match text.starts_with(FOR_PARKER) {
        true => text.split_once('\n').map(|(_, r)| r).unwrap_or(""),
        false => &text,
    };
    text.trim().to_string()
}

/// One line per thing parker did — verb and target, never output. Faces
/// are left out: WEARS names them, and a thread of forty `face` lines
/// says nothing.
fn acts(m: &Value) -> Vec<String> {
    m["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["type"] == "tool")
        .filter_map(|p| {
            let name = p["tool"]
                .as_str()
                .unwrap_or("tool")
                .trim_start_matches("parker_");
            if name == "face" {
                return None;
            }
            let i = &p["state"]["input"];
            let target = [
                "command",
                "filePath",
                "query",
                "pattern",
                "url",
                "what",
                "text",
                "description",
            ]
            .iter()
            .find_map(|k| i[*k].as_str())
            .unwrap_or("");
            let failed = if p["state"]["status"] == "error" {
                " (failed)"
            } else {
                ""
            };
            Some(
                format!("{name} {}{failed}", first_line(target, 120))
                    .trim()
                    .to_string(),
            )
        })
        .collect()
}

/// The thread as the Flatline reads it.
fn transcript(t: &Thread) -> String {
    let mut out = Vec::new();
    for m in &t.messages {
        let said = prose(m);
        if role(m) == "user" {
            if !said.is_empty() {
                out.push(format!("Peter: {said}"));
            }
            continue;
        }
        if !said.is_empty() {
            out.push(format!("parker: {said}"));
        }
        out.extend(acts(m).into_iter().map(|a| format!("  · {a}")));
    }
    let all = out.join("\n");
    if all.len() <= ASK_TOTAL {
        return all;
    }
    let head = cut(&all, ASK_HEAD);
    let tail = tail(&all, ASK_TOTAL - ASK_HEAD);
    format!("{head}\n\n[… the middle of the thread is left out …]\n\n{tail}")
}

/// Turns are Peter's messages; size is what was said, in characters.
fn size(t: &Thread) -> (usize, usize) {
    let turns = t.messages.iter().filter(|m| role(m) == "user").count();
    let chars = t.messages.iter().map(|m| prose(m).chars().count()).sum();
    (turns, chars)
}

/// Exactly the shape opencode mints — `ses_` and letters and digits — and
/// so a file name and never a path (door's `card_path` holds the same line).
fn is_thread_id(id: &str) -> bool {
    id.strip_prefix("ses_").is_some_and(|rest| {
        !rest.is_empty() && rest.len() <= 64 && rest.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

/// visage's thread card (^K) for this thread, if one was written (door's
/// `digest`). Read, never written: cards are door's to keep.
fn card(id: &str) -> Option<Value> {
    if !is_thread_id(id) {
        return None;
    }
    let path = format!("{}/parker/digests/{id}.json", state_dir());
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Where a thread's mark lives, beside the cards and apart from them: a
/// card is door's to write, a mark is this file's.
fn mark_path(id: &str) -> Option<String> {
    is_thread_id(id).then(|| format!("{}/parker/marks/{id}.json", state_dir()))
}

/// The thread's mark as it was written — holding or not; `holds` decides.
fn mark_of(id: &str) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(mark_path(id)?).ok()?).ok()
}

/// A mark holds while the thread has not moved since it was made. A new
/// message is the thread saying more, and the digest gets the next word.
fn holds(updated: f64, mark: &Value) -> bool {
    mark["updated"].as_f64() == Some(updated)
}

/// Where a thread stands: a holding mark's word, else the card's verdict
/// (`open`, `done` or `chat`), else nothing known.
fn standing(updated: f64, card: Option<&Value>, mark: Option<&Value>) -> String {
    match mark.filter(|m| holds(updated, m)) {
        Some(m) => m["state"].as_str().unwrap_or("").to_string(),
        None => card
            .and_then(|c| c["state"].as_str())
            .unwrap_or("")
            .to_string(),
    }
}

/// The lines under a thread that say where it stands: its next steps while
/// it is open, and who settled it when a mark did — with the steps it set
/// aside, kept rather than struck, because nobody can tell from here
/// whether they were done or dropped.
fn standing_lines(updated: f64, card: Option<&Value>, mark: Option<&Value>) -> Vec<String> {
    let state = standing(updated, card, mark);
    let mark = mark.filter(|m| holds(updated, m));
    let steps = |label: &str| -> Vec<String> {
        card.and_then(|c| c["next"].as_array())
            .into_iter()
            .flatten()
            .map(|s| {
                format!(
                    "    {label} · {:<6}  {}",
                    s["who"].as_str().unwrap_or(""),
                    s["step"].as_str().unwrap_or("")
                )
            })
            .collect()
    };
    let mut out = Vec::new();
    if let Some(m) = mark {
        let who = if m["by"] == "parker" {
            "parker"
        } else {
            "Peter"
        };
        let when = date(m["at"].as_f64().unwrap_or(0.0));
        let did = if state == "open" {
            "kept open"
        } else {
            "closed"
        };
        let note = m["note"]
            .as_str()
            .map(|n| format!(": {n}"))
            .unwrap_or_default();
        out.push(format!("    {did} by {who}, {when}{note}"));
    }
    match state.as_str() {
        "open" => {
            let next = steps("next");
            if next.is_empty() && mark.is_none() {
                out.push("    left open".into());
            }
            out.extend(next);
        }
        _ if mark.is_some() => out.extend(steps("set aside")),
        _ => {}
    }
    out
}

// ----------------------------------------------------------------- verbs

pub struct Filter {
    pub since: Option<f64>,
    pub limit: usize,
}

/// `open` keeps only the threads still waiting on someone. A thread with no
/// card and no mark cannot be known to be open, so the count of those comes
/// along: an empty list with three unread threads is not "nothing pending".
pub fn list(f: &Filter, title: Option<&str>, open: bool, json: bool) -> Result<String, String> {
    let needle = title.map(str::to_lowercase);
    let mut unread = 0;
    let mut picked = Vec::new();
    for t in threads(f.since)? {
        if needle
            .as_ref()
            .is_some_and(|n| !t.title.to_lowercase().contains(n))
        {
            continue;
        }
        let (c, m) = (card(&t.id), mark_of(&t.id));
        let state = standing(t.updated, c.as_ref(), m.as_ref());
        if state.is_empty() {
            unread += 1;
        }
        if (!open || state == "open") && picked.len() < f.limit {
            picked.push((t, c, m, state));
        }
    }
    if json {
        let rows: Vec<Value> = picked
            .iter()
            .map(|(t, c, m, state)| {
                let (turns, chars) = size(t);
                json!({
                    "id": t.id, "title": t.title, "created": t.created, "updated": t.updated,
                    "turns": turns, "chars": chars, "card": c,
                    "state": if state.is_empty() { Value::Null } else { Value::from(state.as_str()) },
                    "mark": m.as_ref().filter(|m| holds(t.updated, m)),
                })
            })
            .collect();
        return Ok(Value::from(rows).to_string());
    }
    let unread_line = match unread {
        0 => String::new(),
        1 => " · 1 thread has no card yet".into(),
        n => format!(" · {n} threads have no card yet"),
    };
    if picked.is_empty() {
        return Ok(if open {
            format!("nothing left open{unread_line}")
        } else {
            "no threads match".into()
        });
    }
    let mut lines = Vec::new();
    for (t, c, m, _) in &picked {
        let (turns, chars) = size(t);
        lines.push(format!(
            "{}  {}  {}  · {turns} {} · {}",
            date(t.updated),
            t.id,
            t.title,
            if turns == 1 { "turn" } else { "turns" },
            human(chars)
        ));
        if let Some(c) = c {
            let about = c["about"].as_str().unwrap_or("");
            let landed = c["landed"].as_str().unwrap_or("");
            // A card written before the thread's last turn says so: its
            // steps may already be behind the thread.
            let earlier = if c["updated"].as_f64() == Some(t.updated) {
                ""
            } else {
                " (as of an earlier turn)"
            };
            if !about.is_empty() || !landed.is_empty() {
                lines.push(format!("    {about} → {landed}{earlier}"));
            }
        }
        lines.extend(standing_lines(t.updated, c.as_ref(), m.as_ref()));
    }
    if open {
        let n = picked.len();
        lines.push(format!(
            "{n} left open{}{unread_line}",
            if n == f.limit {
                " (the limit — there may be more)"
            } else {
                ""
            }
        ));
    }
    Ok(lines.join("\n"))
}

/// Who a mark is from. The CLI is Peter's hand (or visage's ^X, which runs
/// it); the MCP verb is parker's.
#[derive(Clone, Copy)]
pub enum By {
    Peter,
    Parker,
}

impl By {
    fn name(self) -> &'static str {
        match self {
            By::Peter => "peter",
            By::Parker => "parker",
        }
    }
}

/// One session from the engine by id, or by any piece of an id that names
/// only one — parker's threads only, never a helper's or a subagent's.
fn session(thread: &str) -> Result<Value, String> {
    let ours = |s: &Value| {
        !HELPERS.contains(&s["title"].as_str().unwrap_or("")) && s["parentID"].is_null()
    };
    if is_thread_id(thread) {
        if let Ok(s) = engine("GET", &format!("/session/{thread}"), None, 10) {
            if s["id"] == thread && ours(&s) {
                return Ok(s);
            }
        }
    }
    let list = engine("GET", "/session", None, 10)?;
    let matched: Vec<&Value> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| ours(s) && s["id"].as_str().is_some_and(|id| id.contains(thread)))
        .collect();
    match matched.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(format!("no thread matches {thread}")),
        many => Err(format!(
            "{thread} matches {} threads — give more of the id",
            many.len()
        )),
    }
}

/// Settle a thread (`done`), keep it waiting (`open`), or take a mark back
/// (`clear`, after which the card speaks for it again). The mark records
/// the thread's `updated` as the engine has it now — so it holds until the
/// next message — and how many turns it had, so the digest can tell what
/// was said after it.
///
/// parker cannot change a mark Peter made: Peter settled it, and parker
/// disagreeing is something to say, not to write.
pub fn mark(
    thread: &str,
    state: &str,
    note: Option<&str>,
    by: By,
    json: bool,
) -> Result<String, String> {
    let state = state.trim();
    if !matches!(state, "done" | "open" | "clear") {
        return Err(format!("a mark is done, open or clear — not {state:?}"));
    }
    let thread = thread.trim();
    if thread.is_empty() {
        return Err("recall mark needs a thread — an id from `parker recall list`".into());
    }
    let s = session(thread)?;
    let id = s["id"].as_str().unwrap_or("").to_string();
    let title = s["title"].as_str().unwrap_or("").to_string();
    let updated = s["time"]["updated"].clone();
    let path = mark_path(&id).ok_or_else(|| format!("not a thread id: {id}"))?;
    let held = mark_of(&id).filter(|m| holds(updated.as_f64().unwrap_or(0.0), m));
    if matches!(by, By::Parker) && held.as_ref().is_some_and(|m| m["by"] == "peter") {
        let word = if held.as_ref().is_some_and(|m| m["state"] == "open") {
            "kept it open"
        } else {
            "closed it"
        };
        return Err(format!(
            "Peter {word} himself (\"{title}\") — that one is his to change"
        ));
    }
    let answer = |mark: Value, said: String| {
        if json {
            json!({ "id": id, "updated": updated, "mark": mark }).to_string()
        } else {
            said
        }
    };
    if state == "clear" {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot take the mark back: {e}")),
        }
        return Ok(answer(
            Value::Null,
            format!("cleared  {title} — its card speaks for it again"),
        ));
    }
    let turns = engine("GET", &format!("/session/{id}/message"), None, 10)?
        .as_array()
        .map(|a| a.iter().filter(|m| role(m) == "user").count())
        .unwrap_or(0);
    let mut m = json!({
        "state": state,
        "by": by.name(),
        "at": now_ms() as u64,
        "updated": updated,
        "turns": turns,
    });
    let note = note.map(|n| cut(&n.split_whitespace().collect::<Vec<_>>().join(" "), 160));
    if let Some(n) = note.filter(|n| !n.is_empty()) {
        m["note"] = Value::from(n);
    }
    // Whole or not at all: visage on the tower and on the porthole, and
    // parker, may all mark the same thread.
    let dir = std::path::Path::new(&path)
        .parent()
        .ok_or("a mark path without a directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let tmp = format!("{path}.{}.tmp", std::process::id());
    std::fs::write(&tmp, m.to_string()).map_err(|e| format!("cannot write a mark: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("cannot keep a mark: {e}"))?;
    let did = if state == "open" {
        "kept open"
    } else {
        "closed"
    };
    Ok(answer(m, format!("{did}  {title}")))
}

/// A line matches when it holds every word of the query, in any order and
/// any case — or, with `exact`, the query as one phrase. A thread shows at
/// most three of its lines; `limit` counts threads.
pub fn search(query: &str, exact: bool, f: &Filter, json: bool) -> Result<String, String> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Err("recall search needs something to look for".into());
    }
    let words: Vec<&str> = if exact {
        vec![query.as_str()]
    } else {
        query.split_whitespace().collect()
    };
    let mut found = Vec::new();
    for t in threads(f.since)? {
        let mut hits = Vec::new();
        let mut total = 0;
        for m in &t.messages {
            let who = if role(m) == "user" { "Peter" } else { "parker" };
            for line in prose(m).lines() {
                let low = line.to_lowercase();
                if words.iter().all(|w| low.contains(w)) {
                    total += 1;
                    if hits.len() < 3 {
                        hits.push((who, snippet(line, &low, words[0])));
                    }
                }
            }
        }
        if total > 0 {
            found.push((t, hits, total));
        }
        if found.len() >= f.limit {
            break;
        }
    }
    if json {
        let rows: Vec<Value> = found
            .iter()
            .map(|(t, hits, total)| {
                json!({
                    "id": t.id, "title": t.title, "updated": t.updated, "matches": total,
                    "hits": hits.iter().map(|(w, s)| json!({ "who": w, "line": s })).collect::<Vec<_>>(),
                })
            })
            .collect();
        return Ok(Value::from(rows).to_string());
    }
    if found.is_empty() {
        return Ok(format!("nothing said matches \"{query}\""));
    }
    let mut lines = Vec::new();
    for (t, hits, total) in &found {
        let more = if *total > hits.len() {
            format!(" · {total} lines match")
        } else {
            String::new()
        };
        lines.push(format!("{}  {}  {}{more}", date(t.updated), t.id, t.title));
        for (who, s) in hits {
            lines.push(format!("    {who}: {s}"));
        }
    }
    Ok(lines.join("\n"))
}

/// One question about one thread, answered by the Flatline. The thread is
/// named by its id or any piece of one that names exactly one thread.
///
/// A throwaway session on the `recall` agent, deleted as soon as it
/// answers — door's `ask`, the same shape and the same guard: the agent is
/// checked against the engine's own list first, because an engine that
/// falls back to its default agent would hand the transcript to parker
/// with every tool.
pub fn ask(thread: &str, question: &str) -> Result<String, String> {
    let question = question.trim();
    if question.is_empty() {
        return Err("recall ask needs a question — \"summarise it\" is one".into());
    }
    let thread = thread.trim();
    if thread.is_empty() {
        return Err("recall ask needs a thread — an id from `parker recall list`".into());
    }
    let agents = engine("GET", "/agent", None, 10)?;
    if !agents
        .as_array()
        .into_iter()
        .flatten()
        .any(|a| a["name"] == "recall")
    {
        return Err("the engine has no `recall` agent yet — engine.nix is not switched in".into());
    }
    let all = threads(None)?;
    let matched: Vec<&Thread> = all.iter().filter(|t| t.id.contains(thread)).collect();
    let t = match matched.as_slice() {
        [one] => *one,
        [] => return Err(format!("no thread matches {thread}")),
        many => {
            return Err(format!(
                "{thread} matches {} threads — give more of the id",
                many.len()
            ))
        }
    };
    let text = format!(
        "Question: {question}\n\nThe conversation, \"{}\", {} to {}:\n\n{}",
        t.title,
        date(t.created),
        date(t.updated),
        transcript(t)
    );
    let session = engine("POST", "/session", Some(&json!({ "title": "recall" })), 10)?;
    let id = session["id"]
        .as_str()
        .ok_or("engine returned a session without an id")?
        .to_string();
    let reply = engine(
        "POST",
        &format!("/session/{id}/message"),
        Some(&json!({ "agent": "recall", "parts": [{ "type": "text", "text": text }] })),
        300,
    );
    // Best-effort: a stray helper session costs a row, and `threads` hides it.
    let _ = engine("DELETE", &format!("/session/{id}"), None, 10);
    let answer = reply?["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["type"] == "text")
        .filter_map(|p| p["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if answer.trim().is_empty() {
        return Err("the Flatline said nothing".into());
    }
    Ok(format!("{} — {}\n\n{}", t.id, t.title, answer.trim()))
}

// ------------------------------------------------------------- formatting

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// `2026-10-10 19:52`, local, from epoch milliseconds. The zone offset is
/// one `date +%z`, as in main.rs's `clock`.
fn date(ms: f64) -> String {
    static OFFSET: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
    let offset = *OFFSET.get_or_init(|| {
        let z = Command::new("date")
            .arg("+%z")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        let z = z.trim();
        let sign = if z.starts_with('-') { -1 } else { 1 };
        let d = z.trim_start_matches(['+', '-']);
        match (
            d.get(..2).and_then(|h| h.parse::<i64>().ok()),
            d.get(2..4).and_then(|m| m.parse::<i64>().ok()),
        ) {
            (Some(h), Some(m)) => sign * (h * 3600 + m * 60),
            _ => 0,
        }
    });
    civil((ms / 1000.0) as i64 + offset)
}

/// Seconds since the epoch → `YYYY-MM-DD HH:MM`. Howard Hinnant's
/// days-to-civil, because a time crate for one date format is surface.
fn civil(t: i64) -> String {
    let days = t.div_euclid(86400);
    let secs = t.rem_euclid(86400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60
    )
}

fn human(chars: usize) -> String {
    match chars {
        c if c < 1000 => format!("{c} chars"),
        c => format!("{}k chars", (c + 500) / 1000),
    }
}

fn first_line(s: &str, max: usize) -> String {
    cut(s.lines().next().unwrap_or(""), max)
}

/// At most `max` characters, on a char boundary, with an ellipsis if cut.
fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

fn tail(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let kept: String = s.chars().skip(n - max + 1).collect();
    format!("…{kept}")
}

/// ~160 characters of `line` around the first match of `word`.
fn snippet(line: &str, low: &str, word: &str) -> String {
    const SPAN: usize = 160;
    let line = line.trim();
    if line.chars().count() <= SPAN {
        return line.to_string();
    }
    // Byte offset in the lowercased line → char offset, which survives the
    // case change for everything but a handful of exotic letters.
    let at = low
        .find(word)
        .map(|b| low[..b].chars().count())
        .unwrap_or(0);
    let start = at.saturating_sub(SPAN / 3);
    let body: String = line.chars().skip(start).take(SPAN).collect();
    let lead = if start > 0 { "…" } else { "" };
    let trail = if start + SPAN < line.chars().count() {
        "…"
    } else {
        ""
    };
    format!("{lead}{}{trail}", body.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_are_right() {
        assert_eq!(civil(0), "1970-01-01 00:00");
        assert_eq!(civil(951_782_400), "2000-02-29 00:00");
        assert_eq!(civil(1_791_655_380), "2026-10-10 18:03");
    }

    #[test]
    fn prose_drops_the_wake_marker_and_synthetic_parts() {
        let m = json!({ "info": { "role": "user" }, "parts": [
            { "type": "text", "text": "⟦parker:woken⟧ wake up\nhello there" },
            { "type": "text", "text": "attached file", "synthetic": true },
            { "type": "tool", "tool": "bash" },
        ]});
        assert_eq!(prose(&m), "hello there");
        let asked = json!({ "info": { "role": "user" }, "parts": [
            { "type": "text", "text": "⟦parker:controls⟧ Peter is asking from ^, on the row \"spotify\".\nquieter" },
        ]});
        assert_eq!(prose(&asked), "quieter");
    }

    #[test]
    fn acts_name_the_target_and_skip_faces() {
        let m = json!({ "parts": [
            { "type": "tool", "tool": "parker_face", "state": { "input": { "expression": "yes" } } },
            { "type": "tool", "tool": "bash", "state": { "status": "completed", "input": { "command": "git status\nmore" } } },
            { "type": "tool", "tool": "read", "state": { "status": "error", "input": { "filePath": "/x" } } },
        ]});
        assert_eq!(acts(&m), vec!["bash git status", "read /x (failed)"]);
    }

    #[test]
    fn a_long_line_is_snipped_around_the_match() {
        let line = format!("{} needle {}", "a".repeat(300), "b".repeat(300));
        let low = line.to_lowercase();
        let s = snippet(&line, &low, "needle");
        assert!(s.contains("needle"));
        assert!(s.starts_with('…') && s.ends_with('…'));
        assert!(s.chars().count() <= 162);
    }

    #[test]
    fn a_card_is_only_read_for_a_thread_id() {
        for bad in ["../engine.env", "ses_../x", "ses_a/b", "msg_abc"] {
            assert!(card(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn a_mark_is_only_a_file_for_a_thread_id() {
        assert!(mark_path("ses_ed9d3b3d4ffe1aDeWWLYKoJm2n").is_some());
        for bad in [
            "",
            "ses_",
            "../engine.env",
            "ses_../x",
            "ses_a/b",
            "ses_a.json",
            "msg_abc",
        ] {
            assert!(mark_path(bad).is_none(), "{bad:?}");
        }
        assert!(mark_path(&format!("ses_{}", "a".repeat(65))).is_none());
    }

    fn open_card() -> Value {
        json!({
            "state": "open", "updated": 100.0,
            "next": [
                { "who": "you", "step": "reboot into 6.18" },
                { "who": "parker", "step": "check nvidia-smi after" },
            ],
        })
    }

    #[test]
    fn a_mark_holds_until_the_thread_moves() {
        let card = open_card();
        let done = json!({ "state": "done", "by": "peter", "at": 0, "updated": 100 });
        assert_eq!(standing(100.0, Some(&card), Some(&done)), "done");
        // A new message: the card's word again, until the digest rereads.
        assert_eq!(standing(200.0, Some(&card), Some(&done)), "open");
        assert_eq!(standing(100.0, None, None), "");
        let kept = json!({ "state": "open", "by": "peter", "at": 0, "updated": 100 });
        assert_eq!(standing(100.0, None, Some(&kept)), "open");
    }

    #[test]
    fn an_open_thread_lists_its_steps_and_a_closed_one_sets_them_aside() {
        let card = open_card();
        let open = standing_lines(100.0, Some(&card), None);
        assert_eq!(
            open,
            vec![
                "    next · you     reboot into 6.18",
                "    next · parker  check nvidia-smi after",
            ]
        );
        let done = json!({ "state": "done", "by": "parker", "at": 0, "updated": 100, "note": "saw 6.18 running" });
        let closed = standing_lines(100.0, Some(&card), Some(&done));
        assert!(
            closed[0].starts_with("    closed by parker, "),
            "{closed:?}"
        );
        assert!(closed[0].ends_with(": saw 6.18 running"), "{closed:?}");
        assert_eq!(closed[1], "    set aside · you     reboot into 6.18");
        // Settled by the digest itself: nothing to say beyond the card line.
        let landed = json!({ "state": "done", "updated": 100.0, "next": [] });
        assert!(standing_lines(100.0, Some(&landed), None).is_empty());
        // Open with no steps written, and nobody's mark: it still says so.
        let bare = json!({ "state": "open", "updated": 100.0 });
        assert_eq!(
            standing_lines(100.0, Some(&bare), None),
            vec!["    left open"]
        );
    }
}
