mod configuration;

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    model::gateway::Ready,
    prelude::*,
};
use tracing::{error, info};

use configuration::Configuration;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, data: Ready) {
        info!("Authenticated as {}#{}", data.user.name, data.user.discriminator);
    }
}

#[tokio::main]
async fn main() {
    let configuration = Configuration::new().expect("configuration error");

    tracing_subscriber::fmt::init();

    let bot_token = configuration.discord.bot.token;
    let intents = GatewayIntents::empty();
    let mut client = Client::builder(&bot_token, intents)
        .event_handler(Handler)
        .await
        .expect("discord client creation error");

    if let Err(e) = client.start().await {
        error!("client runtime error: {:?}", e);
    }
}
