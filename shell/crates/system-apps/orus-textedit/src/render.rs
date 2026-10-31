use jag_draw::{Brush, FillRule, Path, PathCmd, Rect, TextProvider};
use jag_surface::Canvas;
use jag_text::layout::CursorPosition;

use crate::editor::{
    CONTENT_PAD_TOP, EditorBuffer, GUTTER_RIGHT_PAD, GUTTER_TEXT_SIZE, SCROLLBAR_MIN_THUMB,
    SCROLLBAR_WIDTH, TEXT_SIZE,
};
use crate::theme::Theme;

/// Render the full editor: gutter, content, scrollbar.
pub fn render_editor(
    editor: &EditorBuffer,
    canvas: &mut Canvas,
    z: i32,
    provider: &dyn TextProvider,
    theme: &Theme,
) {
    let cr = editor.content_rect();
    let line_h = editor.line_height();
    let gw = editor.gutter_width();

    // Gutter background (full height of editor rect).
    canvas.fill_rect(
        editor.rect.x,
        editor.rect.y,
        gw,
        editor.rect.h,
        Brush::Solid(theme.gutter_bg),
        z,
    );

    // Gutter right border (1px separator).
    canvas.fill_rect(
        editor.rect.x + gw - 1.0,
        editor.rect.y,
        1.0,
        editor.rect.h,
        Brush::Solid(theme.gutter_border),
        z + 1,
    );

    // Clip content area for text, selection, caret.
    canvas.push_clip_rect(cr);

    if let Some(layout) = editor.layout.as_ref() {
        // Active line highlight (only when cursor has no selection).
        if editor.focused && editor.selection.is_collapsed() {
            let cur_line = editor.current_line();
            let active_y = cr.y + cur_line as f32 * line_h - editor.scroll_y;
            if active_y + line_h > cr.y && active_y < cr.y + cr.h {
                canvas.fill_rect(
                    cr.x,
                    active_y,
                    cr.w,
                    line_h,
                    Brush::Solid(theme.active_line_bg),
                    z + 1,
                );
            }
        }

        // Selection rectangles.
        if editor.focused && !editor.selection.is_collapsed() {
            let sel_rects = layout.selection_rects(&editor.selection);
            for sel_rect in sel_rects {
                let hx = cr.x - editor.scroll_x + sel_rect.x;
                let hy = cr.y + sel_rect.y - editor.scroll_y;
                let hw = sel_rect.width;
                let hh = sel_rect.height;

                // Clip to content rect.
                let x0 = hx.max(cr.x);
                let x1 = (hx + hw).min(cr.x + cr.w);
                let y0 = hy.max(cr.y);
                let y1 = (hy + hh).min(cr.y + cr.h);
                if x1 > x0 && y1 > y0 {
                    canvas.fill_rect(
                        x0,
                        y0,
                        x1 - x0,
                        y1 - y0,
                        Brush::Solid(theme.selection_bg),
                        z + 1,
                    );
                }
            }
        }

        // Text lines (only visible ones).
        let text_x = cr.x - editor.scroll_x;
        for (i, line) in layout.lines().iter().enumerate() {
            let line_top = cr.y + i as f32 * line_h - editor.scroll_y;
            if line_top + line_h < cr.y {
                continue;
            }
            if line_top > cr.y + cr.h {
                break;
            }

            let range = line.text_range.clone();
            if range.start > editor.text.len() || range.end > editor.text.len() {
                continue;
            }
            let line_text = &editor.text[range];

            let text_baseline_y = line_top + line_h * 0.75;
            canvas.draw_text_direct(
                [text_x, text_baseline_y],
                line_text,
                TEXT_SIZE,
                theme.editor_fg,
                provider,
                z + 2,
            );
        }

        // Caret.
        if editor.focused && editor.selection.is_collapsed() {
            render_caret(canvas, editor, &cr, z + 3, theme);
        }
    }

    canvas.pop_clip();

    // Gutter line numbers (outside the content clip).
    render_gutter(editor, canvas, z + 2, provider, theme);

    // Scrollbar.
    render_scrollbar(editor, canvas, z + 4, theme);
}

/// Render logical line numbers in the gutter.
fn render_gutter(
    editor: &EditorBuffer,
    canvas: &mut Canvas,
    z: i32,
    provider: &dyn TextProvider,
    theme: &Theme,
) {
    let Some(layout) = editor.layout.as_ref() else {
        return;
    };
    let line_h = editor.line_height();
    let gw = editor.gutter_width();
    let cur_line = editor.current_line();

    // Clip gutter vertically.
    let gutter_clip = Rect {
        x: editor.rect.x,
        y: editor.rect.y + CONTENT_PAD_TOP,
        w: gw,
        h: (editor.rect.h - CONTENT_PAD_TOP * 2.0).max(1.0),
    };
    canvas.push_clip_rect(gutter_clip);

    let mut logical_line = 0usize;
    for (i, line) in layout.lines().iter().enumerate() {
        let is_logical_start = line.text_range.start == 0
            || editor
                .text
                .as_bytes()
                .get(line.text_range.start.wrapping_sub(1))
                == Some(&b'\n');

        if is_logical_start {
            logical_line += 1;
        }

        let line_top = editor.rect.y + CONTENT_PAD_TOP + i as f32 * line_h - editor.scroll_y;
        if line_top + line_h < gutter_clip.y {
            continue;
        }
        if line_top > gutter_clip.y + gutter_clip.h {
            break;
        }

        // Only render number for the first visual line of each logical line.
        if !is_logical_start {
            continue;
        }

        let num_str = logical_line.to_string();
        let text_w = canvas.measure_text_width(&num_str, GUTTER_TEXT_SIZE);
        let x = editor.rect.x + gw - GUTTER_RIGHT_PAD - text_w;
        let baseline_y = line_top + line_h * 0.75;

        let color = if i == cur_line {
            theme.gutter_active_fg
        } else {
            theme.gutter_fg
        };
        canvas.draw_text_direct(
            [x, baseline_y],
            &num_str,
            GUTTER_TEXT_SIZE,
            color,
            provider,
            z,
        );
    }

    canvas.pop_clip();
}

/// Render the blinking caret.
fn render_caret(canvas: &mut Canvas, editor: &EditorBuffer, cr: &Rect, z: i32, theme: &Theme) {
    if !editor.caret.visible {
        return;
    }

    let Some(layout) = editor.layout.as_ref() else {
        return;
    };

    let pos = CursorPosition::new(editor.cursor_pos.min(layout.text().len()));
    let Some(cursor_rect) = layout.cursor_rect_at_position(pos) else {
        return;
    };

    let cx = cr.x - editor.scroll_x + cursor_rect.x;
    let cy0 = cr.y + cursor_rect.y - editor.scroll_y;
    let cy1 = cy0 + cursor_rect.height;

    let mut path = Path {
        cmds: Vec::new(),
        fill_rule: FillRule::NonZero,
    };
    path.cmds.push(PathCmd::MoveTo([cx, cy0]));
    path.cmds.push(PathCmd::LineTo([cx, cy1]));

    canvas.stroke_path(path, 1.5, theme.caret_color, z);
}

/// Render a thin scrollbar track and thumb on the right edge.
fn render_scrollbar(editor: &EditorBuffer, canvas: &mut Canvas, z: i32, theme: &Theme) {
    let cr = editor.content_rect();
    let total = editor.total_height();
    let viewport_h = cr.h;

    // No scrollbar needed if all content fits.
    if total <= viewport_h {
        return;
    }

    let track_x = editor.rect.x + editor.rect.w - SCROLLBAR_WIDTH;
    let track_y = editor.rect.y;
    let track_h = editor.rect.h;

    // Thumb proportional to visible fraction.
    let visible_frac = (viewport_h / total).min(1.0);
    let thumb_h = (track_h * visible_frac).max(SCROLLBAR_MIN_THUMB);

    let max_scroll = (total - viewport_h).max(1.0);
    let scroll_frac = (editor.scroll_y / max_scroll).clamp(0.0, 1.0);
    let thumb_y = track_y + scroll_frac * (track_h - thumb_h);

    // Rounded thumb (simple rect for now — rounded corners in a future pass).
    canvas.fill_rect(
        track_x + 1.0,
        thumb_y,
        SCROLLBAR_WIDTH - 2.0,
        thumb_h,
        Brush::Solid(theme.scrollbar_thumb),
        z,
    );
}
