//! Per-app background threads for Orus OS multi-instance support.
//!
//! Each in-process app instance (terminal, text edit, files, viewer)
//! runs on its own thread.
//! The main thread sends input via channels, and locks shared state
//! briefly during rendering.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use orus_terminal::TerminalKey;
use winit::event::Modifiers as WinitModifiers;
use winit::keyboard::Key as WinitKey;

use crate::window_manager::WindowContent;

// ── Channel message types ─────────────────────────────────────

/// Input events sent from main thread to app thread.
pub enum AppInput {
    TerminalKey(TerminalKey),
    TerminalScrollUp(usize),
    TerminalScrollDown(usize),
    TerminalResize {
        cols: usize,
        rows: usize,
    },
    TextEditKey {
        key: WinitKey,
        modifiers: WinitModifiers,
    },
    TextEditMouseClick {
        x: f32,
        y: f32,
        double: bool,
    },
    TextEditMouseDrag {
        x: f32,
        y: f32,
    },
    TextEditMouseRelease,
    TextEditScroll {
        dy: f32,
    },
    TextEditResize {
        w: f32,
        h: f32,
    },
    FileManagerClick {
        x: f32,
        y: f32,
    },
    FileManagerDoubleClick {
        x: f32,
        y: f32,
    },
    FileManagerRightClick {
        x: f32,
        y: f32,
    },
    FileManagerKey {
        key: orus_filemanager::FileManagerKey,
    },
    FileManagerScroll {
        dy: f32,
    },
    FileManagerResize {
        w: f32,
        h: f32,
    },
    ViewerClick {
        x: f32,
        y: f32,
    },
    ViewerKey {
        key: orus_viewer::ViewerKey,
    },
    ViewerScroll {
        dy: f32,
    },
    ViewerResize {
        w: f32,
        h: f32,
    },
    Shutdown,
}

// ── Shared state types ────────────────────────────────────────

pub struct TerminalState {
    pub core: orus_terminal::TerminalCore,
    pub renderer: orus_terminal::TerminalRenderer,
}

// SAFETY: TerminalState is only accessed through Mutex, serializing all access.
// TerminalCore contains a PTY fd (plain integer) and grid data. No thread-local state.
unsafe impl Send for TerminalState {}

pub struct TextEditState {
    pub app: orus_textedit::TextEditApp,
}

// SAFETY: TextEditState is only accessed through Mutex, serializing all access.
unsafe impl Send for TextEditState {}

pub struct FileManagerState {
    pub app: orus_filemanager::FileManagerApp,
}

// SAFETY: FileManagerState is only accessed through Mutex, serializing all access.
unsafe impl Send for FileManagerState {}

pub struct ViewerState {
    pub app: orus_viewer::ViewerApp,
}

// SAFETY: ViewerState is only accessed through Mutex, serializing all access.
unsafe impl Send for ViewerState {}

// ── Thread signaling ──────────────────────────────────────────

pub struct AppFlags {
    pub needs_redraw: AtomicBool,
    pub exited: AtomicBool,
}

impl Default for AppFlags {
    fn default() -> Self {
        Self {
            needs_redraw: AtomicBool::new(true),
            exited: AtomicBool::new(false),
        }
    }
}

impl AppFlags {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct AppThreadHandle {
    pub input_tx: mpsc::Sender<AppInput>,
    pub flags: Arc<AppFlags>,
    pub join_handle: Option<JoinHandle<()>>,
}

// ── Terminal thread ───────────────────────────────────────────

pub fn spawn_terminal_thread(
    state: Arc<Mutex<TerminalState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("orus-terminal".into())
        .spawn(move || terminal_thread_loop(state, flags, input_rx))
        .expect("failed to spawn terminal thread")
}

fn terminal_thread_loop(
    state: Arc<Mutex<TerminalState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) {
    loop {
        // Drain all pending input
        loop {
            match input_rx.try_recv() {
                Ok(AppInput::Shutdown) => return,
                Ok(AppInput::TerminalKey(k)) => {
                    let mut s = state.lock().unwrap();
                    s.core.send_key(k);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::TerminalResize { cols, rows }) => {
                    let mut s = state.lock().unwrap();
                    s.core.resize(cols, rows);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::TerminalScrollUp(n)) => {
                    let mut s = state.lock().unwrap();
                    s.core.scroll_view_up(n);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::TerminalScrollDown(n)) => {
                    let mut s = state.lock().unwrap();
                    s.core.scroll_view_down(n);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(_) => {} // Ignore non-terminal inputs
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        // Poll PTY for new output
        {
            let mut s = state.lock().unwrap();
            if s.core.poll() {
                flags.needs_redraw.store(true, Ordering::Relaxed);
            }
            if !s.core.is_alive() {
                flags.exited.store(true, Ordering::Relaxed);
                flags.needs_redraw.store(true, Ordering::Relaxed);
                return;
            }
        }

        std::thread::sleep(Duration::from_millis(8));
    }
}

// ── Factory functions ─────────────────────────────────────────

pub fn create_terminal(
    core: orus_terminal::TerminalCore,
    renderer: orus_terminal::TerminalRenderer,
) -> (Arc<Mutex<TerminalState>>, WindowContent) {
    let state = Arc::new(Mutex::new(TerminalState { core, renderer }));
    let flags = Arc::new(AppFlags::new());
    let (input_tx, input_rx) = mpsc::channel();

    let join_handle = spawn_terminal_thread(Arc::clone(&state), Arc::clone(&flags), input_rx);

    let content = WindowContent::Terminal {
        state: Arc::clone(&state),
        thread: AppThreadHandle {
            input_tx,
            flags,
            join_handle: Some(join_handle),
        },
    };

    (state, content)
}

// ── Text edit thread ──────────────────────────────────────────

pub fn spawn_textedit_thread(
    state: Arc<Mutex<TextEditState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("orus-textedit".into())
        .spawn(move || textedit_thread_loop(state, flags, input_rx))
        .expect("failed to spawn textedit thread")
}

fn textedit_thread_loop(
    state: Arc<Mutex<TextEditState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) {
    loop {
        // Block waiting for input (text editor is purely event-driven)
        match input_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppInput::Shutdown) => return,
            Ok(input) => {
                let mut s = state.lock().unwrap();
                let consumed = match input {
                    AppInput::TextEditKey { key, modifiers } => s.app.handle_key(&key, &modifiers),
                    AppInput::TextEditMouseClick { x, y, double } => {
                        s.app.handle_mouse_click(x, y, double)
                    }
                    AppInput::TextEditMouseDrag { x, y } => s.app.handle_mouse_drag(x, y),
                    AppInput::TextEditMouseRelease => {
                        s.app.handle_mouse_release();
                        true
                    }
                    AppInput::TextEditScroll { dy } => {
                        s.app.handle_scroll(dy);
                        true
                    }
                    AppInput::TextEditResize { w, h } => {
                        s.app.update_viewport(w, h);
                        true
                    }
                    _ => false,
                };
                if consumed {
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Periodic wake for caret blink
                flags.needs_redraw.store(true, Ordering::Relaxed);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

pub fn create_textedit(
    app: orus_textedit::TextEditApp,
) -> (Arc<Mutex<TextEditState>>, WindowContent) {
    let state = Arc::new(Mutex::new(TextEditState { app }));
    let flags = Arc::new(AppFlags::new());
    let (input_tx, input_rx) = mpsc::channel();

    let join_handle = spawn_textedit_thread(Arc::clone(&state), Arc::clone(&flags), input_rx);

    let content = WindowContent::TextEdit {
        state: Arc::clone(&state),
        thread: AppThreadHandle {
            input_tx,
            flags,
            join_handle: Some(join_handle),
        },
    };

    (state, content)
}

// ── File manager thread ───────────────────────────────────────

pub fn spawn_filemanager_thread(
    state: Arc<Mutex<FileManagerState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("orus-filemanager".into())
        .spawn(move || filemanager_thread_loop(state, flags, input_rx))
        .expect("failed to spawn filemanager thread")
}

fn filemanager_thread_loop(
    state: Arc<Mutex<FileManagerState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) {
    loop {
        match input_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppInput::Shutdown) => return,
            Ok(input) => {
                let mut s = state.lock().unwrap();
                let consumed = match input {
                    AppInput::FileManagerClick { x, y } => s
                        .app
                        .handle_input(orus_filemanager::FileManagerInput::Click { x, y }),
                    AppInput::FileManagerDoubleClick { x, y } => s
                        .app
                        .handle_input(orus_filemanager::FileManagerInput::DoubleClick { x, y }),
                    AppInput::FileManagerRightClick { x, y } => s
                        .app
                        .handle_input(orus_filemanager::FileManagerInput::RightClick { x, y }),
                    AppInput::FileManagerKey { key } => s
                        .app
                        .handle_input(orus_filemanager::FileManagerInput::KeyPress(key)),
                    AppInput::FileManagerScroll { dy } => s
                        .app
                        .handle_input(orus_filemanager::FileManagerInput::Scroll { dy }),
                    AppInput::FileManagerResize { w, h } => s
                        .app
                        .handle_input(orus_filemanager::FileManagerInput::Resize { w, h }),
                    _ => false,
                };
                if consumed {
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Periodic wake for cursor blink
                let s = state.lock().unwrap();
                if s.app.needs_redraw() {
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

pub fn create_filemanager(
    app: orus_filemanager::FileManagerApp,
) -> (Arc<Mutex<FileManagerState>>, WindowContent) {
    let state = Arc::new(Mutex::new(FileManagerState { app }));
    let flags = Arc::new(AppFlags::new());
    let (input_tx, input_rx) = mpsc::channel();

    let join_handle = spawn_filemanager_thread(Arc::clone(&state), Arc::clone(&flags), input_rx);

    let content = WindowContent::FileManager {
        state: Arc::clone(&state),
        thread: AppThreadHandle {
            input_tx,
            flags,
            join_handle: Some(join_handle),
        },
    };

    (state, content)
}

// ── Viewer thread ──────────────────────────────────────

pub fn spawn_viewer_thread(
    state: Arc<Mutex<ViewerState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("orus-viewer".into())
        .spawn(move || viewer_thread_loop(state, flags, input_rx))
        .expect("failed to spawn viewer thread")
}

fn viewer_thread_loop(
    state: Arc<Mutex<ViewerState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) {
    loop {
        match input_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppInput::Shutdown) => return,
            Ok(input) => {
                let mut s = state.lock().unwrap();
                let consumed = match input {
                    AppInput::ViewerClick { x, y } => s
                        .app
                        .handle_input(orus_viewer::ViewerInput::Click { x, y }),
                    AppInput::ViewerKey { key } => s
                        .app
                        .handle_input(orus_viewer::ViewerInput::KeyPress(key)),
                    AppInput::ViewerScroll { dy } => s
                        .app
                        .handle_input(orus_viewer::ViewerInput::Scroll { dy }),
                    AppInput::ViewerResize { w, h } => s
                        .app
                        .handle_input(orus_viewer::ViewerInput::Resize { w, h }),
                    _ => false,
                };
                if consumed || s.app.poll_scan() {
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Poll for async scan results
                let mut s = state.lock().unwrap();
                if s.app.poll_scan() {
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

pub fn create_viewer(app: orus_viewer::ViewerApp) -> (Arc<Mutex<ViewerState>>, WindowContent) {
    let state = Arc::new(Mutex::new(ViewerState { app }));
    let flags = Arc::new(AppFlags::new());
    let (input_tx, input_rx) = mpsc::channel();

    let join_handle = spawn_viewer_thread(Arc::clone(&state), Arc::clone(&flags), input_rx);

    let content = WindowContent::Viewer {
        state: Arc::clone(&state),
        thread: AppThreadHandle {
            input_tx,
            flags,
            join_handle: Some(join_handle),
        },
    };

    (state, content)
}

pub fn shutdown_thread(handle: &mut AppThreadHandle) {
    let _ = handle.input_tx.send(AppInput::Shutdown);
    if let Some(jh) = handle.join_handle.take() {
        let _ = jh.join();
    }
}
