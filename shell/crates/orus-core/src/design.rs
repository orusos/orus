//! The size the interface is drawn at.
//!
//! Every layout was drawn on a mock-up 1672 pixels wide, which makes the UI too big on a
//! 1366-wide laptop. The layouts keep the mock-up's numbers, and every app draws them at
//! [`SCALE`] of that size. This is not the compositor's scale (that stays at 100%, so windows
//! are never resampled): each app multiplies it into the scale it draws at, and rasterizes its
//! text and shapes at the screen's real resolution, so everything stays crisp. To change the
//! overall size of the interface, change this one number.

/// The share of the mock-up's size that the interface is drawn at.
pub const SCALE: f32 = 0.8;
