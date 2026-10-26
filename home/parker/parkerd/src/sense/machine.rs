// The machine itself: GPU, memory, disk, failed units, clock.
//
// A pile of forks and /proc reads. Measured end-to-end at ~150ms for the
// whole of `parker sense`, which is invisible in a CLI. These are exactly the
// probes the daemon will put behind TTLs — none of them change fast enough to
// deserve polling, and none of them are worth a crate at v0 scale.

use super::{Cpu, Gpu, Machine};
use std::process::Command;

pub fn read() -> Machine {
    // CPU utilisation needs two /proc/stat samples with time between them.
    // The other probes *are* the time between them (~100ms of forks), so the
    // first sample goes down here and the second at the end — no sleeping.
    let stat0 = cpu_stat();
    let (mem_used_gb, mem_total_gb) = memory();
    Machine {
        gpu: gpu(),
        mem_used_gb,
        mem_total_gb,
        disk_used_pct: disk_used_pct(),
        failed_units: failed_units(&["--failed"]),
        failed_user_units: failed_units(&["--user", "--failed"]),
        cpu: cpu(stat0),
    }
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn gpu() -> Option<Gpu> {
    // NVML would be faster and gives no fork, but on NixOS it needs the
    // driver's lib path wired into LD_LIBRARY_PATH explicitly. Not worth the
    // packaging risk for v0; revisit when this runs in a loop.
    let out = run(
        "nvidia-smi",
        &[
            "--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw",
            "--format=csv,noheader,nounits",
        ],
    )?;
    let line = out.lines().next()?;
    let f: Vec<&str> = line.split(',').map(str::trim).collect();
    if f.len() < 5 {
        return None;
    }
    Some(Gpu {
        util_pct: f[0].parse().ok()?,
        mem_used_mb: f[1].parse().ok()?,
        mem_total_mb: f[2].parse().ok()?,
        temp_c: f[3].parse().ok()?,
        watts: f[4].parse().ok()?,
    })
}

/// The aggregate cpu line of /proc/stat, as (busy jiffies, total jiffies).
fn cpu_stat() -> Option<(u64, u64)> {
    let s = std::fs::read_to_string("/proc/stat").ok()?;
    let line = s.lines().find(|l| l.starts_with("cpu "))?;
    let f: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|v| v.parse().ok())
        .collect();
    if f.len() < 5 {
        return None;
    }
    let total: u64 = f.iter().sum();
    let idle = f[3] + f.get(4).copied().unwrap_or(0); // idle + iowait
    Some((total - idle, total))
}

/// Utilisation from the delta against an earlier sample, plus package
/// temperature from hwmon. Both halves degrade to "I don't know".
fn cpu(stat0: Option<(u64, u64)>) -> Option<Cpu> {
    let (busy0, total0) = stat0?;
    let (busy1, total1) = cpu_stat()?;
    let dt = total1.saturating_sub(total0);
    if dt == 0 {
        return None;
    }
    let util = (busy1.saturating_sub(busy0) * 100) / dt;
    Some(Cpu {
        util_pct: util.min(100) as u8,
        temp_c: cpu_temp(),
    })
}

/// The die temperature, wherever this machine's sensor driver hangs it.
/// A name allowlist rather than "first hwmon": hwmon0 is as likely to be an
/// NVMe or the GPU as the CPU.
fn cpu_temp() -> Option<u8> {
    let dirs = std::fs::read_dir("/sys/class/hwmon").ok()?;
    for entry in dirs.flatten() {
        let path = entry.path();
        let name = std::fs::read_to_string(path.join("name")).unwrap_or_default();
        if !matches!(name.trim(), "k10temp" | "coretemp" | "zenpower") {
            continue;
        }
        // temp1_input is Tctl/Package on all three drivers; millidegrees.
        if let Ok(v) = std::fs::read_to_string(path.join("temp1_input")) {
            if let Ok(milli) = v.trim().parse::<i64>() {
                return u8::try_from(milli / 1000).ok();
            }
        }
    }
    None
}

/// Used vs total, in GB. "Used" is total minus *available*, not minus free —
/// cache is not consumption and reporting it as such makes every Linux box
/// look like it is dying.
fn memory() -> (f64, f64) {
    let Ok(s) = std::fs::read_to_string("/proc/meminfo") else {
        return (0.0, 0.0);
    };
    let kb = |key: &str| -> f64 {
        s.lines()
            .find(|l| l.starts_with(key))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    let total = kb("MemTotal:");
    let available = kb("MemAvailable:");
    let gb = |k: f64| k / 1024.0 / 1024.0;
    (gb(total - available), gb(total))
}

/// Percent used of the OS disk.
///
/// Queried through /nix rather than / on purpose: disko puts /, /home and
/// /nix on one btrfs, so they report the same number, but /nix is the one
/// mount that is always the real filesystem — a sandboxed or containerised
/// shell can hand you a tmpfs / and a perfectly honest 0%.
fn disk_used_pct() -> Option<u8> {
    let out = run("df", &["--output=pcent", "/nix"])?;
    out.lines()
        .nth(1)?
        .trim()
        .trim_end_matches('%')
        .parse()
        .ok()
}

fn failed_units(args: &[&str]) -> usize {
    let mut a = args.to_vec();
    a.extend_from_slice(&["--no-legend", "--plain"]);
    run("systemctl", &a)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

pub fn uptime_secs() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .map(|f| f as u64)
        .unwrap_or(0)
}

/// Local wall-clock time. Shelled out rather than pulling in a date crate and
/// a timezone database to render eight characters.
pub fn local_time() -> String {
    run("date", &["+%H:%M:%S"])
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}
