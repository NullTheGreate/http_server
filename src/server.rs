use crate::config::{self, Config};
use crate::data_inserter::DataInserter;
use crate::data_inserter_with_tokio::DataInserterWithTokio;
use crate::request::Request;
use crate::server_state::ServerState;
use mysql::Pool;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

pub struct Server {
    state: Arc<Mutex<ServerState>>,
    rt: Runtime,
    config: Arc<Config>,
}

impl Server {
    pub fn new(pool: Pool, config: Arc<Config>) -> Self {
        Server {
            state: Arc::new(Mutex::new(ServerState::new(pool))),
            rt: Runtime::new().unwrap(),
            config,
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
            ("POST", path) if path.starts_with("/populate") => {
                let query = path.split('?').nth(1).unwrap_or("");
                let params: std::collections::HashMap<String, String> = query
                    .split('&')
                    .filter_map(|pair| {
                        let kv: Vec<&str> = pair.splitn(2, '=').collect();
                        if kv.len() == 2 {
                            Some((kv[0].to_string(), kv[1].to_string()))
                        } else {
                            None
                        }
                    })
                    .collect();
                let count = match params.get("count").and_then(|c| c.parse::<u32>().ok()) {
                    Some(count) => count,
                    None => {
                        return (
                            "HTTP/1.1 400 BAD REQUEST\r\n\r\n",
                            "Missing or invalid count parameter".to_string(),
                        );
                    }
                };

                let state = self.state.lock();
                let state = match state {
                    Ok(state) => state,
                    Err(e) => {
                        eprintln!("Failed to lock server state: {}", e);
                        return (
                            "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n",
                            "Server error".to_string(),
                        );
                    }
                };

                let inserter =
                    DataInserterWithTokio::new(state.pool.clone(), Arc::clone(&self.config));
                match self.rt.block_on(inserter.populate(count)) {
                    Ok(duration) => (
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n",
                        format!("Successfully populated {} records in {:?}", count, duration),
                    ),
                    Err(e) => (
                        "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n",
                        format!("Failed to populate records: {}", e),
                    ),
                }
            }
            ("POST", path) if path.starts_with("/populate2") => {
                let query = path.split('?').nth(1).unwrap_or("");
                let params: std::collections::HashMap<String, String> = query
                    .split('&')
                    .filter_map(|pair| {
                        let kv: Vec<&str> = pair.splitn(2, '=').collect();
                        if kv.len() == 2 {
                            Some((kv[0].to_string(), kv[1].to_string()))
                        } else {
                            None
                        }
                    })
                    .collect();
                let count = match params.get("count").and_then(|c| c.parse::<u32>().ok()) {
                    Some(count) => count,
                    None => {
                        return (
                            "HTTP/1.1 400 BAD REQUEST\r\n\r\n",
                            "Missing or invalid count parameter".to_string(),
                        );
                    }
                };

                let state = self.state.lock();
                let state = match state {
                    Ok(state) => state,
                    Err(e) => {
                        eprintln!("Failed to lock server state: {}", e);
                        return (
                            "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n",
                            "Server error".to_string(),
                        );
                    }
                };

                let inserter = DataInserter::new(state.pool.clone());
                match inserter.populate(count) {
                    Ok(_) => (
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n",
                        format!("Successfully populated {} records", count),
                    ),
                    Err(e) => (
                        "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n",
                        format!("Failed to populate records: {}", e),
                    ),
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
            rt: Runtime::new().unwrap(),
            config: Arc::clone(&self.config),
        }
    }
}
