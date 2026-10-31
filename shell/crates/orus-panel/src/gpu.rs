//! The GPU the panel's surfaces share, and a wgpu surface for a Wayland `wl_surface`.

use std::ptr::NonNull;
use std::sync::Arc;

use anyhow::{Context, Result};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Proxy};

pub struct Gpu {
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl Gpu {
    /// Pick the adapter that can present to `first`; the other surfaces use it too.
    pub fn new(instance: &wgpu::Instance, first: &wgpu::Surface) -> Result<Self> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(first),
        }))
        .context("no GPU adapter can present to the panel surface")?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
        Ok(Self {
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }
}

/// A wgpu surface drawing into `surface`. The caller must drop it before `surface`.
pub fn create_surface(
    instance: &wgpu::Instance,
    conn: &Connection,
    surface: &WlSurface,
) -> Result<wgpu::Surface<'static>> {
    let display = NonNull::new(conn.backend().display_ptr() as *mut _)
        .context("the Wayland connection has no display pointer")?;
    let window = NonNull::new(surface.id().as_ptr() as *mut _)
        .context("the wl_surface has no object pointer")?;
    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display)),
        raw_window_handle: RawWindowHandle::Wayland(WaylandWindowHandle::new(window)),
    };
    // SAFETY: both pointers are live libwayland objects (the connection and the
    // wl_surface). `Surf` drops its wgpu surface before the layer surface that owns
    // the wl_surface, and the connection outlives both.
    Ok(unsafe { instance.create_surface_unsafe(target)? })
}
