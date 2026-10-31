//! greetd greeter for Orus OS. greetd starts it (inside labwc) with GREETD_SOCK
//! set; a successful login exits it, and greetd then starts the session.

use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::Result;
use jag_draw::{ColorLinPremul, Rect, SubpixelOrientation, make_surface_config};
use orus_core::system_status::SystemStatus;
use orus_greeter::form::{Credentials, Field, Form, FormKey};
use orus_greeter::greetd::{self, LoginError};
use orus_greeter::layout::{self, Layout, MENU_ROW_H};
use orus_greeter::view::{self, Scene};
use orus_greeter::{icons, keyboard, users};
use winit::event::{ElementState, Event, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window};

const SESSION_COMMAND: &str = "orus-session";
/// The screen redraws this often when nothing happens: the caret blinks and the
/// clock changes, and a finished login is noticed.
const TICK: Duration = Duration::from_millis(250);
/// How often the network state is read again.
const STATUS_REFRESH: Duration = Duration::from_secs(5);
const DEFAULT_ASSETS_DIR: &str = "/usr/share/orus";
const WALLPAPER: &str = "wallpapers/orus-dunes.png";

fn wallpaper_path() -> Option<PathBuf> {
    let base = std::env::var_os("ORUS_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ASSETS_DIR));
    let path = base.join(WALLPAPER);
    if path.is_file() {
        Some(path)
    } else {
        log::warn!("no wallpaper at {}; using a flat color", path.display());
        None
    }
}

fn connect_and_login(creds: Credentials) -> Result<(), LoginError> {
    let path = std::env::var("GREETD_SOCK").map_err(|_| {
        LoginError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "GREETD_SOCK is not set",
        ))
    })?;
    let mut stream = UnixStream::connect(path)?;
    greetd::login(
        &mut stream,
        &creds.username,
        &creds.password,
        &[SESSION_COMMAND.to_string()],
    )
}

fn start_login(creds: Credentials) -> (String, Receiver<Result<(), LoginError>>) {
    let username = creds.username.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // The receiver outlives the thread until the result arrives.
        let _ = tx.send(connect_and_login(creds));
    });
    (username, rx)
}

/// Ask systemd to power off or restart; returns a message to show if it refuses.
fn power(action: &str) -> Option<String> {
    match Command::new("systemctl").arg(action).status() {
        Ok(status) if status.success() => None,
        Ok(status) => Some(format!(
            "Could not {action}: systemctl exited with {status}"
        )),
        Err(e) => Some(format!("Could not {action}: {e}")),
    }
}

fn main() -> Result<()> {
    env_logger::init();
    icons::register();

    let event_loop = EventLoop::new()?;
    let attrs = Window::default_attributes()
        .with_title("Orus OS Login")
        .with_fullscreen(Some(Fullscreen::Borderless(None)));
    #[allow(deprecated)]
    let window = event_loop.create_window(attrs)?;
    let window: &'static Window = Box::leak(Box::new(window));

    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window)?;
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .ok_or_else(|| anyhow::anyhow!("no suitable GPU adapter"))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;

    let mut size = window.inner_size();
    // screen gets a smaller UI at the screen's real resolution instead of a resampled one.
    // Everything is drawn at 80% of the mock-up's size (orus_core::design), at the screen's own
    // resolution: the compositor stays at 100% and nothing is resampled.
    let mut scale_factor = window.scale_factor() as f32 * orus_core::design::SCALE;
    let mut config = make_surface_config(&adapter, &surface, size.width, size.height);
    surface.configure(&device, &config);

    let mut surf = jag_surface::DetirSurface::new(
        std::sync::Arc::new(device),
        std::sync::Arc::new(queue),
        config.format,
    );
    surf.set_use_intermediate(true);
    surf.set_direct(true);
    surf.set_logical_pixels(true);
    surf.set_dpi_scale(scale_factor);

    let provider: std::sync::Arc<dyn jag_draw::TextProvider> = std::sync::Arc::new(
        jag_draw::DetirTextProvider::from_system_fonts(SubpixelOrientation::RGB)?,
    );

    let last_user_file = Path::new(users::LAST_USER_FILE);
    let mut form = Form::new(users::order(
        users::read_users(),
        users::last_user(last_user_file).as_deref(),
    ));
    let mut pending: Option<(String, Receiver<Result<(), LoginError>>)> = None;
    let mut mouse = (0.0_f32, 0.0_f32);
    let mut power_menu_open = false;
    let wallpaper = wallpaper_path();
    let keyboard_label = keyboard::current_label();
    let mut network = SystemStatus::read().network;
    let mut network_read_at = Instant::now();
    let background = ColorLinPremul::from_srgba_u8([
        (view::BACKGROUND >> 16) as u8,
        (view::BACKGROUND >> 8) as u8,
        view::BACKGROUND as u8,
        255,
    ]);

    #[allow(deprecated)]
    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => elwt.exit(),
            WindowEvent::Resized(new_size) => {
                size = new_size;
                if size.width > 0 && size.height > 0 {
                    config.width = size.width;
                    config.height = size.height;
                    surface.configure(surf.device().as_ref(), &config);
                    surf.prepare_for_resize(size.width, size.height);
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor: new, ..
            } => {
                scale_factor = new as f32 * orus_core::design::SCALE;
                surf.set_dpi_scale(scale_factor);
                size = window.inner_size();
                config.width = size.width;
                config.height = size.height;
                surface.configure(surf.device().as_ref(), &config);
                surf.prepare_for_resize(size.width, size.height);
                window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                mouse = (
                    position.x as f32 / scale_factor,
                    position.y as f32 / scale_factor,
                );
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let viewport = Rect {
                    x: 0.0,
                    y: 0.0,
                    w: size.width as f32 / scale_factor,
                    h: size.height as f32 / scale_factor,
                };
                let l = Layout::new(viewport);
                let at = |rect: Rect| layout::contains(rect, mouse.0, mouse.1);
                if power_menu_open {
                    // A click on a menu row does that; any click closes the menu.
                    if at(l.menu) {
                        let row = ((mouse.1 - l.menu.y) / MENU_ROW_H) as usize;
                        let action = if row == 0 { "poweroff" } else { "reboot" };
                        if let Some(message) = power(action) {
                            form.fail(message);
                        }
                    }
                    power_menu_open = false;
                } else if at(l.power) {
                    power_menu_open = true;
                } else if at(l.eye) {
                    form.toggle_password_visible();
                } else if form.can_switch_user() && at(l.switch_hit) {
                    form.switch_user();
                } else if at(l.password_field) {
                    form.set_focus(Field::Password);
                } else if form.is_typing_username() && at(l.username_field) {
                    form.set_focus(Field::Username);
                }
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                power_menu_open = false;
                let key = match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Enter) => Some(FormKey::Enter),
                    Key::Named(NamedKey::Backspace) => Some(FormKey::Backspace),
                    Key::Named(NamedKey::Tab) => Some(FormKey::Tab),
                    Key::Named(NamedKey::Escape) => Some(FormKey::Escape),
                    _ => event
                        .text
                        .as_ref()
                        .and_then(|t| t.chars().next())
                        .filter(|c| !c.is_control())
                        .map(FormKey::Char),
                };
                if let Some(creds) = key.and_then(|k| form.handle_key(k)) {
                    pending = Some(start_login(creds));
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if size.width == 0 || size.height == 0 {
                    return;
                }
                let frame = match surface.get_current_texture() {
                    Ok(f) => f,
                    Err(_) => {
                        window.request_redraw();
                        return;
                    }
                };
                if network_read_at.elapsed() >= STATUS_REFRESH {
                    network = SystemStatus::read().network;
                    network_read_at = Instant::now();
                }
                let viewport = Rect {
                    x: 0.0,
                    y: 0.0,
                    w: size.width as f32 / scale_factor,
                    h: size.height as f32 / scale_factor,
                };
                let now = chrono::Local::now();
                let clock = now.format("%H:%M").to_string();
                let date = now.format("%A, %-d %B").to_string();
                let mut canvas = surf.begin_frame(size.width, size.height);
                canvas.set_text_provider(provider.clone());
                canvas.clear(background);
                view::render(
                    &mut canvas,
                    viewport,
                    &form,
                    &Scene {
                        clock: &clock,
                        date: &date,
                        wallpaper: wallpaper.as_deref(),
                        keyboard: keyboard_label.as_deref(),
                        network,
                        power_menu_open,
                    },
                );
                let _ = surf.end_frame(frame, canvas);
            }
            _ => {}
        },
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => window.request_redraw(),
        Event::AboutToWait => {
            if let Some((username, rx)) = &pending {
                match rx.try_recv() {
                    Ok(Ok(())) => {
                        if let Err(e) = users::remember_last_user(last_user_file, username) {
                            log::warn!("could not remember the last user: {e}");
                        }
                        elwt.exit();
                    }
                    Ok(Err(e)) => {
                        log::warn!("login failed: {e}");
                        form.fail(e.user_message());
                        pending = None;
                        window.request_redraw();
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => {
                        form.fail("The login attempt stopped unexpectedly".to_string());
                        pending = None;
                        window.request_redraw();
                    }
                }
            }
            elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + TICK));
        }
        _ => {}
    })?;

    Ok(())
}
