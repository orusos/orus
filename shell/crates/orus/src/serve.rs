//! A static file server for a catalog folder: what `orus catalog serve` runs, so an app
//! built on one machine installs on another. Read-only, `GET` only.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::{Result, anyhow};
use tiny_http::{Header, Method, Request, Response, Server};

/// Default port for the development catalog.
pub const DEFAULT_PORT: u16 = 6678;

pub struct Running {
    pub addr: std::net::SocketAddr,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

impl Running {
    /// Stop serving and wait for the thread.
    pub fn stop(mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    /// Serve until the process ends.
    pub fn wait(mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Serve `dir` on `bind` (for example `0.0.0.0:6678`; port 0 picks one).
pub fn start(dir: &Path, bind: &str) -> Result<Running> {
    let server = Arc::new(Server::http(bind).map_err(|e| anyhow!("cannot listen on {bind}: {e}"))?);
    let addr = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| anyhow!("the server has no IP address"))?;
    let root = dir.to_path_buf();
    let worker = server.clone();
    let thread = std::thread::spawn(move || {
        for request in worker.incoming_requests() {
            handle(&root, request);
        }
    });
    Ok(Running {
        addr,
        server,
        thread: Some(thread),
    })
}

/// The file `url_path` names under `root`, if it is a plain path inside it.
fn resolve(root: &Path, url_path: &str) -> Option<PathBuf> {
    let path = url_path.split(['?', '#']).next()?.trim_start_matches('/');
    let mut target = root.to_path_buf();
    for part in Path::new(path).components() {
        match part {
            Component::Normal(name) => target.push(name),
            _ => return None,
        }
    }
    Some(target)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("json") => "application/json",
        Some("orusapp") => "application/zip",
        _ => "application/octet-stream",
    }
}

fn handle(root: &Path, request: Request) {
    if *request.method() != Method::Get {
        let _ = request.respond(Response::empty(405));
        return;
    }
    let Some(path) = resolve(root, request.url()).filter(|p| p.is_file()) else {
        let _ = request.respond(Response::empty(404));
        return;
    };
    match std::fs::File::open(&path) {
        Ok(file) => {
            let header =
                Header::from_bytes("Content-Type", content_type(&path)).expect("static header");
            let _ = request.respond(Response::from_file(file).with_header(header));
        }
        Err(_) => {
            let _ = request.respond(Response::empty(500));
        }
    }
}
