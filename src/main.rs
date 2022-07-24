mod commands;
mod events;

use std::env;
use std::sync::{Mutex, RwLock};

use config::{Config, Environment, File};
use once_cell::sync::Lazy;
use serenity::client::Client;
use serenity::prelude::*;
use tracing::{debug, error};

use events::Handler;

static CONFIG: Lazy<RwLock<Config>> = Lazy::new(|| {
    RwLock::new({
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "dev".into());

        let configuration = Config::builder()
            // Load configuration file for desired run mode
            .add_source(File::with_name(&format!("config.{}.toml", run_mode)).required(false))
            // Load configuration from environment variables
            .add_source(Environment::with_prefix("poucet").separator("_"))
            // Build final configuration object
            .build()
            .expect("configuration error");

        configuration
    })
});
static DEV: Lazy<bool> = Lazy::new(|| {
    let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "dev".into());

    run_mode == "dev"
});
static REDIS: Lazy<Mutex<redis::Connection>> = Lazy::new(|| {
    let config = CONFIG.read().unwrap().clone();
    let redis_address = config.get_string("redis.address").unwrap_or_else(|_| "127.0.0.1:6379".into());
    let redis_username = config.get_string("redis.username").unwrap_or_default();
    let redis_password = config.get_string("redis.password").unwrap_or_default();

    let auth_info = if redis_username.is_empty() && redis_password.is_empty() {
        String::new()
    } else if redis_username.is_empty() {
        redis_password
    } else if redis_password.is_empty() {
        redis_username
    } else {
        format!("{}:{}", redis_username, redis_password)
    };
    let url = if auth_info.is_empty() {
        format!("redis://{}", redis_address)
    } else {
        format!("redis://{}@{}", auth_info, redis_address)
    };

    let client = redis::Client::open(url).expect("redis client creation error");

    Mutex::new(client.get_connection().expect("redis connection error"))
});

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = CONFIG.read().unwrap().clone();
    debug!("Loaded configuration: {:?}", config);

    // Connect to Discord
    let bot_token = config.get_string("discord.bot.token").expect("missing or incorrect discord bot token");
    let intents = GatewayIntents::GUILD_MEMBERS;
    let mut client = Client::builder(&bot_token, intents)
        .event_handler(Handler)
        .await
        .expect("discord client creation error");

    if let Err(e) = client.start().await {
        error!("client runtime error: {:?}", e);
    }
}
