use std::fs::File;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // read from  ./app.yml, or /etc/config/app.yml, or from env CHAT_CONFIG
        let ret = match (
            File::open("./app.yml"),
            File::open("/etc/config/app.yml"),
            std::env::var("CHAT_CONFIG"),
        ) {
            (Ok(f), _, _) => serde_yaml::from_reader(f),
            (_, Ok(f), _) => serde_yaml::from_reader(f),
            (_, _, Ok(f)) => serde_yaml::from_reader(File::open(f)?),
            _ => bail!("Config file not found"),
        };
        Ok(ret?)
    }
}