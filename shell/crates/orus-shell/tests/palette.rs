use orus_core::appearance::{Accent, Theme};
use orus_shell::palette::Palette;

#[test]
fn the_teal_palettes_match_the_concepts() {
    let dark = Palette::new(Theme::Dark, Accent::Teal);
    assert_eq!(
        (dark.accent, dark.dot_running, dark.surface),
        (0x63b1ac, 0x75ccc3, 0x1a2125)
    );
    let light = Palette::new(Theme::Light, Accent::Teal);
    assert_eq!(
        (light.accent, light.dot_running, light.surface),
        (0x1f8285, 0x198c8b, 0xfdfdfe)
    );
}

#[test]
fn the_selection_color_is_the_accent_mixed_into_the_surface() {
    // The concepts' selected rows: #2f4849 on dark, #d7edee on light; the mix
    // ratios give them to within a few shades.
    let near = |a: u32, b: u32| {
        (0..3).all(|i| ((a >> (8 * i)) & 0xff).abs_diff((b >> (8 * i)) & 0xff) <= 4)
    };
    assert!(near(
        Palette::new(Theme::Dark, Accent::Teal).selection,
        0x2f4849
    ));
    assert!(near(
        Palette::new(Theme::Light, Accent::Teal).selection,
        0xd7edee
    ));
}

#[test]
fn another_accent_changes_the_accent_dot_and_selection_but_not_the_surfaces() {
    let teal = Palette::new(Theme::Dark, Accent::Teal);
    let rose = Palette::new(Theme::Dark, Accent::Rose);
    assert_eq!(rose.accent, 0xc66b7e);
    assert_eq!(rose.dot_running, rose.accent);
    assert_ne!(rose.selection, teal.selection);
    assert_eq!(
        (rose.bar, rose.surface, rose.dock),
        (teal.bar, teal.surface, teal.dock)
    );
}
