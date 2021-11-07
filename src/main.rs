
mod sessions;
mod user;
mod message;

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use crate::sessions::{AnonymSession, SessionError};

    #[test]
    fn new_session_with_user_and_message() {
        let (login, password, message1, message2) = data();

        let mut session = AnonymSession::new();

        let valid_session = session.register(&login, &password).unwrap();

        assert_eq!(valid_session.get_messages(0).len(), 0);

        valid_session.add_message(&login, &message1);
        assert_eq!(valid_session.get_messages(0).len(), 1);

        valid_session.add_message(&login, &message2);
        assert_eq!(valid_session.get_messages(0).len(), 2);
    }

    #[test]
    fn message_offset() {
        let (login, password, message1, message2) = data();

        let mut session = AnonymSession::new();

        let valid_session = session.register(&login, &password).unwrap();

        valid_session.add_message(&login, &message1);
        valid_session.add_message(&login, &message2);

        assert_eq!(valid_session.get_messages(0).len(), 2);
        assert_eq!(valid_session.get_messages(1).len(), 1);
        assert_eq!(valid_session.get_messages(2).len(), 0);

        assert_eq!(valid_session.get_messages(1).get(0).unwrap().format(), format!("{}: {}", login, message2));
    }

    #[test]
    fn register_error() {
        let (login, password, _, _) = data();

        let mut session = AnonymSession::new();

        assert!(if let Err(SessionError::EmptyLogin) = session.register("", &password) {
            true
        } else {
            false
        });

        assert!(if let Err(SessionError::EmptyPassword) = session.register(&login, "") {
            true
        } else {
            false
        });

        assert!(if let Err(SessionError::PasswordTooSmall) = session.register(&login, "test") {
            true
        } else {
            false
        });

        session.register(&login, &password).unwrap();

        assert!(if let Err(SessionError::LoginExists) = session.register(&login, "test") {
            true
        } else {
            false
        });
    }

    #[test]
    fn auth_error() {
        let (login, password, _, _) = data();

        let mut session = AnonymSession::new();

        session.register(&login, &password).unwrap();

        assert!(if let Err(SessionError::EmptyLogin) = session.auth("", &password) {
            true
        } else {
            false
        });

        assert!(if let Err(SessionError::EmptyPassword) = session.auth(&login, "") {
            true
        } else {
            false
        });

        assert!(if let Err(SessionError::LoginNotFound) = session.auth("not_exist", &password) {
            true
        } else {
            false
        });

        assert!(if let Err(SessionError::AuthFailed) = session.auth(&login, "wrong_password") {
            true
        } else {
            false
        });
    }

    fn data() -> (String, String, String, String) {
        (
            String::from("login"),
            String::from("password"),
            String::from("This is test message #1."),
            String::from("This is test message #2."),
        )
    }
}