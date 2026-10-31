use std::io::{self, Cursor, Read, Write};

use orus_greeter::greetd::{LoginError, login, read_message};
use serde_json::{Value, json};

/// Serves pre-framed replies and records what the client wrote.
struct Fake {
    replies: Cursor<Vec<u8>>,
    written: Vec<u8>,
}

fn frame(value: Value) -> Vec<u8> {
    let body = serde_json::to_vec(&value).unwrap();
    let mut out = (body.len() as u32).to_ne_bytes().to_vec();
    out.extend(body);
    out
}

impl Fake {
    fn new(replies: Vec<Value>) -> Self {
        Self {
            replies: Cursor::new(replies.into_iter().flat_map(frame).collect()),
            written: Vec::new(),
        }
    }

    fn sent(&self) -> Vec<Value> {
        let mut data = Cursor::new(self.written.clone());
        let mut out = Vec::new();
        let mut prefix = [0u8; 4];
        while data.read_exact(&mut prefix).is_ok() {
            let mut body = vec![0u8; u32::from_ne_bytes(prefix) as usize];
            data.read_exact(&mut body).unwrap();
            out.push(serde_json::from_slice(&body).unwrap());
        }
        out
    }
}

impl Read for Fake {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.replies.read(buf)
    }
}

impl Write for Fake {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn secret() -> Value {
    json!({"type": "auth_message", "auth_message_type": "secret", "auth_message": "Password: "})
}

fn session() -> Vec<String> {
    vec!["orus-session".to_string()]
}

#[test]
fn good_password_creates_authenticates_and_starts_the_session() {
    let mut s = Fake::new(vec![
        secret(),
        json!({"type": "success"}),
        json!({"type": "success"}),
    ]);
    login(&mut s, "ada", "hunter2", &session()).unwrap();
    assert_eq!(
        s.sent(),
        vec![
            json!({"type": "create_session", "username": "ada"}),
            json!({"type": "post_auth_message_response", "response": "hunter2"}),
            json!({"type": "start_session", "cmd": ["orus-session"], "env": []}),
        ]
    );
}

#[test]
fn wrong_password_is_rejected_and_the_session_cancelled() {
    let mut s = Fake::new(vec![
        secret(),
        json!({"type": "error", "error_type": "auth_error", "description": "pam"}),
        json!({"type": "success"}),
    ]);
    let err = login(&mut s, "ada", "nope", &session()).unwrap_err();
    assert!(matches!(err, LoginError::Rejected));
    assert_eq!(s.sent().last().unwrap(), &json!({"type": "cancel_session"}));
}

#[test]
fn info_messages_are_acknowledged_without_a_response() {
    let mut s = Fake::new(vec![
        json!({"type": "auth_message", "auth_message_type": "info", "auth_message": "welcome"}),
        secret(),
        json!({"type": "success"}),
        json!({"type": "success"}),
    ]);
    login(&mut s, "ada", "pw", &session()).unwrap();
    assert_eq!(
        s.sent()[1],
        json!({"type": "post_auth_message_response", "response": null})
    );
}

#[test]
fn a_second_password_prompt_is_a_rejection() {
    let mut s = Fake::new(vec![secret(), secret(), json!({"type": "success"})]);
    let err = login(&mut s, "ada", "pw", &session()).unwrap_err();
    assert!(matches!(err, LoginError::Rejected));
}

#[test]
fn a_greetd_error_reports_its_description() {
    let mut s = Fake::new(vec![
        json!({"type": "error", "error_type": "error", "description": "session in progress"}),
        json!({"type": "success"}),
    ]);
    let err = login(&mut s, "ada", "pw", &session()).unwrap_err();
    assert_eq!(err.user_message(), "session in progress");
}

#[test]
fn a_corrupt_length_prefix_is_refused() {
    let mut bytes = Cursor::new(u32::MAX.to_ne_bytes().to_vec());
    assert!(matches!(
        read_message(&mut bytes),
        Err(LoginError::Protocol(_))
    ));
}

#[test]
fn a_closed_socket_is_an_io_error() {
    let mut s = Fake::new(vec![]);
    let err = login(&mut s, "ada", "pw", &session()).unwrap_err();
    assert_eq!(err.user_message(), "Cannot reach the login service");
}
