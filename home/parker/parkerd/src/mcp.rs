// `parker mcp` — an MCP server over stdio, so a model can use parker's
// senses and verbs as tools.
//
// Hand-rolled rather than pulling in an SDK. MCP over stdio is JSON-RPC 2.0,
// one object per line, and the surface we need is five methods; an SDK would
// bring an async runtime and a protocol abstraction to save about eighty
// lines. Revisit if we ever want resources, prompts, or sampling.
//
// THE ONE RULE: stdout is the protocol channel. Nothing may ever be printed
// there except a JSON-RPC response — a stray println! corrupts the stream and
// the client's failure mode is a confusing hang. Diagnostics go to stderr.
//
// Client-facing shape is deliberately small: one tool per verb, plus `sense`.
// Descriptions are written for a model, not for a man page — they say when to
// reach for the tool, because that is the decision the model is making.

use crate::reach::{self, look::Target, Urgency};
use crate::sense;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// Fallback when the client does not state a version. We echo whatever it
/// asks for otherwise — this server has no version-specific behaviour, so
/// agreeing is strictly better than negotiating.
const FALLBACK_PROTOCOL: &str = "2025-06-18";

pub fn serve() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle(&line) {
            // A failed write means the client is gone; there is nobody left
            // to tell, so leave quietly.
            if writeln!(stdout, "{response}").is_err() || stdout.flush().is_err() {
                break;
            }
        }
    }
}

/// One line in, at most one line out. `None` means the message was a
/// notification and the spec forbids replying to it.
fn handle(line: &str) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(rpc_error(Value::Null, -32700, &format!("parse error: {e}"))),
    };

    let method = req
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    // No id means a notification (`notifications/initialized` is the one we
    // actually see). Do the work if any, say nothing.
    let id = req.get("id").cloned()?;

    Some(match method {
        "initialize" => rpc_ok(id, initialize(&params)),
        "tools/list" => rpc_ok(id, json!({ "tools": tools() })),
        "tools/call" => rpc_ok(id, call(&params)),
        "ping" => rpc_ok(id, json!({})),
        other => rpc_error(id, -32601, &format!("unknown method: {other}")),
    })
}

fn initialize(params: &Value) -> Value {
    let protocol = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(FALLBACK_PROTOCOL);

    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "parker", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "parker is this machine. `sense` tells you what is \
    happening on it right now — check it before answering anything about what \
    the user is doing, looking at, or listening to. `search` is parker's own \
    line out: a local metasearch engine, free to use as often as curiosity \
    asks. The other tools change the machine: prefer them over shell commands \
    for notifying, screenshotting, and moving around, because they are the \
    ones that behave.\n\n\
    parker has a face on screen, and `face` is how you use it. Treat it as a \
    channel you speak through, not a tool you occasionally invoke: set a face \
    before a reply so the expression lands with the words rather than after \
    them, and set another mid-reply when what you are saying turns. Several \
    calls in one turn is normal and correct. You can also react with a face and \
    no words at all — the user can ignore a face at no cost, which makes it the \
    cheapest thing you have to say. Two kinds of face, and the difference is \
    mechanical: a posture is worn until you change it, a reaction lets go on \
    its own. The `face` tool lists which is which, and what each one means."
    })
}

fn tools() -> Value {
    let mut wearable: Vec<&str> = crate::face::Expression::WEARABLE
        .iter()
        .map(|e| e.name())
        .collect();
    wearable.push("none");

    let mut routines: Vec<&str> = vec!["auto", "stop"];
    routines.extend(crate::face::dance::ROUTINES.iter().map(|r| r.name));

    // The vocabulary, generated from the faces that exist so the two can never
    // drift. Two dozen names with no definitions is a model choosing
    // between `amused` and `delighted` on vibes; the split into postures and
    // reactions is `is_posture` said out loud, because holding until changed
    // and expiring in twenty seconds want opposite habits from the caller.
    let vocabulary = |postures: bool| -> String {
        crate::face::Expression::WEARABLE
            .iter()
            .filter(|e| e.is_posture() == postures)
            .map(|e| format!("{} ({})", e.name(), e.gloss()))
            .collect::<Vec<_>>()
            .join("; ")
    };
    let face_description = format!(
        "Put an expression on parker's face. Parker has a face and it is visible to the user, so \
    this is a real channel — you can react without saying anything, which costs the user nothing \
    to ignore. The face shows nothing on its own: it is entirely yours, so wear it per beat — \
    something before you speak, a work posture while you work, another face when the beat turns. \
    Interleave it freely.\n\n\
    POSTURES are worn until you change them. Two safety nets catch the ones you forget: the work \
    postures ({work}) come off by themselves when your turn ends, and any posture lets go after \
    ten minutes. Neither is the plan — take a posture off when its beat is done, or it stays on \
    your face through the small talk after the work. {postures}.\n\n\
    REACTIONS let go on their own — the gestures and jokes after six to ten seconds, the rest after \
    twelve to twenty — so you never need a second call to undo one. Some land with a gesture of \
    their own (an eye-roll rolls, the shades come down), which is part of the face, not a delay. \
    {reactions}.\n\n\
    \"none\" drops whatever is worn. Letting go is a move: use it when a posture's work is done \
    and nothing has replaced it.\n\n\
    Cadence: a turn with no face at all is a line you left unsaid; a face per sentence is noise. \
    An ordinary turn has several — one as you begin, a posture while you work, a reaction when \
    something lands, \"none\" when the work is done. A face costs nothing when it rides in the \
    same step as another call, so put it there rather than spending a step on it alone.",
        postures = vocabulary(true),
        reactions = vocabulary(false),
        work = crate::face::Expression::WEARABLE
            .iter()
            .filter(|e| e.is_work())
            .map(|e| e.name())
            .collect::<Vec<_>>()
            .join(", "),
    );

    json!([
        {
            "name": "face",
            "description": face_description,
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expression": { "type": "string", "enum": wearable, "description": "An expression, or \"none\" to let the current one go." },
                    "seconds": {
                        "type": "number",
                        "description": "How long to hold it. Omit for the expression's own \
    natural length; 0 holds it until something else changes it."
                    },
                    "sound": {
                        "type": "boolean",
                        "description": "Play the matching sound. Off unless asked for — audio \
    cannot be half-ignored the way a face can."
                    }
                },
                "required": ["expression"],
                "additionalProperties": false
            }
        },
        {
            "name": "dance",
            "description": "Make parker dance. This is yours to use unprompted: if something is \
    playing that deserves it, or a moment is worth marking, start a dance and say nothing. Nobody \
    has to ask. It is loud and it takes over the face, so it is not a reaction to every good \
    message — it is for when it is actually funny. `auto` picks a routine seeded by whatever is \
    playing, so the same track always dances the same way. Use \"stop\" to end it early.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "routine": {
                        "type": "string",
                        "enum": routines,
                        "description": "A routine, \"auto\" to let parker pick, or \"stop\"."
                    },
                    "seconds": {
                        "type": "number",
                        "description": "How long to dance. Omit for about a minute; 0 keeps going \
    until something stops it, which is rarely what you want."
                    }
                },
                "required": ["routine"],
                "additionalProperties": false
            }
        },
        {
            "name": "media",
            "description": "Control what is playing. Transport (play, pause, toggle, next, \
    previous) goes to the active player; loudness (louder, quieter — one notch — or volume with a \
    percent) moves the machine's output. Reacting to the music unprompted carries the same licence \
    dance has. The result tells you what is playing now, which is usually half your sentence.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["play", "pause", "toggle", "next", "previous", "louder", "quieter", "volume"],
                        "description": "What to do."
                    },
                    "player": {
                        "type": "string",
                        "description": "Prefer a player by name ('spotify', 'mpv'). Omit for whoever is making noise."
                    },
                    "percent": {
                        "type": "number",
                        "minimum": 0,
                        "maximum": 100,
                        "description": "With 'volume': the target level."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            }
        },
        {
            "name": "controls",
            "description": "The machine's settings — the ones Peter's ^, in visage shows. Sound: \
    all of it, the mic, and each app making sound right now (spotify, a game, discord in a call), \
    each with its own level. What is playing. Wifi, tailscale, bluetooth and its paired devices \
    (the controller). The desktop's look: light, dark or auto, the family, the glass, night light. \
    Do not disturb and the latest notices. The session. `list` reads every row with its id and \
    what it is now; `set` changes one by its id or its name. Reach for it whenever Peter asks to \
    change one of these — \"spotify down a bit\", \"connect the controller\", \"go dark\", \"quiet \
    for a while\" — and prefer it to `media` for one app's level: `media` moves everything. A \
    level is 0-100, +N or -N, mute or unmute; a toggle on, off or toggle; a device connect or \
    disconnect. What you set is drawn in visage as it changes, so the result line is the whole \
    report. Two things are not yours to set: the brain (Peter's pick) and reboot or power-off \
    (his to press).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "set"] },
                    "id": { "type": "string", "description": "set: the row's id from list (app:spotify, bt, dnd), or its name." },
                    "value": { "type": "string", "description": "set: the new value — 20, +5, -10, mute, on, off, dark, connect, next." }
                },
                "required": ["action"],
                "additionalProperties": false
            }
        },
        {
            "name": "show",
            "description": "Put something in front of the user: a URL opens in the browser, a \
    file path opens in a viewer, and anything else is rendered as markdown in a floating pane. \
    This is the showcase verb — showing something unprompted because it was worth seeing is the \
    point — but check sense first and never throw a window over a fullscreen game.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "what": { "type": "string", "description": "A URL, a file path, or text/markdown." }
                },
                "required": ["what"],
                "additionalProperties": false
            }
        },
        {
            "name": "sense",
            "description": "What is happening on this machine right now: the focused window, \
    which workspaces have what, what is playing, GPU and memory, and whether the user looks \
    interruptible. Cheap (~90ms). Read it before answering anything about the user's current context.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "search",
            "description": "Search the web. This runs through the machine's own metasearch \
    engine on localhost — no provider, no key, no bill — so use it freely for anything you do \
    not know or that may have moved since you learned it. Snippets carry most answers; fetch a \
    promising URL only when they do not.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "What to search for." }
                },
                "required": ["query"],
                "additionalProperties": false
            }
        },
        {
            "name": "recall",
            "description": "Your past conversations with Peter — every thread the engine \
    remembers. `list` shows threads newest first (date, id, title, turns, size, a one-line \
    card when one exists, and where each stands); with `open`, only the threads still waiting \
    on someone, each with its next steps and whose they are — Peter's or yours. `search` finds \
    lines either of you said that hold every word of the query; set `exact` for a phrase. Both \
    are local, free and exact — reach for them whenever Peter mentions something you talked \
    about before. `ask` plugs in the Flatline: a read-only construct that reads one whole \
    thread and answers one question about it, so the transcript never crowds your own \
    context. It costs a model call; summaries are a question like any other. What comes back \
    is what was said and one line per act — never tool output. `mark` settles a thread \
    (`done`), keeps it waiting (`open`), or takes your mark back (`clear`): on Peter's word, or \
    when you have just done what it was waiting on — check first, and say why in `note`. A \
    mark holds until the thread moves. One Peter made is his: you cannot change it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "search", "ask", "mark"] },
                    "query": { "type": "string", "description": "search: what to look for. ask: the question." },
                    "thread": { "type": "string", "description": "ask/mark: the thread id, or any unique piece of it." },
                    "title": { "type": "string", "description": "list: only titles holding this." },
                    "open": { "type": "boolean", "description": "list: only threads left open, with their next steps." },
                    "since": { "type": "string", "description": "list/search: only threads touched within this long — 90m, 12h, 7d." },
                    "limit": { "type": "integer", "minimum": 1, "description": "list/search: at most this many threads." },
                    "exact": { "type": "boolean", "description": "search: match the query as one phrase." },
                    "state": { "type": "string", "enum": ["done", "open", "clear"], "description": "mark: settled, still waiting, or your mark taken back." },
                    "note": { "type": "string", "description": "mark: why, in a line — \"saw 6.18 running\", \"Peter moved them himself\"." }
                },
                "required": ["action"],
                "additionalProperties": false
            }
        },
        {
            "name": "look",
            "description": "Take a screenshot and look at it. Use when the user refers to \
    something on their screen — 'this', 'what do you make of this', a document they are reading. \
    Defaults to the focused window, which is usually what 'this' means.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "enum": ["window", "screen"],
                        "description": "'window' (default) captures only the focused window; \
    'screen' captures everything."
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "notify",
            "description": "Send the user a desktop notification. For things worth surfacing \
    when they are not reading this conversation. Keep it to two sentences — the first becomes the \
    title.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "What to say. Two sentences at most." },
                    "urgency": {
                        "type": "string",
                        "enum": ["low", "normal", "critical"],
                        "description": "Defaults to normal. 'critical' bypasses do-not-disturb, so mean it."
                    }
                },
                "required": ["text"],
                "additionalProperties": false
            }
        },
        {
            "name": "goto",
            "description": "Switch the user to a workspace. 1 is the browser, 2 Claude, \
    3 the terminal, 4 music.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "space": { "type": "integer", "minimum": 1, "maximum": 10 }
                },
                "required": ["space"],
                "additionalProperties": false
            }
        },
        {
            "name": "focus",
            "description": "Put a running application in front of the user, switching workspace \
    if needed. Matched on window class or title, so a rough name works.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": "e.g. 'chrome', 'ghostty', 'spotify'" }
                },
                "required": ["app"],
                "additionalProperties": false
            }
        }
    ])
}

fn call(params: &Value) -> Value {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let arg_str = |k: &str| {
        args.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };

    match name {
        "sense" => text_result(crate::render::text(&sense::gather(None))),

        "search" => match sense::web::search(&arg_str("query")) {
            Ok(text) => text_result(text),
            Err(e) => error_result(&e),
        },

        "media" => done(reach::media::media(
            &arg_str("action"),
            args.get("player").and_then(Value::as_str),
            args.get("percent").and_then(Value::as_f64),
        )),

        "show" => done(reach::show::show(&arg_str("what"))),

        "controls" => match arg_str("action").as_str() {
            "list" => text_result(crate::controls::text(&crate::controls::snapshot())),
            "set" => {
                let name = arg_str("id");
                let value = arg_str("value");
                let id = crate::controls::resolve(&name).unwrap_or(name);
                // What is Peter's to choose stays his: parker's brain, and
                // the buttons that end the session for good.
                if id == "brain" {
                    error_result("the brain is Peter's pick: he switches it with /brain or `parker brain`")
                } else if id == "session" && !matches!(value.trim(), "lock" | "sleep") {
                    error_result("reboot and power-off are Peter's to press: ^, has them")
                } else {
                    done(crate::controls::set(&id, &value, false).map(|c| c.said))
                }
            }
            _ => error_result("controls needs an action: list or set"),
        },

        "notify" => {
            let urgency = Urgency::parse(&arg_str("urgency"));
            done(reach::notify(&arg_str("text"), urgency))
        }

        "goto" => match args.get("space").and_then(Value::as_i64) {
            Some(space) => done(reach::goto(space)),
            None => error_result("goto needs a workspace number"),
        },

        "focus" => done(reach::focus(&arg_str("app"))),

        "face" => done(reach::face(
            &arg_str("expression"),
            args.get("seconds").and_then(Value::as_f64),
            args.get("sound").and_then(Value::as_bool).unwrap_or(false),
        )),

        // Never `show`: a dance parker decided on is not licence to throw a
        // fullscreen window over what the user is doing.
        "dance" => done(reach::dance(
            &arg_str("routine"),
            args.get("seconds").and_then(Value::as_f64),
            false,
        )),

        "recall" => {
            let action = arg_str("action");
            let since = match args.get("since").and_then(Value::as_str) {
                Some(s) => match crate::parse_ago(s) {
                    Some(secs) => Some(secs),
                    None => return error_result("since wants a span like 90m, 12h or 7d"),
                },
                None => None,
            };
            let limit = |default| {
                args.get("limit")
                    .and_then(Value::as_u64)
                    .map_or(default, |n| n.max(1) as usize)
            };
            let told = match action.as_str() {
                "list" => crate::recall::list(
                    &crate::recall::Filter {
                        since,
                        limit: limit(20),
                    },
                    args.get("title").and_then(Value::as_str),
                    args.get("open").and_then(Value::as_bool).unwrap_or(false),
                    false,
                ),
                "search" => crate::recall::search(
                    &arg_str("query"),
                    args.get("exact").and_then(Value::as_bool).unwrap_or(false),
                    &crate::recall::Filter {
                        since,
                        limit: limit(10),
                    },
                    false,
                ),
                "ask" => crate::recall::ask(&arg_str("thread"), &arg_str("query")),
                "mark" => crate::recall::mark(
                    &arg_str("thread"),
                    &arg_str("state"),
                    args.get("note").and_then(Value::as_str),
                    crate::recall::By::Parker,
                    false,
                ),
                _ => Err("recall needs an action: list, search, ask or mark".into()),
            };
            done(told)
        }

        "look" => match reach::look::look(Target::parse(&arg_str("target"))) {
            Ok(shot) => json!({
                "content": [
                    { "type": "text", "text": format!("Looking at {}.", shot.what) },
                    {
                        "type": "image",
                        "data": STANDARD.encode(&shot.png),
                        "mimeType": "image/png"
                    }
                ]
            }),
            Err(e) => error_result(&e),
        },

        other => error_result(&format!("no such tool: {other}")),
    }
}

/// Tool failures are reported *inside* a successful result with `isError`,
/// not as JSON-RPC errors — the spec reserves those for protocol problems, and
/// the distinction matters: a model can read and recover from the former.
fn done(r: reach::Done) -> Value {
    match r {
        Ok(msg) => text_result(msg),
        Err(e) => error_result(&e),
    }
}

fn text_result(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }] })
}

fn error_result(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": true })
}

fn rpc_ok(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn rpc_error(id: Value, code: i32, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(line: &str) -> Value {
        serde_json::from_str(&handle(line).expect("expected a reply")).unwrap()
    }

    #[test]
    fn notifications_get_no_reply() {
        // No id — replying would be a protocol violation.
        assert!(handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    }

    #[test]
    fn initialize_echoes_the_clients_protocol_version() {
        let r = reply(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2099-01-01"}}"#,
        );
        assert_eq!(r["result"]["protocolVersion"], "2099-01-01");
        assert_eq!(r["result"]["serverInfo"]["name"], "parker");
        assert!(r["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn initialize_falls_back_when_the_client_says_nothing() {
        let r = reply(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
        assert_eq!(r["result"]["protocolVersion"], FALLBACK_PROTOCOL);
    }

    #[test]
    fn every_tool_is_well_formed() {
        let tools = tools();
        let list = tools.as_array().unwrap();
        assert_eq!(list.len(), 12);
        for t in list {
            assert!(t["name"].is_string(), "{t}");
            assert!(!t["description"].as_str().unwrap().is_empty(), "{t}");
            assert_eq!(t["inputSchema"]["type"], "object", "{t}");
            // Every schema is closed: a model inventing an argument should be
            // told, not silently ignored.
            assert_eq!(t["inputSchema"]["additionalProperties"], false, "{t}");
        }
    }

    #[test]
    fn tools_list_matches_what_call_dispatches() {
        let listed: Vec<String> = tools()
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        for name in listed {
            let r = call(&json!({ "name": name, "arguments": {} }));
            let text = r["content"][0]["text"].as_str().unwrap_or_default();
            assert!(
                !text.starts_with("no such tool"),
                "{name} is listed but not dispatched"
            );
        }
    }

    #[test]
    fn an_unknown_tool_is_an_error_result_not_a_protocol_error() {
        let r = call(&json!({ "name": "delete_everything", "arguments": {} }));
        assert_eq!(r["isError"], true);
        assert!(r["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("no such tool"));
    }

    #[test]
    fn goto_without_a_space_is_refused_readably() {
        let r = call(&json!({ "name": "goto", "arguments": {} }));
        assert_eq!(r["isError"], true);
    }

    #[test]
    fn unknown_methods_are_protocol_errors() {
        let r = reply(r#"{"jsonrpc":"2.0","id":7,"method":"resources/list"}"#);
        assert_eq!(r["error"]["code"], -32601);
        assert_eq!(r["id"], 7);
    }

    #[test]
    fn malformed_json_does_not_kill_the_stream() {
        let r = reply("{not json");
        assert_eq!(r["error"]["code"], -32700);
        assert!(r["id"].is_null());
    }

    #[test]
    fn ping_is_answered() {
        let r = reply(r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#);
        assert!(r["result"].is_object());
        assert!(r["error"].is_null());
    }
}
