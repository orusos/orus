use orus_shell::paths;
use orus_shell::platform::PlatformHooks;
use std::path::PathBuf;
use winit::event::{KeyEvent, Modifiers};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

pub struct LinuxPlatformHooks;

impl PlatformHooks for LinuxPlatformHooks {
    fn startup_tweaks(&self, _window: &Window) {}

    fn data_dir(&self) -> PathBuf {
        paths::orus_data_dir()
    }

    fn primary_modifier_active(&self, mods: &Modifiers) -> bool {
        mods.state().control_key()
    }

    fn word_jump_modifier_active(&self, mods: &Modifiers) -> bool {
        mods.state().control_key()
    }

    fn line_jump_modifier_active(&self, _mods: &Modifiers) -> bool {
        false
    }

    fn is_launcher_shortcut(&self, event: &KeyEvent, mods: &Modifiers) -> bool {
        mods.state().control_key() && event.physical_key == PhysicalKey::Code(KeyCode::KeyP)
    }

    fn is_dock_shortcut(&self, event: &KeyEvent, mods: &Modifiers) -> bool {
        mods.state().control_key() && event.physical_key == PhysicalKey::Code(KeyCode::KeyD)
    }
}
