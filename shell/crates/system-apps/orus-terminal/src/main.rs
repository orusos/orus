use std::sync::Arc;

use anyhow::Result;
use jag_draw::{
    Brush, ColorLinPremul, FontStyle, SubpixelOrientation, TextProvider, TextRun,
    make_surface_config,
};
use jag_surface::DetirSurface;
use pollster::FutureExt;
use winit::event::{ElementState, Event, Modifiers, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

use orus_terminal::{CellAttrs, TerminalConfig, TerminalCore, TerminalKey};

fn main() -> Result<()> {
    let cfg = TerminalConfig::default();

    // --- Window + GPU setup ---

    let event_loop = EventLoop::new()?;
    let window_attrs = Window::default_attributes().with_title("Detir Terminal");
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

    // --- Text provider ---

    let provider: Arc<dyn TextProvider + Send + Sync> = Arc::new(
        jag_draw::DetirTextProvider::from_system_fonts(SubpixelOrientation::RGB)?,
    );

    // --- Measure cell dimensions ---

    let metrics = provider
        .line_metrics(cfg.font_size)
        .expect("font metrics unavailable");
    let cell_height = metrics.ascent + metrics.descent + metrics.line_gap;

    let cell_width = {
        let run = TextRun {
            text: "MM".to_string(),
            pos: [0.0, 0.0],
            size: cfg.font_size,
            logical_size: 0.0,
            color: ColorLinPremul::from_srgba_u8([255, 255, 255, 255]),
            weight: 400.0,
            style: FontStyle::Normal,
            family: Some(cfg.font_family.clone()),
        };
        let glyphs = provider.rasterize_run(&run);
        if glyphs.len() >= 2 {
            (glyphs[1].offset[0] - glyphs[0].offset[0]).abs()
        } else {
            cfg.font_size * 0.6
        }
    };

    let bg_color = ColorLinPremul::from_srgba_u8(cfg.bg);
    let cursor_color = ColorLinPremul::from_srgba_u8([204, 204, 204, 200]);

    // --- Terminal state ---

    let w = size.width as f32 / scale_factor;
    let h = size.height as f32 / scale_factor;
    let pad = cfg.padding;
    let init_cols = ((w - pad * 2.0) / cell_width).floor().max(1.0) as usize;
    let init_rows = ((h - pad * 2.0) / cell_height).floor().max(1.0) as usize;

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let mut core = TerminalCore::new(init_cols, init_rows, &cwd)?;

    let mut current_modifiers = Modifiers::default();
    // Accumulate sub-line pixel scroll so trackpad works smoothly.
    let mut scroll_accum: f32 = 0.0;

    // --- Event loop ---

    #[allow(deprecated)]
    event_loop.run(move |event, target| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
        } if window_id == window.id() => {
            target.exit();
        }

        Event::WindowEvent {
            event: WindowEvent::Resized(new_size),
            window_id,
        } if window_id == window.id() => {
            size = new_size;
            if size.width > 0 && size.height > 0 {
                surf_config.width = size.width;
                surf_config.height = size.height;
                surface.configure(surf.device().as_ref(), &surf_config);
                surf.prepare_for_resize(size.width, size.height);

                let w = size.width as f32 / scale_factor;
                let h = size.height as f32 / scale_factor;
                let new_cols = ((w - pad * 2.0) / cell_width).floor().max(1.0) as usize;
                let new_rows = ((h - pad * 2.0) / cell_height).floor().max(1.0) as usize;
                core.resize(new_cols, new_rows);
            }
            window.request_redraw();
        }

        Event::WindowEvent {
            event:
                WindowEvent::ScaleFactorChanged {
                    scale_factor: new_sf,
                    ..
                },
            window_id,
        } if window_id == window.id() => {
            scale_factor = new_sf as f32 * orus_core::design::SCALE;
            surf.set_dpi_scale(scale_factor);
            size = window.inner_size();
            surf_config.width = size.width;
            surf_config.height = size.height;
            surface.configure(surf.device().as_ref(), &surf_config);
            window.request_redraw();
        }

        // --- Track modifier keys ---
        Event::WindowEvent {
            event: WindowEvent::ModifiersChanged(new_mods),
            window_id,
        } if window_id == window.id() => {
            current_modifiers = new_mods;
        }

        // --- Mouse wheel scroll ---
        Event::WindowEvent {
            event: WindowEvent::MouseWheel { delta, .. },
            window_id,
        } if window_id == window.id() => {
            match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => {
                    let n = y.abs().max(1.0) as usize;
                    if y > 0.0 {
                        core.scroll_view_up(n);
                    } else if y < 0.0 {
                        core.scroll_view_down(n);
                    }
                }
                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                    scroll_accum += pos.y as f32;
                    let lines = (scroll_accum / cell_height).trunc() as i32;
                    if lines != 0 {
                        scroll_accum -= lines as f32 * cell_height;
                        if lines > 0 {
                            core.scroll_view_up(lines as usize);
                        } else {
                            core.scroll_view_down((-lines) as usize);
                        }
                    }
                }
            }
            window.request_redraw();
        }

        // --- Keyboard input ---
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            window_id,
        } if window_id == window.id() && event.state == ElementState::Pressed => {
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
                core.send_key(k);
                window.request_redraw();
            }
        }

        // --- Render ---
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            window_id,
        } if window_id == window.id() => {
            if size.width == 0 || size.height == 0 {
                return;
            }

            // Poll PTY for new output
            core.poll();

            let frame = match surface.get_current_texture() {
                Ok(f) => f,
                Err(_) => {
                    window.request_redraw();
                    return;
                }
            };

            let mut canvas = surf.begin_frame(size.width, size.height);
            canvas.clear(bg_color);
            canvas.set_text_provider(provider.clone());

            let w = size.width as f32 / scale_factor;
            let h = size.height as f32 / scale_factor;

            // Background
            canvas.fill_rect(0.0, 0.0, w, h, Brush::Solid(bg_color), 0);

            let grid = &core.state.grid;
            let total = grid.scrollback_len() + grid.rows;
            let max_visible = (h / cell_height).floor() as usize;
            let view_start = total
                .saturating_sub(max_visible)
                .saturating_sub(core.scroll_offset);

            let family = Some(cfg.font_family.clone());

            for view_row in 0..max_visible.min(total) {
                let global_row = view_start + view_row;
                let row_data = if global_row < grid.scrollback_len() {
                    &grid.scrollback[global_row]
                } else {
                    &grid.cells[global_row - grid.scrollback_len()]
                };

                let y_base = pad + (view_row as f32) * cell_height;
                let y_text = y_base + metrics.ascent;

                // Group adjacent cells by attrs into spans
                let mut col = 0;
                while col < row_data.len() {
                    let span_attrs = row_data[col].attrs;
                    let span_start = col;
                    while col < row_data.len() && row_data[col].attrs == span_attrs {
                        col += 1;
                    }
                    let span_len = col - span_start;

                    // Resolve colors (handle inverse flag)
                    let (fg, bg) = if span_attrs.flags & CellAttrs::INVERSE != 0 {
                        (span_attrs.bg, span_attrs.fg)
                    } else {
                        (span_attrs.fg, span_attrs.bg)
                    };

                    let x = pad + span_start as f32 * cell_width;

                    // Draw background if non-default
                    if bg != CellAttrs::DEFAULT_BG {
                        canvas.fill_rect(
                            x,
                            y_base,
                            span_len as f32 * cell_width,
                            cell_height,
                            Brush::Solid(ColorLinPremul::from_srgba_u8([bg[0], bg[1], bg[2], 255])),
                            1,
                        );
                    }

                    // Collect text
                    let text: String = row_data[span_start..col].iter().map(|c| c.c).collect();

                    let weight = if span_attrs.flags & CellAttrs::BOLD != 0 {
                        700.0
                    } else if span_attrs.flags & CellAttrs::DIM != 0 {
                        300.0
                    } else {
                        400.0
                    };

                    let style = if span_attrs.flags & CellAttrs::ITALIC != 0 {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    };

                    let fg_color = ColorLinPremul::from_srgba_u8([fg[0], fg[1], fg[2], 255]);

                    canvas.draw_text_run_styled(
                        [x, y_text],
                        text,
                        cfg.font_size,
                        weight,
                        style,
                        family.clone(),
                        fg_color,
                        10,
                    );
                }
            }

            // Cursor (only when viewing bottom and cursor is visible)
            if core.scroll_offset == 0 && core.state.cursor_visible {
                let (cr, cc) = core.state.cursor;
                let cursor_x = pad + cc as f32 * cell_width;
                let cursor_row_on_screen = max_visible.saturating_sub(grid.rows) + cr;
                let cursor_y = pad + cursor_row_on_screen as f32 * cell_height;

                if cursor_row_on_screen < max_visible {
                    canvas.fill_rect(
                        cursor_x,
                        cursor_y,
                        cell_width,
                        cell_height,
                        Brush::Solid(cursor_color),
                        11,
                    );
                }
            }

            // Scrollbar
            if total > max_visible && max_visible > 0 {
                let bar_w = 6.0_f32;
                let track_x = w - bar_w - 2.0;
                let track_h = h;

                canvas.fill_rect(
                    track_x,
                    0.0,
                    bar_w,
                    track_h,
                    Brush::Solid(ColorLinPremul::from_srgba_u8([50, 50, 50, 100])),
                    20,
                );

                let ratio = max_visible as f32 / total as f32;
                let thumb_h = (track_h * ratio).max(20.0);
                let max_off = total.saturating_sub(max_visible) as f32;
                let scroll_frac = if max_off > 0.0 {
                    1.0 - (core.scroll_offset as f32 / max_off)
                } else {
                    1.0
                };
                let thumb_y = (track_h - thumb_h) * scroll_frac;
                canvas.fill_rect(
                    track_x,
                    thumb_y,
                    bar_w,
                    thumb_h,
                    Brush::Solid(ColorLinPremul::from_srgba_u8([150, 150, 150, 180])),
                    21,
                );
            }

            surf.end_frame(frame, canvas).ok();

            // Continuous redraw while terminal is alive
            if core.is_alive() {
                window.request_redraw();
            }
        }

        _ => {}
    })?;

    Ok(())
}
