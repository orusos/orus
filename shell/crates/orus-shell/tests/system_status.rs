use std::fs;
use std::path::Path;

use orus_shell::system_status::{Battery, Network, SystemStatus};

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn a_wireless_interface_that_is_up_is_wifi_even_with_a_wired_one_up() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "class/net/lo/operstate", "unknown\n");
    write(dir.path(), "class/net/enp0s1/operstate", "up\n");
    write(dir.path(), "class/net/wlan0/operstate", "up\n");
    fs::create_dir_all(dir.path().join("class/net/wlan0/wireless")).unwrap();
    assert_eq!(SystemStatus::from_sysfs(dir.path()).network, Network::Wifi);
}

#[test]
fn a_wired_interface_that_is_up_is_wired() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "class/net/enp0s1/operstate", "up\n");
    write(dir.path(), "class/net/wlan0/operstate", "down\n");
    fs::create_dir_all(dir.path().join("class/net/wlan0/wireless")).unwrap();
    assert_eq!(SystemStatus::from_sysfs(dir.path()).network, Network::Wired);
}

#[test]
fn only_loopback_or_down_interfaces_is_offline() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "class/net/lo/operstate", "up\n");
    write(dir.path(), "class/net/enp0s1/operstate", "down\n");
    assert_eq!(
        SystemStatus::from_sysfs(dir.path()).network,
        Network::Offline
    );
    assert_eq!(
        SystemStatus::from_sysfs(Path::new("/nonexistent")).network,
        Network::Offline
    );
}

#[test]
fn battery_reads_percent_and_charging_and_skips_mains_supplies() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "class/power_supply/AC/type", "Mains\n");
    write(dir.path(), "class/power_supply/BAT0/type", "Battery\n");
    write(dir.path(), "class/power_supply/BAT0/capacity", "73\n");
    write(dir.path(), "class/power_supply/BAT0/status", "Charging\n");
    assert_eq!(
        SystemStatus::from_sysfs(dir.path()).battery,
        Some(Battery {
            percent: 73,
            charging: true
        })
    );
}

#[test]
fn a_machine_without_a_battery_has_none() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "class/power_supply/AC/type", "Mains\n");
    assert_eq!(SystemStatus::from_sysfs(dir.path()).battery, None);
}
