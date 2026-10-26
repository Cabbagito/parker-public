// porthole — parker's face for the tailnet: giskard, the phone.
//
// The second frame on ../door. The Tauri window turns door's calls into
// tauri commands for parker's own screen; this turns the same calls into
// HTTP, so a browser anywhere on the tailnet gets the same face and the same
// conversation. It adds four things and has no opinions beyond them: the
// gate, `POST /api/<command>`, one event stream fanned out to every page,
// and the page's files (docs/visage/porthole/README.md).
//
// What reaches this can run bash as peki — the engine allows every tool — so
// the bar is "as strong as `ssh parker`", and the gate below is what holds it.

use axum::{
    extract::{Path, Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{self, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use futures_util::{stream, Stream, StreamExt};
use serde_json::Value;
use std::{
    collections::HashMap,
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    path::{Path as FsPath, PathBuf},
    process::ExitCode,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{broadcast, watch};
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};
use visage_door::{Engine, Event};

#[derive(Parser)]
#[command(name = "porthole", about = "parker's face, served to the tailnet")]
struct Args {
    /// Where to listen. Loopback only: `tailscale serve` is the way in.
    #[arg(long, default_value = "127.0.0.1:4097")]
    listen: SocketAddr,
    /// The built page — the same dist the Tauri app embeds.
    #[arg(long)]
    dist: PathBuf,
    /// A Tailscale login let in. Repeatable; nobody else is.
    #[arg(long = "allow", required = true)]
    allow: Vec<String>,
    /// The page's own origin (https://parker.<tailnet>.ts.net) — the only
    /// Origin a write may carry.
    #[arg(long)]
    origin: String,
}

/// Who may in. Kept pure so the tests can hold it to account.
///
/// Every request — the page's files included — must carry a
/// `Tailscale-User-Login` naming an allowed login. `tailscale serve` adds that
/// header and strips any copy the client sent (Tailscale KB 1312), and tagged
/// devices get none. The header is only worth anything because the listener
/// is loopback-only (`loopback`): the one way to this port from another
/// machine is through serve. A request that reaches it any other way — a
/// page open in parker's own browser, say — carries no header and gets 403.
///
/// Writes also need `Origin` equal to the page's own and a JSON body. A page
/// from another site in giskard's browser rides the same tailnet and would
/// get the same identity header; it cannot forge its Origin, and it cannot
/// send JSON without a preflight that this gate refuses, because a preflight
/// carries the foreign Origin too.
struct Gate {
    allow: Vec<String>,
    origin: String,
}

impl Gate {
    fn admit(&self, method: &Method, headers: &HeaderMap) -> Result<(), &'static str> {
        let text = |name| {
            headers
                .get(name)
                .and_then(|v: &HeaderValue| v.to_str().ok())
        };
        match text("tailscale-user-login") {
            Some(who) if self.allow.iter().any(|a| a == who) => {}
            Some(_) => return Err("not a login visage lets in"),
            None => {
                return Err("no Tailscale identity — visage is reached through tailscale serve")
            }
        }
        if method == Method::GET || method == Method::HEAD {
            return Ok(());
        }
        if text("origin") != Some(self.origin.as_str()) {
            return Err("a write from another origin");
        }
        let json = text("content-type")
            .and_then(|ct| ct.split(';').next())
            .is_some_and(|ct| ct.trim().eq_ignore_ascii_case("application/json"));
        if !json {
            return Err("writes are JSON");
        }
        Ok(())
    }
}

/// The header above is trustworthy only if serve is the sole way in. Bound
/// to anything but loopback, any LAN or tailnet client could hand this server
/// its own `Tailscale-User-Login` — so that is not a configuration, it is a
/// refusal.
fn loopback(addr: &SocketAddr) -> Result<(), String> {
    if addr.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!(
            "{addr} is not loopback — the identity header is only trustworthy behind tailscale serve"
        ))
    }
}

async fn gate(State(gate): State<Arc<Gate>>, req: Request, next: Next) -> Response {
    match gate.admit(req.method(), req.headers()) {
        Ok(()) => next.run(req).await,
        Err(why) => {
            eprintln!(
                "porthole: refused {} {}: {why}",
                req.method(),
                req.uri().path()
            );
            (StatusCode::FORBIDDEN, why).into_response()
        }
    }
}

/// What every page shares: one engine, one upstream event stream, one face,
/// and what is known about where each page is.
#[derive(Clone)]
struct Door {
    engine: Engine,
    events: broadcast::Sender<Value>,
    status: Arc<watch::Sender<&'static str>>,
    face: Arc<watch::Sender<Value>>,
    spaces: Arc<watch::Sender<Value>>,
    /// Levels moved by something other than parker or ^, (door's
    /// `watch_levels`). Not a watch like the face and the spaces: those are
    /// how things are, and a page that connects should start from them;
    /// this is something that happened, and a page that connects later
    /// should not hear it as news.
    levels: broadcast::Sender<Value>,
    places: Places,
}

impl Door {
    fn new(engine: Engine) -> Self {
        Door {
            engine,
            events: broadcast::channel(1024).0,
            // Not `disconnected`: that is the status strip's word, and a
            // page arriving before the first upstream connect should not
            // flash it.
            status: Arc::new(watch::channel("connecting").0),
            face: Arc::new(watch::channel(Value::Null).0),
            spaces: Arc::new(watch::channel(Value::Null).0),
            levels: broadcast::channel(64).0,
            places: Places::default(),
        }
    }

    /// Tail the engine once for every page. Status is deduped here — door's
    /// pump repeats `disconnected` on every 2 s retry, and each page only
    /// needs to hear about a change.
    fn pump(&self) {
        let door = self.clone();
        tokio::spawn(async move {
            door.engine
                .pump(|event| match event {
                    Event::Status(s) => {
                        door.status.send_if_modified(|cur| replace(cur, s));
                    }
                    Event::Engine(v) => {
                        let _ = door.events.send(v);
                    }
                })
                .await
        });
    }

    /// Read the face here, on parker, and push it on change. The page polls
    /// `face_state` every 250 ms; over a network that poll would be a request
    /// four times a second from every page for a value that changes a few
    /// times a minute. The page's poll is answered from the last push instead
    /// (src/lib/shell.js). Deadlines are judged by parker's clock either way.
    fn watch_face(&self) {
        let face = self.face.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(100));
            loop {
                tick.tick().await;
                if let Ok(now) = tokio::task::spawn_blocking(visage_door::face_state).await {
                    face.send_if_modified(|cur| replace(cur, now));
                }
            }
        });
    }

    /// The corner's spaces, pushed as Hyprland moves them (door's
    /// `watch_spaces`, on the event socket), the way the face is: a page
    /// asks once and hears every change, rather than polling over the
    /// network for a number that moves when Peter presses SUPER+2.
    fn watch_spaces(&self) {
        let spaces = self.spaces.clone();
        visage_door::watch_spaces(move |v| {
            spaces.send_replace(v);
        });
    }

    /// The keys on the desk moving a level, pushed to every page as it
    /// happens (door's `watch_levels`), for the transcript's quiet line.
    fn watch_levels(&self) {
        let levels = self.levels.clone();
        visage_door::watch_levels(move |v| {
            let _ = levels.send(v);
        });
    }
}

/// Where a page is, as the sentence parker reads with its messages (door's
/// `away`), keyed by the page's tailnet address.
///
/// serve sets `X-Forwarded-For` to the peer's tailnet address itself and
/// replaces any copy the client sent (tailscale ipn/ipnlocal/serve.go,
/// `addProxyForwardedHeaders`). So the address is as trustworthy as the
/// identity header, and for the same reason: serve is the only way in.
/// `tailscale whois` turns it into the machine's name and needs no root.
/// Answers are kept for the life of the process. A device is renamed about
/// never, and a fork per message would be a fork too many.
#[derive(Clone, Default)]
struct Places(Arc<Mutex<HashMap<IpAddr, String>>>);

impl Places {
    async fn of(&self, from: Option<IpAddr>) -> Option<String> {
        let ip = from?;
        let known = self.0.lock().ok().and_then(|m| m.get(&ip).cloned());
        if known.is_some() {
            return known;
        }
        let named = tokio::task::spawn_blocking(move || whois(ip))
            .await
            .ok()
            .flatten();
        // Unnamed is still away: the address came through serve, so it is
        // a tailnet device, just not one whois would describe. Not cached,
        // so the next message asks again.
        let line = visage_door::away(named.as_deref().unwrap_or("a device on the tailnet"));
        if let Some(device) = named {
            eprintln!("porthole: {ip} is {device}");
            if let Ok(mut m) = self.0.lock() {
                m.insert(ip, line.clone());
            }
        }
        Some(line)
    }
}

/// The page's tailnet address, as serve forwarded it. Absent when a request
/// did not come through serve — a local curl, say — and then nothing is
/// said about where Peter is rather than something wrong.
fn forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("x-forwarded-for")?
        .to_str()
        .ok()?
        .split(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

fn whois(ip: IpAddr) -> Option<String> {
    let out = std::process::Command::new("tailscale")
        .args(["whois", "--json", &ip.to_string()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    device(&serde_json::from_slice(&out.stdout).ok()?)
}

/// `giskard (macOS)`, from a `tailscale whois --json` answer.
fn device(who: &Value) -> Option<String> {
    let name = who["Node"]["ComputedName"]
        .as_str()
        .filter(|s| !s.is_empty())?;
    Some(
        match who["Node"]["Hostinfo"]["OS"]
            .as_str()
            .filter(|s| !s.is_empty())
        {
            Some(os) => format!("{name} ({os})"),
            None => name.to_string(),
        },
    )
}

fn replace<T: PartialEq>(cur: &mut T, new: T) -> bool {
    if *cur == new {
        false
    } else {
        *cur = new;
        true
    }
}

/// The page's one stream: `engine-status`, `face`, `spaces`, `levels` and every
/// `engine-event`, named as the Tauri events are so the page cannot tell which
/// frame it is in. Status and face arrive first with their current values, so a
/// page that just loaded — or just woke up — starts from the truth; `levels`
/// only as they happen.
async fn events(
    State(door): State<Door>,
) -> Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
    let push =
        |name: &'static str, v: &Value| Ok(sse::Event::default().event(name).data(v.to_string()));
    let status = WatchStream::new(door.status.subscribe())
        .map(move |s| push("engine-status", &Value::from(s)))
        .boxed();
    let face = WatchStream::new(door.face.subscribe())
        .map(move |v| push("face", &v))
        .boxed();
    let spaces = WatchStream::new(door.spaces.subscribe())
        .map(move |v| push("spaces", &v))
        .boxed();
    // A page that falls behind skips what it missed: the transcript refetches
    // on the next event that names its session, so nothing is lost for long.
    let engine = BroadcastStream::new(door.events.subscribe())
        .filter_map(move |r| async move { r.ok().map(|v| push("engine-event", &v)) })
        .boxed();
    let levels = BroadcastStream::new(door.levels.subscribe())
        .filter_map(move |r| async move { r.ok().map(|v| push("levels", &v)) })
        .boxed();
    Sse::new(stream::select_all([status, face, spaces, levels, engine]))
        .keep_alive(KeepAlive::default())
}

/// One door call. Detached on purpose: `send_message` resolves when the turn
/// ends, which can be minutes, and a phone that sleeps drops the request. A
/// client that hangs up abandons the answer — never the turn.
async fn call(
    State(door): State<Door>,
    Path(cmd): Path<String>,
    headers: HeaderMap,
    Json(args): Json<Value>,
) -> Response {
    let from = forwarded_for(&headers);
    match tokio::spawn(async move { dispatch(&door, &cmd, &args, from).await }).await {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err((code, msg))) => (code, msg).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

type Failure = (StatusCode, String);

/// The tauri command table, by the same names and the same camelCase
/// arguments the page already sends through `invoke`. `from` is the page's
/// tailnet address, for the one command that says where Peter is.
async fn dispatch(
    door: &Door,
    cmd: &str,
    args: &Value,
    from: Option<IpAddr>,
) -> Result<Value, Failure> {
    let engine = &door.engine;
    let up = |r: Result<Value, String>| r.map_err(|e| (StatusCode::BAD_GATEWAY, e));
    match cmd {
        "create_session" => up(engine.create_session().await),
        "get_messages" => up(engine.get_messages(arg(args, "sessionId")?).await),
        "send_message" => {
            let place = door.places.of(from).await;
            up(engine
                .send_message(
                    arg(args, "sessionId")?,
                    arg(args, "text")?,
                    args.get("model").and_then(Value::as_str),
                    place.as_deref(),
                )
                .await)
        }
        "list_sessions" => up(engine.list_sessions().await),
        "interrupt_session" => up(engine.interrupt_session(arg(args, "sessionId")?).await),
        "summarize" => up(engine.summarize(arg(args, "text")?).await.map(Value::from)),
        "digests" | "marks" => {
            let ids = args
                .get("ids")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing `ids`".to_string()))?;
            let read = if cmd == "marks" {
                visage_door::marks
            } else {
                visage_door::digests
            };
            Ok(tokio::task::spawn_blocking(move || read(&ids))
                .await
                .unwrap_or(Value::Null))
        }
        // door checks the id and the word before anything reaches the CLI.
        "mark" => up(visage_door::mark(
            arg(args, "sessionId")?.to_string(),
            arg(args, "state")?.to_string(),
        )
        .await),
        "digest" => up(engine
            .digest(
                arg(args, "sessionId")?,
                arg(args, "text")?,
                args.get("facts").cloned().unwrap_or(Value::Null),
            )
            .await),
        "face_state" => Ok(tokio::task::spawn_blocking(visage_door::face_state)
            .await
            .unwrap_or(Value::Null)),
        "dance" => {
            visage_door::dance(
                arg(args, "routine")?.to_string(),
                args.get("seconds").and_then(Value::as_f64),
            );
            Ok(Value::Null)
        }
        "sense_snapshot" => Ok(visage_door::sense_snapshot().await),
        "controls" => Ok(visage_door::controls().await),
        // Always away: a page here is not at the tower, so parkerd refuses
        // what would cut the line it rides on (wifi off, tailscale, power
        // off). door checks the id and value before anything is spawned.
        "set_control" => up(visage_door::set_control(
            arg(args, "id")?.to_string(),
            arg(args, "value")?.to_string(),
            true,
        )
        .await),
        "spaces" => Ok(door.spaces.borrow().clone()),
        // The allowlist is door's, so a browser gets exactly the forms the
        // tower's palette gets and nothing more.
        "theme" => {
            let words = args
                .get("args")
                .and_then(Value::as_array)
                .and_then(|a| {
                    a.iter()
                        .map(|w| w.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()
                })
                .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing `args`".to_string()))?;
            up(visage_door::theme(words).await.map(Value::from))
        }
        // Same rule: door switches only to a brain its set already lists.
        "brain" => match args.get("name").and_then(Value::as_str) {
            Some(name) => up(visage_door::set_brain(name.to_string()).await),
            None => Ok(visage_door::brain().await),
        },
        // `dismiss` is absent on purpose: it flips a Hyprland workspace on
        // parker's screen, and someone on the phone has nothing to put away.
        // The page's web shell answers it locally.
        _ => Err((
            StatusCode::NOT_FOUND,
            format!("visage has no command `{cmd}`"),
        )),
    }
}

fn arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, Failure> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("missing `{key}`")))
}

/// What the browser may do with a page that can reach a shell. The CSP is
/// the second wall behind DOMPurify (visage/AGENTS.md): no script but the
/// page's own, no requests but to itself, and no images from elsewhere —
/// which also means a reply quoting the web cannot load a tracking pixel or
/// carry data out in an image URL. No framing, so no clickjacking.
const CSP: &str = "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; \
                   frame-ancestors 'none'; base-uri 'none'; form-action 'none'";

fn router(door: Door, gate_: Gate, dist: &FsPath) -> Router {
    // Vite names everything under assets/ by content hash, so it can be kept
    // forever. Nothing else may be cached at all: every file in the nix
    // store is dated 1970, so a browser revalidating index.html with
    // If-Modified-Since would hear "not modified" after every deploy and keep
    // loading assets that no longer exist.
    let assets = Router::new()
        .nest_service("/assets", ServeDir::new(dist.join("assets")))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ));
    Router::new()
        .route("/api/events", get(events))
        .route("/api/{cmd}", post(call))
        .merge(assets)
        .fallback_service(ServeDir::new(dist))
        .with_state(door)
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        // Outermost, so nothing above runs for a request it refuses.
        .layer(middleware::from_fn_with_state(Arc::new(gate_), gate))
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    if let Err(e) = loopback(&args.listen) {
        eprintln!("porthole: {e}");
        return ExitCode::FAILURE;
    }
    let engine = match Engine::from_env() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("porthole: {e}");
            return ExitCode::FAILURE;
        }
    };
    let door = Door::new(engine);
    door.pump();
    door.watch_face();
    door.watch_spaces();
    door.watch_levels();

    let gate_ = Gate {
        allow: args.allow,
        origin: args.origin,
    };
    let app = router(door, gate_, &args.dist);
    let listener = match tokio::net::TcpListener::bind(args.listen).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("porthole: cannot listen on {}: {e}", args.listen);
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "porthole: serving {} on {}",
        args.dist.display(),
        args.listen
    );
    match axum::serve(listener, app).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("porthole: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    const ME: &str = "peki@example.com";
    const ORIGIN: &str = "https://parker.example.ts.net";

    fn gate() -> Gate {
        Gate {
            allow: vec![ME.into()],
            origin: ORIGIN.into(),
        }
    }

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_static(v));
        }
        h
    }

    #[test]
    fn reads_need_an_allowed_identity() {
        let g = gate();
        assert!(g.admit(&Method::GET, &headers(&[])).is_err());
        assert!(g
            .admit(
                &Method::GET,
                &headers(&[("tailscale-user-login", "someone@else.com")])
            )
            .is_err());
        assert!(g
            .admit(&Method::GET, &headers(&[("tailscale-user-login", ME)]))
            .is_ok());
    }

    #[test]
    fn writes_need_our_origin_and_json() {
        let g = gate();
        let me = ("tailscale-user-login", ME);
        let json = ("content-type", "application/json");
        let ours = ("origin", ORIGIN);
        assert!(
            g.admit(&Method::POST, &headers(&[me, json])).is_err(),
            "no origin"
        );
        assert!(g
            .admit(
                &Method::POST,
                &headers(&[me, json, ("origin", "https://evil.example")])
            )
            .is_err());
        assert!(g
            .admit(
                &Method::POST,
                &headers(&[me, ours, ("content-type", "text/plain")])
            )
            .is_err());
        assert!(
            g.admit(&Method::POST, &headers(&[me, ours])).is_err(),
            "no content type"
        );
        assert!(g.admit(&Method::POST, &headers(&[me, ours, json])).is_ok());
        assert!(g
            .admit(
                &Method::POST,
                &headers(&[
                    me,
                    ours,
                    ("content-type", "application/json; charset=utf-8")
                ])
            )
            .is_ok());
        // A preflight from another site carries its own Origin.
        assert!(g
            .admit(
                &Method::OPTIONS,
                &headers(&[me, ("origin", "https://evil.example")])
            )
            .is_err());
    }

    #[test]
    fn only_loopback_listens() {
        assert!(loopback(&"127.0.0.1:4097".parse().unwrap()).is_ok());
        assert!(loopback(&"[::1]:4097".parse().unwrap()).is_ok());
        assert!(loopback(&"0.0.0.0:4097".parse().unwrap()).is_err());
        assert!(loopback(&"100.74.88.66:4097".parse().unwrap()).is_err());
    }

    /// The gate is only a gate if it is wired in front of everything — a
    /// layer added in the wrong place leaves the shell open with every unit
    /// test above still green. So ask the assembled router.
    #[tokio::test]
    async fn the_router_is_gated() {
        let dist = std::env::temp_dir().join(format!("porthole-test-{}", std::process::id()));
        std::fs::create_dir_all(&dist).unwrap();
        std::fs::write(dist.join("index.html"), "<p>parker</p>").unwrap();
        let app = router(Door::new(Engine::new("x".into())), gate(), &dist);

        let send = |req: Request<Body>| app.clone().oneshot(req);
        let page = |login: Option<&'static str>| {
            let mut r = Request::get("/");
            if let Some(l) = login {
                r = r.header("tailscale-user-login", l);
            }
            r.body(Body::empty()).unwrap()
        };

        assert_eq!(
            send(page(None)).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        let ok = send(page(Some(ME))).await.unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.headers()[header::CACHE_CONTROL], "no-store");
        assert!(ok.headers().contains_key(header::CONTENT_SECURITY_POLICY));

        let events = Request::get("/api/events").body(Body::empty()).unwrap();
        assert_eq!(send(events).await.unwrap().status(), StatusCode::FORBIDDEN);

        let forged = Request::post("/api/dance")
            .header("tailscale-user-login", ME)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"routine":"stop"}"#))
            .unwrap();
        assert_eq!(send(forged).await.unwrap().status(), StatusCode::FORBIDDEN);

        let unknown = Request::post("/api/dismiss")
            .header("tailscale-user-login", ME)
            .header("origin", ORIGIN)
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        assert_eq!(send(unknown).await.unwrap().status(), StatusCode::NOT_FOUND);

        let _ = std::fs::remove_dir_all(&dist);
    }

    /// `theme` reaches a CLI, so the browser must meet door's allowlist, not
    /// a pass-through. Refused before anything is spawned — this runs nothing.
    #[tokio::test]
    async fn theme_is_allowlisted_over_http_too() {
        let door = Door::new(Engine::new("x".into()));
        for words in [
            serde_json::json!(["set", "../../etc"]),
            serde_json::json!(["light", "--now"]),
            serde_json::json!(["rebuild"]),
        ] {
            let refused =
                dispatch(&door, "theme", &serde_json::json!({ "args": words }), None).await;
            assert!(refused.is_err(), "theme {words} got through");
        }
        let malformed = dispatch(
            &door,
            "theme",
            &serde_json::json!({ "args": "light" }),
            None,
        )
        .await;
        assert_eq!(malformed.unwrap_err().0, StatusCode::BAD_REQUEST);
    }

    /// `brain` reaches a CLI too, so a browser gets a listed brain or
    /// nothing. Without parker on PATH (the build sandbox) there is no set
    /// and every name is refused; on parker these are refused by name.
    #[tokio::test]
    async fn brain_is_allowlisted_over_http_too() {
        let door = Door::new(Engine::new("x".into()));
        for name in ["--help", "opus", "deep thought --json", "../../etc"] {
            let refused =
                dispatch(&door, "brain", &serde_json::json!({ "name": name }), None).await;
            assert!(refused.is_err(), "brain {name:?} got through");
        }
    }

    /// A thread id becomes a file name on parker (door's card store), so a
    /// page must not be able to hand it a path — not to read a card, not to
    /// write one. Refused before the engine is asked anything.
    #[tokio::test]
    async fn a_thread_id_is_never_a_path() {
        let door = Door::new(Engine::new("x".into()));
        for id in [
            "../engine.env",
            "ses_../../engine",
            "/etc/passwd",
            "ses_a/b",
        ] {
            let wrote = dispatch(
                &door,
                "digest",
                &serde_json::json!({ "sessionId": id, "text": "t", "facts": {} }),
                None,
            )
            .await;
            assert!(wrote.is_err(), "{id} was digested");
            let marked = dispatch(
                &door,
                "mark",
                &serde_json::json!({ "sessionId": id, "state": "done" }),
                None,
            )
            .await;
            assert!(marked.is_err(), "{id} was marked");
            for cmd in ["digests", "marks"] {
                let read = dispatch(&door, cmd, &serde_json::json!({ "ids": [id] }), None)
                    .await
                    .unwrap();
                assert_eq!(read, serde_json::json!({}), "{id} was read by {cmd}");
            }
        }
    }

    /// The shape `tailscale whois --json` answers with (trimmed), as measured
    /// against giskard, Oct 2026.
    #[test]
    fn a_device_is_named_as_the_tailnet_names_it() {
        let who = serde_json::json!({
            "Node": { "ComputedName": "giskard", "Hostinfo": { "OS": "macOS" } },
            "UserProfile": { "LoginName": ME },
        });
        assert_eq!(device(&who).as_deref(), Some("giskard (macOS)"));
        let bare = serde_json::json!({ "Node": { "ComputedName": "iphone-15-pro-max" } });
        assert_eq!(device(&bare).as_deref(), Some("iphone-15-pro-max"));
        assert_eq!(device(&serde_json::json!({})), None);
    }

    #[test]
    fn the_address_is_the_one_serve_forwarded() {
        let h = headers(&[("x-forwarded-for", "100.121.125.95")]);
        assert_eq!(forwarded_for(&h), Some("100.121.125.95".parse().unwrap()));
        assert_eq!(forwarded_for(&headers(&[])), None);
        assert_eq!(
            forwarded_for(&headers(&[("x-forwarded-for", "nonsense")])),
            None
        );
    }

    /// No forwarded address, no claim: a request that did not come through
    /// serve says nothing about where Peter is.
    #[tokio::test]
    async fn no_address_no_place() {
        assert_eq!(Places::default().of(None).await, None);
    }
}
