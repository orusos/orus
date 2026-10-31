//! The accounts the login screen offers, and which one logged in last.

use std::fs;
use std::io;
use std::path::Path;

/// Where the greeter remembers the user who logged in last. Its directory is
/// created for `_greetd` when the package is installed.
pub const LAST_USER_FILE: &str = "/var/cache/orus-greeter/last-user";

/// Debian's `UID_MIN` and the first uid above ordinary accounts (`nobody` is 65534).
const FIRST_LOGIN_UID: u32 = 1000;
const LAST_LOGIN_UID: u32 = 60000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginUser {
    /// The account name greetd authenticates.
    pub name: String,
    /// Full name from the account's GECOS field, or the account name if it has none.
    pub display: String,
}

/// Ordinary login accounts in `/etc/passwd` text: a uid in the normal range and
/// a shell that allows login.
pub fn parse_passwd(text: &str) -> Vec<LoginUser> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() < 7 {
                return None;
            }
            let uid: u32 = f[2].parse().ok()?;
            let shell = f[6];
            if !(FIRST_LOGIN_UID..LAST_LOGIN_UID).contains(&uid)
                || shell.ends_with("nologin")
                || shell.ends_with("/false")
            {
                return None;
            }
            let full_name = f[4].split(',').next().unwrap_or("").trim();
            Some(LoginUser {
                name: f[0].to_string(),
                display: if full_name.is_empty() {
                    f[0].to_string()
                } else {
                    full_name.to_string()
                },
            })
        })
        .collect()
}

/// `users` with the one named `last` moved to the front.
pub fn order(mut users: Vec<LoginUser>, last: Option<&str>) -> Vec<LoginUser> {
    if let Some(i) = last.and_then(|name| users.iter().position(|u| u.name == name)) {
        let user = users.remove(i);
        users.insert(0, user);
    }
    users
}

pub fn read_users() -> Vec<LoginUser> {
    match fs::read_to_string("/etc/passwd") {
        Ok(text) => parse_passwd(&text),
        Err(e) => {
            log::warn!("cannot read /etc/passwd: {e}");
            Vec::new()
        }
    }
}

pub fn last_user(path: &Path) -> Option<String> {
    let name = fs::read_to_string(path).ok()?.trim().to_string();
    (!name.is_empty()).then_some(name)
}

pub fn remember_last_user(path: &Path, name: &str) -> io::Result<()> {
    fs::write(path, name)
}
