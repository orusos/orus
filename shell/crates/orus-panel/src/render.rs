//! Drawing one panel surface with the shared GPU.

use std::sync::Arc;

use jag_draw::{TextProvider, make_surface_config};
use jag_surface::{Canvas, DetirSurface};

use crate::gpu::Gpu;

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    surf: DetirSurface,
    device: Arc<wgpu::Device>,
    provider: Arc<dyn TextProvider>,
}

/// Panels have see-through parts (the dock's margin, the launcher's bar strip), so
/// pick an alpha mode that composites with them.
fn alpha_mode(caps: &wgpu::SurfaceCapabilities) -> wgpu::CompositeAlphaMode {
    [
        wgpu::CompositeAlphaMode::PreMultiplied,
        wgpu::CompositeAlphaMode::PostMultiplied,
    ]
    .into_iter()
    .find(|m| caps.alpha_modes.contains(m))
    .unwrap_or_else(|| {
        log::warn!("the surface cannot be transparent; panel margins will draw opaque");
        caps.alpha_modes[0]
    })
}

impl Renderer {
    /// `logical` is the size in logical pixels; the buffer is `scale` times that.
    pub fn new(
        gpu: &Gpu,
        surface: wgpu::Surface<'static>,
        logical: (u32, u32),
        scale: i32,
        provider: Arc<dyn TextProvider>,
    ) -> Self {
        let (w, h) = physical(logical, scale);
        let mut config = make_surface_config(&gpu.adapter, &surface, w, h);
        config.alpha_mode = alpha_mode(&surface.get_capabilities(&gpu.adapter));
        surface.configure(&gpu.device, &config);
        let mut surf = DetirSurface::new(gpu.device.clone(), gpu.queue.clone(), config.format);
        surf.set_use_intermediate(true);
        surf.set_direct(true);
        surf.set_logical_pixels(true);
        surf.set_dpi_scale(scale as f32 * orus_core::design::SCALE);
        Self {
            surface,
            config,
            surf,
            device: gpu.device.clone(),
            provider,
        }
    }

    pub fn resize(&mut self, logical: (u32, u32), scale: i32) {
        let (w, h) = physical(logical, scale);
        if (w, h) != (self.config.width, self.config.height) {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
        }
        self.surf.set_dpi_scale(scale as f32 * orus_core::design::SCALE);
    }

    /// Draw one frame. `paint` gets a transparent canvas in logical pixels.
    pub fn draw(&mut self, paint: impl FnOnce(&mut Canvas)) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                log::warn!("no frame to draw into: {e}");
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let mut canvas = self.surf.begin_frame(self.config.width, self.config.height);
        canvas.set_text_provider(self.provider.clone());
        paint(&mut canvas);
        if let Err(e) = self.surf.end_frame(frame, canvas) {
            log::warn!("drawing a panel frame failed: {e}");
        }
    }
}

fn physical(logical: (u32, u32), scale: i32) -> (u32, u32) {
    let s = scale.max(1) as u32;
    ((logical.0 * s).max(1), (logical.1 * s).max(1))
}
