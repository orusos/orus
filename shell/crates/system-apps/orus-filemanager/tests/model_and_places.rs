use std::path::PathBuf;

use orus_filemanager::fsinfo::{available_bytes, format_available};
use orus_filemanager::model::Tab;
use orus_filemanager::places::{PlaceKind, discover, removable_mounts};

#[test]
fn going_to_a_folder_keeps_the_old_one_for_back_and_clears_forward() {
    let mut tab = Tab::new(PathBuf::from("/home/a"));
    tab.go(PathBuf::from("/home/a/x"));
    tab.go(PathBuf::from("/home/a/x/y"));
    assert_eq!(tab.go_back().unwrap(), PathBuf::from("/home/a/x"));
    assert!(tab.can_go_forward());
    tab.go(PathBuf::from("/home/a/z"));
    assert!(!tab.can_go_forward());
    assert_eq!(tab.go_back().unwrap(), PathBuf::from("/home/a/x"));
}

#[test]
fn back_and_forward_do_nothing_at_the_ends() {
    let mut tab = Tab::new(PathBuf::from("/a"));
    assert!(!tab.can_go_back() && !tab.can_go_forward());
    assert!(tab.go_back().is_none() && tab.go_forward().is_none());
    tab.go(PathBuf::from("/a"));
    assert!(!tab.can_go_back());
}

#[test]
fn a_tab_is_titled_by_its_folder_and_home_is_home() {
    let home = PathBuf::from("/home/a");
    assert_eq!(Tab::new(home.clone()).title(&home), "Home");
    assert_eq!(
        Tab::new(PathBuf::from("/home/a/Projects")).title(&home),
        "Projects"
    );
    assert_eq!(Tab::new(PathBuf::from("/")).title(&home), "System");
}

#[test]
fn only_the_folders_that_exist_are_listed_and_system_is_always_a_device() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join("Documents")).unwrap();
    std::fs::create_dir(home.path().join("Pictures")).unwrap();
    let places = discover(home.path(), "");
    let kinds: Vec<_> = places.favorites.iter().map(|p| p.kind).collect();
    assert_eq!(
        kinds,
        vec![PlaceKind::Home, PlaceKind::Documents, PlaceKind::Pictures]
    );
    assert_eq!(places.devices.len(), 1);
    assert_eq!(places.devices[0].kind, PlaceKind::System);
}

#[test]
fn removable_media_comes_from_proc_mounts() {
    let mounts = "\
/dev/vda2 / ext4 rw 0 0
/dev/sda1 /media/partha/USB\\040STICK vfat rw 0 0
/dev/sdb1 /run/media/partha/DATA ext4 rw 0 0
tmpfs /run/user/1000 tmpfs rw 0 0
";
    assert_eq!(
        removable_mounts(mounts),
        vec![
            PathBuf::from("/media/partha/USB STICK"),
            PathBuf::from("/run/media/partha/DATA")
        ]
    );
    let home = tempfile::tempdir().unwrap();
    let places = discover(home.path(), mounts);
    assert_eq!(places.devices.len(), 3);
    assert_eq!(places.devices[1].label, "USB STICK");
}

#[test]
fn free_space_reads_for_a_real_folder_and_formats_in_decimal_units() {
    assert!(available_bytes(&std::env::temp_dir()).is_some());
    assert_eq!(
        available_bytes(&PathBuf::from("/definitely/not/here")),
        None
    );
    assert_eq!(format_available(42_400_000_000), "42 GB available");
    assert_eq!(format_available(1_540_000_000_000), "1.5 TB available");
    assert_eq!(format_available(800_000_000), "800 MB available");
}

#[test]
fn breadcrumbs_start_at_home_inside_it_and_at_system_outside_it() {
    use orus_filemanager::model::breadcrumbs;
    let home = PathBuf::from("/home/a");
    let crumbs = breadcrumbs(&home, &PathBuf::from("/home/a/Projects/x"));
    let labels: Vec<_> = crumbs.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(labels, ["Home", "Projects", "x"]);
    assert_eq!(crumbs[1].1, PathBuf::from("/home/a/Projects"));
    let crumbs = breadcrumbs(&home, &PathBuf::from("/usr/lib"));
    let labels: Vec<_> = crumbs.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(labels, ["System", "usr", "lib"]);
    assert_eq!(
        breadcrumbs(&home, &PathBuf::from("/"))[0].1,
        PathBuf::from("/")
    );
}

#[test]
fn search_matches_names_ignoring_case() {
    use orus_filemanager::model::{DirEntry, filter_entries};
    let entry = |name: &str| DirEntry {
        name: name.into(),
        is_dir: false,
        size: 0,
        modified: None,
    };
    let entries = [
        entry("Notes.md"),
        entry("Preview.png"),
        entry("notes-old.txt"),
    ];
    assert_eq!(filter_entries(&entries, "NOTES"), vec![0, 2]);
    assert_eq!(filter_entries(&entries, ""), vec![0, 1, 2]);
    assert!(filter_entries(&entries, "zzz").is_empty());
}
