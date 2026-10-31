//! greetd IPC client. Each message is JSON behind a 4-byte native-endian
//! length prefix. Written here rather than taken from `greetd_ipc`, which is GPL-3.0.

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

/// greetd messages are a few hundred bytes. A larger length prefix means the
/// stream is corrupt, and is refused instead of allocated.
const MAX_MESSAGE_LEN: usize = 1 << 20;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request<'a> {
    CreateSession {
        username: &'a str,
    },
    PostAuthMessageResponse {
        response: Option<&'a str>,
    },
    StartSession {
        cmd: &'a [String],
        env: &'a [String],
    },
    CancelSession,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Success,
    Error {
        error_type: ErrorType,
        description: String,
    },
    AuthMessage {
        auth_message_type: AuthMessageType,
        auth_message: String,
    },
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    AuthError,
    Error,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMessageType {
    Visible,
    Secret,
    Info,
    Error,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("wrong username or password")]
    Rejected,
    #[error("{0}")]
    Greetd(String),
    #[error("cannot reach greetd: {0}")]
    Io(#[from] io::Error),
    #[error("unexpected reply from greetd: {0}")]
    Protocol(String),
}

impl LoginError {
    /// The line shown under the form.
    pub fn user_message(&self) -> String {
        match self {
            LoginError::Rejected => "Incorrect username or password".to_string(),
            LoginError::Greetd(description) => description.clone(),
            LoginError::Io(_) => "Cannot reach the login service".to_string(),
            LoginError::Protocol(_) => "Unexpected reply from the login service".to_string(),
        }
    }
}

pub fn write_message<W: Write>(stream: &mut W, request: &Request) -> io::Result<()> {
    let body = serde_json::to_vec(request)?;
    stream.write_all(&(body.len() as u32).to_ne_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}

pub fn read_message<R: Read>(stream: &mut R) -> Result<Response, LoginError> {
    let mut prefix = [0u8; 4];
    stream.read_exact(&mut prefix)?;
    let len = u32::from_ne_bytes(prefix) as usize;
    if len > MAX_MESSAGE_LEN {
        return Err(LoginError::Protocol(format!("message of {len} bytes")));
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|e| LoginError::Protocol(e.to_string()))
}

fn exchange<S: Read + Write>(stream: &mut S, request: &Request) -> Result<Response, LoginError> {
    write_message(stream, request)?;
    read_message(stream)
}

fn cancel<S: Read + Write>(stream: &mut S) {
    if let Err(e) = exchange(stream, &Request::CancelSession) {
        log::warn!("cancel_session failed: {e}");
    }
}

/// Authenticate `username` with `password`, then ask greetd to start `cmd`.
/// greetd launches the session once the greeter exits after `Ok`.
pub fn login<S: Read + Write>(
    stream: &mut S,
    username: &str,
    password: &str,
    cmd: &[String],
) -> Result<(), LoginError> {
    let mut reply = exchange(stream, &Request::CreateSession { username })?;
    let mut password_sent = false;
    loop {
        match reply {
            Response::Success => break,
            Response::Error {
                error_type: ErrorType::AuthError,
                ..
            } => {
                cancel(stream);
                return Err(LoginError::Rejected);
            }
            Response::Error { description, .. } => {
                cancel(stream);
                return Err(LoginError::Greetd(description));
            }
            Response::AuthMessage {
                auth_message_type: AuthMessageType::Secret,
                ..
            } => {
                // A second secret prompt (a forced password change) cannot be
                // answered with the same password.
                if password_sent {
                    cancel(stream);
                    return Err(LoginError::Rejected);
                }
                password_sent = true;
                reply = exchange(
                    stream,
                    &Request::PostAuthMessageResponse {
                        response: Some(password),
                    },
                )?;
            }
            Response::AuthMessage {
                auth_message_type: AuthMessageType::Info | AuthMessageType::Error,
                ..
            } => {
                reply = exchange(stream, &Request::PostAuthMessageResponse { response: None })?;
            }
            Response::AuthMessage {
                auth_message_type: AuthMessageType::Visible,
                auth_message,
            } => {
                cancel(stream);
                return Err(LoginError::Protocol(format!(
                    "visible prompt \"{auth_message}\""
                )));
            }
        }
    }

    match exchange(stream, &Request::StartSession { cmd, env: &[] })? {
        Response::Success => Ok(()),
        Response::Error { description, .. } => Err(LoginError::Greetd(description)),
        other => Err(LoginError::Protocol(format!("{other:?}"))),
    }
}
