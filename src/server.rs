use std::collections::HashMap;
use std::io::{Read, Write, self};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::{thread, fs};
use std::str::{self, Utf8Error};

type Job = Box<dyn Fn(Vec<String>, String) -> (String, String) + Sync + Send>;

enum Impulse {
    Handler(String, String, Job),
    Request(TcpStream),
    Shutdown,
}

pub struct Server {
    controller_tx: Sender<Impulse>,
}

impl Server {
    pub fn new(addr: &str, max_threads_number: usize) -> Server {
        let listener = TcpListener::bind(addr).unwrap();
        let (controller_tx, controller_rx) = mpsc::channel();
        
        // Main server thread
        let controller_tx_copy = controller_tx.clone();

        thread::spawn(move || {
            println!("i: main server thread is started");

            for stream in listener.incoming() {
                if let Err(error) = controller_tx.send(Impulse::Request(stream.unwrap())) {
                    println!("e: main server thread is stopped: {}", &error);
                    break;
                }
            }
            
            println!("i: main server thread is stopped");
        });

        // Controller thread
        thread::spawn(move || {
            println!("i: controller thread is started");

            let mut handlers: Arc<HashMap<String, Job>> = Arc::new(HashMap::new());

            loop {
                let impulse = controller_rx.recv().unwrap();

                if let Impulse::Request(mut stream) = impulse {
                    println!("i: got Request impulse");

                    if Arc::strong_count(&handlers) > max_threads_number {
                        println!("i: max threads number was achieved");

                        Response::from_text_file(
                            &mut stream,
                            "HTTP/1.1 503 Service Unavailable",
                            "htdocs/503.html"
                        ).unwrap();

                        continue;
                    }

                    let handlers = Arc::clone(&handlers);

                    thread::spawn(move || {
                        match Request::new(&mut stream) {
                            Ok(request) => {
                                println!("i: connect {} {}", request.method, request.path);

                                match handlers.get(&format!("{} {}", request.method, request.path)) {
                                    Some(closure) => {
                                        let result = closure(request.headers, request.body);

                                        Response::from_text_content(
                                            &mut stream,
                                            &result.0,
                                            &result.1
                                        ).unwrap();
                                    },
                                    None => {
                                        println!("i: handler not found");

                                        Response::from_text_file(
                                            &mut stream,
                                            "HTTP/1.1 404 Not Found",
                                            "htdocs/404.html"
                                        ).unwrap();
                                        
                                        return;
                                    },
                                };
                            },
                            Err(_) => {
                                println!("i: incorrect request");

                                Response::from_text_file(
                                    &mut stream,
                                    "HTTP/1.1 400 Bad Request",
                                    "htdocs/400.html"
                                ).unwrap();
                
                                return;
                            },
                        };
                    });

                    continue;
                }

                if let Impulse::Handler(method, path, closure) = impulse {
                    println!("i: got Handler impulse");
                    
                    if let Some(handlers) = Arc::get_mut(&mut handlers) {
                        handlers.insert(format!("{} {}", method, path), closure);
                    }
                    
                    continue;
                }

                if let Impulse::Shutdown = impulse {
                    println!("i: got Shutdown impulse");
                    break;
                }
            }

            println!("i: controller thread is stopped");
        });

        Server {
            controller_tx: controller_tx_copy
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

struct Request {
    method: String,
    path: String,
    headers: Vec<String>,
    body: String,
}

impl Request {
    fn new(stream: &mut TcpStream) -> Result<Request, Utf8Error> {
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();

        let buffer = str::from_utf8(&buffer)?
            .replace("\r\n", "\n");

        let request = buffer.split("\n\n").map(String::from).collect::<Vec<String>>();

        let mut headers = match request.get(0) {
            Some(headers) => headers.split("\n").map(String::from).collect::<Vec<String>>(),
            None => Vec::new(),
        };

        let request_line = headers.remove(0);
        let request_line: Vec<String> = request_line.split(" ").map(String::from).collect();

        let method = request_line.get(0).unwrap().to_string();
        let path = request_line.get(1).unwrap().to_string();

        let body = match request.get(1) {
            Some(body) => body.to_string(),
            None => String::new(),
        };

        Ok(
            Request {
                method,
                path,
                headers,
                body,
            }
        )
    }
}

struct Response;

impl Response {
    fn from_text_file(stream: &mut TcpStream, status_line: &str, file_name: &str) -> io::Result<usize> {
        let content = fs::read_to_string(file_name).unwrap();
        
        stream.write(format!(
            "{}\r\nContent-length: {}\r\n\r\n{}",
            status_line, content.len(), content
        ).as_bytes())
    }

    fn from_text_content(stream: &mut TcpStream, status_line: &str, content: &str) -> io::Result<usize> {
        stream.write(format!(
            "{}\r\nContent-length: {}\r\n\r\n{}",
            status_line, content.len(), content
        ).as_bytes())
    }
}