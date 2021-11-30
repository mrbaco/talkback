use std::{io::stdin, time::Duration, thread};

use server::{Server, Responser};

mod server;
mod sessions;
mod user;
mod message;

fn main() {
    let server = Server::new("0.0.0.0:80", 2);

    server.add_handler("GET", "/hello.html", Box::new(|_, _| {
        println!("hello endpoint");
        Responser::file("HTTP/1.1 200 OK", "htdocs/hello.html")
    }));

    server.add_handler("GET", "/highload.html", Box::new(|_, _|{
        println!("highload endpoint");
        thread::sleep(Duration::from_secs(10));
        Responser::content("HTTP/1.1 200 OK", "DONE!")
    }));
    
    println!("Press Enter to shutdown...");

    let mut input = String::new();

    stdin().read_line(&mut input).unwrap();
}

#[cfg(test)]
mod tests {
    use std::{fs::{self, File}, path::Path};

    use crate::sessions::{AnonymSession, SessionError};
    use crate::server;

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

        assert!(if let Err(SessionError::EmptyPassword) = session.register("test_login", "") {
            true
        } else {
            false
        });

        assert!(if let Err(SessionError::PasswordTooSmall) = session.register("test_login", "test") {
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

    #[test]
    fn users_storage() {
        {
            let mut session = AnonymSession::new();

            session.register("test1", "password").expect("Can't register user!");
            session.register("test2", "password").expect("Can't register user!");
        }

        {
            let mut session = AnonymSession::new();

            session.auth("test1", "password").expect("Can't find registered user!");
            session.auth("test2", "password").expect("Can't find registered user!");
        }

        File::open("users.csv").expect("Users storage wasn't created!");
    }

    fn data() -> (String, String, String, String) {
        if Path::new("users.csv").exists() {
            fs::remove_file("users.csv").unwrap();
        }

        (
            String::from("login"),
            String::from("password"),
            String::from("This is test message #1."),
            String::from("This is test message #2."),
        )
    }

    #[test]
    fn start_server() {
        server::Server::new("0.0.0.0:80", 10);
    }
}