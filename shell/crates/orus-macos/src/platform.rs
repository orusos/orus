use orus_shell::paths;
use orus_shell::platform::PlatformHooks;
use std::path::PathBuf;
use winit::event::{KeyEvent, Modifiers};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

pub struct MacosPlatformHooks;

impl PlatformHooks for MacosPlatformHooks {
    fn startup_tweaks(&self, window: &Window) {
        disable_macos_overscroll(window);
    }

    fn data_dir(&self) -> PathBuf {
        paths::orus_data_dir()
    }

    fn primary_modifier_active(&self, mods: &Modifiers) -> bool {
        mods.state().super_key()
    }

    fn word_jump_modifier_active(&self, mods: &Modifiers) -> bool {
        mods.state().alt_key()
    }

    fn line_jump_modifier_active(&self, mods: &Modifiers) -> bool {
        mods.state().super_key()
    }

    fn is_launcher_shortcut(&self, event: &KeyEvent, mods: &Modifiers) -> bool {
        mods.state().super_key() && event.physical_key == PhysicalKey::Code(KeyCode::KeyP)
    }

    fn is_dock_shortcut(&self, event: &KeyEvent, mods: &Modifiers) -> bool {
        mods.state().super_key() && event.physical_key == PhysicalKey::Code(KeyCode::KeyD)
    }
}

/// Disable macOS elastic overscroll on the window's backing NSView.
///
/// Without this, scrolling past the content boundary causes the entire
/// window to rubber-band bounce — the default macOS behavior for views
/// that receive scroll events without fully consuming them.
fn disable_macos_overscroll(window: &Window) {
    use raw_window_handle::HasWindowHandle;

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let raw_window_handle::RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };

    let ns_view = appkit.ns_view.as_ptr() as *mut objc2::runtime::AnyObject;

    unsafe {
        // Disable elasticity on the enclosing NSScrollView if one exists.
        let scroll_view: *mut objc2::runtime::AnyObject =
            objc2::msg_send![ns_view, enclosingScrollView];
        if !scroll_view.is_null() {
            // NSScrollElasticityNone = 0
            let _: () = objc2::msg_send![scroll_view, setVerticalScrollElasticity: 0i64];
            let _: () = objc2::msg_send![scroll_view, setHorizontalScrollElasticity: 0i64];
        }

        // Also walk up to the NSWindow and disable its content view's
        // scroll elasticity in case the window itself is the source.
        let ns_window: *mut objc2::runtime::AnyObject = objc2::msg_send![ns_view, window];
        if !ns_window.is_null() {
            let content_view: *mut objc2::runtime::AnyObject =
                objc2::msg_send![ns_window, contentView];
            if !content_view.is_null() {
                let inner_scroll: *mut objc2::runtime::AnyObject =
                    objc2::msg_send![content_view, enclosingScrollView];
                if !inner_scroll.is_null() {
                    let _: () = objc2::msg_send![inner_scroll, setVerticalScrollElasticity: 0i64];
                    let _: () = objc2::msg_send![inner_scroll, setHorizontalScrollElasticity: 0i64];
                }
            }
        }
    }
}
