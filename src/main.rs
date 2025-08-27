mod config;
mod model;
mod request;
mod server;
mod server_state;

use mysql::Pool;
use server::Server;

use crate::config::Config;

fn main() {
    let config = Config::load();
    let pool = Pool::new(config.database.url.as_str()).unwrap();
    let server = Server::new(pool);
    server.run("127.0.0.1:8080");
}
