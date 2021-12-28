use std::collections::HashMap;
use std::io::{Read, Write, self};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::str::{self, Utf8Error};

type Job = Box<dyn Fn(&HashMap<String, String>, &Vec<String>, &str) -> (Vec<String>, String) + Sync + Send>;

#[derive(Debug, PartialEq, Eq, Hash)]
/// Possible request's error for this server
pub enum RequestError {
    NotFound,
    BadRequest,
    ServiceUnavailable,
}

enum Impulse {
    Handler(String, String, Job),
    ErrorHandler(RequestError, Job),
    Request(TcpStream),
    Shutdown,
}

pub struct Server {
    controller_tx: Sender<Impulse>,
}

impl Server {
    /// Start new server and binding on giving addr
    pub fn new(addr: &str, max_threads_number: usize) -> Server {
        // Start binding on addr
        let listener = TcpListener::bind(addr).unwrap();

        // Create channel for data exchange
        let (controller_tx, controller_rx) = mpsc::channel();
        let controller_tx_copy = controller_tx.clone();
        
        // Start main server thread
        thread::spawn(move || {
            println!("i: main server thread is started");

            // Catch income connection and send it to controller thread
            for stream in listener.incoming() {
                if let Err(error) = controller_tx.send(Impulse::Request(stream.unwrap())) {
                    println!("e: main server thread is stopped: {}", &error);
                    break;
                }
            }
            
            println!("i: main server thread is stopped");
        });

        // Start controller thread
        thread::spawn(move || {
            println!("i: controller thread is started");

            let mut handlers: Arc<HashMap<String, Job>> = Arc::new(HashMap::new());
            let mut error_handlers: Arc<HashMap<RequestError, Job>> = Arc::new(HashMap::new());

            // Start processing loop
            loop {
                let impulse = controller_rx.recv().unwrap();

                // Proccess request
                if let Impulse::Request(mut stream) = impulse {
                    println!("i: got Request impulse");

                    // Check maximal number of connections
                    if Arc::strong_count(&handlers) > max_threads_number {
                        println!("i: max threads number was achieved");

                        match error_handlers.get(&RequestError::ServiceUnavailable) {
                            Some(closure) => {
                                let result = closure(&HashMap::new(), &Vec::new(), "");
                                Response::process(&mut stream, &result.0, &result.1).unwrap();
                            },
                            None => {
                                let mut headers = Vec::new();
                                headers.push(String::from("HTTP/1.1 503 Service Unavailable"));

                                Response::process(&mut stream, &headers, "").unwrap();
                            }
                        }

                        continue;
                    }

                    let handlers = Arc::clone(&handlers);
                    let error_handlers = Arc::clone(&error_handlers);

                    // Start processor thread
                    thread::spawn(move || {
                        match Request::process(&mut stream) {
                            Ok(request) => {
                                println!("i: connect {} {}", request.method, request.path);

                                match handlers.get(&format!("{} {}", request.method, request.path)) {
                                    Some(closure) => {
                                        // Process request and run specified closure
                                        let result = closure(&request.params, &request.headers, &request.body);
                                        Response::process(&mut stream, &result.0, &result.1).unwrap();
                                    },
                                    None => {
                                        println!("i: handler not found");

                                        // 404 error
                                        match error_handlers.get(&RequestError::NotFound) {
                                            Some(closure) => {
                                                let result = closure(&HashMap::new(), &Vec::new(), "");
                                                Response::process(&mut stream, &result.0, &result.1).unwrap();
                                            },
                                            None => {
                                                let mut headers = Vec::new();
                                                headers.push(String::from("HTTP/1.1 404 Not Found"));
                
                                                Response::process(&mut stream, &headers, "").unwrap();
                                            }
                                        }
                                        
                                        return;
                                    },
                                };
                            },
                            Err(_) => {
                                println!("i: incorrect request");

                                // 400 error
                                match error_handlers.get(&RequestError::BadRequest) {
                                    Some(closure) => {
                                        let result = closure(&HashMap::new(), &Vec::new(), "");
                                        Response::process(&mut stream, &result.0, &result.1).unwrap();
                                    },
                                    None => {
                                        let mut headers = Vec::new();
                                        headers.push(String::from("HTTP/1.1 400 Bad Request"));
        
                                        Response::process(&mut stream, &headers, "").unwrap();
                                    }
                                }
                
                                return;
                            },
                        };
                    });

                    continue;
                }

                // Process handler (endpoint)
                if let Impulse::Handler(method, path, closure) = impulse {
                    println!("i: got Handler impulse");
                    
                    if let Some(handlers) = Arc::get_mut(&mut handlers) {
                        handlers.insert(format!("{} {}", method, path), closure);
                    }
                    
                    continue;
                }

                // Process error handler (like 400, 404 and 503)
                if let Impulse::ErrorHandler(error, closure) = impulse {
                    println!("i: got ErrorHandler impulse");
                    
                    if let Some(error_handlers) = Arc::get_mut(&mut error_handlers) {
                        error_handlers.insert(error, closure);
                    }

                    continue;
                }

                // Process shutdown (break loop and stop server)
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

    /// Add server endpoint with processor (closure)
    pub fn add_handler(&self, method: &str, path: &str, closure: Job) {
        self.controller_tx.send(Impulse::Handler(method.to_string(), path.to_string(), closure))
            .expect("Fail to add new handler for server!");
    }

    /// Add error handler with processor (closure)
    pub fn add_error_handler(&self, error: RequestError, closure: Job) {
        self.controller_tx.send(Impulse::ErrorHandler(error, closure))
            .expect("Fail to add new error handler for server!");
    }

    /// Stop server
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
    params: HashMap<String, String>,
    headers: Vec<String>,
    body: String,
}

impl Request {
    fn process(stream: &mut TcpStream) -> Result<Request, Utf8Error> {
        let mut buffer = [0; 1024];

        // Read data from stream to buffer
        if let Err(_) = stream.read(&mut buffer) {
            println!("e: problems with reading from stream");
        }

        // Create utf-8 string from buffer of bytes
        let buffer = str::from_utf8(&buffer)?
            .replace("\r\n", "\n");

        // Trim zero-bytes
        let buffer = buffer.trim_matches(char::from(0));

        // Split request to headers and body
        let request = buffer.split("\n\n").map(String::from).collect::<Vec<String>>();

        // Split headers
        let mut headers: Vec<String> = match request.get(0) {
            Some(headers) => headers.split("\n").map(String::from).collect(),
            None => Vec::new(),
        };

        // Remove request line from headers and save it separately
        let request_line = headers.remove(0);

        // Split request line to parts (request method, path and version)
        let request_line: Vec<String> = request_line.split(" ").map(String::from).collect();

        // Save request method
        let method = match request_line.get(0) {
            Some(e) => e.clone(),
            None => {
                println!("e: request processing error {:?}", buffer);
                String::new()
            }
        };
        
        // Save request path
        let path = match request_line.get(1) {
            Some(e) => e.clone(),
            None => {
                println!("e: request processing error {:?}", buffer);
                String::new()
            }
        };

        // Split path to path and params (for GETs)
        let path_with_params: Vec<&str> = path.split("?").collect();
        let params: HashMap<String, String> = HashMap::new();

        let (path, params) = if path_with_params.len() > 1 {
            // Save clear path (with no params)
            let path = match path_with_params.get(0) {
                Some(e) => e.to_string(),
                None => {
                    println!("e: request processing error {:?}", buffer);
                    String::new()
                }
            };

            // Save params line
            let params = match path_with_params.get(1) {
                Some(e) => e.to_string(),
                None => {
                    println!("e: request processing error {:?}, {:?}", request_line, headers);
                    String::new()
                }
            };

            // Split params line to vector
            let params = params.split("&").map(|e| {
                let e = e.split("=").collect::<Vec<&str>>();
    
                if e.len() == 2 {
                    (e.get(0).unwrap().to_string(), e.get(1).unwrap().to_string())
                } else {
                    (String::new(), String::new())
                }
            }).collect::<HashMap<String, String>>();

            (path, params)
        } else {
            (path, params)
        };

        // Process body part of request
        let body = match request.get(1) {
            Some(body) => body.to_string(),
            None => String::new(),
        };

        Ok(
            Request {
                method,
                path,
                params,
                headers,
                body,
            }
        )
    }
}

struct Response;

impl Response {
    fn process(stream: &mut TcpStream, headers: &Vec<String>, body: &str) -> io::Result<usize> {
        // Send response
        stream.write(format!("{}\n\n{}",
            headers.join("\n"),
            body
        ).as_bytes())
    }
}