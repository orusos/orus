//! One layer surface and what draws into it.

use smithay_client_toolkit::shell::{WaylandSurface, wlr_layer::LayerSurface};
use wayland_client::protocol::wl_surface::WlSurface;

use crate::render::Renderer;

pub struct Surf {
    /// Declared before `layer`: the wgpu surface must drop before its wl_surface.
    pub renderer: Option<Renderer>,
    pub layer: LayerSurface,
    /// The size asked for; the compositor may answer with another.
    pub requested: (u32, u32),
    /// The size the compositor gave, in logical pixels. Zero until the first configure.
    pub size: (u32, u32),
    pub scale: i32,
    pub dirty: bool,
}

impl Surf {
    pub fn new(layer: LayerSurface, requested: (u32, u32)) -> Self {
        Self {
            renderer: None,
            layer,
            requested,
            size: (0, 0),
            scale: 1,
            dirty: false,
        }
    }

    pub fn wl(&self) -> &WlSurface {
        self.layer.wl_surface()
    }

    pub fn logical(&self) -> (f32, f32) {
        (self.size.0 as f32 / orus_core::design::SCALE, self.size.1 as f32 / orus_core::design::SCALE)
    }
}
