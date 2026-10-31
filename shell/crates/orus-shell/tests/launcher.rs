use orus_shell::external_app::{OFFICE_INSTALL_COMMAND, on_path};
use orus_shell::launcher::Launcher;

#[test]
fn the_installer_shows_on_live_media_and_office_on_an_installed_system() {
    let live = Launcher::with_installer(true).entry_names();
    assert_eq!(live.last().map(String::as_str), Some("Install Orus OS"));
    assert!(!live.iter().any(|name| name == "Office"));

    let installed = Launcher::with_installer(false).entry_names();
    assert!(!installed.iter().any(|name| name == "Install Orus OS"));
    assert_eq!(installed.last().map(String::as_str), Some("Office"));
}

#[test]
fn a_program_is_found_only_when_it_is_on_the_path() {
    assert!(on_path("sh"));
    assert!(!on_path("orus-no-such-program"));
}

#[test]
fn the_office_install_command_asks_for_sudo_and_installs_the_three_apps() {
    assert!(OFFICE_INSTALL_COMMAND.contains("sudo apt-get install -y"));
    for package in [
        "libreoffice-writer",
        "libreoffice-calc",
        "libreoffice-impress",
    ] {
        assert!(OFFICE_INSTALL_COMMAND.contains(package), "{package}");
    }
}
