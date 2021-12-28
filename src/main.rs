use std::io::stdin;
use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::fs;
use std::str;

use server::Server;

use crate::server::RequestError;
use crate::sessions::AnonymSession;
use crate::sessions::SessionError;

mod server;
mod sessions;
mod user;
mod message;

fn main() {
    let session = Arc::new(Mutex::new(AnonymSession::new()));

    let server = Server::new("0.0.0.0:80", 5);

    // 404 error handler
    server.add_error_handler(RequestError::NotFound, Box::new(|_, _, _| {
        let mut headers = Vec::new();
        let body = fs::read_to_string("htdocs/404.html").unwrap();

        headers.push(String::from("HTTP/1.1 404 Not Found"));
        headers.push(String::from("Content-type: text/html; charset=utf-8"));
        headers.push(format!("Content-length: {}", body.len()));

        (
            headers,
            body,
        )
    }));

    // 400 error handler
    server.add_error_handler(RequestError::BadRequest, Box::new(|_, _, _| {
        let mut headers = Vec::new();
        let body = fs::read_to_string("htdocs/400.html").unwrap();

        headers.push(String::from("HTTP/1.1 400 Bad Request"));
        headers.push(String::from("Content-type: text/html; charset=utf-8"));
        headers.push(format!("Content-length: {}", body.len()));

        (
            headers,
            body,
        )
    }));

    // 503 error handler
    server.add_error_handler(RequestError::ServiceUnavailable, Box::new(|_, _, _| {
        let mut headers = Vec::new();
        let body = fs::read_to_string("htdocs/503.html").unwrap();

        headers.push(String::from("HTTP/1.1 503 Service Unavailable"));
        headers.push(String::from("Content-type: text/html; charset=utf-8"));
        headers.push(format!("Content-length: {}", body.len()));

        (
            headers,
            body,
        )
    }));

    // Homepage handler
    server.add_handler("GET", "/", Box::new(|_, _, _| {
        println!("get homepage");

        let mut headers = Vec::new();
        let body = fs::read_to_string("htdocs/index.html").unwrap();

        headers.push(String::from("HTTP/1.1 200 OK"));
        headers.push(String::from("Content-type: text/html; charset=utf-8"));
        headers.push(format!("Content-length: {}", body.len()));

        (
            headers,
            body,
        )
    }));

    // API
    // Sign up
    let session_copy_1 = Arc::clone(&session);
    server.add_handler("POST", "/api/register", Box::new(move |_, _, request_body| {
        println!("post api/register");

        let mut headers = Vec::new();
        let body: String;

        let mut session = session_copy_1.lock().unwrap();
        let params = params_from_body(request_body);

        match session.register(
            params.get("login").or(Some(&String::new())).unwrap(),
            params.get("password").or(Some(&String::new())).unwrap()
        ) {
            Ok(_) => {
                headers.push(String::from("HTTP/1.1 201 Created"));                
                body = format!("{{\"result\":\"{}\"}}", "ok");

                println!("i: user {} was registered", params.get("login").unwrap());
            },
            Err(e) => {
                headers.push(String::from("HTTP/1.1 400 Bad Request"));

                let error = match e {
                    SessionError::EmptyLogin => String::from("Empty login!"),
                    SessionError::LoginExists => String::from("Login exists!"),
                    SessionError::EmptyPassword => String::from("Empty password!"),
                    SessionError::PasswordTooSmall => String::from("Password too small!"),
                    _ => String::from("Unknown error!"),
                };

                body = format!("{{\"result\":\"{}\"}}", error);
            },
        }

        headers.push(String::from("Content-type: application/json; charset=utf-8"));
        headers.push(format!("Content-length: {}", body.len()));

        (
            headers,
            body,
        )
    }));

    // Sign in
    let session_copy_2 = Arc::clone(&session);
    server.add_handler("POST", "/api/auth", Box::new(move |_, _, request_body| {
        println!("post api/auth");

        let mut headers = Vec::new();
        let body: String;

        let mut session = session_copy_2.lock().unwrap();
        let params = params_from_body(request_body);

        match session.auth(
            params.get("login").or(Some(&String::new())).unwrap(),
            params.get("password").or(Some(&String::new())).unwrap()
        ) {
            Ok(_) => {
                headers.push(String::from("HTTP/1.1 200 Ok"));                
                body = format!("{{\"result\":\"{}\"}}", "ok");

                println!("i: user {} was authed", params.get("login").unwrap());
            },
            Err(e) => {
                headers.push(String::from("HTTP/1.1 400 Bad Request"));

                let error = match e {
                    SessionError::EmptyLogin => String::from("Empty login!"),
                    SessionError::EmptyPassword => String::from("Empty password!"),
                    SessionError::LoginNotFound => String::from("Login not found!"),
                    SessionError::AuthFailed => String::from("Auth failed!"),
                    _ => String::from("Unknown error!"),
                };

                body = format!("{{\"result\":\"{}\"}}", error);
            },
        }

        headers.push(String::from("Content-type: application/json; charset=utf-8"));
        headers.push(format!("Content-length: {}", body.len()));

        (
            headers,
            body,
        )
    }));

    // Get messages list (sign in required)
    let session_copy_3 = Arc::clone(&session);
    server.add_handler("GET", "/api/messages", Box::new(move |params, _, _| {
        println!("get api/messages");

        let mut headers = Vec::new();
        let body: String;

        let mut session = session_copy_3.lock().unwrap();

        match session.auth(
            params.get("login").or(Some(&String::new())).unwrap(),
            params.get("password").or(Some(&String::new())).unwrap()
        ) {
            Ok(valid_session) => {
                let messages = valid_session.get_messages(0).iter()
                    .map(|message| message.format())
                    .collect::<Vec<String>>().join("<br />");

                headers.push(String::from("HTTP/1.1 200 Ok"));  
                body = format!("{{\"result\":\"{}\"}}", messages);

                println!("i: user {} requested messages", params.get("login").unwrap());
            },
            Err(_) => {
                headers.push(String::from("HTTP/1.1 401 Unauthorized"));
                body = format!("{{\"result\":\"{}\"}}", "auth failed");
            }
        };

        headers.push(String::from("Content-type: application/json; charset=utf-8"));
        headers.push(format!("Content-length: {}", body.len()));

        (
            headers,
            body,
        )
    }));

    // Send message (sign in required)
    let session_copy_4 = Arc::clone(&session);
    server.add_handler("POST", "/api/message", Box::new(move |_, _, request_body| {
        println!("post api/message");

        let mut headers = Vec::new();
        let body: String;

        let mut session = session_copy_4.lock().unwrap();
        let params = params_from_body(request_body);

        match session.auth(
            params.get("login").or(Some(&String::new())).unwrap(),
            params.get("password").or(Some(&String::new())).unwrap()
        ) {
            Ok(valid_session) => {
                valid_session.add_message(
                    params.get("login").unwrap(),
                    params.get("message").unwrap()
                );

                headers.push(String::from("HTTP/1.1 201 Created"));  
                body = format!("{{\"result\":\"{}\"}}", "ok");

                println!("i: user {} sent a message: {}",
                    params.get("login").unwrap(),
                    params.get("message").unwrap()
                );
            },
            Err(err) => {
                headers.push(String::from("HTTP/1.1 401 Unauthorized"));
                body = format!("{{\"result\":\"{:?}\"}}", err);
            }
        };

        headers.push(String::from("Content-type: application/json; charset=utf-8"));
        headers.push(format!("Content-length: {}", body.len()));

        (
            headers,
            body,
        )
    }));
    
    println!("Rust TalkBack Server");
    println!("Press Enter to shutdown...");
    stdin()
        .read_line(&mut String::new())
        .unwrap();
}

fn params_from_body(body: &str) -> HashMap<String, String> {
    body.split("&").map(|e| {
        let e: Vec<&str> = e.split("=").collect();

        (
            e.get(0).or(Some(&"")).unwrap().to_string(),
            e.get(1).or(Some(&"")).unwrap().to_string(),
        )
    }).collect()
}

#[cfg(test)]
mod tests {
    use std::{fs::{self, File}, path::Path, time::Duration, net::TcpStream, io::{Write, Read}, thread};
    use crate::{sessions::{AnonymSession, SessionError}, server::{Server, RequestError}};

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

        server.add_error_handler(RequestError::NotFound, Box::new(|_, _, _| {
            let mut headers = Vec::new();
            let content = fs::read_to_string("htdocs/404.html").unwrap();

            headers.push(String::from("HTTP/1.1 404 Not Found"));
            headers.push(String::from("Content-type: text/html; charset=utf-8"));
            headers.push(format!("Content-length: {}", content.len()));

            (
                headers,
                content,
            )
        }));

        server.add_error_handler(RequestError::BadRequest, Box::new(|_, _, _| {
            let mut headers = Vec::new();
            let content = fs::read_to_string("htdocs/400.html").unwrap();

            headers.push(String::from("HTTP/1.1 400 Bad Request"));
            headers.push(String::from("Content-type: text/html; charset=utf-8"));
            headers.push(format!("Content-length: {}", content.len()));

            (
                headers,
                content,
            )
        }));

        server.add_error_handler(RequestError::ServiceUnavailable, Box::new(|_, _, _| {
            let mut headers = Vec::new();
            let content = fs::read_to_string("htdocs/503.html").unwrap();

            headers.push(String::from("HTTP/1.1 503 Service Unavailable"));
            headers.push(String::from("Content-type: text/html; charset=utf-8"));
            headers.push(format!("Content-length: {}", content.len()));

            (
                headers,
                content,
            )
        }));

        server.add_handler("GET", "/hello.html", Box::new(|_, _, _| {
            println!("hello endpoint");

            let mut headers = Vec::new();
            let content = fs::read_to_string("htdocs/hello.html").unwrap();

            headers.push(String::from("HTTP/1.1 200 OK"));
            headers.push(String::from("Content-type: text/html; charset=utf-8"));
            headers.push(format!("Content-length: {}", content.len()));

            (
                headers,
                content,
            )
        }));
    
        server.add_handler("GET", "/highload.html", Box::new(|_, _, _| {
            println!("highload endpoint");
            thread::sleep(Duration::from_secs(10));
            
            let mut headers = Vec::new();
            let content = String::from("DONE!");

            headers.push(String::from("HTTP/1.1 200 OK"));
            headers.push(String::from("Content-type: text/html; charset=utf-8"));
            headers.push(format!("Content-length: {}", content.len()));

            (
                headers,
                content,
            )
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