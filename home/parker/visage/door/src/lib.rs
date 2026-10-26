// visage-door — everything a face can ask the machine for, and nothing else.
//
// The webview's only door is src/lib/engine.js; this is the far side of it.
// It owns every conversation with the engine (opencode serve on
// 127.0.0.1:4096, password-gated) and the three runtime reads and verbs the
// face needs from parkerd. Two frames hang it and neither has opinions of its
// own: the Tauri window on parker's screen (../src-tauri) turns these into
// tauri commands, and the web server (../porthole) turns them into HTTP for the
// tailnet. A fix made here is made for both, which is the whole reason it is
// a crate (docs/visage/porthole/README.md).

use futures_util::StreamExt;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

const BASE: &str = "http://127.0.0.1:4096";

/// `$XDG_STATE_HOME`, or where it defaults to. The engine's password lives
/// under it, and so do the thread cards.
fn state_dir() -> String {
    std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.local/state")
    })
}

/// The engine, reachable. Cheap to clone — the client is shared inside, and
/// the web server hands a copy to every detached call.
#[derive(Clone)]
pub struct Engine {
    http: reqwest::Client,
    password: String,
}

/// One thing the engine's event bus said, as a face hears it.
pub enum Event {
    /// `connected` or `disconnected`. Re-sent on every reconnect attempt,
    /// not just on the transition — a frame that cares should dedupe.
    Status(&'static str),
    /// One event off the engine's SSE bus, passed through unparsed.
    Engine(Value),
}

impl Engine {
    /// The password is minted by home-manager activation into the engine's
    /// state dir (engine.nix); reading it is how a face proves it is local.
    pub fn from_env() -> Result<Self, String> {
        let path = format!("{}/parker/engine.env", state_dir());
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {path}: {e} — is parker-engine set up?"))?;
        let password = text
            .lines()
            .find_map(|l| l.strip_prefix("OPENCODE_SERVER_PASSWORD="))
            .ok_or_else(|| format!("no OPENCODE_SERVER_PASSWORD in {path}"))?
            .trim()
            .to_string();
        Ok(Engine::new(password))
    }

    /// An engine reached with this password. `from_env` is the real way in;
    /// this is for a frame's tests, which have no engine.env to read.
    pub fn new(password: String) -> Self {
        Engine {
            http: reqwest::Client::new(),
            password,
        }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{BASE}{path}"))
            .basic_auth("opencode", Some(&self.password))
    }

    async fn json(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, String> {
        let mut r = self.req(method, path);
        if let Some(b) = body {
            r = r.json(&b);
        }
        let resp = r.send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("{status}: {text}"));
        }
        serde_json::from_str(&text).map_err(|e| format!("bad json from engine: {e}"))
    }

    pub async fn create_session(&self) -> Result<Value, String> {
        self.json(
            reqwest::Method::POST,
            "/session",
            Some(serde_json::json!({})),
        )
        .await
    }

    /// The transcript. **Not** `/api/session/{id}/message` — that route exists,
    /// answers 200, and returns `{"data":[],"cursor":{…}}` forever, whatever the
    /// session actually holds. The bare route returns the real array, in the
    /// `{info, parts}` shape state.svelte.js already parses.
    ///
    /// Reading the wrong one is invisible rather than loud: the send succeeds, the
    /// engine bills the tokens, the reply is generated — and then the refetch
    /// replaces the transcript with nothing, taking the user's own message with
    /// it. It looks exactly like a model that never answered. Verified against the
    /// live engine, July 2026: same session id, one route empty and the other
    /// holding both turns.
    ///
    /// The two namespaces are not interchangeable and it is not consistent about
    /// which is the real one — `/api/session` is fine, and `/api/event` was until
    /// 1.18.35 (see `pump`). Check any new route against the running engine
    /// rather than pattern-matching off the ones already here.
    pub async fn get_messages(&self, session_id: &str) -> Result<Value, String> {
        self.json(
            reqwest::Method::GET,
            &format!("/session/{session_id}/message"),
            None,
        )
        .await
    }

    /// Send one message through the parker agent. Resolves with the finished
    /// assistant message; the live word-by-word arrives on the event bus
    /// (message.part.delta) long before this returns.
    ///
    /// `place` is where Peter is (`at_the_tower`, `away`), carried in the
    /// message's own `system` field: opencode appends the newest user
    /// message's `system` to the end of the system prompt for every model
    /// call in that turn (session/llm/request.ts), so parker reads it and the
    /// transcript never shows it.
    ///
    /// With no `model`, the message goes out on parker's brain (`brain`),
    /// named explicitly every time — each thread carries a model of its own
    /// (`/session` lists one per thread), so leaving it out risks an old
    /// thread answering on an old brain — and the same `system` field tells
    /// him which one he is on.
    pub async fn send_message(
        &self,
        session_id: &str,
        text: &str,
        model: Option<&str>,
        place: Option<&str>,
    ) -> Result<Value, String> {
        let mut body = serde_json::json!({
            "agent": "parker",
            "parts": [{ "type": "text", "text": text }],
        });
        let mut system: Vec<String> = place.map(str::to_string).into_iter().collect();
        let brain = match model {
            Some(_) => Value::Null,
            None => brain().await,
        };
        let model = model.or_else(|| brain["model"].as_str());
        if let Some(m) = model {
            if let Some((provider, model_id)) = m.split_once('/') {
                body["model"] = serde_json::json!({ "providerID": provider, "modelID": model_id });
            }
        }
        if let (Some(name), Some(m)) = (brain["name"].as_str(), brain["model"].as_str()) {
            system.push(thinking_with(name, m));
        }
        if !system.is_empty() {
            body["system"] = Value::from(system.join(" "));
        }
        let done = self
            .json(
                reqwest::Method::POST,
                &format!("/session/{session_id}/message"),
                Some(body),
            )
            .await;
        // The POST blocks until the turn is over — finished, failed or
        // interrupted alike — so this is the turn's end. A work posture
        // (digging, pointing, …) is false from here on, and parker forgets to
        // take it off; `parker reach settle` does, and leaves every other
        // face alone. parkerd decides which faces are work, not this crate.
        parker(&["reach".into(), "settle".into()]);
        done
    }

    /// Every session the engine remembers. The bare route, same trap as
    /// `get_messages` above: `/api/session` answers 200 with a different,
    /// paginated shape. Filtering and sorting stay in the webview — this door
    /// moves bytes, it does not have opinions.
    pub async fn list_sessions(&self) -> Result<Value, String> {
        self.json(reqwest::Method::GET, "/session", None).await
    }

    pub async fn interrupt_session(&self, session_id: &str) -> Result<Value, String> {
        self.json(
            reqwest::Method::POST,
            &format!("/api/session/{session_id}/interrupt"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// One question to one of the engine's helper agents — `ledger` or
    /// `digest` (engine.nix: haiku, no tools, each with its own prompt) — and
    /// its whole text reply.
    ///
    /// A throwaway session, deleted as soon as it answers so the palette never
    /// lists it and the engine's session store does not fill with them. The
    /// session is created with a title so opencode has no reason to spend a
    /// second call naming it.
    ///
    /// The agent must be one the engine has. A visage built ahead of the
    /// engine.nix that names its agent would otherwise be asking an engine
    /// that may answer with whatever agent it falls back to — and the one
    /// with every tool is parker, holding a transcript he was never meant to
    /// act on. So the engine's own list is asked first, every time: a local
    /// read, next to a model call.
    async fn ask(&self, agent: &str, text: &str) -> Result<String, String> {
        let agents = self.json(reqwest::Method::GET, "/agent", None).await?;
        if !agents
            .as_array()
            .into_iter()
            .flatten()
            .any(|a| a["name"] == agent)
        {
            return Err(format!(
                "the engine has no `{agent}` agent yet — engine.nix is not switched in"
            ));
        }
        let session = self
            .json(
                reqwest::Method::POST,
                "/session",
                Some(serde_json::json!({ "title": agent })),
            )
            .await?;
        let id = session["id"]
            .as_str()
            .ok_or("engine returned a session without an id")?
            .to_string();
        let reply = self
            .json(
                reqwest::Method::POST,
                &format!("/session/{id}/message"),
                Some(serde_json::json!({
                    "agent": agent,
                    "parts": [{ "type": "text", "text": text }],
                })),
            )
            .await;
        // Best-effort: a stray helper session costs nothing but a row.
        let _ = self
            .json(reqwest::Method::DELETE, &format!("/session/{id}"), None)
            .await;
        Ok(reply?["parts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|p| p["type"] == "text")
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"))
    }

    /// One line describing a finished run of acts, for the transcript's fold
    /// row, from the `ledger` agent. The webview sends verbs and targets only
    /// — never tool output — and this moves the bytes.
    pub async fn summarize(&self, text: &str) -> Result<String, String> {
        let reply = self.ask("ledger", text).await?;
        Ok(reply.lines().next().unwrap_or("").trim().to_string())
    }

    /// Write one thread's card for the palette, and give the thread the title
    /// the card chose.
    ///
    /// `text` is the conversation as the webview reads it — what each of them
    /// said and one line per act, never tool output, the line the ledger
    /// draws — and `facts` is what the webview counted (turns, steps, work).
    /// The `digest` agent reads the text and answers with a title, what the
    /// thread was about, where it landed, whether it is still open, and what
    /// it left waiting and on whom (`next`). When someone marked the thread
    /// (`parker recall mark`), the agent is told who, what and after how many
    /// turns, so a thread Peter closed is not reopened by its old steps. The
    /// card is kept under `$XDG_STATE_HOME/parker/digests/`, on parker, so
    /// the tower and the porthole share one copy and each thread is paid for
    /// once; `updated` in it is the thread's own, and a card whose `updated`
    /// no longer matches is stale.
    ///
    /// The title goes back to the engine, so the palette and everything else
    /// that lists threads names it the same way. Renaming does not move the
    /// thread's `updated` (measured against 1.18.35, Oct 2026), so the list
    /// keeps its order and the card stays fresh. The title the engine had
    /// first — opencode's, named from the opening message alone — is kept in
    /// the card as `was`, through every later rewrite.
    pub async fn digest(
        &self,
        session_id: &str,
        text: &str,
        facts: Value,
    ) -> Result<Value, String> {
        // First, and before the network: the id becomes a file name.
        let path = card_path(session_id).ok_or_else(|| format!("not a thread id: {session_id}"))?;
        let session = self
            .json(
                reqwest::Method::GET,
                &format!("/session/{session_id}"),
                None,
            )
            .await?;
        let current = session["title"].as_str().unwrap_or("").to_string();
        let earlier = mark_path(session_id)
            .and_then(|p| read_card(&p))
            .map(|m| told(&m, read_card(&path).as_ref()))
            .unwrap_or_default();
        let reply = self
            .ask(
                "digest",
                &format!("Current title: {current}\n{earlier}\n{text}"),
            )
            .await?;
        let mut card = card(&reply).ok_or("the digest agent did not answer with a card")?;
        let title = card["title"].as_str().unwrap_or("").to_string();
        if title.is_empty() {
            card["title"] = Value::from(current.as_str());
        } else if title != current {
            self.json(
                reqwest::Method::PATCH,
                &format!("/session/{session_id}"),
                Some(serde_json::json!({ "title": title })),
            )
            .await?;
        }
        let was = read_card(&path)
            .and_then(|old| old.get("was").cloned())
            .unwrap_or_else(|| Value::from(current));
        // 2: the card has `next`. A version-1 card left open is read again
        // (state.svelte.js `fresh`), so its steps get written.
        card["v"] = Value::from(2);
        card["updated"] = session["time"]["updated"].clone();
        card["was"] = was;
        card["facts"] = facts;
        card["at"] = Value::from(now_ms());
        write_card(&path, &card)?;
        Ok(card)
    }

    /// Tail the engine's SSE bus forever, handing every event to `emit`.
    /// Reconnects on any failure: the face outliving an engine restart is the
    /// point ("any face can die" — and so can the engine).
    ///
    /// The bare `/event`, not `/api/event`: on opencode 1.18.35 the `/api`
    /// stream is closed by the server whenever *any* message is posted — every
    /// prompt, every ledger summary — so visage spent ~2 s of each turn showing
    /// "engine unreachable" while it reconnected, and missed the events in
    /// between. `/event` stays open through the same calls with the same event
    /// types (measured with curl, Oct 2026). The webview only looks for its
    /// session id in an event, so the route's shape is not load-bearing.
    pub async fn pump(&self, mut emit: impl FnMut(Event)) {
        loop {
            let resp = self.req(reqwest::Method::GET, "/event").send().await;
            match resp {
                Ok(resp) if resp.status().is_success() => {
                    emit(Event::Status("connected"));
                    let mut buf = String::new();
                    let mut stream = resp.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        let Ok(chunk) = chunk else { break };
                        buf.push_str(&String::from_utf8_lossy(&chunk));
                        // SSE frames end on a blank line; data may span lines.
                        while let Some(end) = buf.find("\n\n") {
                            let frame: String = buf[..end].to_string();
                            buf.drain(..end + 2);
                            let data: String = frame
                                .lines()
                                .filter_map(|l| l.strip_prefix("data:"))
                                .map(|l| l.trim_start())
                                .collect::<Vec<_>>()
                                .join("\n");
                            if data.is_empty() {
                                continue;
                            }
                            if let Ok(v) = serde_json::from_str::<Value>(&data) {
                                emit(Event::Engine(v));
                            }
                        }
                    }
                }
                _ => {}
            }
            emit(Event::Status("disconnected"));
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

/// The cards door has for these threads (`Engine::digest`), by id, as they
/// were written — fresh or stale is the webview's call, by comparing each
/// card's `updated` with the thread's. A thread with no card is left out.
pub fn digests(ids: &[String]) -> Value {
    let cards = ids
        .iter()
        .take(200)
        .filter_map(|id| Some((id.clone(), read_card(&card_path(id)?)?)))
        .collect();
    Value::Object(cards)
}

/// The marks door has for these threads, by id, as `parker recall mark`
/// wrote them — holding or not: a mark holds while its `updated` is the
/// thread's, and that is the webview's call, as with a card. A thread with
/// no mark is left out.
pub fn marks(ids: &[String]) -> Value {
    let marks = ids
        .iter()
        .take(200)
        .filter_map(|id| Some((id.clone(), read_card(&mark_path(id)?)?)))
        .collect();
    Value::Object(marks)
}

/// The palette's ^X: Peter's word on a thread — `done`, `open`, or `clear`
/// to let its card speak again — through `parker recall mark`, the one
/// writer of marks (parkerd's recall.rs). What reaches the CLI is a thread
/// id and one of three words, never a command line: the id is checked to be
/// exactly a thread id first, so it can never be read as a flag. Resolves
/// with what the CLI kept: `{ id, updated, mark }`, `mark` null when cleared,
/// and `updated` the thread's as the engine has it now.
pub async fn mark(session_id: String, state: String) -> Result<Value, String> {
    if !is_thread_id(&session_id) {
        return Err(format!("not a thread id: {session_id}"));
    }
    if !matches!(state.as_str(), "done" | "open" | "clear") {
        return Err(format!("not a mark: {state}"));
    }
    let bin = std::env::var("PARKER_BIN").unwrap_or_else(|_| "parker".into());
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new(bin)
            .args(["recall", "mark", &session_id, &state, "--json"])
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("parker is not runnable: {e}"))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if said.is_empty() {
            format!("parker recall mark exited {}", out.status)
        } else {
            said
        });
    }
    serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("parker recall mark said something unreadable: {e}"))
}

/// What the digest is told about a thread's mark, so that a thread someone
/// settled stays settled unless what was said after the mark opens it again
/// — and one someone kept open is not closed just because it went quiet.
/// `turns` places the mark in the conversation: the digest counts Peter's
/// lines. A done mark names the steps it set aside, from the card it set
/// them aside on.
fn told(mark: &Value, card: Option<&Value>) -> String {
    let who = if mark["by"] == "parker" {
        "parker"
    } else {
        "Peter"
    };
    let turns = mark["turns"].as_u64().unwrap_or(0);
    let note = mark["note"]
        .as_str()
        .map(|n| format!(" (\"{n}\")"))
        .unwrap_or_default();
    if mark["state"] == "open" {
        return format!(
            "{who} marked this conversation still open when Peter had written {turns} messages{note}.\n"
        );
    }
    let aside: Vec<&str> = card
        .and_then(|c| c["next"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|s| s["step"].as_str())
        .collect();
    let aside = if aside.is_empty() {
        String::new()
    } else {
        format!(" Steps set aside then: {}.", aside.join("; "))
    };
    format!(
        "{who} marked this conversation done when Peter had written {turns} messages{note}.{aside}\n"
    )
}

/// Exactly the shape opencode mints — `ses_` and letters and digits. An id
/// arrives from a page (over the tailnet, in the porthole) and becomes a file
/// name or a CLI argument, so anything else names nothing at all: never a
/// path, never a flag.
fn is_thread_id(session_id: &str) -> bool {
    session_id.strip_prefix("ses_").is_some_and(|rest| {
        !rest.is_empty() && rest.len() <= 64 && rest.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

/// Where a thread's card lives.
fn card_path(session_id: &str) -> Option<PathBuf> {
    is_thread_id(session_id)
        .then(|| PathBuf::from(format!("{}/parker/digests/{session_id}.json", state_dir())))
}

/// Where a thread's mark lives: beside the cards, written only by
/// `parker recall mark`.
fn mark_path(session_id: &str) -> Option<PathBuf> {
    is_thread_id(session_id)
        .then(|| PathBuf::from(format!("{}/parker/marks/{session_id}.json", state_dir())))
}

fn read_card(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Whole or not at all: both frames may write the same card, so each writes
/// its own temporary file and renames it into place.
fn write_card(path: &Path, card: &Value) -> Result<(), String> {
    let dir = path.parent().ok_or("a card path without a directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let tmp = PathBuf::from(format!("{}.{}.tmp", path.display(), std::process::id()));
    std::fs::write(&tmp, card.to_string()).map_err(|e| format!("cannot write a card: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("cannot keep a card: {e}"))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The digest agent's reply as a card: `title`, `about`, `landed`, `state`,
/// and `next` — what the thread left waiting, each step `{ who, step }`.
/// Haiku wraps the object in a code fence or a sentence now and then, so
/// this takes the outermost braces and nothing else. Each value is one line
/// with a cap, because the reply was written from a transcript that can
/// quote the open web, and it lands in a list — as text, never markup, but
/// a list all the same. A state outside the three is no state, a step with
/// nobody to take it is no step, and three steps is the most a card holds.
fn card(reply: &str) -> Option<Value> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    let v: Value = serde_json::from_str(reply.get(start..=end)?).ok()?;
    let field = |key: &str, max: usize| tidy(v.get(key).and_then(Value::as_str).unwrap_or(""), max);
    let state = v
        .get("state")
        .and_then(Value::as_str)
        .filter(|s| matches!(*s, "done" | "open" | "chat"))
        .unwrap_or("");
    let next: Vec<Value> = v
        .get("next")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let who = match s
                .get("who")
                .and_then(Value::as_str)?
                .to_lowercase()
                .as_str()
            {
                "you" | "peter" => "you",
                "parker" => "parker",
                _ => return None,
            };
            let step = tidy(s.get("step").and_then(Value::as_str)?, 120);
            (!step.is_empty()).then(|| serde_json::json!({ "who": who, "step": step }))
        })
        .take(3)
        .collect();
    let card = serde_json::json!({
        "title": field("title", 60),
        "about": field("about", 160),
        "landed": field("landed", 180),
        "state": state,
        "next": next,
    });
    let said = ["title", "about", "landed"].iter().any(|k| card[k] != "");
    said.then_some(card)
}

fn tidy(s: &str, max: usize) -> String {
    let line = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let line = line.trim_end_matches('.').trim_end();
    if line.chars().count() <= max {
        return line.to_string();
    }
    let cut: String = line.chars().take(max - 1).collect();
    format!("{}…", cut.trim_end())
}

/// Where Peter is, as parker reads it with every message from visage
/// (`Engine::send_message`). What parker does with it is his mind's business
/// (mind/hands.md); this only states the fact.
///
/// Each place is one fixed sentence, and must stay fixed. It lands at the
/// end of the system prompt, so a sentence that changed per message — a
/// time, a counter — would cost the prompt cache on every turn; one that
/// changes only when Peter moves costs it once.
pub fn at_the_tower() -> String {
    "Peter is at the tower: this message came through visage on parker's own screen.".into()
}

/// Away from the tower, on `device` — giskard or the phone, as the tailnet
/// names it (the porthole asks `tailscale whois`).
pub fn away(device: &str) -> String {
    format!(
        "Peter is away from the tower: this message came through the porthole, on {device}, over the tailnet."
    )
}

/// Which brain parker is on, as he reads it beside where Peter is. Fixed per
/// brain for the same reason the place sentences are fixed per place: it
/// sits at the end of the system prompt. Switching brains costs the cache
/// anyway — a cache belongs to one model — so naming the brain here costs
/// nothing a switch did not already spend.
pub fn thinking_with(name: &str, model: &str) -> String {
    format!("parker's brain for this message is {name} ({model}), the one Peter picked with `parker brain`.")
}

/// Which brain parker is on: `parker brain --json` — the brain in use, the
/// default, and the set (parkerd's brain.rs, the one place a name becomes a
/// model). Null when parker is not on PATH or has no brains; a message then
/// names no model and the engine's own default answers it, which is a
/// different brain, not a broken parker.
pub async fn brain() -> Value {
    let bin = std::env::var("PARKER_BIN").unwrap_or_else(|_| "parker".into());
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new(bin)
            .args(["brain", "--json"])
            .output()
    })
    .await;
    match out {
        Ok(Ok(o)) if o.status.success() => serde_json::from_slice(&o.stdout).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// Whether `name` is a brain in `brain()`'s set, by its exact name.
fn is_brain(set: &Value, name: &str) -> bool {
    set["brains"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|b| b["name"].as_str() == Some(name))
}

/// The palette's `/brain`. Through `parker brain <name>`, the one
/// implementation, and only for a name the set already lists: this webview
/// renders text from the open web, so what it can hand the CLI is a brain,
/// never a command line — the reason `theme` is an allowlist too. Resolves
/// with `brain()` as it stands afterwards.
pub async fn set_brain(name: String) -> Result<Value, String> {
    if !is_brain(&brain().await, &name) {
        return Err(format!("not a brain: {name}"));
    }
    let bin = std::env::var("PARKER_BIN").unwrap_or_else(|_| "parker".into());
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new(bin)
            .arg("brain")
            .arg(&name)
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("parker is not runnable: {e}"))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if said.is_empty() {
            format!("parker brain exited {}", out.status)
        } else {
            said
        });
    }
    Ok(brain().await)
}

/// What parker is wearing, and whether he is dancing.
///
/// Two one-line files in the runtime dir, written by `parker reach face` and
/// `parker reach dance` (parkerd's face/state.rs). Reading them here is what
/// makes those verbs reach visage: until then `reach face curious` changed
/// the terminal face and visage never heard about it, which made the main
/// face the one place parker could not choose an expression.
///
/// Each line is `<word> <unix-deadline>`, and a deadline of 0 means "until
/// something changes it". Expiry is evaluated on read, exactly as in parkerd
/// — nothing has to be running for a beat to end. Read on parker, so the
/// deadlines are judged by parker's clock wherever the face is drawn.
///
/// It is a poll rather than a watch on purpose. This is a two-field read of
/// two tiny files a few times a second; a file watcher would be more code,
/// another failure mode, and no faster than the frame that consumes it. It
/// becomes a subscription when parkerd grows a channel to subscribe to, and
/// `state.svelte.js` says the same thing about the same seam.
pub fn face_state() -> Value {
    fn slot(name: &str) -> Option<String> {
        let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        let raw = std::fs::read_to_string(format!("{base}/parker/{name}")).ok()?;
        let mut parts = raw.split_whitespace();
        let word = parts.next()?.to_string();
        let deadline: f64 = parts.next()?.parse().ok()?;
        if deadline > 0.0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            if now >= deadline {
                return None;
            }
        }
        Some(word)
    }
    // The beat cue crosses raw — deadline included — because the webview
    // keys change-detection on the whole line: the deadline differs per
    // write, so the same beat cued twice running still plays twice.
    fn raw(name: &str) -> Option<String> {
        let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        let raw = std::fs::read_to_string(format!("{base}/parker/{name}")).ok()?;
        let deadline: f64 = raw.split_whitespace().nth(1)?.parse().ok()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        if now >= deadline {
            return None;
        }
        Some(raw.trim().to_string())
    }
    serde_json::json!({
        "expression": slot("face"),
        "dance": slot("dance"),
        "beat": raw("beat"),
    })
}

/// Start or stop a dance — Ctrl+Space, and Esc's first tier.
///
/// Runs `parker reach dance` because that verb *is* the one implementation.
/// It writes the runtime slot that `face_state` reads back, so the keybind,
/// an SSH session, a phone and parker deciding to dance on his own all travel
/// the identical path and end up in the identical state. Toggling a local
/// flag in a frame instead would work perfectly until the moment two of those
/// disagreed.
pub fn dance(routine: String, seconds: Option<f64>) {
    let mut args = vec!["reach".to_string(), "dance".to_string(), routine];
    if let Some(s) = seconds {
        args.push("--for".into());
        args.push(s.to_string());
    }
    parker(&args);
}

/// The palette's `/theme` commands, through the `theme` CLI (home/theming).
///
/// An allowlist, not a pass-through: this webview renders text from the
/// open web, and a command that ran whatever arguments it was handed would
/// be a shell door behind every `{@html}`. So exactly the forms the palette
/// offers — `light`, `dark`, `auto` (seconds: a specialisation flip) and
/// `set <family>` (a full rebuild, ~30 s) — and nothing else. The families
/// are themes/default.nix's three; a new one is added here on purpose.
///
/// Resolves with the command's last line of output, so the palette can say
/// what happened; a failure carries its last line of stderr.
pub async fn theme(args: Vec<String>) -> Result<String, String> {
    const FAMILIES: [&str; 3] = ["garden", "drift", "forge"];
    let allowed = match args.as_slice() {
        [mode] => ["light", "dark", "auto"].contains(&mode.as_str()),
        [set, family] => set == "set" && FAMILIES.contains(&family.as_str()),
        _ => false,
    };
    if !allowed {
        return Err(format!(
            "not a palette theme command: theme {}",
            args.join(" ")
        ));
    }
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new("theme").args(&args).output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("theme is not runnable: {e}"))?;
    let stream = if out.status.success() {
        &out.stdout
    } else {
        &out.stderr
    };
    let last = String::from_utf8_lossy(stream)
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string();
    if out.status.success() {
        Ok(last)
    } else {
        Err(if last.is_empty() {
            format!("theme exited {}", out.status)
        } else {
            last
        })
    }
}

/// One reading of the machine, for the corner: `parker sense --json`,
/// parsed just enough to prove it is JSON. The webview polls this slowly
/// (seconds, not frames — sense forks nvidia-smi) and keeps its own history
/// for the sparkline meters. Null when parker is not on PATH or the read
/// fails; the corner shows what it knows and nothing else.
pub async fn sense_snapshot() -> Value {
    let bin = std::env::var("PARKER_BIN").unwrap_or_else(|_| "parker".into());
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new(bin)
            .args(["sense", "--json"])
            .output()
    })
    .await;
    match out {
        Ok(Ok(o)) if o.status.success() => serde_json::from_slice(&o.stdout).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// The controls, for ^,: `parker controls --json` — every setting as a row
/// (parkerd's controls/, the one list; docs/visage/controls/README.md).
/// Null when parker is not on PATH or the read fails; ^, then says it
/// cannot reach them rather than drawing a guess.
pub async fn controls() -> Value {
    let bin = std::env::var("PARKER_BIN").unwrap_or_else(|_| "parker".into());
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new(bin)
            .args(["controls", "--json"])
            .output()
    })
    .await;
    match out {
        Ok(Ok(o)) if o.status.success() => serde_json::from_slice(&o.stdout).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// A row's id as parkerd names them: `output`, `app:titanfall-2`,
/// `bt:d0:bc:c1:d7:55:69`. Never anything flag-shaped or path-shaped.
fn is_control_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, ':' | '-' | '.' | '_'))
}

/// A value as ^, sends one: a level (`45`, `+5`, `-5`), a word (`on`,
/// `mute`, `connect`), a choice (`PEKI`, `deep thought`). Short, printable,
/// no control characters.
fn is_control_value(s: &str) -> bool {
    !s.trim().is_empty()
        && s.len() <= 48
        && s.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '+' | '-' | '.' | '_' | '\''))
}

/// One change from ^, (or ^K's generated "/"): `parker controls set`, the
/// one implementation. The id and value are checked for shape here and
/// handed over after `--`, so neither can ever be read as a flag; parkerd
/// then refuses an id or a value its list does not have. `away` is the
/// porthole's: the hand is not at the tower, so what would cut the line it
/// is on (wifi off, tailscale, power off) is refused by parkerd. Resolves
/// with what changed: `{ id, label, kind, from, to, said }`.
pub async fn set_control(id: String, value: String, away: bool) -> Result<Value, String> {
    if !is_control_id(&id) {
        return Err(format!("not a control: {id}"));
    }
    if !is_control_value(&value) {
        return Err(format!("not a value ^, sends: {value}"));
    }
    let bin = std::env::var("PARKER_BIN").unwrap_or_else(|_| "parker".into());
    let out = tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new(bin);
        cmd.args(["controls", "set", "--json"]);
        if away {
            cmd.arg("--away");
        }
        cmd.arg("--").arg(&id).arg(&value).output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("parker is not runnable: {e}"))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if said.is_empty() {
            format!("parker controls exited {}", out.status)
        } else {
            said
        });
    }
    serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("parker controls said something unreadable: {e}"))
}

/// The numbered spaces and which one is in front, for the corner's
/// `space 1 2 3 4` line: `{ spaces: [{ id, windows }], active }`.
///
/// Read from Hyprland directly, two `hyprctl -j` calls, because it is read
/// on every workspace event (`watch_spaces`) and `parker sense` forks
/// nvidia-smi. Special workspaces (visage's own among them) are left out:
/// the line says where you are under visage. Null without Hyprland.
pub fn spaces() -> Value {
    fn hyprctl(what: &str) -> Option<Value> {
        let mut cmd = std::process::Command::new("hyprctl");
        cmd.args(["-j", what]);
        if let Some(his) = hypr_signature() {
            cmd.env("HYPRLAND_INSTANCE_SIGNATURE", his);
        }
        let out = cmd.output().ok()?;
        if !out.status.success() {
            return None;
        }
        serde_json::from_slice(&out.stdout).ok()
    }
    let Some(Value::Array(list)) = hyprctl("workspaces") else {
        return Value::Null;
    };
    let active = hyprctl("activeworkspace").and_then(|w| w["id"].as_i64());
    let mut spaces: Vec<(i64, i64)> = list
        .iter()
        .filter_map(|w| Some((w["id"].as_i64()?, w["windows"].as_i64().unwrap_or(0))))
        .filter(|(id, _)| *id > 0)
        .collect();
    spaces.sort();
    serde_json::json!({
        "spaces": spaces.iter().map(|(id, n)| serde_json::json!({ "id": id, "windows": n })).collect::<Vec<_>>(),
        "active": active,
    })
}

/// Hyprland's instance, from the environment or the newest one in the
/// runtime dir — the same fallback parkerd's sense/hypr.rs makes, for a
/// frame started where the variable did not reach.
fn hypr_signature() -> Option<String> {
    if let Ok(his) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        if !his.is_empty() {
            return Some(his);
        }
    }
    let dir = format!("{}/hypr", std::env::var("XDG_RUNTIME_DIR").ok()?);
    let mut all: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.file_name().to_string_lossy().into_owned())))
        .collect();
    all.sort();
    all.pop().map(|(_, n)| n)
}

/// Follow the spaces as they change, for the corner: Hyprland's event
/// socket (`.socket2.sock`, which is safe to hold open; the request socket
/// is not — parkerd's sense/hypr.rs), re-read on every event that can move
/// them, and handed to `emit` when the answer differs. Runs forever on its
/// own thread, reconnecting every few seconds when Hyprland is away.
///
/// visage sits on a special workspace, so the space underneath can change
/// while it is up — SUPER+1 with the face showing — and the line has to
/// follow at once, not on the next poll.
pub fn watch_spaces(emit: impl Fn(Value) + Send + 'static) {
    use std::io::{BufRead, BufReader};
    std::thread::spawn(move || {
        let mut last = Value::Null;
        let mut send = |now: Value| {
            if now != last {
                last = now.clone();
                emit(now);
            }
        };
        loop {
            send(spaces());
            let sock = hypr_signature().and_then(|his| {
                let dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
                std::os::unix::net::UnixStream::connect(format!("{dir}/hypr/{his}/.socket2.sock")).ok()
            });
            if let Some(sock) = sock {
                for line in BufReader::new(sock).lines() {
                    let Ok(line) = line else { break };
                    let event = line.split(">>").next().unwrap_or_default();
                    if matches!(
                        event,
                        "workspace" | "workspacev2" | "focusedmon" | "focusedmonv2" | "createworkspace"
                            | "createworkspacev2" | "destroyworkspace" | "destroyworkspacev2"
                            | "moveworkspace" | "moveworkspacev2" | "openwindow" | "closewindow"
                            | "movewindow" | "movewindowv2"
                    ) {
                        send(spaces());
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    });
}

/// The sound levels, as door's level watch compares them: every row of
/// `parker controls --only sound` that has a level — all sound, the mic,
/// each app — by id. None when parker will not answer.
fn levels() -> Option<HashMap<String, Level>> {
    let bin = std::env::var("PARKER_BIN").unwrap_or_else(|_| "parker".into());
    let out = std::process::Command::new(bin)
        .args(["controls", "--json", "--only", "sound"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(levels_of(&serde_json::from_slice(&out.stdout).ok()?))
}

/// One level as the watch keeps it.
#[derive(Clone, Debug, PartialEq)]
struct Level {
    label: String,
    level: i64,
    muted: bool,
    /// Where it plays (`monitor speakers`): a different one is a different
    /// device, not a move.
    sub: String,
}

fn levels_of(v: &Value) -> HashMap<String, Level> {
    let mut out = HashMap::new();
    let rows = v["sections"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|s| s["rows"].as_array().into_iter().flatten())
        .flat_map(|r| std::iter::once(r).chain(r["children"].as_array().into_iter().flatten()));
    for r in rows {
        let (Some(id), Some(level)) = (r["id"].as_str(), r["value"].as_i64()) else {
            continue;
        };
        if r["kind"] != "slider" {
            continue;
        }
        out.insert(
            id.to_string(),
            Level {
                label: r["label"].as_str().unwrap_or(id).to_string(),
                level,
                muted: r["muted"].as_bool().unwrap_or(false),
                sub: r["sub"].as_str().unwrap_or_default().to_string(),
            },
        );
    }
    out
}

/// What one batch from `pw-dump --monitor` says to the watch.
#[derive(Debug, Default, PartialEq)]
struct Seen {
    /// A node's params changed (a volume, a mute), or a node came or went:
    /// read the levels again.
    stir: bool,
    /// A `wpctl` was connected when a node changed, or had just left. The
    /// desk's volume keys are wpctl (hyprland.nix). Its client's removal
    /// nearly always comes after the node's params, for a stream and for
    /// the sink alike, but now and then the node's word lands a moment
    /// after the wpctl has gone (measured, Oct 2026), hence WPCTL_GRACE.
    /// parkerd's changes are wpctl too; they are stamped and never said.
    wpctl: bool,
}

/// How lately a wpctl may have left and still be what moved a level. A
/// wpctl that only read a level, a moment before something else moved it,
/// is taken for the keys too: rare, and a word wrong on a quiet line.
const WPCTL_GRACE: Duration = Duration::from_millis(300);

/// What the monitor has said so far: every node id, because a removal
/// says only `{ id, info: null }`; the clients connected now, by the
/// binary that opened them; and when a wpctl was last there.
#[derive(Default)]
struct Monitor {
    nodes: HashSet<i64>,
    clients: HashMap<i64, String>,
    wpctl_at: Option<std::time::Instant>,
}

impl Monitor {
    /// Read one batch, in order. A client coming or going never stirs: the
    /// watch's own reads are clients, and a read that stirred a read would
    /// be a loop.
    fn read(&mut self, batch: &Value, now: std::time::Instant) -> Seen {
        let mut s = Seen::default();
        for o in batch.as_array().into_iter().flatten() {
            let Some(id) = o["id"].as_i64() else { continue };
            match o["type"].as_str() {
                Some("PipeWire:Interface:Node") => {
                    let fresh = self.nodes.insert(id);
                    let params = o["info"]["change-mask"]
                        .as_array()
                        .is_some_and(|m| m.iter().any(|x| x == "params"));
                    if fresh || params {
                        s.stir = true;
                        s.wpctl |= self.clients.values().any(|b| b == "wpctl")
                            || self.wpctl_at.is_some_and(|t| now - t < WPCTL_GRACE);
                    }
                }
                Some("PipeWire:Interface:Client") => {
                    if let Some(b) = o["info"]["props"]["application.process.binary"].as_str() {
                        if b == "wpctl" {
                            self.wpctl_at = Some(now);
                        }
                        self.clients.insert(id, b.to_string());
                    }
                }
                None if o["info"].is_null() => {
                    if self.clients.remove(&id).is_some_and(|b| b == "wpctl") {
                        self.wpctl_at = Some(now);
                    }
                    s.stir |= self.nodes.remove(&id);
                }
                _ => {}
            }
        }
        s
    }
}

/// parkerd's stamps (controls/sound.rs, "whose hand"): `<unix-seconds> <row
/// id> <level | muted | on>`, one per level parkerd set.
fn hand() -> String {
    let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    std::fs::read_to_string(format!("{base}/parker/sound-hand")).unwrap_or_default()
}

/// How long a stamp says "parker did that". Wide, because the read that
/// meets it comes a settle later; narrow enough that the keys a little
/// after parker are still the keys.
const HAND_SECS: f64 = 5.0;

/// A row that just appeared is not watched for this long: WirePlumber puts
/// an app's remembered level on its stream after the stream is there, and
/// that is not anyone moving it.
const YOUNG: Duration = Duration::from_secs(3);

/// Whether parkerd set `id` to `what` in the last HAND_SECS.
fn stamped(hand: &str, now: f64, id: &str, what: &str) -> bool {
    hand.lines().any(|l| {
        let mut w = l.split_whitespace();
        let at = w.next().and_then(|t| t.parse::<f64>().ok());
        at.is_some_and(|t| (now - t).abs() < HAND_SECS)
            && w.next() == Some(id)
            && w.next() == Some(what)
    })
}

/// What moved from `was` to `now` without parkerd's hand on it: one change
/// per level and per mute, `{ id, label, kind, from, to }`. `young` rows,
/// rows on another device, and what parkerd stamped are left out.
fn moved(
    was: &HashMap<String, Level>,
    now: &HashMap<String, Level>,
    young: &HashSet<String>,
    hand: &str,
    at: f64,
) -> Vec<Value> {
    let mut out = Vec::new();
    let mut ids: Vec<&String> = now.keys().collect();
    ids.sort();
    let word = |m: bool| if m { "muted" } else { "on" };
    for id in ids {
        let (Some(a), b) = (was.get(id), &now[id]) else {
            continue;
        };
        if young.contains(id) || a.sub != b.sub {
            continue;
        }
        if a.level != b.level && !stamped(hand, at, id, &b.level.to_string()) {
            out.push(serde_json::json!({
                "id": id, "label": b.label, "kind": "level", "from": a.level, "to": b.level,
            }));
        }
        if a.muted != b.muted && !stamped(hand, at, id, word(b.muted)) {
            out.push(serde_json::json!({
                "id": id, "label": b.label, "kind": "mute", "from": word(a.muted), "to": word(b.muted),
            }));
        }
    }
    out
}

/// How long the levels must be still before they are read, and the longest
/// a read waits while they keep moving. A held key (35 a second) becomes a
/// few events, and the page folds those into one line.
const SETTLE: Duration = Duration::from_millis(250);
const SETTLE_MAX: Duration = Duration::from_millis(1200);

/// Follow the sound levels — all sound, the mic, each app — and hand `emit`
/// whatever moved that parkerd did not move: `{ changes: [{ id, label,
/// kind, from, to }], keys }`, `kind` `level` (numbers) or `mute` (`on`,
/// `muted`), `keys` when a wpctl was connected as it moved, which is what
/// the desk's volume keys are. visage says it as a quiet line in the
/// transcript ("all sound 42 → 50 · the keys on the desk, not parker"); a
/// change parker or ^, made is not said, because the transcript already
/// draws it, and parkerd stamps it (controls/sound.rs, "whose hand").
///
/// `pw-dump --monitor` is only the doorbell: every batch it prints is
/// looked at for a node whose params changed, and then the levels are read
/// the one way they are read anywhere, through parkerd — names, the mixer's
/// friendly apps and all. Runs forever on its own thread, and starts again
/// a few seconds after PipeWire goes away.
pub fn watch_levels(emit: impl Fn(Value) + Send + 'static) {
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::Instant;
    std::thread::spawn(move || loop {
        let child = Command::new("pw-dump")
            .args(["--monitor", "--no-colors"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = child else {
            std::thread::sleep(Duration::from_secs(5));
            continue;
        };
        let (tx, rx) = mpsc::channel::<Seen>();
        if let Some(out) = child.stdout.take() {
            std::thread::spawn(move || {
                let mut monitor = Monitor::default();
                let batches = serde_json::Deserializer::from_reader(std::io::BufReader::new(out))
                    .into_iter::<Value>();
                for batch in batches {
                    let Ok(batch) = batch else { break };
                    let s = monitor.read(&batch, Instant::now());
                    if s.stir && tx.send(s).is_err() {
                        break;
                    }
                }
            });
        }
        let mut was = levels().unwrap_or_default();
        let mut born: HashMap<String, Instant> = HashMap::new();
        // Wait for a stir, then for the levels to settle.
        'watch: while let Ok(first) = rx.recv() {
            let mut keys = first.wpctl;
            let start = Instant::now();
            while start.elapsed() < SETTLE_MAX {
                match rx.recv_timeout(SETTLE) {
                    Ok(s) => keys |= s.wpctl,
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break 'watch,
                }
            }
            let Some(now) = levels() else { continue };
            for id in now.keys().filter(|id| !was.contains_key(*id)) {
                born.insert(id.clone(), Instant::now());
            }
            born.retain(|_, t| t.elapsed() < YOUNG);
            let young: HashSet<String> = born.keys().cloned().collect();
            let changes = moved(&was, &now, &young, &hand(), now_ms() as f64 / 1000.0);
            if !changes.is_empty() {
                emit(serde_json::json!({ "changes": changes, "keys": keys }));
            }
            was = now;
        }
        let _ = child.kill();
        let _ = child.wait();
        std::thread::sleep(Duration::from_secs(3));
    });
}

/// Run a parker verb, detached and best-effort. A verb that will not run is
/// not worth an error banner on a face — parker is never in the critical
/// path, least of all of himself. The thread only exists to reap the child.
pub fn parker(args: &[String]) {
    let bin = std::env::var("PARKER_BIN").unwrap_or_else(|_| "parker".into());
    let args: Vec<String> = args.to_vec();
    std::thread::spawn(move || {
        let _ = std::process::Command::new(bin).args(&args).status();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_listed_brain_reaches_the_cli() {
        let set = serde_json::json!({
            "name": "deep thought",
            "brains": [
                { "name": "deep thought", "model": "anthropic/claude-opus-5-5" },
                { "name": "speedy", "model": "anthropic/claude-haiku-5-5" },
            ],
        });
        assert!(is_brain(&set, "speedy"));
        assert!(is_brain(&set, "deep thought"));
        // The CLI would forgive these (brain.rs `find`); the page gets no
        // such latitude — exact names only, and nothing flag-shaped.
        for bad in ["opus", "Speedy", "--help", "speedy --json", "", "../brain"] {
            assert!(!is_brain(&set, bad), "{bad:?} passed");
        }
        assert!(!is_brain(&Value::Null, "speedy"), "no set, no brain");
    }

    #[test]
    fn only_a_control_shaped_id_and_value_reach_the_cli() {
        for ok in ["output", "app:titanfall-2", "bt:d0:bc:c1:d7:55:69", "dnd"] {
            assert!(is_control_id(ok), "{ok}");
        }
        for bad in ["", "--away", "-x", "Output", "../x", "app:a b", "a/b", &"a".repeat(65)] {
            assert!(!is_control_id(bad), "{bad:?}");
        }
        for ok in ["45", "+5", "-5", "mute", "deep thought", "PEKI", "on"] {
            assert!(is_control_value(ok), "{ok}");
        }
        for bad in ["", "  ", "a;b", "$(x)", "a\nb", "`x`", &"9".repeat(49)] {
            assert!(!is_control_value(bad), "{bad:?}");
        }
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        assert!(rt.block_on(set_control("--help".into(), "on".into(), false)).is_err());
        assert!(rt.block_on(set_control("dnd".into(), "on; reboot".into(), false)).is_err());
    }

    #[test]
    fn a_card_is_named_by_a_thread_id_and_nothing_else() {
        assert!(card_path("ses_ed9d3b3d4ffe1aDeWWLYKoJm2n").is_some());
        for bad in [
            "",
            "ses_",
            "../engine.env",
            "ses_../../engine",
            "ses_abc/def",
            "ses_abc.json",
            "ses_ab c",
            "msg_abc",
            "SES_abc",
        ] {
            assert!(card_path(bad).is_none(), "{bad:?} named a file");
        }
        assert!(card_path(&format!("ses_{}", "a".repeat(65))).is_none());
    }

    #[test]
    fn a_reply_becomes_a_card_through_a_fence() {
        let reply = "Here you go:\n```json\n{\"title\": \"hide the top bar\", \"about\": \"turn off the bar.\", \"landed\": \"auto-hide on, super+b pins it\", \"state\": \"done\"}\n```";
        let c = card(reply).unwrap();
        assert_eq!(c["title"], "hide the top bar");
        assert_eq!(c["about"], "turn off the bar");
        assert_eq!(c["state"], "done");
    }

    #[test]
    fn a_card_is_one_line_with_a_cap_and_a_known_state() {
        let long = "word ".repeat(80);
        let reply = format!(
            "{{\"title\": \"two\\nlines\", \"about\": \"{long}\", \"landed\": \"x\", \"state\": \"finished\"}}"
        );
        let c = card(&reply).unwrap();
        assert_eq!(c["title"], "two lines");
        assert!(c["about"].as_str().unwrap().chars().count() <= 160);
        assert!(c["about"].as_str().unwrap().ends_with('…'));
        assert_eq!(c["state"], "");
    }

    #[test]
    fn a_card_keeps_three_steps_each_with_someone_to_take_it() {
        let reply = r#"{"title": "kernel", "about": "a", "landed": "b", "state": "open", "next": [
            {"who": "you", "step": "reboot\ninto 6.18."},
            {"who": "Peter", "step": "pick a theme"},
            {"who": "parker", "step": "check nvidia-smi"},
            {"who": "parker", "step": "a fourth"}
        ]}"#;
        let c = card(reply).unwrap();
        let next = c["next"].as_array().unwrap();
        assert_eq!(next.len(), 3);
        assert_eq!(
            next[0],
            serde_json::json!({ "who": "you", "step": "reboot into 6.18" })
        );
        assert_eq!(next[1]["who"], "you");
        assert_eq!(next[2]["who"], "parker");
        let odd = r#"{"title": "t", "state": "open", "next": [
            {"who": "someone", "step": "x"}, {"who": "you"}, {"who": "you", "step": "  "}, "a string"
        ]}"#;
        assert_eq!(card(odd).unwrap()["next"], serde_json::json!([]));
        assert_eq!(
            card(r#"{"title": "t"}"#).unwrap()["next"],
            serde_json::json!([])
        );
    }

    #[test]
    fn the_digest_is_told_who_settled_a_thread_and_when() {
        let card = serde_json::json!({ "next": [{ "who": "you", "step": "reboot into 6.18" }] });
        let done = serde_json::json!({ "state": "done", "by": "peter", "turns": 5 });
        assert_eq!(
            told(&done, Some(&card)),
            "Peter marked this conversation done when Peter had written 5 messages. Steps set aside then: reboot into 6.18.\n"
        );
        let open = serde_json::json!({ "state": "open", "by": "parker", "turns": 2, "note": "waiting on the sale" });
        assert_eq!(
            told(&open, None),
            "parker marked this conversation still open when Peter had written 2 messages (\"waiting on the sale\").\n"
        );
    }

    #[test]
    fn a_mark_is_named_by_a_thread_id_and_never_reaches_the_cli_otherwise() {
        assert!(mark_path("ses_ed9d3b3d4ffe1aDeWWLYKoJm2n").is_some());
        for bad in ["", "ses_", "../x", "ses_a/b", "--help", "ses_a --json"] {
            assert!(mark_path(bad).is_none(), "{bad:?} named a file");
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        for bad in ["--help", "ses_a b", "../engine.env"] {
            assert!(
                rt.block_on(mark(bad.into(), "done".into())).is_err(),
                "{bad:?}"
            );
        }
        let id = "ses_ed9d3b3d4ffe1aDeWWLYKoJm2n".to_string();
        for bad in ["finished", "--help", "done --json", ""] {
            assert!(
                rt.block_on(mark(id.clone(), bad.into())).is_err(),
                "{bad:?}"
            );
        }
    }

    fn client(id: i64, binary: &str) -> Value {
        serde_json::json!([{ "id": id, "type": "PipeWire:Interface:Client",
            "info": { "change-mask": ["props"], "props": { "application.process.binary": binary } } }])
    }

    fn params(id: i64) -> Value {
        serde_json::json!([{ "id": id, "type": "PipeWire:Interface:Node",
            "info": { "change-mask": ["params"], "params": { "Props": [] } } }])
    }

    fn gone(id: i64) -> Value {
        serde_json::json!([{ "id": id, "info": null }])
    }

    /// The batch shapes `pw-dump --monitor` printed on parker, Oct 2026,
    /// trimmed, in the orders it printed them: a wpctl connects, the level
    /// moves, the wpctl leaves — or, now and then, the wpctl leaves a
    /// moment before the level's word arrives.
    #[test]
    fn the_keys_are_a_wpctl_there_as_the_level_moves() {
        let t0 = std::time::Instant::now();
        let at = |ms| t0 + Duration::from_millis(ms);
        let keys = Seen {
            stir: true,
            wpctl: true,
        };
        let elsewhere = Seen {
            stir: true,
            wpctl: false,
        };
        let mut m = Monitor::default();
        m.nodes.insert(74);
        assert_eq!(m.read(&client(68, "wpctl"), at(0)), Seen::default());
        assert_eq!(m.read(&params(74), at(4)), keys);
        assert_eq!(
            m.read(&gone(68), at(6)),
            Seen::default(),
            "a client leaving"
        );
        assert_eq!(m.read(&client(68, "wpctl"), at(2000)), Seen::default());
        assert_eq!(m.read(&gone(68), at(2005)), Seen::default());
        assert_eq!(m.read(&params(74), at(2020)), keys, "just gone");
        // Nothing of wpctl's for a while, and something else moves it.
        assert_eq!(m.read(&client(71, "pw-cli"), at(5000)), Seen::default());
        assert_eq!(m.read(&params(74), at(5004)), elsewhere);
    }

    /// Only a node stirs a read. The watch's own reads are clients, and a
    /// read that stirred a read would be a loop.
    #[test]
    fn the_monitor_stirs_on_a_node_and_never_on_a_client() {
        let now = std::time::Instant::now();
        let mut m = Monitor::default();
        let opened = serde_json::json!([{ "id": 80, "type": "PipeWire:Interface:Node",
            "info": { "change-mask": ["state", "props", "params"] } }]);
        assert!(m.read(&opened, now).stir, "a stream opening");
        let state = serde_json::json!([{ "id": 80, "type": "PipeWire:Interface:Node",
            "info": { "change-mask": ["state"] } }]);
        assert_eq!(
            m.read(&state, now),
            Seen::default(),
            "a known node changing state"
        );
        assert_eq!(m.read(&client(90, "pw-dump"), now), Seen::default());
        assert_eq!(m.read(&gone(90), now), Seen::default());
        assert!(m.read(&gone(80), now).stir, "a stream closing");
        assert!(m.nodes.is_empty() && m.clients.is_empty());
    }

    fn level(label: &str, level: i64, muted: bool) -> Level {
        Level {
            label: label.into(),
            level,
            muted,
            sub: String::new(),
        }
    }

    #[test]
    fn levels_are_the_sound_rows_with_a_level() {
        let v = serde_json::json!({ "sections": [{ "id": "sound", "rows": [
            { "id": "output", "kind": "slider", "label": "all sound", "value": 42, "muted": false, "sub": "monitor speakers" },
            { "id": "apps", "kind": "group", "label": "apps", "value": 1, "children": [
                { "id": "app:spotify", "kind": "slider", "label": "spotify", "value": 45, "muted": true }
            ] },
            { "id": "mute", "kind": "toggle", "label": "mute", "value": false }
        ] }] });
        let l = levels_of(&v);
        assert_eq!(l.len(), 2);
        assert_eq!(l["output"].sub, "monitor speakers");
        assert_eq!(l["app:spotify"], level("spotify", 45, true));
    }

    #[test]
    fn what_parker_stamped_is_not_said() {
        let was = HashMap::from([
            ("output".to_string(), level("all sound", 42, false)),
            ("app:spotify".to_string(), level("spotify", 45, false)),
        ]);
        let now = HashMap::from([
            ("output".to_string(), level("all sound", 50, false)),
            ("app:spotify".to_string(), level("spotify", 20, true)),
        ]);
        let none = HashSet::new();
        // Nobody stamped anything: the keys, and spotify's own slider.
        let all = moved(&was, &now, &none, "", 1000.0);
        assert_eq!(all.len(), 3);
        assert_eq!(
            all[0],
            serde_json::json!({ "id": "app:spotify", "label": "spotify", "kind": "level", "from": 45, "to": 20 })
        );
        assert_eq!(all[1]["kind"], "mute");
        assert_eq!(
            (all[1]["from"].as_str(), all[1]["to"].as_str()),
            (Some("on"), Some("muted"))
        );
        assert_eq!(all[2]["id"], "output");
        // parker set spotify (level and mute) a second ago; the output was
        // stamped too, but to another level, and long ago.
        let hand = "999.000 app:spotify 20\n999.100 app:spotify muted\n900.000 output 50\n999.500 output 45\n";
        let left = moved(&was, &now, &none, hand, 1000.0);
        assert_eq!(left.len(), 1);
        assert_eq!(left[0]["id"], "output");
        // A stream that just opened, or a level on another device, is no move.
        let young = HashSet::from(["output".to_string(), "app:spotify".to_string()]);
        assert!(moved(&was, &now, &young, "", 1000.0).is_empty());
        let mut elsewhere = now.clone();
        elsewhere.get_mut("output").unwrap().sub = "headphones".into();
        assert_eq!(moved(&was, &elsewhere, &none, hand, 1000.0).len(), 0);
    }

    #[test]
    fn no_object_no_card() {
        assert!(card("I could not summarize this.").is_none());
        assert!(card("{\"state\": \"done\"}").is_none());
        assert!(card("} {").is_none());
    }
}
