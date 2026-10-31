use std::path::Path;

use orus_filemanager::icons;
use orus_shell::icons::builtin_svg_bytes;

#[test]
fn every_file_manager_icon_is_embedded() {
    for name in icons::ALL {
        let bytes = builtin_svg_bytes(Path::new(name))
            .unwrap_or_else(|| panic!("{name} is not embedded in the shell"));
        let text = std::str::from_utf8(bytes).expect("svg is utf-8");
        assert!(text.contains("<svg"), "{name} is not an svg");
    }
}

#[test]
fn unknown_names_are_not_embedded() {
    assert!(builtin_svg_bytes(Path::new("no-such-icon.svg")).is_none());
}
