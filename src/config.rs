use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub rate_limit: u32,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn load() -> Self {
        let content = fs::read_to_string("config.toml").expect("Failed to read config.toml");
        toml::from_str(&content).expect("Failed to parse config.toml")
    }
}
