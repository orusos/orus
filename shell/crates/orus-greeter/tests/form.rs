use orus_greeter::form::{Field, Form, FormKey, Status};
use orus_greeter::users::LoginUser;

fn user(name: &str, display: &str) -> LoginUser {
    LoginUser {
        name: name.to_string(),
        display: display.to_string(),
    }
}

fn type_str(form: &mut Form, s: &str) {
    for c in s.chars() {
        form.handle_key(FormKey::Char(c));
    }
}

fn one_user() -> Form {
    Form::new(vec![user("ada", "Ada Lovelace")])
}

#[test]
fn the_first_listed_user_is_chosen_with_the_password_focused() {
    let form = one_user();
    assert_eq!(
        (form.username(), form.display_name()),
        ("ada", "Ada Lovelace")
    );
    assert!(!form.is_typing_username());
    assert_eq!(form.focus, Field::Password);
}

#[test]
fn with_no_listed_users_the_account_name_is_typed_first() {
    let mut form = Form::new(Vec::new());
    assert!(form.is_typing_username());
    assert!(!form.can_switch_user());
    type_str(&mut form, "root");
    assert_eq!(form.username(), "root");
    assert!(form.handle_key(FormKey::Enter).is_none());
    assert_eq!(form.focus, Field::Password);
}

#[test]
fn enter_submits_the_chosen_user_with_the_typed_password() {
    let mut form = one_user();
    type_str(&mut form, "pw");
    let creds = form.handle_key(FormKey::Enter).unwrap();
    assert_eq!(
        (creds.username.as_str(), creds.password.as_str()),
        ("ada", "pw")
    );
    assert!(form.is_busy());
}

#[test]
fn an_empty_password_does_not_submit() {
    let mut form = one_user();
    assert!(form.handle_key(FormKey::Enter).is_none());
    assert!(!form.is_busy());
}

#[test]
fn switch_user_steps_through_the_list_then_typing_then_back() {
    let mut form = Form::new(vec![user("ada", "Ada"), user("bob", "Bob")]);
    assert_eq!(form.username(), "ada");
    form.switch_user();
    assert_eq!(form.username(), "bob");
    form.switch_user();
    assert!(form.is_typing_username());
    assert_eq!(form.focus, Field::Username);
    form.switch_user();
    assert_eq!(form.username(), "ada");
}

#[test]
fn switching_clears_the_password_and_the_error() {
    let mut form = Form::new(vec![user("ada", "Ada"), user("bob", "Bob")]);
    type_str(&mut form, "pw");
    form.fail("Incorrect username or password".to_string());
    type_str(&mut form, "x");
    form.switch_user();
    assert_eq!(form.password, "");
    assert_eq!(form.status, Status::Idle);
}

#[test]
fn input_is_ignored_while_busy() {
    let mut form = one_user();
    type_str(&mut form, "pw");
    form.handle_key(FormKey::Enter);
    type_str(&mut form, "xx");
    form.switch_user();
    assert_eq!((form.password.as_str(), form.username()), ("pw", "ada"));
}

#[test]
fn failure_clears_the_password_keeps_the_user_and_refocuses_the_password() {
    let mut form = one_user();
    type_str(&mut form, "pw");
    form.handle_key(FormKey::Enter);
    form.fail("Incorrect username or password".to_string());
    assert_eq!((form.password.as_str(), form.username()), ("", "ada"));
    assert_eq!(
        form.status,
        Status::Failed("Incorrect username or password".to_string())
    );
    type_str(&mut form, "p");
    assert_eq!(form.status, Status::Idle);
}

#[test]
fn the_eye_toggles_whether_the_password_is_shown() {
    let mut form = one_user();
    assert!(!form.show_password);
    form.toggle_password_visible();
    assert!(form.show_password);
}

#[test]
fn a_listed_user_cannot_be_retyped_into_the_name_field() {
    let mut form = one_user();
    form.set_focus(Field::Username);
    assert_eq!(form.focus, Field::Password);
    type_str(&mut form, "zz");
    assert_eq!(form.username(), "ada");
    assert_eq!(form.password, "zz");
}
