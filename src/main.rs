use std::env;

use config::{Config, Environment, File};
use lazy_static::lazy_static;
use serenity::{
    async_trait,
    client::{Client, EventHandler},
    model::gateway::Ready,
    prelude::*,
};
use tracing::{error, info};

lazy_static! {
    static ref CONFIG: RwLock<Config> = RwLock::new({
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
    });
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, data: Ready) {
        info!("Authenticated as {}#{}", data.user.name, data.user.discriminator);
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = CONFIG.read().await.clone();
    let bot_token = config.get_string("discord.bot.token").expect("missing or incorrect discord bot token");
    let intents = GatewayIntents::empty();
    let mut client = Client::builder(&bot_token, intents)
        .event_handler(Handler)
        .await
        .expect("discord client creation error");

    if let Err(e) = client.start().await {
        error!("client runtime error: {:?}", e);
    }
}
