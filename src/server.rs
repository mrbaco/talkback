use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::{thread, fs};
use std::str;

type Job = Box<dyn Fn(Vec<&str>, &str) -> String + Sync + Send + 'static>;

pub struct Server {
    controller_tx: Sender<Impulse>,
}

enum Impulse {
    Handler(String, String, Job),
    Request(TcpStream),
    Shutdown,
}

impl Server {
    pub fn new(addr: &str, max_threads_number: usize) -> Server {
        let listener = TcpListener::bind(addr).unwrap();

        let (controller_tx1, controller_rx) = mpsc::channel();
        let controller_tx2 = controller_tx1.clone();

        // Main server thread
        thread::spawn(move || {
            println!("main server thread is started");

            for stream in listener.incoming() {
                println!("new connection");

                if let Err(error) = controller_tx1.send(Impulse::Request(stream.unwrap())) {
                    println!("main server thread is stopped: {}", &error);

                    break;
                }
            }
            
            println!("main server thread is stopped");
        });

        // Controller thread
        thread::spawn(move || {
            println!("controller thread is started");

            let mut handlers: Arc<HashMap<String, Job>> = Arc::new(HashMap::new());

            loop {
                let impulse = controller_rx.recv().unwrap();

                if let Impulse::Request(mut stream) = impulse {
                    if Arc::strong_count(&handlers) > max_threads_number {
                        stream.write(Responser::file(
                            "HTTP/1.1 503 Service Unavailable",
                            "htdocs/503.html").as_bytes()).unwrap();

                        continue;
                    }

                    let handlers = Arc::clone(&handlers);

                    thread::spawn(move || {
                        let mut buffer = [0; 1024];
                        stream.read(&mut buffer).unwrap();

                        match str::from_utf8(&buffer) {
                            Ok(buffer) => {
                                let buffer = buffer
                                    .replace("\r\n", "\n");

                                let request: Vec<&str> = buffer
                                    .split("\n\n").collect();

                                let headers = match request.get(0) {
                                    Some(headers) => headers
                                        .split("\n")
                                        .collect(),
                                    None => Vec::new(),
                                };

                                let request_line = match headers.get(0) {
                                    Some(line) => {
                                        let line: Vec<&str> = line.split(" ").collect();

                                        (
                                            *line.get(0).unwrap(),
                                            *line.get(1).unwrap(),
                                        )
                                    },
                                    None => ("", ""),
                                };

                                let body = match request.get(1) {
                                    Some(body) => body,
                                    None => "",
                                };

                                match handlers.get(&format!("{} {}", request_line.0, request_line.1)) {
                                    Some(closure) => {
                                        stream.write(closure(headers, body).as_bytes()).unwrap();
                                    },
                                    None => {
                                        stream.write(Responser::file(
                                            "HTTP/1.1 404 Not Found",
                                            "htdocs/404.html").as_bytes()).unwrap();
                                        
                                        return;
                                    },
                                }
                            },
                            Err(_) => {
                                stream.write(Responser::file(
                                    "HTTP/1.1 400 Bad Request",
                                    "htdocs/400.html").as_bytes()).unwrap();

                                return;
                            }
                        };
                    });

                    continue;
                }

                if let Impulse::Handler(method, path, closure) = impulse {
                    if let Some(handlers) = Arc::get_mut(&mut handlers) {
                        handlers.insert(format!("{} {}", method, path), closure);
                    }
                    
                    continue;
                }

                if let Impulse::Shutdown = impulse {
                    break;
                }
            }

            println!("controller thread is stopped");
        });

        Server {
            controller_tx: controller_tx2
        }
    }

    pub fn add_handler(&self, method: &str, path: &str, closure: Job) {
        self.controller_tx.send(Impulse::Handler(method.to_string(), path.to_string(), closure))
            .expect("Fail to add new handler for server!");
    }

    pub fn stop(&self) {
        self.controller_tx.send(Impulse::Shutdown)
            .expect("Fail to shutdown the server!");
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct Responser;

impl Responser {
    pub fn file(status_line: &str, file_name: &str) -> String {
        let content = fs::read_to_string(file_name).unwrap();

        format!(
            "{}\r\nContent-length: {}\r\n\r\n{}",
            status_line,
            content.len(),
            content
        )
    }

    pub fn content(status_line: &str, content: &str) -> String {
        format!(
            "{}\r\nContent-length: {}\r\n\r\n{}",
            status_line,
            content.len(),
            content
        )
    }
}