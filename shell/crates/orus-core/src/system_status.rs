//! Network and battery state for the shell bar, read from Linux sysfs.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Wifi,
    Wired,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemStatus {
    pub network: Network,
    /// `None` on a machine with no battery.
    pub battery: Option<Battery>,
}

impl SystemStatus {
    /// Read the live status from `/sys`.
    pub fn read() -> Self {
        Self::from_sysfs(Path::new("/sys"))
    }

    /// Read the status from a sysfs tree rooted at `root`.
    pub fn from_sysfs(root: &Path) -> Self {
        Self {
            network: network(&root.join("class/net")),
            battery: battery(&root.join("class/power_supply")),
        }
    }
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// A wireless interface that is up wins over a wired one; the loopback interface does not count.
fn network(net_dir: &Path) -> Network {
    let Ok(entries) = fs::read_dir(net_dir) else {
        return Network::Offline;
    };
    let mut wired_up = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_name() == "lo"
            || read_trimmed(&path.join("operstate")).as_deref() != Some("up")
        {
            continue;
        }
        if path.join("wireless").is_dir() {
            return Network::Wifi;
        }
        wired_up = true;
    }
    if wired_up {
        Network::Wired
    } else {
        Network::Offline
    }
}

fn battery(power_dir: &Path) -> Option<Battery> {
    let mut supplies: Vec<_> = fs::read_dir(power_dir).ok()?.flatten().collect();
    supplies.sort_by_key(|e| e.file_name());
    supplies.into_iter().find_map(|entry| {
        let path = entry.path();
        if read_trimmed(&path.join("type")).as_deref() != Some("Battery") {
            return None;
        }
        let percent = read_trimmed(&path.join("capacity"))?.parse::<u8>().ok()?;
        let charging = read_trimmed(&path.join("status")).as_deref() == Some("Charging");
        Some(Battery {
            percent: percent.min(100),
            charging,
        })
    })
}
