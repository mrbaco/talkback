use crate::{message::Message, user::User};
use std::collections::HashMap;

#[derive(Debug)]
pub enum SessionError {
    EmptyLogin,
    EmptyPassword,
    PasswordTooSmall,
    LoginExists,
    LoginNotFound,
    AuthFailed,
}

pub struct AnonymSession {
    users: HashMap<String, User>,
    valid_session: ValidSession,
}

impl AnonymSession {
    pub fn new() -> AnonymSession {
        AnonymSession {
            // TODO: реализовать считывание пользователей из файла
            users: HashMap::new(),
            valid_session: ValidSession {
                messages: Vec::new(),
            },
        }
    }

    pub fn register(&mut self, login: &str, password: &str) -> Result<&mut ValidSession, SessionError> {
        if login.is_empty() {
            return Err(SessionError::EmptyLogin);
        }

        if self.users.contains_key(login) {
            return Err(SessionError::LoginExists);
        }

        if password.is_empty() {
            return Err(SessionError::EmptyPassword);
        }

        if password.len() < 6 {
            return Err(SessionError::PasswordTooSmall);
        }

        self.users.insert(String::from(login), User::new(
            String::from(login), 
            String::from(password)
        ));

        Ok(&mut self.valid_session)
    }

    pub fn auth(&mut self, login: &str, password: &str) -> Result<&mut ValidSession, SessionError> {
        if login.is_empty() {
            return Err(SessionError::EmptyLogin);
        }

        if password.is_empty() {
            return Err(SessionError::EmptyPassword);
        }

        match self.users.get(login) {
            Some(user) => if user.auth(String::from(password)) {
                return Ok(&mut self.valid_session)
            },
            None => return Err(SessionError::LoginNotFound)
        }
        
        Err(SessionError::AuthFailed)
    }
}

impl Drop for AnonymSession {
    fn drop(&mut self) {
        // TODO: реализовать запись пользователей в файл
    }
}

pub struct ValidSession {
    messages: Vec<Message>,
}

impl ValidSession {
    pub fn add_message(&mut self, login: &str, text: &str) {
        self.messages.push(Message::new(
            self.messages.len(), 
            String::from(login), 
            String::from(text))
        );
    }

    pub fn get_messages(&self, offset: usize) -> Vec<Message> {
        if offset < self.messages.len() {
            self.messages[offset..].to_vec()
        } else {
            Vec::new()
        }
    }
}