mod config;
mod data_generator;
mod data_inserter;
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
    server.run(&format!(
        "{}:{}",
        &config.server.host,
        &config.server.port.to_string()
    ));
}
