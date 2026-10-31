//! The panel's Wayland state: three layer surfaces (bar, dock, and an overlay while
//! the launcher or power menu is open), the pointer and keyboard, and the window list.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use jag_draw::{Rect, SubpixelOrientation};
use orus_shell::appearance_watch::AppearanceWatch;
use orus_shell::dock_bar::{BAND_HEIGHT, DockBar, dock_width};
use orus_shell::launcher::{Launcher, LauncherAction};
use orus_shell::layout::SHELL_BAR_HEIGHT;
use orus_shell::power_menu::PowerMenu;
use orus_shell::shell_bar::ShellBar;
use smithay_client_toolkit::compositor::{CompositorState, Region};
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle};
use smithay_client_toolkit::reexports::calloop::{Interest, Mode, PostAction};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface,
};
use wayland_client::globals::{GlobalList, registry_queue_init};
use wayland_client::protocol::{wl_keyboard, wl_pointer, wl_surface};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1;
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_manager_v1::WpCursorShapeManagerV1;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1;

use crate::dock_model::{self, DockItem};
use crate::dock_state::DockVisibility;
use crate::gpu::{self, Gpu};
use crate::overlay::{Overlay, OverlayKind};
use crate::render::Renderer;
use crate::surface::Surf;
use crate::toplevels::Toplevels;

/// Height of the strip along the bottom edge that brings a hidden dock back.
const EDGE_HEIGHT: u32 = 4;

/// How often the panel wakes to hide the dock and refresh the clock.
pub(crate) const TICK: Duration = Duration::from_millis(200);
/// Ticks between refreshes of the clock, the status icons and the running dots.
const TICKS_PER_REFRESH: u32 = 5;

pub(crate) const BTN_LEFT: u32 = 0x110;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    Bar,
    Dock,
    Overlay,
}

pub struct State {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) compositor: CompositorState,
    pub(crate) layer_shell: LayerShell,
    pub(crate) conn: Connection,
    pub(crate) qh: QueueHandle<State>,
    pub(crate) cursor_shapes: Option<WpCursorShapeManagerV1>,
    pub(crate) cursor_device: Option<WpCursorShapeDeviceV1>,

    pub(crate) instance: std::sync::Arc<wgpu::Instance>,
    pub(crate) gpu: Option<Gpu>,
    pub(crate) provider: std::sync::Arc<dyn jag_draw::TextProvider>,

    pub(crate) bar: Surf,
    pub(crate) shell_bar: ShellBar,
    pub(crate) dock: Surf,
    pub(crate) dock_bar: DockBar,
    pub(crate) dock_visibility: DockVisibility,
    /// What the dock shows, from the window list.
    pub(crate) dock_items: Vec<DockItem>,
    /// Each window's protocol object, for raising it.
    pub(crate) handles: HashMap<u32, ZwlrForeignToplevelHandleV1>,
    pub(crate) overlay: Option<Overlay>,

    pub(crate) pointer: Option<wl_pointer::WlPointer>,
    pub(crate) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(crate) toplevels: Toplevels,
    pub(crate) appearance: AppearanceWatch,
    pub(crate) ticks: u32,
    pub(crate) exit: bool,
}

/// The layer surface for `layer`, anchored and sized as given.
#[allow(clippy::too_many_arguments)]
fn new_layer(
    compositor: &CompositorState,
    shell: &LayerShell,
    qh: &QueueHandle<State>,
    layer: Layer,
    namespace: &str,
    anchor: Anchor,
    size: (u32, u32),
    exclusive_zone: i32,
    keyboard: KeyboardInteractivity,
) -> LayerSurface {
    let surface = compositor.create_surface(qh);
    let layer = shell.create_layer_surface(qh, surface, layer, Some(namespace), None);
    layer.set_anchor(anchor);
    layer.set_size(size.0, size.1);
    layer.set_exclusive_zone(exclusive_zone);
    layer.set_keyboard_interactivity(keyboard);
    layer.commit();
    layer
}

/// The dock's surface is as wide as the screen and as tall as the dock with its margin,
/// and never resizes: the dock is drawn centered in it, and what takes pointer input is
/// set by `apply_dock_input`. Resizing a layer surface would drop the pointer's focus.
fn dock_layout() -> (Anchor, (u32, u32)) {
    (
        Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        (0, (BAND_HEIGHT * orus_core::design::SCALE).ceil() as u32),
    )
}

impl Which {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Which::Bar => "bar",
            Which::Dock => "dock",
            Which::Overlay => "overlay",
        }
    }
}

impl State {
    fn new(conn: Connection, globals: &GlobalList, qh: QueueHandle<State>) -> Result<Self> {
        let compositor = CompositorState::bind(globals, &qh).context("no wl_compositor")?;
        let layer_shell = LayerShell::bind(globals, &qh)
            .context("the compositor does not offer wlr-layer-shell")?;
        let cursor_shapes = globals.bind(&qh, 1..=1, ()).ok();
        let _: Option<ZwlrForeignToplevelManagerV1> = globals.bind(&qh, 1..=3, ()).ok().or_else(|| {
            log::warn!("no wlr-foreign-toplevel: the bar cannot name the focused app and the dock never hides");
            None
        });

        let bar_size = (0, (SHELL_BAR_HEIGHT * orus_core::design::SCALE).round() as u32);
        let bar = new_layer(
            &compositor,
            &layer_shell,
            &qh,
            Layer::Top,
            "orus-bar",
            Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            bar_size,
            (SHELL_BAR_HEIGHT * orus_core::design::SCALE).round() as i32,
            KeyboardInteractivity::None,
        );
        let (dock_anchor, dock_size) = dock_layout();
        let dock = new_layer(
            &compositor,
            &layer_shell,
            &qh,
            Layer::Top,
            "orus-dock",
            dock_anchor,
            dock_size,
            0,
            KeyboardInteractivity::None,
        );

        let appearance = AppearanceWatch::new(&orus_shell::paths::orus_data_dir());
        orus_shell::palette::set(orus_shell::palette::Palette::new(
            appearance.current.theme,
            appearance.current.accent,
        ));
        let provider: std::sync::Arc<dyn jag_draw::TextProvider> = std::sync::Arc::new(
            jag_draw::DetirTextProvider::from_system_fonts(SubpixelOrientation::RGB)?,
        );

        Ok(Self {
            registry_state: RegistryState::new(globals),
            seat_state: SeatState::new(globals, &qh),
            output_state: OutputState::new(globals, &qh),
            compositor,
            layer_shell,
            conn,
            qh,
            cursor_shapes,
            cursor_device: None,
            instance: std::sync::Arc::new(wgpu::Instance::default()),
            gpu: None,
            provider,
            bar: Surf::new(bar, bar_size),
            shell_bar: ShellBar::new("Orus OS"),
            dock: Surf::new(dock, dock_size),
            dock_bar: DockBar::new(),
            dock_visibility: DockVisibility::default(),
            dock_items: dock_model::items(&Toplevels::default()),
            handles: HashMap::new(),
            overlay: None,
            pointer: None,
            keyboard: None,
            toplevels: Toplevels::default(),
            appearance,
            ticks: 0,
            exit: false,
        })
    }

    /// The screen size in the units the shell draws in (the compositor's pixels divided by
    /// `design::SCALE`), for sizing the apps the panel starts.
    pub(crate) fn screen(&self) -> (f64, f64) {
        let from_output = self
            .output_state
            .outputs()
            .filter_map(|o| self.output_state.info(&o))
            .find_map(|info| info.logical_size);
        match from_output {
            Some((w, h)) => (
                f64::from(w) / f64::from(orus_core::design::SCALE),
                f64::from(h) / f64::from(orus_core::design::SCALE),
            ),
            None => (1280.0 / f64::from(orus_core::design::SCALE), 720.0 / f64::from(orus_core::design::SCALE)),
        }
    }

    pub(crate) fn which(&self, surface: &wl_surface::WlSurface) -> Option<Which> {
        if surface == self.bar.wl() {
            Some(Which::Bar)
        } else if surface == self.dock.wl() {
            Some(Which::Dock)
        } else if self
            .overlay
            .as_ref()
            .is_some_and(|o| surface == o.surf.wl())
        {
            Some(Which::Overlay)
        } else {
            None
        }
    }

    pub(crate) fn surf_mut(&mut self, which: Which) -> Option<&mut Surf> {
        match which {
            Which::Bar => Some(&mut self.bar),
            Which::Dock => Some(&mut self.dock),
            Which::Overlay => self.overlay.as_mut().map(|o| &mut o.surf),
        }
    }

    /// Make the renderer for a surface that just got its first size, or resize it.
    pub(crate) fn configure_surface(&mut self, which: Which, size: (u32, u32)) {
        let Some(surf) = self.surf_mut(which) else {
            return;
        };
        surf.size = size;
        surf.dirty = true;
        if let Some(renderer) = surf.renderer.as_mut() {
            renderer.resize(size, surf.scale);
            return;
        }
        let (scale, wl) = (surf.scale, surf.wl().clone());
        let made = (|| -> Result<Renderer> {
            let wgpu_surface = gpu::create_surface(&self.instance, &self.conn, &wl)?;
            if self.gpu.is_none() {
                self.gpu = Some(Gpu::new(&self.instance, &wgpu_surface)?);
            }
            let gpu = self.gpu.as_ref().context("no gpu")?;
            Ok(Renderer::new(
                gpu,
                wgpu_surface,
                size,
                scale,
                self.provider.clone(),
            ))
        })();
        match made {
            Ok(renderer) => {
                if let Some(surf) = self.surf_mut(which) {
                    surf.renderer = Some(renderer);
                }
            }
            Err(e) => {
                log::error!("cannot draw a panel surface: {e:#}");
                self.exit = true;
            }
        }
    }

    pub(crate) fn bar_rect(&self) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w: self.bar.size.0 as f32 / orus_core::design::SCALE,
            h: SHELL_BAR_HEIGHT,
        }
    }

    /// The area under the bar, which the launcher fills.
    pub(crate) fn below_bar(&self, size: (f32, f32)) -> Rect {
        Rect {
            x: 0.0,
            y: SHELL_BAR_HEIGHT,
            w: size.0,
            h: (size.1 - SHELL_BAR_HEIGHT).max(0.0),
        }
    }

    pub(crate) fn dock_viewport(&self) -> Rect {
        let (w, h) = self.dock.logical();
        Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
        }
    }

    // ── Opening and closing ────────────────────────────────────────

    pub(crate) fn open_overlay(&mut self, kind: OverlayKind) {
        let layer = new_layer(
            &self.compositor,
            &self.layer_shell,
            &self.qh,
            Layer::Overlay,
            "orus-overlay",
            Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            (0, 0),
            -1,
            KeyboardInteractivity::Exclusive,
        );
        self.overlay = Some(Overlay {
            surf: Surf::new(layer, (0, 0)),
            kind,
            pointer: (0.0, 0.0),
        });
    }

    pub(crate) fn close_overlay(&mut self) {
        self.overlay = None;
    }

    pub(crate) fn toggle_launcher(&mut self) {
        if matches!(
            self.overlay.as_ref().map(|o| &o.kind),
            Some(OverlayKind::Launcher(_))
        ) {
            self.close_overlay();
            return;
        }
        self.close_overlay();
        let mut launcher = Launcher::default();
        launcher.toggle();
        launcher.set_installed(&orus_core::installed_apps::list(
            &orus_shell::paths::apps_dir(),
        ));
        launcher.set_desktop(&orus_shell::launcher::desktop_apps());
        self.open_overlay(OverlayKind::Launcher(launcher));
    }

    pub(crate) fn show_power_menu(&mut self) {
        self.close_overlay();
        self.open_overlay(OverlayKind::Power(PowerMenu { visible: true }));
    }

    pub(crate) fn apply_launcher_action(&mut self, action: Option<LauncherAction>) {
        match action {
            Some(LauncherAction::Dismiss) => self.close_overlay(),
            Some(LauncherAction::Launch(app)) => {
                self.close_overlay();
                orus_shell::start::start(app, self.screen());
            }
            None => {}
        }
    }

    /// Draw the dock out or hidden, and take pointer input where it is: over the dock
    /// and its margin when out, along the bottom edge when hidden.
    pub(crate) fn apply_dock_input(&mut self) {
        let (w, h) = (self.dock.size.0 as i32, self.dock.size.1 as i32);
        if w == 0 {
            return;
        }
        let open = self.dock_visibility.is_open();
        let region = match Region::new(&self.compositor) {
            Ok(region) => region,
            Err(e) => {
                log::warn!("no input region for the dock: {e}");
                return;
            }
        };
        if open {
            let dock_w = (dock_width(self.dock_items.len()) * orus_core::design::SCALE).ceil() as i32;
            region.add((w - dock_w) / 2, 0, dock_w, h);
        } else {
            region.add(0, h - EDGE_HEIGHT as i32, w, EDGE_HEIGHT as i32);
        }
        self.dock.wl().set_input_region(Some(region.wl_region()));
        self.dock.dirty = true;
        self.dock.layer.commit();
    }

    pub(crate) fn toplevels_changed(&mut self) {
        if let Some(title) = self.toplevels.active_title() {
            self.shell_bar.app_name = title.to_string();
        } else {
            self.shell_bar.app_name = "Orus OS".to_string();
        }
        self.bar.dirty = true;
        self.dock_items = dock_model::items(&self.toplevels);
        self.dock_bar.set_tile_count(self.dock_items.len());
        self.dock_visibility
            .set_covered(self.toplevels.covers_screen(), Instant::now());
        self.apply_dock_input();
    }

    /// Bring a window of the dock tile `index` to the front, or start the app if it
    /// has none open.
    fn dock_click(&mut self, index: usize) {
        let Some(item) = self.dock_items.get(index).cloned() else {
            return;
        };
        let Some(kind) = item.kind.as_ref() else {
            self.toggle_launcher();
            return;
        };
        let target = dock_model::window_to_raise(&item, self.toplevels.active_id());
        match (target.and_then(|id| self.handles.get(&id)), kind) {
            (Some(handle), _) => {
                let seat = self.seat_state.seats().next();
                handle.unset_minimized();
                if let Some(seat) = seat {
                    handle.activate(&seat);
                }
            }
            (None, dock_model::Kind::Fixed(app)) => {
                if let Some(id) = app.app_id() {
                    orus_shell::start::start(id, self.screen());
                }
            }
            (None, dock_model::Kind::Extra { .. }) => {}
        }
    }

    // ── Pointer ────────────────────────────────────────────────────

    pub(crate) fn pointer_press(&mut self, which: Which, pos: (f32, f32)) {
        match which {
            Which::Bar => {
                let bar = self.bar_rect();
                if ShellBar::apps_button_contains(bar, pos.0, pos.1) {
                    self.toggle_launcher();
                } else if ShellBar::status_area_contains(bar, pos.0, pos.1) {
                    self.show_power_menu();
                }
            }
            Which::Dock => {
                if let Some(index) = self.dock_bar.hit_tile(self.dock_viewport(), pos.0, pos.1) {
                    self.dock_click(index);
                }
            }
            Which::Overlay => {
                let Some(overlay) = self.overlay.as_ref() else {
                    return;
                };
                match &overlay.kind {
                    OverlayKind::Launcher(launcher) => {
                        let viewport = self.below_bar(overlay.surf.logical());
                        let action = launcher.hit_test(pos.0, pos.1, viewport);
                        self.apply_launcher_action(action);
                    }
                    OverlayKind::Power(_) => {
                        let bar = self.bar_rect();
                        if let Some(action) = PowerMenu::action_at(bar, pos.0, pos.1)
                            && let Err(e) = action.run()
                        {
                            log::error!("{} failed: {e}", action.label());
                        }
                        self.close_overlay();
                    }
                }
            }
        }
    }

    // ── Drawing ────────────────────────────────────────────────────

    pub(crate) fn draw(&mut self) {
        if self.bar.dirty && self.bar.renderer.is_some() {
            self.shell_bar.refresh_status();
            let rect = Rect {
                x: 0.0,
                y: 0.0,
                w: self.bar.size.0 as f32 / orus_core::design::SCALE,
                h: self.bar.size.1 as f32 / orus_core::design::SCALE,
            };
            let bar = &self.shell_bar;
            if let Some(r) = self.bar.renderer.as_mut() {
                r.draw(|canvas| bar.render(canvas, rect));
            }
            self.bar.dirty = false;
        }
        if self.dock.dirty && self.dock.renderer.is_some() {
            let viewport = self.dock_viewport();
            let open = self.dock_visibility.is_open();
            let tiles = dock_model::tiles(&self.dock_items);
            let dock = &self.dock_bar;
            if let Some(r) = self.dock.renderer.as_mut() {
                r.draw(|canvas| {
                    if open {
                        dock.render_tiles(canvas, viewport, &tiles);
                    }
                });
            }
            self.dock.dirty = false;
        }
        let below = self
            .overlay
            .as_ref()
            .map(|o| self.below_bar(o.surf.logical()));
        let bar = self.bar_rect();
        if let (Some(overlay), Some(below)) = (self.overlay.as_mut(), below)
            && overlay.surf.dirty
            && let Some(r) = overlay.surf.renderer.as_mut()
        {
            let (kind, pointer) = (&overlay.kind, overlay.pointer);
            r.draw(|canvas| match kind {
                OverlayKind::Launcher(launcher) => launcher.render(canvas, below),
                OverlayKind::Power(menu) => menu.render(canvas, bar, pointer),
            });
            overlay.surf.dirty = false;
        }
    }

    /// Run the command a client sent over the socket.
    pub(crate) fn read_command(&mut self, stream: std::os::unix::net::UnixStream) {
        use std::io::BufRead;
        // The client writes one line and closes; do not wait on one that does not.
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
        let mut line = String::new();
        if std::io::BufReader::new(stream)
            .read_line(&mut line)
            .is_err()
        {
            return;
        }
        match line.trim() {
            crate::ipc::TOGGLE_LAUNCHER => self.toggle_launcher(),
            other => log::warn!("unknown panel command \"{other}\""),
        }
    }

    // ── Timer ──────────────────────────────────────────────────────

    pub(crate) fn tick(&mut self) {
        if self.dock_visibility.tick(Instant::now()) {
            self.apply_dock_input();
        }
        if let Some(overlay) = self.overlay.as_mut()
            && let OverlayKind::Launcher(launcher) = &overlay.kind
            && launcher.needs_redraw()
        {
            overlay.surf.dirty = true;
        }
        self.ticks += 1;
        if !self.ticks.is_multiple_of(TICKS_PER_REFRESH) {
            return;
        }
        self.bar.dirty = true;
        self.dock.dirty = true;
        if self.appearance.poll() {
            orus_shell::palette::set(orus_shell::palette::Palette::new(
                self.appearance.current.theme,
                self.appearance.current.accent,
            ));
            self.bar.dirty = true;
            self.dock.dirty = true;
            if let Some(o) = self.overlay.as_mut() {
                o.surf.dirty = true;
            }
        }
    }
}

/// Connect to the compositor and run the panel until it closes.
pub fn run() -> Result<()> {
    orus_shell::icons::register();
    let conn = Connection::connect_to_env().context("cannot connect to the Wayland compositor")?;
    let (globals, event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<State> = EventLoop::try_new()?;
    let handle: LoopHandle<State> = event_loop.handle();
    WaylandSource::new(conn.clone(), event_queue)
        .insert(handle.clone())
        .map_err(|e| anyhow::anyhow!("cannot watch the Wayland socket: {e}"))?;
    let mut state = State::new(conn, &globals, qh)?;
    handle
        .insert_source(Timer::from_duration(TICK), |_, _, state| {
            state.tick();
            TimeoutAction::ToDuration(TICK)
        })
        .map_err(|e| anyhow::anyhow!("cannot start the panel timer: {e}"))?;
    let listener = crate::ipc::listen()?;
    handle
        .insert_source(
            Generic::new(listener, Interest::READ, Mode::Level),
            |_, listener, state| {
                while let Ok((stream, _)) = listener.accept() {
                    state.read_command(stream);
                }
                Ok(PostAction::Continue)
            },
        )
        .map_err(|e| anyhow::anyhow!("cannot watch the panel socket: {e}"))?;
    while !state.exit {
        event_loop.dispatch(Some(TICK), &mut state)?;
        state.draw();
    }
    Ok(())
}
