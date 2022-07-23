use std::env;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Bot {
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Discord {
    pub bot: Bot,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Configuration {
    pub debug: bool,
    pub discord: Discord,
}

impl Configuration {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "dev".into());

        let configuration = Config::builder()
            // Load configuration file for desired run mode
            .add_source(File::with_name(&format!("config.{}.toml", run_mode)).required(false))
            // Load configuration from environment variables
            .add_source(Environment::with_prefix("poucet").separator("_"))
            // Build final configuration object
            .build()?;

        configuration.try_deserialize()
    }
}
