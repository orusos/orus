//! Orus OS colors for the shell's own chrome: the palette lives in `orus-core`
//! (the system apps use it too); this adds the conversion to a canvas color.

use jag_draw::ColorLinPremul;

pub use orus_core::palette::{Palette, get, set};

pub fn color(hex: u32) -> ColorLinPremul {
    ColorLinPremul::from_srgba_u8([(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255])
}
