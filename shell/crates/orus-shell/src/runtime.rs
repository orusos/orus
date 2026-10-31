//! Orus OS Universal shared runtime.
//!
//! Own window + wgpu + Canvas render loop using engine-core directly.
//! The shell bar and app window chrome are GPU-rendered.
//! Uses WindowManager for multi-window management and OrusDock for
//! window switching.

#![allow(clippy::collapsible_if)]

use crate::app_kind::AppKind;
use crate::app_thread::{self, AppInput};
use crate::app_window::{AppWindow, ResizeEdge};
use crate::dock::{DockAction, DockEntry, OrusDock};
use crate::dock_bar::{DockApp, DockBar};
use crate::launcher::{Launcher, LauncherAction, LauncherKey};
use crate::layout::{SHELL_BAR_HEIGHT, ShellLayout};
use crate::platform::PlatformHooks;
use crate::power_menu::PowerMenu;
use crate::sandbox::SandboxContext;
use crate::session;
use crate::shell_bar::ShellBar;
use crate::window_manager::{WindowContent, WindowId, WindowManager};
use crate::window_state;
use anyhow::Result;
use jag_draw::{Rect, SubpixelOrientation, make_surface_config};
use orus_terminal::{TerminalConfig, TerminalCore, TerminalKey, TerminalRenderer};
use orus_textedit::TextEditApp;
use winit::event::{ElementState, Event, Modifiers, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

/// Below every window and the shell bar.
const WALLPAPER_Z: i32 = -1000;

/// How often an idle shell redraws, so the clock and the dock's running dots stay current.
const IDLE_REDRAW: std::time::Duration = std::time::Duration::from_secs(1);

/// Whether this process is the shell or one app running as a window of its own.
#[derive(Debug, Clone, Copy)]
pub enum RunMode {
    Shell,
    /// Only the wallpaper: the bar, dock and launcher are separate layer surfaces
    /// (the `orus-panel` crate), so windows can never cover them.
    Desktop,
    /// `kind` in a window of `size` logical pixels.
    App {
        kind: AppKind,
        size: (f64, f64),
    },
}

impl RunMode {
    fn is_shell(self) -> bool {
        matches!(self, RunMode::Shell)
    }

    fn draws_wallpaper(self) -> bool {
        matches!(self, RunMode::Shell | RunMode::Desktop)
    }
}

/// The layout for a window of `w`x`h`: with the shell bar for the shell, without
/// it for an app that fills its own window.
fn layout_for(mode: RunMode, w: u32, h: u32) -> ShellLayout {
    match mode {
        RunMode::Shell => ShellLayout::calculate(w, h),
        RunMode::Desktop | RunMode::App { .. } => ShellLayout::without_bar(w, h),
    }
}

pub fn run<P: PlatformHooks + 'static>(platform: P) -> Result<()> {
    run_mode(platform, RunMode::Shell)
}

/// Run only the wallpaper window; the panel process draws the bar and dock.
pub fn run_desktop<P: PlatformHooks + 'static>(platform: P) -> Result<()> {
    run_mode(platform, RunMode::Desktop)
}

/// Run `kind` as a window of its own, `size` logical pixels, under the compositor.
pub fn run_app<P: PlatformHooks + 'static>(
    platform: P,
    kind: AppKind,
    size: (f64, f64),
) -> Result<()> {
    run_mode(platform, RunMode::App { kind, size })
}

fn run_mode<P: PlatformHooks + 'static>(platform: P, mode: RunMode) -> Result<()> {
    log::info!("Orus OS Universal starting...");

    // Register SVG icons so the file manager and viewer can render them
    crate::icons::register();

    // Restore saved window geometry (or use defaults on first run)
    let sandbox = SandboxContext::new(platform.data_dir())?;
    log::info!("sandbox root: {}", sandbox.root().display());
    let saved_state = if mode.is_shell() {
        window_state::load(&sandbox).unwrap_or_default()
    } else {
        Default::default()
    };
    let mut win_state = saved_state.clone();

    let event_loop = EventLoop::new()?;
    let mut window_attrs = Window::default_attributes().with_title(match mode {
        RunMode::Shell | RunMode::Desktop => "Orus OS Universal",
        RunMode::App { kind, .. } => kind.title(),
    });

    if saved_state.has_valid_size() {
        window_attrs = window_attrs.with_inner_size(winit::dpi::LogicalSize::new(
            saved_state.width,
            saved_state.height,
        ));
    } else {
        window_attrs =
            window_attrs.with_inner_size(winit::dpi::LogicalSize::new(1400.0_f64, 900.0));
    }
    if saved_state.has_valid_position() {
        window_attrs = window_attrs.with_position(winit::dpi::LogicalPosition::new(
            saved_state.x,
            saved_state.y,
        ));
    }
    if saved_state.maximized {
        window_attrs = window_attrs.with_maximized(true);
    }

    if let RunMode::App { size, .. } = mode {
        window_attrs = window_attrs.with_inner_size(winit::dpi::LogicalSize::new(size.0, size.1));
    }
    #[cfg(target_os = "linux")]
    if let RunMode::App { kind, .. } = mode {
        use winit::platform::wayland::WindowAttributesExtWayland;
        window_attrs = window_attrs.with_name(kind.app_id(), kind.app_id());
    }

    #[allow(deprecated)]
    let window = event_loop.create_window(window_attrs)?;
    let window: &'static Window = Box::leak(Box::new(window));

    platform.startup_tweaks(window);

    // wgpu setup
    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window)?;
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .expect("No suitable GPU adapters found");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;

    let mut size = window.inner_size();
    // Everything is drawn at 80% of the mock-up's size (orus_core::design), at the screen's own
    // resolution: the compositor stays at 100% and nothing is resampled.
    let mut scale_factor = window.scale_factor() as f32 * orus_core::design::SCALE;
    let mut config = make_surface_config(&adapter, &surface, size.width, size.height);
    surface.configure(&device, &config);

    // Canvas wrapper
    let mut surf = jag_surface::DetirSurface::new(
        std::sync::Arc::new(device),
        std::sync::Arc::new(queue),
        config.format,
    );
    surf.set_use_intermediate(true);
    surf.set_direct(true);
    surf.set_logical_pixels(true);
    surf.set_dpi_scale(scale_factor);

    // Text provider
    let provider: std::sync::Arc<dyn jag_draw::TextProvider> = std::sync::Arc::new(
        jag_draw::DetirTextProvider::from_system_fonts(SubpixelOrientation::RGB)?,
    );

    // ── WindowManager setup ──────────────────────────────────────────
    let mut wm = WindowManager::new();
    let mut dock = OrusDock::new();

    let logical_w = size.width as f32 / scale_factor;
    let logical_h = size.height as f32 / scale_factor;
    let layout = layout_for(mode, logical_w as u32, logical_h as u32);
    let vp = layout.app_viewport;

    // ── Session restore or default window creation ─────────────────────
    let saved_session = if mode.is_shell() {
        session::load(&sandbox)
    } else {
        None
    };
    let has_saved_windows = saved_session
        .as_ref()
        .is_some_and(|s| !s.windows.is_empty());

    if has_saved_windows {
        let ss = saved_session.as_ref().unwrap();
        for entry in &ss.windows {
            let bounds = Rect {
                x: entry.bounds.x,
                y: entry.bounds.y,
                w: entry.bounds.w,
                h: entry.bounds.h,
            };
            match entry.app_id.as_str() {
                // A terminal is opened from the launcher, never at startup.
                "terminal" => {}
                "textedit" => {
                    let mut aw = AppWindow::new("Text Edit", bounds);
                    if entry.maximized {
                        aw.toggle_maximize(vp);
                    }
                    match TextEditApp::new_with_provider(provider.clone()) {
                        Ok(app) => {
                            let (_state, content) = app_thread::create_textedit(app);
                            wm.add("textedit", aw, content);
                        }
                        Err(e) => {
                            log::error!("Failed to restore TextEditApp: {e}");
                        }
                    }
                }
                other => {
                    log::warn!("Unknown app_id in session: {other}, skipping");
                }
            }
        }
        // Restore focus. If the saved focused_app_id doesn't match any
        // restored window, fall back to the last window so that
        // update_z_order() always runs and windows get proper z layering.
        let focus_id = ss
            .focused_app_id
            .as_ref()
            .and_then(|fid| wm.windows().find(|mw| mw.app_id == *fid).map(|mw| mw.id))
            .or_else(|| wm.windows().last().map(|mw| mw.id));
        if let Some(id) = focus_id {
            wm.focus(id);
        }
    }
    // With no saved session the shell starts with an empty desktop: apps
    // are opened from the launcher, each as a window of its own.
    if let RunMode::App { kind, .. } = mode
        && open_app(kind, &mut wm, &provider, &sandbox, vp).is_none()
    {
        anyhow::bail!("could not start {}", kind.title());
    }

    // Shell state
    let initial_title = wm
        .focused_ref()
        .map(|mw| mw.app_window.title.as_str())
        .unwrap_or("Orus OS");
    let mut shell_bar = ShellBar::new(initial_title);
    let mut launcher = Launcher::default();
    let mut dock_bar = DockBar::new();
    let mut power_menu = PowerMenu::default();
    let mut dock_target = true;
    let mut appearance_watch = crate::appearance_watch::AppearanceWatch::new(&platform.data_dir());
    crate::palette::set(palette_for(&appearance_watch.current));
    let mut wallpaper = wallpaper_for(&appearance_watch.current);
    let mut current_modifiers = Modifiers::default();
    let mut scroll_accum: f32 = 0.0;

    // Drag/resize tracking (uses WindowId now)
    let mut drag_offset: Option<(WindowId, f32, f32)> = None;
    let mut resize_state: Option<(WindowId, ResizeEdge)> = None;
    let mut mouse_pos: (f32, f32) = (0.0, 0.0);
    let mut last_mouse_pos: (f32, f32) = (0.0, 0.0);
    let mut current_cursor = winit::window::CursorIcon::Default;
    let mut last_title_bar_click: Option<std::time::Instant> = None;
    let mut last_fm_click: Option<(std::time::Instant, f32, f32)> = None;

    #[allow(deprecated)]
    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => {
                if mode.is_shell() {
                    window_state::save(&sandbox, &win_state);

                    // Save internal window session
                    let session_state = wm.snapshot_session();
                    session::save(&sandbox, &session_state);
                }

                // Shutdown all app threads
                for mw in wm.iter_back_to_front_mut() {
                    match &mut mw.content {
                        WindowContent::Terminal { thread, .. }
                        | WindowContent::TextEdit { thread, .. }
                        | WindowContent::FileManager { thread, .. }
                        | WindowContent::Viewer { thread, .. } => {
                            app_thread::shutdown_thread(thread);
                        }
                    }
                }

                elwt.exit();
            }

            WindowEvent::Resized(new_size) => {
                size = new_size;
                if size.width > 0 && size.height > 0 {
                    config.width = size.width;
                    config.height = size.height;
                    surface.configure(surf.device().as_ref(), &config);
                    surf.prepare_for_resize(size.width, size.height);

                    let logical = new_size.to_logical::<f64>(window.scale_factor() * f64::from(orus_core::design::SCALE));
                    win_state.width = logical.width;
                    win_state.height = logical.height;
                    win_state.maximized = window.is_maximized();

                    let lw = (size.width as f32 / scale_factor) as u32;
                    let lh = (size.height as f32 / scale_factor) as u32;
                    let new_layout = layout_for(mode, lw, lh);
                    let new_vp = new_layout.app_viewport;

                    // Resize maximized windows and update terminal sizes
                    wm.resize_maximized(new_vp);
                    for mw in wm.iter_back_to_front_mut() {
                        if let WindowContent::Terminal { state, .. } = &mw.content {
                            let s = state.lock().unwrap();
                            let cr = mw.app_window.content_rect();
                            let cols = s
                                .renderer
                                .metrics
                                .cols_for_width(cr.w, s.renderer.cfg.padding);
                            let rows = s
                                .renderer
                                .metrics
                                .rows_for_height(cr.h, s.renderer.cfg.padding);
                            drop(s);
                            mw.content
                                .send_input(AppInput::TerminalResize { cols, rows });
                        }
                    }
                }
                window.request_redraw();
            }

            WindowEvent::Moved(position) => {
                let logical = position.to_logical::<f64>(window.scale_factor() * f64::from(orus_core::design::SCALE));
                win_state.x = logical.x.round() as i32;
                win_state.y = logical.y.round() as i32;
            }

            WindowEvent::ScaleFactorChanged {
                scale_factor: new_sf,
                ..
            } => {
                scale_factor = new_sf as f32 * orus_core::design::SCALE;
                surf.set_dpi_scale(scale_factor);
                size = window.inner_size();
                config.width = size.width;
                config.height = size.height;
                surface.configure(surf.device().as_ref(), &config);
                surf.prepare_for_resize(size.width, size.height);
                window.request_redraw();
            }

            WindowEvent::ModifiersChanged(new_mods) => {
                current_modifiers = new_mods;
            }

            WindowEvent::CursorMoved { position, .. } => {
                let lx = position.x as f32 / scale_factor;
                let ly = position.y as f32 / scale_factor;
                mouse_pos = (lx, ly);

                if power_menu.visible {
                    window.request_redraw();
                }

                // Slide the dock out or in as soon as the pointer decides it.
                let screen = Rect {
                    x: 0.0,
                    y: 0.0,
                    w: size.width as f32 / scale_factor,
                    h: size.height as f32 / scale_factor,
                };
                if mode.is_shell()
                    && dock_bar.wants_to_show(
                        screen,
                        mouse_pos,
                        wm.windows().any(|mw| mw.app_window.is_maximized()),
                    ) != dock_target
                {
                    window.request_redraw();
                }

                if launcher.visible || dock.visible {
                    // no-op: just track mouse_pos
                } else if let Some((target_id, ox, oy)) = drag_offset {
                    if let Some(mw) = wm.get_mut(target_id) {
                        mw.app_window.bounds.x = lx - ox;
                        mw.app_window.bounds.y = (ly - oy).max(SHELL_BAR_HEIGHT);
                        if let WindowContent::Terminal { state, .. } = &mw.content {
                            let s = state.lock().unwrap();
                            let cr = mw.app_window.content_rect();
                            let cols = s
                                .renderer
                                .metrics
                                .cols_for_width(cr.w, s.renderer.cfg.padding);
                            let rows = s
                                .renderer
                                .metrics
                                .rows_for_height(cr.h, s.renderer.cfg.padding);
                            drop(s);
                            mw.content
                                .send_input(AppInput::TerminalResize { cols, rows });
                        }
                    }
                    window.request_redraw();
                } else if let Some((target_id, edge)) = resize_state {
                    let dx = lx - last_mouse_pos.0;
                    let dy = ly - last_mouse_pos.1;
                    if let Some(mw) = wm.get_mut(target_id) {
                        mw.app_window.apply_resize(edge, dx, dy);
                        if let WindowContent::Terminal { state, .. } = &mw.content {
                            let s = state.lock().unwrap();
                            let cr = mw.app_window.content_rect();
                            let cols = s
                                .renderer
                                .metrics
                                .cols_for_width(cr.w, s.renderer.cfg.padding);
                            let rows = s
                                .renderer
                                .metrics
                                .rows_for_height(cr.h, s.renderer.cfg.padding);
                            drop(s);
                            mw.content
                                .send_input(AppInput::TerminalResize { cols, rows });
                        }
                    }
                    window.request_redraw();
                } else {
                    // Update cursor icon
                    use winit::window::CursorIcon;
                    let icon = wm
                        .iter_front_to_back()
                        .find_map(|mw| mw.app_window.hit_test_resize(lx, ly))
                        .map(|edge| match edge {
                            ResizeEdge::Left | ResizeEdge::Right => CursorIcon::EwResize,
                            ResizeEdge::Top | ResizeEdge::Bottom => CursorIcon::NsResize,
                            ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorIcon::NwseResize,
                            ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorIcon::NeswResize,
                        })
                        .unwrap_or(CursorIcon::Default);
                    if icon != current_cursor {
                        window.set_cursor(icon);
                        current_cursor = icon;
                    }
                }

                // Forward mouse drag to focused text editor (only during active text selection).
                if !launcher.visible
                    && !dock.visible
                    && let Some(mw) = wm.focused_mut()
                    && let WindowContent::TextEdit { state, .. } = &mw.content
                    && state.lock().unwrap().app.is_mouse_selecting()
                {
                    let cr = mw.app_window.content_rect();
                    let lx_local = lx - cr.x;
                    let ly_local = ly - cr.y;
                    mw.content.send_input(AppInput::TextEditMouseDrag {
                        x: lx_local,
                        y: ly_local,
                    });
                    window.request_redraw();
                }

                last_mouse_pos = (lx, ly);
            }

            // While the power menu is open a click picks a row, or just closes it.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } if power_menu.visible => {
                let bar = layout_for(
                    mode,
                    (size.width as f32 / scale_factor) as u32,
                    (size.height as f32 / scale_factor) as u32,
                )
                .shell_bar;
                if let Some(action) = PowerMenu::action_at(bar, mouse_pos.0, mouse_pos.1)
                    && let Err(e) = action.run()
                {
                    log::error!("{} failed: {e}", action.label());
                }
                power_menu.visible = false;
                window.request_redraw();
            }

            // The right end of the shell bar (network, battery, clock) opens the power menu.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } if mode.is_shell()
                && ShellBar::status_area_contains(
                    layout_for(
                        mode,
                        (size.width as f32 / scale_factor) as u32,
                        (size.height as f32 / scale_factor) as u32,
                    )
                    .shell_bar,
                    mouse_pos.0,
                    mouse_pos.1,
                ) =>
            {
                power_menu.visible = true;
                launcher.visible = false;
                window.request_redraw();
            }

            // Dock click handling
            // "Apps" button in the shell bar toggles the launcher.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } if mode.is_shell()
                && ShellBar::apps_button_contains(
                    layout_for(
                        mode,
                        (size.width as f32 / scale_factor) as u32,
                        (size.height as f32 / scale_factor) as u32,
                    )
                    .shell_bar,
                    mouse_pos.0,
                    mouse_pos.1,
                ) =>
            {
                toggle_launcher(&mut launcher);
                window.request_redraw();
            }

            // A click on a dock icon opens that app, or the launcher for the first icon.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } if mode.is_shell()
                && dock_bar
                    .hit_test(
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            w: size.width as f32 / scale_factor,
                            h: size.height as f32 / scale_factor,
                        },
                        mouse_pos.0,
                        mouse_pos.1,
                    )
                    .is_some() =>
            {
                let viewport = Rect {
                    x: 0.0,
                    y: 0.0,
                    w: size.width as f32 / scale_factor,
                    h: size.height as f32 / scale_factor,
                };
                if let Some(app) = dock_bar.hit_test(viewport, mouse_pos.0, mouse_pos.1) {
                    match app.app_id() {
                        None => toggle_launcher(&mut launcher),
                        Some(id) => {
                            handle_launcher_action(
                                Some(LauncherAction::Launch(id)),
                                &mut launcher,
                                size,
                                scale_factor,
                            );
                        }
                    }
                    window.request_redraw();
                }
            }

            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } if dock.visible => {
                if state == ElementState::Pressed {
                    let logical_w = (size.width as f32 / scale_factor) as u32;
                    let logical_h = (size.height as f32 / scale_factor) as u32;
                    let layout = layout_for(mode, logical_w, logical_h);

                    // Build dock entries from WindowManager
                    let entries = build_dock_entries(&wm);

                    // Check hit regions from the canvas display list
                    // Since the dock uses its own hit regions, we check region IDs
                    let (mx, my) = mouse_pos;
                    let action =
                        dock_hit_test_by_position(&dock, &entries, mx, my, layout.app_viewport);

                    match action {
                        Some(DockAction::FocusWindow(id)) => {
                            wm.focus(id);
                            dock.hide();
                            if let Some(mw) = wm.focused_ref() {
                                shell_bar.app_name = mw.content.label().to_string();
                            }
                        }
                        Some(DockAction::CloseWindow(id)) => {
                            if let Some(mw) = wm.get_mut(id) {
                                match &mut mw.content {
                                    WindowContent::Terminal { thread, .. }
                                    | WindowContent::TextEdit { thread, .. }
                                    | WindowContent::FileManager { thread, .. }
                                    | WindowContent::Viewer { thread, .. } => {
                                        app_thread::shutdown_thread(thread);
                                    }
                                }
                            }
                            wm.remove(id);
                            if wm.is_empty() {
                                dock.hide();
                            }
                            if let Some(mw) = wm.focused_ref() {
                                shell_bar.app_name = mw.content.label().to_string();
                            }
                        }
                        Some(DockAction::Dismiss) => {
                            dock.hide();
                        }
                        None => {}
                    }
                    window.request_redraw();
                }
            }

            // Launcher click handling
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } if launcher.visible => {
                if state == ElementState::Pressed {
                    let (mx, my) = mouse_pos;
                    let logical_w = (size.width as f32 / scale_factor) as u32;
                    let logical_h = (size.height as f32 / scale_factor) as u32;
                    let layout = layout_for(mode, logical_w, logical_h);
                    let action = launcher.hit_test(mx, my, layout.app_viewport);
                    handle_launcher_action(action, &mut launcher, size, scale_factor);
                    window.request_redraw();
                }
            }

            // Normal left-click handling
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                let (mx, my) = mouse_pos;

                // Focus switching: click inside a window → bring it to front
                if state == ElementState::Pressed {
                    if let Some(hit_id) = wm.window_at(mx, my) {
                        if wm.focused_id() != Some(hit_id) {
                            wm.focus(hit_id);
                            if let Some(mw) = wm.focused_ref() {
                                shell_bar.app_name = mw.content.label().to_string();
                            }
                            window.request_redraw();
                        }
                    }
                }

                let focused_id = wm.focused_id();

                // Forward clicks to the focused app's content area.
                let mut content_consumed = false;
                if let Some(id) = focused_id {
                    if let Some(mw) = wm.get_mut(id) {
                        // Forward clicks to text editor content area.
                        if !content_consumed
                            && let WindowContent::TextEdit {
                                state: te_state, ..
                            } = &mw.content
                        {
                            let cr = mw.app_window.content_rect();
                            let in_content =
                                mx >= cr.x && mx < cr.x + cr.w && my >= cr.y && my < cr.y + cr.h;
                            if state == ElementState::Pressed && in_content {
                                // Translate to editor-local coordinates
                                let lx = mx - cr.x;
                                let ly = my - cr.y;
                                mw.content.send_input(AppInput::TextEditMouseClick {
                                    x: lx,
                                    y: ly,
                                    double: false,
                                });
                                window.request_redraw();
                                content_consumed = true;
                            } else if state == ElementState::Released
                                && te_state.lock().unwrap().app.is_mouse_selecting()
                            {
                                mw.content.send_input(AppInput::TextEditMouseRelease);
                                window.request_redraw();
                            }
                        }

                        // Forward clicks to file manager content area.
                        if !content_consumed
                            && matches!(mw.content, WindowContent::FileManager { .. })
                        {
                            let cr = mw.app_window.content_rect();
                            let in_content =
                                mx >= cr.x && mx < cr.x + cr.w && my >= cr.y && my < cr.y + cr.h;
                            if state == ElementState::Pressed && in_content {
                                let lx = mx - cr.x;
                                let ly = my - cr.y;
                                let now = std::time::Instant::now();
                                let is_double = last_fm_click
                                    .map(|(t, px, py)| {
                                        now.duration_since(t).as_millis() < 400
                                            && (lx - px).abs() < 5.0
                                            && (ly - py).abs() < 5.0
                                    })
                                    .unwrap_or(false);
                                if is_double {
                                    last_fm_click = None;
                                    mw.content.send_input(AppInput::FileManagerDoubleClick {
                                        x: lx,
                                        y: ly,
                                    });
                                } else {
                                    last_fm_click = Some((now, lx, ly));
                                    mw.content
                                        .send_input(AppInput::FileManagerClick { x: lx, y: ly });
                                }
                                window.request_redraw();
                                content_consumed = true;
                            }
                        }

                        // Forward clicks to viewer content area.
                        if !content_consumed && matches!(mw.content, WindowContent::Viewer { .. }) {
                            let cr = mw.app_window.content_rect();
                            let in_content =
                                mx >= cr.x && mx < cr.x + cr.w && my >= cr.y && my < cr.y + cr.h;
                            if state == ElementState::Pressed && in_content {
                                let lx = mx - cr.x;
                                let ly = my - cr.y;
                                mw.content
                                    .send_input(AppInput::ViewerClick { x: lx, y: ly });
                                window.request_redraw();
                                content_consumed = true;
                            }
                        }
                    }
                }

                if !content_consumed && state == ElementState::Pressed {
                    // Hit-test windows for buttons, resize edges, title bar drag
                    use crate::app_window::WindowButton;
                    let mut hit_btn: Option<(WindowId, WindowButton)> = None;
                    let mut hit_edge: Option<(WindowId, ResizeEdge)> = None;
                    let mut hit_drag: Option<(WindowId, f32, f32)> = None;

                    for mw in wm.iter_front_to_back() {
                        if hit_btn.is_some() || hit_edge.is_some() || hit_drag.is_some() {
                            break;
                        }
                        if let Some(btn) = mw.app_window.hit_test_buttons(mx, my) {
                            hit_btn = Some((mw.id, btn));
                        } else if let Some(edge) = mw.app_window.hit_test_resize(mx, my) {
                            hit_edge = Some((mw.id, edge));
                        } else if mw.app_window.hit_test_title_bar(mx, my) {
                            hit_drag = Some((
                                mw.id,
                                mx - mw.app_window.bounds.x,
                                my - mw.app_window.bounds.y,
                            ));
                        }
                    }

                    if let Some((target_id, btn)) = hit_btn {
                        match btn {
                            WindowButton::Close => {
                                if let Some(mw) = wm.get_mut(target_id) {
                                    match &mut mw.content {
                                        WindowContent::Terminal { thread, .. }
                                        | WindowContent::TextEdit { thread, .. }
                                        | WindowContent::FileManager { thread, .. }
                                        | WindowContent::Viewer { thread, .. } => {
                                            app_thread::shutdown_thread(thread);
                                        }
                                    }
                                }
                                wm.remove(target_id);
                                drag_offset = None;
                                resize_state = None;
                                if let Some(mw) = wm.focused_ref() {
                                    shell_bar.app_name = mw.content.label().to_string();
                                }
                                window.request_redraw();
                            }
                            WindowButton::Maximize => {
                                let lw = (size.width as f32 / scale_factor) as u32;
                                let lh = (size.height as f32 / scale_factor) as u32;
                                let viewport = layout_for(mode, lw, lh).app_viewport;
                                if let Some(mw) = wm.get_mut(target_id) {
                                    mw.app_window.toggle_maximize(viewport);
                                    if let WindowContent::Terminal { ref state, .. } = mw.content {
                                        let s = state.lock().unwrap();
                                        let cr = mw.app_window.content_rect();
                                        let cols = s
                                            .renderer
                                            .metrics
                                            .cols_for_width(cr.w, s.renderer.cfg.padding);
                                        let rows = s
                                            .renderer
                                            .metrics
                                            .rows_for_height(cr.h, s.renderer.cfg.padding);
                                        drop(s);
                                        mw.content
                                            .send_input(AppInput::TerminalResize { cols, rows });
                                    }
                                }
                                window.request_redraw();
                            }
                            _ => {}
                        }
                    } else if let Some(target_edge) = hit_edge {
                        resize_state = Some(target_edge);
                    } else if let Some((target_id, ox, oy)) = hit_drag {
                        let now = std::time::Instant::now();
                        let is_double = last_title_bar_click
                            .map(|t| now.duration_since(t).as_millis() < 400)
                            .unwrap_or(false);
                        last_title_bar_click = Some(now);

                        if is_double {
                            last_title_bar_click = None;
                            let lw = (size.width as f32 / scale_factor) as u32;
                            let lh = (size.height as f32 / scale_factor) as u32;
                            let viewport = layout_for(mode, lw, lh).app_viewport;
                            if let Some(mw) = wm.get_mut(target_id) {
                                mw.app_window.toggle_maximize(viewport);
                                if let WindowContent::Terminal { ref state, .. } = mw.content {
                                    let s = state.lock().unwrap();
                                    let cr = mw.app_window.content_rect();
                                    let cols = s
                                        .renderer
                                        .metrics
                                        .cols_for_width(cr.w, s.renderer.cfg.padding);
                                    let rows = s
                                        .renderer
                                        .metrics
                                        .rows_for_height(cr.h, s.renderer.cfg.padding);
                                    drop(s);
                                    mw.content
                                        .send_input(AppInput::TerminalResize { cols, rows });
                                }
                            }
                            window.request_redraw();
                        } else {
                            drag_offset = Some((target_id, ox, oy));
                        }
                    }
                } else if !content_consumed {
                    drag_offset = None;
                    resize_state = None;
                }
            }

            // Right/middle button → Canvas3D pan
            WindowEvent::MouseInput {
                state,
                button:
                    button @ (winit::event::MouseButton::Right | winit::event::MouseButton::Middle),
                ..
            } => {
                let (mx, my) = mouse_pos;
                if let Some(mw) = wm.focused_mut() {
                    // Right-click in FileManager → context menu
                    if button == winit::event::MouseButton::Right
                        && matches!(mw.content, WindowContent::FileManager { .. })
                    {
                        let cr = mw.app_window.content_rect();
                        let in_content =
                            mx >= cr.x && mx < cr.x + cr.w && my >= cr.y && my < cr.y + cr.h;
                        if state == ElementState::Pressed && in_content {
                            let lx = mx - cr.x;
                            let ly = my - cr.y;
                            mw.content
                                .send_input(AppInput::FileManagerRightClick { x: lx, y: ly });
                            window.request_redraw();
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { .. } if launcher.visible || dock.visible => {
                // Suppress scroll when overlays are open
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let (mx, my) = mouse_pos;

                // Find the topmost window under the cursor
                if let Some(hit_id) = wm.window_at(mx, my) {
                    if let Some(mw) = wm.get_mut(hit_id) {
                        match &mw.content {
                            WindowContent::Terminal { state, .. } => {
                                let ch = state.lock().unwrap().renderer.metrics.cell_height;
                                match delta {
                                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                                        let n = y.abs().max(1.0) as usize;
                                        if y > 0.0 {
                                            mw.content.send_input(AppInput::TerminalScrollUp(n));
                                        } else if y < 0.0 {
                                            mw.content.send_input(AppInput::TerminalScrollDown(n));
                                        }
                                    }
                                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                        scroll_accum += pos.y as f32;
                                        let lines = (scroll_accum / ch).trunc() as i32;
                                        if lines != 0 {
                                            scroll_accum -= lines as f32 * ch;
                                            if lines > 0 {
                                                mw.content.send_input(AppInput::TerminalScrollUp(
                                                    lines as usize,
                                                ));
                                            } else {
                                                mw.content.send_input(
                                                    AppInput::TerminalScrollDown((-lines) as usize),
                                                );
                                            }
                                        }
                                    }
                                }
                                window.request_redraw();
                            }
                            WindowContent::TextEdit { .. } => {
                                let dy = match delta {
                                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 20.0,
                                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                                };
                                mw.content.send_input(AppInput::TextEditScroll { dy });
                                window.request_redraw();
                            }
                            WindowContent::FileManager { .. } => {
                                let dy = match delta {
                                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                        pos.y as f32 / 20.0
                                    }
                                };
                                mw.content.send_input(AppInput::FileManagerScroll { dy });
                                window.request_redraw();
                            }
                            WindowContent::Viewer { .. } => {
                                let dy = match delta {
                                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                        pos.y as f32 / 20.0
                                    }
                                };
                                mw.content.send_input(AppInput::ViewerScroll { dy });
                                window.request_redraw();
                            }
                        }
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let ctrl_key = current_modifiers.state().control_key();

                // Ctrl+Tab cycles focus between windows
                if power_menu.visible {
                    // Any key closes the power menu; the key does nothing else.
                    power_menu.visible = false;
                    window.request_redraw();
                } else if ctrl_key && event.logical_key == Key::Named(NamedKey::Tab) {
                    wm.cycle_focus();
                    if let Some(mw) = wm.focused_ref() {
                        shell_bar.app_name = mw.content.label().to_string();
                    }
                    window.request_redraw();
                // Cmd+P toggles launcher
                } else if mode.is_shell()
                    && platform.is_launcher_shortcut(&event, &current_modifiers)
                {
                    toggle_launcher(&mut launcher);
                    dock.hide(); // hide dock when launcher opens
                    window.request_redraw();
                } else if mode.is_shell() && platform.is_dock_shortcut(&event, &current_modifiers) {
                    dock.toggle();
                    launcher.visible = false; // hide launcher when dock opens
                    window.request_redraw();
                } else if dock.visible {
                    // Escape dismisses dock
                    if event.logical_key == Key::Named(NamedKey::Escape) {
                        dock.hide();
                        window.request_redraw();
                    }
                } else if launcher.visible {
                    // Launcher consumes keyboard input
                    let lk = match event.logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => Some(LauncherKey::Escape),
                        Key::Named(NamedKey::Enter) => Some(LauncherKey::Enter),
                        Key::Named(NamedKey::ArrowLeft) => Some(LauncherKey::ArrowLeft),
                        Key::Named(NamedKey::ArrowRight) => Some(LauncherKey::ArrowRight),
                        Key::Named(NamedKey::ArrowUp) => Some(LauncherKey::ArrowUp),
                        Key::Named(NamedKey::ArrowDown) => Some(LauncherKey::ArrowDown),
                        Key::Named(NamedKey::Backspace) => Some(LauncherKey::Backspace),
                        Key::Character(ch) => ch.chars().next().map(LauncherKey::Char),
                        _ => None,
                    };
                    let action = lk.and_then(|k| launcher.handle_key(k));
                    handle_launcher_action(action, &mut launcher, size, scale_factor);
                    window.request_redraw();
                } else {
                    // Route keyboard to focused window
                    if let Some(mw) = wm.focused_mut() {
                        let is_terminal = matches!(mw.content, WindowContent::Terminal { .. });
                        let is_textedit = matches!(mw.content, WindowContent::TextEdit { .. });
                        let is_filemanager =
                            matches!(mw.content, WindowContent::FileManager { .. });
                        let is_viewer = matches!(mw.content, WindowContent::Viewer { .. });

                        if is_terminal {
                            let ctrl = current_modifiers.state().control_key();
                            let key = match event.logical_key.as_ref() {
                                Key::Named(NamedKey::Enter) => Some(TerminalKey::Enter),
                                Key::Named(NamedKey::Backspace) => Some(TerminalKey::Backspace),
                                Key::Named(NamedKey::Tab) => Some(TerminalKey::Tab),
                                Key::Named(NamedKey::Escape) => Some(TerminalKey::Escape),
                                Key::Named(NamedKey::ArrowUp) => Some(TerminalKey::ArrowUp),
                                Key::Named(NamedKey::ArrowDown) => Some(TerminalKey::ArrowDown),
                                Key::Named(NamedKey::ArrowLeft) => Some(TerminalKey::ArrowLeft),
                                Key::Named(NamedKey::ArrowRight) => Some(TerminalKey::ArrowRight),
                                Key::Named(NamedKey::Home) => Some(TerminalKey::Home),
                                Key::Named(NamedKey::End) => Some(TerminalKey::End),
                                Key::Named(NamedKey::PageUp) => Some(TerminalKey::PageUp),
                                Key::Named(NamedKey::PageDown) => Some(TerminalKey::PageDown),
                                Key::Named(NamedKey::Delete) => Some(TerminalKey::Delete),
                                Key::Named(NamedKey::Space) => Some(TerminalKey::Char(' ')),
                                Key::Character(ch) => {
                                    if ctrl {
                                        ch.chars().next().map(TerminalKey::Ctrl)
                                    } else {
                                        ch.chars().next().map(TerminalKey::Char)
                                    }
                                }
                                _ => None,
                            };
                            if let Some(k) = key {
                                mw.content.send_input(AppInput::TerminalKey(k));
                                window.request_redraw();
                            }
                        } else if is_textedit {
                            mw.content.send_input(AppInput::TextEditKey {
                                key: event.logical_key.clone(),
                                modifiers: current_modifiers,
                            });
                            window.request_redraw();
                        } else if is_filemanager {
                            let fm_key = match event.logical_key.as_ref() {
                                Key::Named(NamedKey::Escape) => {
                                    Some(orus_filemanager::FileManagerKey::Escape)
                                }
                                Key::Named(NamedKey::Enter) => {
                                    Some(orus_filemanager::FileManagerKey::Enter)
                                }
                                Key::Named(NamedKey::Delete) => {
                                    Some(orus_filemanager::FileManagerKey::Delete)
                                }
                                Key::Named(NamedKey::Backspace) => {
                                    Some(orus_filemanager::FileManagerKey::Backspace)
                                }
                                Key::Named(NamedKey::F2) => {
                                    Some(orus_filemanager::FileManagerKey::F2)
                                }
                                Key::Named(NamedKey::ArrowUp) => {
                                    Some(orus_filemanager::FileManagerKey::ArrowUp)
                                }
                                Key::Named(NamedKey::ArrowDown) => {
                                    Some(orus_filemanager::FileManagerKey::ArrowDown)
                                }
                                Key::Named(NamedKey::ArrowLeft) => {
                                    Some(orus_filemanager::FileManagerKey::ArrowLeft)
                                }
                                Key::Named(NamedKey::ArrowRight) => {
                                    Some(orus_filemanager::FileManagerKey::ArrowRight)
                                }
                                Key::Character(ch) => ch
                                    .chars()
                                    .next()
                                    .map(orus_filemanager::FileManagerKey::Char),
                                _ => None,
                            };
                            if let Some(k) = fm_key {
                                mw.content.send_input(AppInput::FileManagerKey { key: k });
                                window.request_redraw();
                            }
                        } else if is_viewer {
                            let iv_key = match event.logical_key.as_ref() {
                                Key::Named(NamedKey::Escape) => {
                                    Some(orus_viewer::ViewerKey::Escape)
                                }
                                Key::Named(NamedKey::Enter) => {
                                    Some(orus_viewer::ViewerKey::Enter)
                                }
                                Key::Named(NamedKey::ArrowLeft) => {
                                    Some(orus_viewer::ViewerKey::ArrowLeft)
                                }
                                Key::Named(NamedKey::ArrowRight) => {
                                    Some(orus_viewer::ViewerKey::ArrowRight)
                                }
                                Key::Named(NamedKey::ArrowUp) => {
                                    Some(orus_viewer::ViewerKey::ArrowUp)
                                }
                                Key::Named(NamedKey::ArrowDown) => {
                                    Some(orus_viewer::ViewerKey::ArrowDown)
                                }
                                _ => None,
                            };
                            if let Some(k) = iv_key {
                                mw.content.send_input(AppInput::ViewerKey { key: k });
                                window.request_redraw();
                            }
                        }
                    }
                }
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

                let logical_w = (size.width as f32 / scale_factor) as u32;
                let logical_h = (size.height as f32 / scale_factor) as u32;
                let layout = layout_for(mode, logical_w, logical_h);

                let mut canvas = surf.begin_frame(size.width, size.height);
                canvas.set_text_provider(provider.clone());
                canvas.clear(crate::palette::color(crate::palette::get().surface));
                if mode.draws_wallpaper()
                    && let Some(path) = &wallpaper
                {
                    canvas.draw_image(
                        path.clone(),
                        [0.0, 0.0],
                        [logical_w as f32, logical_h as f32],
                        jag_draw::ImageFitMode::Cover,
                        WALLPAPER_Z,
                    );
                }

                // Shell bar (always visible)
                if mode.is_shell() {
                    shell_bar.refresh_status();
                    shell_bar.render(&mut canvas, layout.shell_bar);
                }

                // Render all windows back-to-front
                let mut needs_redraw = false;
                for mw in wm.iter_back_to_front_mut() {
                    mw.app_window.render(&mut canvas);

                    match &mut mw.content {
                        WindowContent::Terminal { state, thread } => {
                            let mut s = state.lock().unwrap();
                            let cr = mw.app_window.content_rect();
                            s.renderer
                                .render(&s.core, &mut canvas, cr, mw.app_window.z_base + 10);
                            if s.core.is_alive()
                                || thread
                                    .flags
                                    .needs_redraw
                                    .swap(false, std::sync::atomic::Ordering::Relaxed)
                            {
                                needs_redraw = true;
                            }
                        }
                        WindowContent::TextEdit { state, thread } => {
                            let mut s = state.lock().unwrap();
                            let cr = mw.app_window.content_rect();
                            canvas.push_clip_rect(cr);
                            canvas.set_text_provider(s.app.provider.clone());
                            s.app.set_origin(cr.x, cr.y);
                            s.app.update_viewport(cr.w, cr.h);
                            s.app
                                .render(&mut canvas, cr.w, cr.h, mw.app_window.z_base + 10);
                            canvas.pop_clip();
                            if thread
                                .flags
                                .needs_redraw
                                .swap(false, std::sync::atomic::Ordering::Relaxed)
                            {
                                needs_redraw = true;
                            }
                        }
                        WindowContent::FileManager { state, thread } => {
                            let mut s = state.lock().unwrap();
                            let cr = mw.app_window.content_rect();
                            canvas.push_clip_rect(cr);
                            canvas.set_text_provider(s.app.provider.clone());
                            s.app.set_origin(cr.x, cr.y);
                            s.app.update_viewport(cr.w, cr.h);
                            s.app
                                .render(&mut canvas, cr.w, cr.h, mw.app_window.z_base + 10);
                            canvas.pop_clip();
                            if s.app.needs_redraw()
                                || thread
                                    .flags
                                    .needs_redraw
                                    .swap(false, std::sync::atomic::Ordering::Relaxed)
                            {
                                needs_redraw = true;
                            }
                        }
                        WindowContent::Viewer { state, thread } => {
                            let mut s = state.lock().unwrap();
                            let cr = mw.app_window.content_rect();
                            canvas.push_clip_rect(cr);
                            canvas.set_text_provider(s.app.provider.clone());
                            s.app.set_origin(cr.x, cr.y);
                            s.app.update_viewport(cr.w, cr.h);
                            s.app
                                .render(&mut canvas, cr.w, cr.h, mw.app_window.z_base + 10);
                            canvas.pop_clip();

                            drop(s);
                            if thread
                                .flags
                                .needs_redraw
                                .swap(false, std::sync::atomic::Ordering::Relaxed)
                            {
                                needs_redraw = true;
                            }
                        }
                    }
                }

                // Remove windows whose apps have exited
                let exited_ids: Vec<WindowId> = wm
                    .windows()
                    .filter(|mw| mw.content.has_exited())
                    .map(|mw| mw.id)
                    .collect();
                for id in exited_ids {
                    if let Some(mw) = wm.get_mut(id) {
                        match &mut mw.content {
                            WindowContent::Terminal { thread, .. }
                            | WindowContent::TextEdit { thread, .. }
                            | WindowContent::FileManager { thread, .. }
                            | WindowContent::Viewer { thread, .. } => {
                                app_thread::shutdown_thread(thread);
                            }
                        }
                    }
                    wm.remove(id);
                    if let Some(mw) = wm.focused_ref() {
                        shell_bar.app_name = mw.content.label().to_string();
                    }
                }
                // An app in a window of its own ends with its content (a terminal's shell exiting).
                if matches!(mode, RunMode::App { .. }) && wm.windows().next().is_none() {
                    elwt.exit();
                }

                // The persistent dock, above windows and below the overlays
                let screen = Rect {
                    x: 0.0,
                    y: 0.0,
                    w: logical_w as f32,
                    h: logical_h as f32,
                };
                let mut dock_moving = false;
                if mode.is_shell() {
                    dock_target = dock_bar.wants_to_show(
                        screen,
                        mouse_pos,
                        wm.windows().any(|mw| mw.app_window.is_maximized()),
                    );
                    dock_moving =
                        dock_bar.tick(dock_target, appearance_watch.current.reduce_motion);
                    dock_bar.render(&mut canvas, screen, &running_apps(&wm));
                }

                // Render Orus OS dock overlay
                if dock.visible {
                    let entries = build_dock_entries(&wm);
                    dock.render(
                        &mut canvas,
                        &entries,
                        layout.app_viewport,
                        provider.as_ref(),
                    );
                }

                // Launcher overlay (renders above everything when visible)
                if mode.is_shell() {
                    launcher.render(&mut canvas, layout.app_viewport);
                    power_menu.render(&mut canvas, layout.shell_bar, mouse_pos);
                }

                // Present
                let _ = surf.end_frame(frame, canvas);

                if needs_redraw || launcher.needs_redraw() || dock.visible || dock_moving {
                    window.request_redraw();
                }
            }
            _ => {}
        },
        Event::AboutToWait => {
            if appearance_watch.poll() {
                crate::palette::set(palette_for(&appearance_watch.current));
                wallpaper = wallpaper_for(&appearance_watch.current);
                window.request_redraw();
            }
            elwt.set_control_flow(ControlFlow::WaitUntil(
                std::time::Instant::now() + IDLE_REDRAW,
            ));
        }
        // The idle tick: redraw once a second for the clock and the dock's dots.
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => window.request_redraw(),
        Event::LoopExiting if mode.is_shell() => window_state::save(&sandbox, &win_state),
        _ => {}
    })?;

    Ok(())
}

fn palette_for(appearance: &orus_core::appearance::Appearance) -> crate::palette::Palette {
    crate::palette::Palette::new(appearance.theme, appearance.accent)
}

/// The wallpaper image for `appearance`, or `None` (a flat desktop color) if the
/// file is not installed.
fn wallpaper_for(appearance: &orus_core::appearance::Appearance) -> Option<std::path::PathBuf> {
    let file = appearance.effective_wallpaper().file();
    let path = crate::paths::asset(&format!("wallpapers/{file}"));
    if path.is_none() {
        log::warn!("no wallpaper image {file}; using a flat desktop color");
    }
    path
}

/// Dock apps with a window open now: shell windows by app id, and the programs
/// this shell started and has not seen exit.
fn running_apps(wm: &WindowManager) -> std::collections::HashSet<DockApp> {
    let mut running = std::collections::HashSet::new();
    for mw in wm.windows() {
        match mw.app_id.as_str() {
            "filemanager" => running.insert(DockApp::Files),
            "terminal" => running.insert(DockApp::Terminal),
            _ => false,
        };
    }
    running.extend(DockApp::running_from_processes());
    running
}

/// Open or close the launcher, refreshing the installed apps it lists.
fn toggle_launcher(launcher: &mut Launcher) {
    launcher.toggle();
    if launcher.visible {
        let apps = orus_core::installed_apps::list(&crate::paths::apps_dir());
        launcher.set_installed(&apps);
        launcher.set_desktop(&crate::launcher::desktop_apps());
    }
}

/// Build dock entries from the WindowManager.
fn build_dock_entries(wm: &WindowManager) -> Vec<DockEntry> {
    let focused_id = wm.focused_id();
    wm.windows()
        .map(|mw| DockEntry {
            window_id: mw.id,
            title: mw.app_window.title.clone(),
            is_terminal: matches!(mw.content, WindowContent::Terminal { .. }),
            focused: Some(mw.id) == focused_id,
        })
        .collect()
}

/// Position-based dock hit test (since we can't use HitIndex from the dock's
/// own rendering — it happens after canvas drawing).
fn dock_hit_test_by_position(
    dock: &OrusDock,
    entries: &[DockEntry],
    mx: f32,
    my: f32,
    viewport: Rect,
) -> Option<DockAction> {
    if !dock.visible || entries.is_empty() {
        return None;
    }

    // Panel geometry (must match dock.rs rendering)
    let n = entries.len() as f32;
    let item_w = 120.0;
    let item_gap = 16.0;
    let panel_padding_h = 24.0;
    let panel_padding_v = 20.0;
    let content_w = n * item_w + (n - 1.0).max(0.0) * item_gap;
    let panel_w = content_w + panel_padding_h * 2.0;
    let panel_h = 80.0 + panel_padding_v * 2.0 + 32.0;
    let panel_x = viewport.x + (viewport.w - panel_w) / 2.0;
    let panel_y = viewport.y + (viewport.h - panel_h) / 2.0;

    // Check if inside panel
    let in_panel =
        mx >= panel_x && mx < panel_x + panel_w && my >= panel_y && my < panel_y + panel_h;

    if !in_panel {
        // Click on scrim → dismiss
        return Some(DockAction::Dismiss);
    }

    // Check individual items
    let items_y = panel_y + panel_padding_v + 32.0;
    let items_start_x = panel_x + panel_padding_h;

    for (i, entry) in entries.iter().enumerate() {
        let item_x = items_start_x + i as f32 * (item_w + item_gap);
        let item_h = 80.0;

        // Item click → focus window
        if mx >= item_x && mx < item_x + item_w && my >= items_y && my < items_y + item_h {
            return Some(DockAction::FocusWindow(entry.window_id));
        }
    }

    None // Click on panel background (absorbed)
}

/// Process a launcher action: dismiss, or start an app. Terminal, Files, Text Edit
/// and Viewer each start as a window of their own.
fn handle_launcher_action(
    action: Option<LauncherAction>,
    launcher: &mut Launcher,
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f32,
) {
    let shell = (
        (size.width as f32 / scale_factor) as f64,
        (size.height as f32 / scale_factor) as f64,
    );
    match action {
        Some(LauncherAction::Dismiss) => launcher.visible = false,
        Some(LauncherAction::Launch(app)) => {
            launcher.visible = false;
            crate::start::start(app, shell);
        }
        None => {}
    }
}

/// Put `kind`'s content in a frameless window filling `viewport`. Returns `None`
/// if the app could not start.
fn open_app(
    kind: AppKind,
    wm: &mut WindowManager,
    provider: &std::sync::Arc<dyn jag_draw::TextProvider>,
    sandbox: &SandboxContext,
    viewport: Rect,
) -> Option<WindowId> {
    let aw = AppWindow::frameless(kind.title(), viewport);
    match kind {
        AppKind::Terminal => {
            let cr = aw.content_rect();
            let renderer = TerminalRenderer::new(provider.as_ref(), TerminalConfig::default());
            let cols = renderer.metrics.cols_for_width(cr.w, renderer.cfg.padding);
            let rows = renderer.metrics.rows_for_height(cr.h, renderer.cfg.padding);
            match TerminalCore::new(cols, rows, &sandbox.default_terminal_cwd()) {
                Ok(mut core) => {
                    // The shell reads this once it starts; the pty holds the text until then.
                    if let Some(command) =
                        std::env::var_os(crate::external_app::TERMINAL_COMMAND_ENV)
                    {
                        core.write_bytes(command.to_string_lossy().as_bytes());
                        core.write_bytes(b"\n");
                    }
                    let (_state, content) = app_thread::create_terminal(core, renderer);
                    Some(wm.add("terminal", aw, content))
                }
                Err(e) => {
                    log::error!("Failed to start the terminal: {e}");
                    None
                }
            }
        }
        AppKind::TextEdit => match TextEditApp::new_with_provider(provider.clone()) {
            Ok(app) => {
                let (_state, content) = app_thread::create_textedit(app);
                Some(wm.add("textedit", aw, content))
            }
            Err(e) => {
                log::error!("Failed to create TextEditApp: {e}");
                None
            }
        },
        AppKind::Files => {
            let home = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| sandbox.root().to_path_buf());
            let app = orus_filemanager::FileManagerApp::new(home, provider.clone());
            let (_state, content) = app_thread::create_filemanager(app);
            Some(wm.add("filemanager", aw, content))
        }
        AppKind::Viewer => {
            // Like Files, the viewer browses the home folder; it opens on Pictures
            // when there is one.
            let home = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| sandbox.root().to_path_buf());
            let pictures = home.join("Pictures");
            let app = if pictures.is_dir() {
                orus_viewer::ViewerApp::new_with_folder(home, pictures, provider.clone())
            } else {
                orus_viewer::ViewerApp::new(home, provider.clone())
            };
            let (_state, content) = app_thread::create_viewer(app);
            Some(wm.add("viewer", aw, content))
        }
    }
}
