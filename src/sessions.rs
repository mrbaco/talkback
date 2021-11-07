use crate::{message::Message, user::User};
use std::{collections::HashMap, fs::{self, File}, io::{BufRead, BufReader}};

const USERS_STORAGE: &str = "users.csv";

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
        let mut users = HashMap::new();

        if let Ok(file) = File::open(USERS_STORAGE) {
            let reader = BufReader::new(file);

            for line in reader.lines() {
                if let Ok(line) = line {
                    let mut line = line.split(";").into_iter();

                    let login = line.next().expect("Users storage is invalid!");
                    let password_hash = line.next().expect("Users storage is invalid!");

                    users.insert(String::from(login), User::fill(
                        String::from(login), 
                        String::from(password_hash))
                    );
                }
            }
        }

        AnonymSession {
            users,
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
        let mut contents = String::new();

        for (_, user) in &self.users {
            contents = format!("{}{}\n", contents, user.format());
        }
        
        fs::write(USERS_STORAGE, contents).expect("Can't write to users storage!");
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