//! The full-screen surface that carries the launcher or the power menu.

use orus_shell::launcher::Launcher;
use orus_shell::power_menu::PowerMenu;

use crate::surface::Surf;

pub enum OverlayKind {
    Launcher(Launcher),
    Power(PowerMenu),
}

pub struct Overlay {
    pub surf: Surf,
    pub kind: OverlayKind,
    pub pointer: (f32, f32),
}
