use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Config {
    pub fn load() -> Self {
        let content = fs::read_to_string("config.toml").expect("Failed to read config.toml");
        toml::from_str(&content).expect("Failed to parse config.toml")
    }
}
