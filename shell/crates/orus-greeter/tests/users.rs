use orus_greeter::users::{LoginUser, last_user, order, parse_passwd, remember_last_user};

const PASSWD: &str = "\
root:x:0:0:root:/root:/bin/bash
daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
ada:x:1000:1000:Ada Lovelace,,,:/home/ada:/bin/bash
bob:x:1001:1001::/home/bob:/bin/zsh
svc:x:1002:1002:Service:/home/svc:/usr/sbin/nologin
off:x:1003:1003:Off:/home/off:/bin/false
nobody:x:65534:65534:nobody:/nonexistent:/usr/sbin/nologin
broken line
";

#[test]
fn only_ordinary_accounts_that_can_log_in_are_listed() {
    let users = parse_passwd(PASSWD);
    assert_eq!(
        users.iter().map(|u| u.name.as_str()).collect::<Vec<_>>(),
        vec!["ada", "bob"]
    );
}

#[test]
fn the_display_name_is_the_first_gecos_field_or_the_account_name() {
    let users = parse_passwd(PASSWD);
    assert_eq!(users[0].display, "Ada Lovelace");
    assert_eq!(users[1].display, "bob");
}

#[test]
fn the_last_user_moves_to_the_front() {
    let users = parse_passwd(PASSWD);
    let ordered = order(users.clone(), Some("bob"));
    assert_eq!(ordered[0].name, "bob");
    assert_eq!(ordered[1].name, "ada");
    assert_eq!(order(users.clone(), Some("nobody")), users);
    assert_eq!(order(users.clone(), None), users);
}

#[test]
fn the_last_user_is_remembered_in_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("last-user");
    assert_eq!(last_user(&file), None);
    remember_last_user(&file, "ada").unwrap();
    assert_eq!(last_user(&file), Some("ada".to_string()));
    std::fs::write(&file, "  \n").unwrap();
    assert_eq!(last_user(&file), None);
}

#[test]
fn login_users_compare_by_value() {
    let a = LoginUser {
        name: "a".into(),
        display: "A".into(),
    };
    assert_eq!(a.clone(), a);
}
