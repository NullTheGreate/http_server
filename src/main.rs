mod config;
mod data_generator;
mod data_inserter;
mod data_inserter_with_tokio;
mod model;
mod request;
mod server;
mod server_state;

use mysql::Pool;
use server::Server;
// use tokio::task::JoinHandle;

use crate::config::Config;

// #[tokio::main]
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
