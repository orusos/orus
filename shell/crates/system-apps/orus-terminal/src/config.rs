/// Terminal configuration with sensible defaults.
pub struct TerminalConfig {
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Font family for terminal text (must be monospace).
    pub font_family: String,
    /// Inner padding in logical pixels.
    pub padding: f32,
    /// Background color (sRGB).
    pub bg: [u8; 4],
    /// Default foreground color (sRGB).
    pub fg: [u8; 4],
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            font_family: "monospace".to_string(),
            padding: 4.0,
            bg: [30, 30, 30, 255],
            fg: [204, 204, 204, 255],
        }
    }
}
