use crate::request::Request;
use crate::server_state::ServerState;
use mysql::Pool;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Server {
    state: Arc<Mutex<ServerState>>,
}

impl Server {
    pub fn new(pool: Pool) -> Self {
        Server {
            state: Arc::new(Mutex::new(ServerState::new(pool))),
        }
    }

    pub fn handle_request(&self, request: Request) -> (&'static str, String) {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", path) if path.starts_with("/person/") => {
                let id_str = path.strip_prefix("/person/").unwrap_or("");
                match id_str.parse::<u32>() {
                    Ok(id) => {
                        let state = self.state.lock().unwrap();
                        match state.get_person(id) {
                            Some(person) => (
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n",
                                format!(
                                    "ID: {}, Name: {}, Age: {}",
                                    person.id, person.name, person.email
                                ),
                            ),
                            None => (
                                "HTTP/1.1 404 NOT FOUND\r\n\r\n",
                                "Person not found".to_string(),
                            ),
                        }
                    }
                    Err(_) => ("HTTP/1.1 400 BAD REQUEST\r\n\r\n", "Invalid ID".to_string()),
                }
            }
            ("POST", "/person") => {
                let params = request.parse_body();
                let name = match params.get("name") {
                    Some(name) => name.clone(),
                    None => {
                        return (
                            "HTTP/1.1 400 BAD REQUEST\r\n\r\n",
                            "Missing name".to_string(),
                        );
                    }
                };
                let age = match params.get("age").and_then(|age| age.parse::<u32>().ok()) {
                    Some(age) => age,
                    None => {
                        return (
                            "HTTP/1.1 400 BAD REQUEST\r\n\r\n",
                            "Invalid or missing age".to_string(),
                        );
                    }
                };

                let mut state = self.state.lock().unwrap();
                let id = state.add_person(name, age);
                (
                    "HTTP/1.1 201 CREATED\r\nContent-Type: text/plain\r\n\r\n",
                    format!("Person created with ID: {}", id),
                )
            }
            ("PUT", path) if path.starts_with("/person/") => {
                let id_str = path.strip_prefix("/person/").unwrap_or("");
                match id_str.parse::<u32>() {
                    Ok(id) => {
                        let params = request.parse_body();
                        let name = match params.get("name") {
                            Some(name) => name.clone(),
                            None => {
                                return (
                                    "HTTP/1.1 400 BAD REQUEST\r\n\r\n",
                                    "Missing name".to_string(),
                                );
                            }
                        };
                        let age = match params.get("age").and_then(|age| age.parse::<u32>().ok()) {
                            Some(age) => age,
                            None => {
                                return (
                                    "HTTP/1.1 400 BAD REQUEST\r\n\r\n",
                                    "Invalid or missing age".to_string(),
                                );
                            }
                        };

                        let mut state = self.state.lock().unwrap();
                        if state.update_person(id, name, age) {
                            (
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n",
                                "Person updated".to_string(),
                            )
                        } else {
                            (
                                "HTTP/1.1 404 NOT FOUND\r\n\r\n",
                                "Person not found".to_string(),
                            )
                        }
                    }
                    Err(_) => ("HTTP/1.1 400 BAD REQUEST\r\n\r\n", "Invalid ID".to_string()),
                }
            }
            _ => (
                "HTTP/1.1 404 NOT FOUND\r\n\r\n",
                "404 - Endpoint not found".to_string(),
            ),
        }
    }

    pub fn handle_client(&self, mut stream: TcpStream) {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(_) => {
                let request_str = String::from_utf8_lossy(&buffer[..]);
                let request = match Request::parse(&request_str) {
                    Some(req) => req,
                    None => {
                        let response = "HTTP/1.1 400 BAD REQUEST\r\n\r\nBad Request";
                        stream.write_all(response.as_bytes()).unwrap();
                        stream.flush().unwrap();
                        return;
                    }
                };
                let (status_line, contents) = self.handle_request(request);
                let response = format!("{}{}", status_line, contents);
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
            Err(e) => eprintln!("Failed to read from stream: {}", e),
        }
    }

    pub fn run(&self, addr: &str) {
        let listener = TcpListener::bind(addr).unwrap();
        println!("Server running on http://{}", addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let server = self.clone();
                    thread::spawn(move || {
                        server.handle_client(stream);
                    });
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Server {
            state: Arc::clone(&self.state),
        }
    }
}
