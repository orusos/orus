use jag_draw::ColorLinPremul;

/// One Dark Pro–inspired color palette for the editor.
pub struct Theme {
    // Editor
    pub bg: ColorLinPremul,
    pub editor_fg: ColorLinPremul,
    pub active_line_bg: ColorLinPremul,
    pub selection_bg: ColorLinPremul,
    pub caret_color: ColorLinPremul,

    // Gutter (line numbers)
    pub gutter_bg: ColorLinPremul,
    pub gutter_fg: ColorLinPremul,
    pub gutter_active_fg: ColorLinPremul,
    pub gutter_border: ColorLinPremul,

    // Status bar
    pub status_bg: ColorLinPremul,
    pub status_fg: ColorLinPremul,
    pub status_border: ColorLinPremul,

    // Scrollbar
    pub scrollbar_thumb: ColorLinPremul,

    // Markdown rendering
    pub md_heading: ColorLinPremul,
    pub md_code_fg: ColorLinPremul,
    pub md_code_bg: ColorLinPremul,
    pub md_code_block_bg: ColorLinPremul,
    pub md_code_block_border: ColorLinPremul,
    pub md_blockquote_border: ColorLinPremul,
    pub md_blockquote_fg: ColorLinPremul,
    pub md_link: ColorLinPremul,
    pub md_strikethrough: ColorLinPremul,
    pub md_hr: ColorLinPremul,
    pub md_table_border: ColorLinPremul,
    pub md_table_header_bg: ColorLinPremul,
    pub md_bullet: ColorLinPremul,
    pub md_checkbox: ColorLinPremul,

    // WYSIWYG focused-block editing
    pub md_focused_bg: ColorLinPremul,
    pub md_focused_border: ColorLinPremul,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: ColorLinPremul::from_srgba_u8([40, 44, 52, 255]),
            editor_fg: ColorLinPremul::from_srgba_u8([171, 178, 191, 255]),
            active_line_bg: ColorLinPremul::from_srgba_u8([255, 255, 255, 10]),
            selection_bg: ColorLinPremul::from_srgba_u8([63, 130, 246, 80]),
            caret_color: ColorLinPremul::from_srgba_u8([97, 175, 239, 255]),

            gutter_bg: ColorLinPremul::from_srgba_u8([33, 37, 43, 255]),
            gutter_fg: ColorLinPremul::from_srgba_u8([73, 81, 98, 255]),
            gutter_active_fg: ColorLinPremul::from_srgba_u8([171, 178, 191, 255]),
            gutter_border: ColorLinPremul::from_srgba_u8([24, 26, 31, 255]),

            status_bg: ColorLinPremul::from_srgba_u8([33, 37, 43, 255]),
            status_fg: ColorLinPremul::from_srgba_u8([99, 109, 131, 255]),
            status_border: ColorLinPremul::from_srgba_u8([24, 26, 31, 255]),

            scrollbar_thumb: ColorLinPremul::from_srgba_u8([255, 255, 255, 35]),

            // Markdown — One Dark Pro accent colors
            md_heading: ColorLinPremul::from_srgba_u8([224, 108, 117, 255]), // red
            md_code_fg: ColorLinPremul::from_srgba_u8([152, 195, 121, 255]), // green
            md_code_bg: ColorLinPremul::from_srgba_u8([255, 255, 255, 12]),  // subtle
            md_code_block_bg: ColorLinPremul::from_srgba_u8([30, 33, 39, 255]), // darker
            md_code_block_border: ColorLinPremul::from_srgba_u8([60, 65, 75, 255]),
            md_blockquote_border: ColorLinPremul::from_srgba_u8([73, 81, 98, 255]),
            md_blockquote_fg: ColorLinPremul::from_srgba_u8([139, 148, 167, 255]),
            md_link: ColorLinPremul::from_srgba_u8([97, 175, 239, 255]), // blue
            md_strikethrough: ColorLinPremul::from_srgba_u8([92, 99, 112, 255]), // dimmed
            md_hr: ColorLinPremul::from_srgba_u8([60, 65, 75, 255]),
            md_table_border: ColorLinPremul::from_srgba_u8([55, 60, 70, 255]),
            md_table_header_bg: ColorLinPremul::from_srgba_u8([35, 39, 47, 255]),
            md_bullet: ColorLinPremul::from_srgba_u8([198, 120, 221, 255]), // purple
            md_checkbox: ColorLinPremul::from_srgba_u8([97, 175, 239, 255]), // blue

            md_focused_bg: ColorLinPremul::from_srgba_u8([50, 55, 65, 255]),
            md_focused_border: ColorLinPremul::from_srgba_u8([97, 175, 239, 255]),
        }
    }
}
