// BLUETOOTH: the radio, and each paired device — the DualSense, today.
//
// BlueZ over the system bus, with the zbus parkerd already has, rather than
// `bluetoothctl`: one GetManagedObjects answers everything, where the CLI
// would be a fork and an agent per device. The bus policy lets the active
// session's user power the adapter and connect a device, which is all this
// does. Pairing stays bluetui's (DECISIONS.md, Bluetooth).

use super::{toggle, Changed, Kind, Row, Set};
use std::collections::HashMap;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

const BLUEZ: &str = "org.bluez";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";

type Objects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

fn objects(conn: &Connection) -> Option<Objects> {
    let om = Proxy::new(conn, BLUEZ, "/", "org.freedesktop.DBus.ObjectManager").ok()?;
    om.call("GetManagedObjects", &()).ok()
}

struct Adapter {
    path: String,
    powered: bool,
}

struct Device {
    path: String,
    address: String,
    name: String,
    connected: bool,
    battery: Option<u8>,
}

fn read(objs: &Objects) -> (Option<Adapter>, Vec<Device>) {
    let flag = |props: &HashMap<String, OwnedValue>, k: &str| {
        props.get(k).and_then(|v| bool::try_from(v).ok()).unwrap_or(false)
    };
    let text = |props: &HashMap<String, OwnedValue>, k: &str| {
        props
            .get(k)
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or_default()
            .to_string()
    };
    let mut adapter = None;
    let mut devices = Vec::new();
    for (path, ifaces) in objs {
        if let Some(a) = ifaces.get(ADAPTER) {
            if adapter.is_none() {
                adapter = Some(Adapter {
                    path: path.as_str().to_string(),
                    powered: flag(a, "Powered"),
                });
            }
        }
        if let Some(d) = ifaces.get(DEVICE) {
            if !flag(d, "Paired") {
                continue;
            }
            let alias = text(d, "Alias");
            devices.push(Device {
                path: path.as_str().to_string(),
                address: text(d, "Address").to_lowercase(),
                name: if alias.is_empty() { text(d, "Name") } else { alias },
                connected: flag(d, "Connected"),
                battery: ifaces
                    .get(BATTERY)
                    .and_then(|b| b.get("Percentage"))
                    .and_then(|v| u8::try_from(v).ok()),
            });
        }
    }
    devices.sort_by(|a, b| a.name.cmp(&b.name));
    (adapter, devices)
}

pub fn rows() -> Vec<Row> {
    let Ok(conn) = Connection::system() else {
        return Vec::new();
    };
    let Some(objs) = objects(&conn) else {
        return Vec::new();
    };
    let (adapter, devices) = read(&objs);
    let Some(adapter) = adapter else {
        return Vec::new();
    };
    let mut out = vec![Row::new("bt", "bluetooth", Kind::Toggle, adapter.powered).slash("bluetooth")];
    for d in devices {
        let label = d.name.to_lowercase();
        let mut row = Row::new(
            format!("bt:{}", d.address),
            label.clone(),
            Kind::Action,
            if d.connected { "connected" } else { "paired" },
        )
        .slash(format!("connect {label}"));
        if !adapter.powered {
            row = row.why("bluetooth is off");
        } else if let (true, Some(b)) = (d.connected, d.battery) {
            row = row.hint(format!("{b}%"));
        }
        out.push(row);
    }
    out
}

pub fn set(id: &str, value: &str) -> Set {
    let conn = Connection::system().map_err(|e| format!("no system bus: {e}"))?;
    let objs = objects(&conn).ok_or("bluetooth would not answer")?;
    let (adapter, devices) = read(&objs);
    let adapter = adapter.ok_or("there is no bluetooth adapter")?;

    if id == "bt" {
        let on = toggle(value, adapter.powered)?;
        if on != adapter.powered {
            Proxy::new(&conn, BLUEZ, adapter.path.as_str(), ADAPTER)
                .map_err(|e| format!("no adapter: {e}"))?
                .set_property("Powered", on)
                .map_err(|e| format!("bluetooth refused: {e}"))?;
        }
        return Ok(Changed::new("bt", "bluetooth", Kind::Toggle, super::onoff(adapter.powered), super::onoff(on)));
    }

    let address = id.strip_prefix("bt:").unwrap_or(id);
    let d = devices
        .iter()
        .find(|d| d.address == address)
        .ok_or_else(|| format!("no paired device {address}"))?;
    let label = d.name.to_lowercase();
    let connect = match value.to_lowercase().as_str() {
        "connect" | "on" => true,
        "disconnect" | "off" => false,
        "toggle" => !d.connected,
        other => return Err(format!("{label} takes connect, disconnect or toggle, not '{other}'")),
    };
    if !adapter.powered && connect {
        return Err("bluetooth is off".into());
    }
    let was = if d.connected { "connected" } else { "paired" };
    if connect != d.connected {
        Proxy::new(&conn, BLUEZ, d.path.as_str(), DEVICE)
            .map_err(|e| format!("no device: {e}"))?
            .call::<_, _, ()>(if connect { "Connect" } else { "Disconnect" }, &())
            .map_err(|e| {
                let e = e.to_string();
                // A pad that is asleep cannot be reached; its PS button
                // reconnects it (FastConnectable, system/bluetooth.nix).
                if connect && (e.contains("Page Timeout") || e.contains("br-connection")) {
                    format!("{label} did not answer — is it on? its PS button wakes it")
                } else {
                    format!("{label}: {e}")
                }
            })?;
    }
    let now = if connect { "connected" } else { "paired" };
    Ok(Changed::new(id, &label, Kind::Action, was, now))
}
