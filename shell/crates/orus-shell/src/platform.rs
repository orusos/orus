use std::path::PathBuf;

use winit::event::{KeyEvent, Modifiers};
use winit::window::Window;

/// OS-specific hooks used by the shared Orus OS runtime.
pub trait PlatformHooks {
    /// Apply any platform-specific tweaks after the window is created.
    fn startup_tweaks(&self, window: &Window);

    /// Base data directory used for host-window persistence.
    fn data_dir(&self) -> PathBuf;

    /// Whether the platform's primary shortcut modifier is currently active.
    fn primary_modifier_active(&self, mods: &Modifiers) -> bool;

    /// Whether the platform's word-navigation modifier is currently active.
    fn word_jump_modifier_active(&self, mods: &Modifiers) -> bool;

    /// Whether the platform's line-jump modifier is currently active.
    fn line_jump_modifier_active(&self, mods: &Modifiers) -> bool;

    /// Whether this event should open the launcher.
    fn is_launcher_shortcut(&self, event: &KeyEvent, mods: &Modifiers) -> bool;

    /// Whether this event should toggle the dock.
    fn is_dock_shortcut(&self, event: &KeyEvent, mods: &Modifiers) -> bool;
}
