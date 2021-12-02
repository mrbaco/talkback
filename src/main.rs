use std::{io::stdin, sync::{Mutex, Arc}, collections::HashMap};

use server::Server;

use crate::{sessions::AnonymSession, server::Responser};

mod server;
mod sessions;
mod user;
mod message;

fn main() {
    let session = Arc::new(Mutex::new(AnonymSession::new()));

    let server = Server::new("0.0.0.0:80", 5);

    // Главная страница (авторизация и регистрация)
    server.add_handler("GET", "/", Box::new(|_, _| {
        println!("get homepage");
        Responser::file("HTTP/1.1 200 OK", "htdocs/index.html")
    }));

    // Страница с чатом
    server.add_handler("GET", "/talkback.html", Box::new(|_, _| {
        println!("get talkback");
        Responser::file("HTTP/1.1 200 OK", "htdocs/talkback.html")
    }));

    // API
    // Регистрация
    let session_copy_1 = Arc::clone(&session);
    server.add_handler("POST", "/api/register", Box::new(move |_, body| {
        println!("post api/register");

        let mut session = session_copy_1.lock().unwrap();
        let mut params = HashMap::new();

        for pair in body.split("&") {
            let pair: Vec<&str> = pair.split("=").collect();
            params.insert(*pair.get(0).unwrap(), *pair.get(1).unwrap());
        }

        session.register(
            params.get("login").unwrap(),
            params.get("password").unwrap()
        ).unwrap();

        Responser::content("HTTP/1.1 200 OK", "register endpoint")
    }));

    // Авторизация
    let session_copy_2 = Arc::clone(&session);
    server.add_handler("POST", "/api/auth", Box::new(move |_, body| {
        let mut session = session_copy_2.lock().unwrap();
        let mut params = HashMap::new();

        for pair in body.split("&") {
            let pair: Vec<&str> = pair.split("=").collect();
            params.insert(*pair.get(0).unwrap(), *pair.get(1).unwrap());
        }

        session.auth(
            params.get("login").unwrap(),
            params.get("password").unwrap()
        ).unwrap();

        Responser::content("HTTP/1.1 200 OK", "auth endpoint")
    }));

    // Получение списка сообщений
    let session_copy_3 = Arc::clone(&session);
    server.add_handler("GET", "/api/messages", Box::new(move |_, body| {
        let mut session = session_copy_3.lock().unwrap();
        let mut params = HashMap::new();

        for pair in body.split("&") {
            let pair: Vec<&str> = pair.split("=").collect();
            params.insert(*pair.get(0).unwrap(), *pair.get(1).unwrap());
        }

        let valid_session = session.auth(
            params.get("login").unwrap(),
            params.get("password").unwrap()
        ).unwrap();

        valid_session.get_messages(params.get("offset").unwrap().parse::<usize>().unwrap());

        Responser::content("HTTP/1.1 200 OK", "messages endpoint")
    }));

    // Отправка сообщения
    let session_copy_4 = Arc::clone(&session);
    server.add_handler("PUT", "/api/message", Box::new(move |_, body| {
        let mut session = session_copy_4.lock().unwrap();
        let mut params = HashMap::new();

        for pair in body.split("&") {
            let pair: Vec<&str> = pair.split("=").collect();
            params.insert(*pair.get(0).unwrap(), *pair.get(1).unwrap());
        }

        let valid_session = session.auth(
            params.get("login").unwrap(),
            params.get("password").unwrap()
        ).unwrap();

        valid_session.add_message(
            params.get("login").unwrap(),
            params.get("message").unwrap()
        );

        Responser::content("HTTP/1.1 200 OK", "message endpoint")
    }));
    
    println!("Press Enter to shutdown...");
    stdin()
        .read_line(&mut String::new())
        .unwrap();
}

#[cfg(test)]
mod tests {
    use std::{fs::{self, File}, path::Path, time::Duration, net::TcpStream, io::{Write, Read}, thread};
    use crate::{sessions::{AnonymSession, SessionError}, server::{Responser, Server}};

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
        
        thread::sleep(Duration::from_secs(5));

        if let Ok(mut stream) = TcpStream::connect("localhost:80") {
            // hello endpoint
            stream.write_all(b"GET /hello.html HTTP/1.1").unwrap();

            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

            let mut buffer = [0; 1024];
            stream.read(&mut buffer).unwrap();

            assert_ne!(std::str::from_utf8(&buffer).unwrap().find("Welcome"), None);

        }

        if let Ok(mut stream) = TcpStream::connect("localhost:80") {
            // 404 endpoint
            stream.write_all(b"GET /asdasd.html HTTP/1.1").unwrap();

            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

            let mut buffer = [0; 1024];
            stream.read(&mut buffer).unwrap();

            assert_ne!(std::str::from_utf8(&buffer).unwrap().find("Not Found"), None);
        }

        for _ in 0..2 {
            thread::spawn(|| {
                thread::sleep(Duration::from_secs(1));

                if let Ok(mut stream) = TcpStream::connect("localhost:80") {
                    stream.write_all(b"GET /highload.html HTTP/1.1").unwrap();

                    stream.set_read_timeout(Some(Duration::from_secs(15))).unwrap();

                    let mut buffer = [0; 1024];
                    stream.read(&mut buffer).unwrap();

                    assert_ne!(std::str::from_utf8(&buffer).unwrap().find("DONE"), None);
                }
            });
        }

        thread::sleep(Duration::from_secs(4));

        if let Ok(mut stream) = TcpStream::connect("localhost:80") {
            stream.write_all(b"GET /highload.html HTTP/1.1").unwrap();

            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

            let mut buffer = [0; 1024];
            stream.read(&mut buffer).unwrap();

            assert_ne!(std::str::from_utf8(&buffer).unwrap().find("Service Unavailable"), None);
        }
    }
}