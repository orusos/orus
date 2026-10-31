use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use jag_draw::make_surface_config;
use jag_surface::DetirSurface;
use orus_textedit::TextEditApp;
use pollster::FutureExt;
use winit::event::{ElementState, Event, Modifiers, MouseButton, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::Window;

fn main() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let window_attrs = Window::default_attributes()
        .with_title("detir-textedit — untitled")
        .with_inner_size(winit::dpi::LogicalSize::new(900.0, 650.0));
    #[allow(deprecated)]
    let window = event_loop.create_window(window_attrs)?;
    let window: &'static Window = Box::leak(Box::new(window));

    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window)?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .block_on()
        .expect("no suitable GPU adapter found");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .block_on()?;

    let mut size = window.inner_size();
    // Everything is drawn at 80% of the mock-up's size (orus_core::design), at the screen's own
    // resolution: the compositor stays at 100% and nothing is resampled.
    let mut scale_factor = window.scale_factor() as f32 * orus_core::design::SCALE;
    let mut surf_config = make_surface_config(&adapter, &surface, size.width, size.height);
    surface.configure(&device, &surf_config);

    let mut surf = DetirSurface::new(Arc::new(device), Arc::new(queue), surf_config.format);
    surf.set_use_intermediate(true);
    surf.set_direct(true);
    surf.set_logical_pixels(true);
    surf.set_dpi_scale(scale_factor);

    let mut app = TextEditApp::new()?;

    // Initial viewport
    let w = size.width as f32 / scale_factor;
    let h = size.height as f32 / scale_factor;
    app.update_viewport(w, h);

    let mut current_modifiers = Modifiers::default();
    let mut mouse_pos = (0.0_f32, 0.0_f32);
    let mut click_count: u32 = 0;
    let mut last_click_time = Instant::now();

    #[allow(deprecated)]
    event_loop.run(move |event, target| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            target.exit();
        }

        Event::WindowEvent {
            event: WindowEvent::Resized(new_size),
            ..
        } => {
            size = new_size;
            if size.width > 0 && size.height > 0 {
                surf_config.width = size.width;
                surf_config.height = size.height;
                surface.configure(surf.device().as_ref(), &surf_config);
                surf.prepare_for_resize(size.width, size.height);

                let w = size.width as f32 / scale_factor;
                let h = size.height as f32 / scale_factor;
                app.update_viewport(w, h);
            }
            window.request_redraw();
        }

        Event::WindowEvent {
            event:
                WindowEvent::ScaleFactorChanged {
                    scale_factor: new_sf,
                    ..
                },
            ..
        } => {
            scale_factor = new_sf as f32 * orus_core::design::SCALE;
            surf.set_dpi_scale(scale_factor);
            size = window.inner_size();
            surf_config.width = size.width;
            surf_config.height = size.height;
            surface.configure(surf.device().as_ref(), &surf_config);

            let w = size.width as f32 / scale_factor;
            let h = size.height as f32 / scale_factor;
            app.update_viewport(w, h);
            window.request_redraw();
        }

        Event::WindowEvent {
            event: WindowEvent::ModifiersChanged(new_mods),
            ..
        } => {
            current_modifiers = new_mods;
        }

        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            mouse_pos = (
                position.x as f32 / scale_factor,
                position.y as f32 / scale_factor,
            );
            if app.handle_mouse_drag(mouse_pos.0, mouse_pos.1) {
                window.request_redraw();
            }
        }

        Event::WindowEvent {
            event: WindowEvent::MouseInput { state, button, .. },
            ..
        } => {
            if button == MouseButton::Left {
                if state == ElementState::Pressed {
                    let now = Instant::now();
                    if now.duration_since(last_click_time).as_millis() < 400 {
                        click_count += 1;
                    } else {
                        click_count = 1;
                    }
                    last_click_time = now;

                    let double = click_count >= 2;
                    app.handle_mouse_click(mouse_pos.0, mouse_pos.1, double);
                    window.request_redraw();
                } else {
                    app.handle_mouse_release();
                }
            }
        }

        Event::WindowEvent {
            event: WindowEvent::MouseWheel { delta, .. },
            ..
        } => {
            let dy = match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => y * 20.0,
                winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
            };
            app.handle_scroll(dy);
            window.request_redraw();
        }

        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } if event.state == ElementState::Pressed => {
            if app.handle_key(&event.logical_key, &current_modifiers) {
                // Update window title on changes
                window.set_title(&app.editor.window_title());
                window.request_redraw();
            }
        }

        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
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

            let w = size.width as f32 / scale_factor;
            let h = size.height as f32 / scale_factor;

            let mut canvas = surf.begin_frame(size.width, size.height);
            canvas.set_text_provider(app.provider.clone());

            app.render(&mut canvas, w, h, 0);

            surf.end_frame(frame, canvas).ok();

            // Keep redrawing for caret blink
            window.request_redraw();
        }

        _ => {}
    })?;

    Ok(())
}
