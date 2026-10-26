// NETWORK: wifi through NetworkManager, the tailnet through tailscale.
//
// nmcli runs as Peter because he is in the `networkmanager` group
// (system/users.nix). tailscale up and down run as him because he is the
// tailnet's operator on this machine (system/tailscale.nix sets
// `--operator`); without that both need root.
//
// Both rows are the tower's from the porthole: the porthole reaches the
// tower over this wifi and this tailnet (controls/mod.rs, `tower`).

use super::{output, run, toggle, Changed, Kind, Row, Set};
use std::process::Command;

pub fn rows() -> Vec<Row> {
    let mut out = Vec::new();
    if let Some(r) = wifi() {
        out.push(r);
    }
    if let Some(r) = tailscale() {
        out.push(r);
    }
    out
}

/// nmcli's terse fields, split on unescaped colons (an SSID may hold one,
/// written `\:`).
pub(crate) fn fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.last_mut().unwrap().push(n);
                }
            }
            ':' => out.push(String::new()),
            c => out.last_mut().unwrap().push(c),
        }
    }
    out
}

struct Wifi {
    radio: bool,
    active: Option<String>,
    signal: Option<i64>,
    saved: Vec<String>,
}

fn read_wifi() -> Option<Wifi> {
    let radio = run("nmcli", &["-t", "-f", "WIFI", "general"]).ok()? == "enabled";
    let connections = run("nmcli", &["-t", "-f", "NAME,TYPE,ACTIVE", "connection", "show"]).ok()?;
    let mut saved = Vec::new();
    let mut active = None;
    for l in connections.lines() {
        let f = fields(l);
        if f.get(1).map(String::as_str) != Some("802-11-wireless") {
            continue;
        }
        saved.push(f[0].clone());
        if f.get(2).map(String::as_str) == Some("yes") {
            active = Some(f[0].clone());
        }
    }
    let signal = if active.is_some() {
        run("nmcli", &["-t", "-f", "IN-USE,SIGNAL", "device", "wifi", "list", "--rescan", "no"])
            .ok()
            .and_then(|list| {
                list.lines()
                    .map(fields)
                    .find(|f| f.first().map(String::as_str) == Some("*"))
                    .and_then(|f| f.get(1)?.parse().ok())
            })
    } else {
        None
    };
    Some(Wifi { radio, active, signal, saved })
}

fn wifi() -> Option<Row> {
    let w = read_wifi()?;
    let value = if !w.radio {
        "off".to_string()
    } else {
        w.active.clone().unwrap_or_default()
    };
    let mut choices = w.saved.clone();
    choices.push("off".into());
    let hint = match (&w.active, w.signal, w.radio) {
        (_, _, false) => String::new(),
        (Some(_), Some(s), _) => format!("{s}%"),
        (None, _, true) => "not connected".into(),
        _ => String::new(),
    };
    Some(
        Row::new("wifi", "wifi", Kind::Choice, value)
            .choices(choices)
            .hint(hint)
            .slash("wifi"),
    )
}

fn tailscale() -> Option<Row> {
    let raw = run("tailscale", &["status", "--json"]).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let state = v["BackendState"].as_str().unwrap_or_default();
    // Peers by their MagicDNS name: the phone's HostName is `localhost`.
    let online: Vec<String> = v["Peer"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .filter(|p| p["Online"].as_bool().unwrap_or(false))
        .filter_map(|p| p["DNSName"].as_str()?.split('.').next().map(str::to_string))
        .filter(|n| !n.is_empty())
        .collect();
    let row = Row::new("tailscale", "tailscale", Kind::Toggle, state == "Running").slash("tailscale");
    Some(match state {
        "Running" if online.is_empty() => row.hint("no one else online"),
        "Running" => row.hint(online.join(", ")),
        "NeedsLogin" => row.why("needs a login: sudo tailscale up"),
        _ => row,
    })
}

pub fn set(id: &str, value: &str) -> Set {
    match id {
        "wifi" => set_wifi(value),
        _ => set_tailscale(value),
    }
}

fn set_wifi(value: &str) -> Set {
    let w = read_wifi().ok_or("NetworkManager would not answer")?;
    let from = if !w.radio { "off".to_string() } else { w.active.clone().unwrap_or_else(|| "on".into()) };
    match value {
        "off" => {
            run("nmcli", &["radio", "wifi", "off"])?;
            Ok(Changed::new("wifi", "wifi", Kind::Choice, from, "off"))
        }
        "on" => {
            run("nmcli", &["radio", "wifi", "on"])?;
            Ok(Changed::new("wifi", "wifi", Kind::Choice, from, "on"))
        }
        name => {
            let known = w.saved.iter().find(|s| s.eq_ignore_ascii_case(name)).ok_or_else(|| {
                format!("no saved wifi '{name}'. saved: {}", w.saved.join(", "))
            })?;
            if !w.radio {
                run("nmcli", &["radio", "wifi", "on"])?;
            }
            run("nmcli", &["connection", "up", "id", known])?;
            Ok(Changed::new("wifi", "wifi", Kind::Choice, from, known.clone()))
        }
    }
}

fn set_tailscale(value: &str) -> Set {
    let was = tailscale()
        .and_then(|r| r.value.as_bool())
        .ok_or("tailscale would not answer")?;
    let on = toggle(value, was)?;
    if on != was {
        output(Command::new("tailscale").arg(if on { "up" } else { "down" }), "tailscale").map_err(|e| {
            if e.contains("ccess denied") || e.contains("permission") {
                "tailscale wants root here: the operator flag is not set (system/tailscale.nix)".to_string()
            } else {
                e
            }
        })?;
    }
    Ok(Changed::new("tailscale", "tailscale", Kind::Toggle, super::onoff(was), super::onoff(on)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terse_fields_keep_an_escaped_colon() {
        assert_eq!(fields("PEKI:802-11-wireless:yes"), ["PEKI", "802-11-wireless", "yes"]);
        assert_eq!(fields(r"cafe\:2:802-11-wireless:no"), ["cafe:2", "802-11-wireless", "no"]);
        assert_eq!(fields("*:58"), ["*", "58"]);
    }
}
