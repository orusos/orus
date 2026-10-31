//! Login form state: which account, the password, and what the last attempt did.

use std::time::Instant;

use crate::users::LoginUser;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    /// The typed account name, used when no listed account is chosen.
    Username,
    Password,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Status {
    Idle,
    /// Waiting for greetd. Input is ignored until it answers.
    Busy,
    Failed(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FormKey {
    Char(char),
    Backspace,
    Tab,
    Enter,
    Escape,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub struct Form {
    users: Vec<LoginUser>,
    /// Index into `users`; `users.len()` is the "type an account name" entry.
    choice: usize,
    pub typed_username: String,
    pub password: String,
    pub show_password: bool,
    pub focus: Field,
    pub status: Status,
    caret_epoch: Instant,
}

/// Half of the caret's blink period.
const CARET_BLINK_MS: u128 = 530;

impl Form {
    /// The first of `users` is chosen; with none listed, an account name is typed.
    pub fn new(users: Vec<LoginUser>) -> Self {
        let mut form = Self {
            users,
            choice: 0,
            typed_username: String::new(),
            password: String::new(),
            show_password: false,
            focus: Field::Password,
            status: Status::Idle,
            caret_epoch: Instant::now(),
        };
        form.focus = form.default_focus();
        form
    }

    fn default_focus(&self) -> Field {
        if self.is_typing_username() {
            Field::Username
        } else {
            Field::Password
        }
    }

    pub fn caret_visible(&self) -> bool {
        self.caret_epoch.elapsed().as_millis() % (CARET_BLINK_MS * 2) < CARET_BLINK_MS
    }

    pub fn is_busy(&self) -> bool {
        self.status == Status::Busy
    }

    /// True when the account name is typed rather than chosen from the list.
    pub fn is_typing_username(&self) -> bool {
        self.choice >= self.users.len()
    }

    /// The account greetd will authenticate.
    pub fn username(&self) -> &str {
        self.users
            .get(self.choice)
            .map_or(self.typed_username.as_str(), |u| u.name.as_str())
    }

    /// The name shown under the avatar.
    pub fn display_name(&self) -> &str {
        self.users
            .get(self.choice)
            .map_or(self.typed_username.as_str(), |u| u.display.as_str())
    }

    /// Whether "Switch user" does anything: there is a list to step through.
    pub fn can_switch_user(&self) -> bool {
        !self.users.is_empty()
    }

    /// Step to the next listed account, then to typing a name, then back to the first.
    pub fn switch_user(&mut self) {
        if self.is_busy() || !self.can_switch_user() {
            return;
        }
        self.choice = (self.choice + 1) % (self.users.len() + 1);
        self.password.clear();
        self.status = Status::Idle;
        self.focus = self.default_focus();
        self.caret_epoch = Instant::now();
    }

    pub fn toggle_password_visible(&mut self) {
        self.show_password = !self.show_password;
    }

    pub fn set_focus(&mut self, field: Field) {
        if self.is_busy() || (field == Field::Username && !self.is_typing_username()) {
            return;
        }
        self.focus = field;
        self.caret_epoch = Instant::now();
    }

    /// Start a login if the account and password are filled. Moves the form to `Busy`.
    pub fn submit(&mut self) -> Option<Credentials> {
        if self.is_busy() || self.username().is_empty() || self.password.is_empty() {
            return None;
        }
        self.status = Status::Busy;
        Some(Credentials {
            username: self.username().to_string(),
            password: self.password.clone(),
        })
    }

    /// greetd refused the login: show why, empty the password, and refocus it.
    pub fn fail(&mut self, message: String) {
        self.status = Status::Failed(message);
        self.password.clear();
        self.focus = Field::Password;
        self.caret_epoch = Instant::now();
    }

    pub fn handle_key(&mut self, key: FormKey) -> Option<Credentials> {
        if self.is_busy() {
            return None;
        }
        self.caret_epoch = Instant::now();
        match key {
            FormKey::Char(c) => {
                self.status = Status::Idle;
                self.field_mut().push(c);
                None
            }
            FormKey::Backspace => {
                self.field_mut().pop();
                None
            }
            FormKey::Tab => {
                if self.is_typing_username() {
                    self.focus = match self.focus {
                        Field::Username => Field::Password,
                        Field::Password => Field::Username,
                    };
                }
                None
            }
            FormKey::Enter => {
                if self.focus == Field::Username && !self.typed_username.is_empty() {
                    self.focus = Field::Password;
                    None
                } else {
                    self.submit()
                }
            }
            FormKey::Escape => {
                self.password.clear();
                self.status = Status::Idle;
                None
            }
        }
    }

    fn field_mut(&mut self) -> &mut String {
        match self.focus {
            Field::Username if self.is_typing_username() => &mut self.typed_username,
            _ => &mut self.password,
        }
    }
}
