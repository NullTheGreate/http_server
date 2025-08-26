mod model;
mod request;
mod server;
mod server_state;

use server::Server;

fn main() {
    let server = Server::new();
    server.run("127.0.0.1:8080");
}
