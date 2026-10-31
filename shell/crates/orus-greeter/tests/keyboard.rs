use orus_greeter::keyboard::{layout_label, xkb_layout};

#[test]
fn known_layouts_get_their_names_and_others_show_their_code() {
    assert_eq!(layout_label("us").as_deref(), Some("English (US)"));
    assert_eq!(layout_label("gb").as_deref(), Some("English (UK)"));
    assert_eq!(layout_label("de").as_deref(), Some("DE"));
}

#[test]
fn only_the_first_of_several_layouts_is_shown() {
    assert_eq!(layout_label("de,us").as_deref(), Some("DE"));
    assert_eq!(layout_label(""), None);
}

#[test]
fn the_layout_is_read_from_etc_default_keyboard() {
    let text = "XKBMODEL=\"pc105\"\nXKBLAYOUT=\"us\"\nXKBVARIANT=\"\"\n";
    assert_eq!(xkb_layout(text).as_deref(), Some("us"));
    assert_eq!(xkb_layout("XKBMODEL=\"pc105\"\n"), None);
}
