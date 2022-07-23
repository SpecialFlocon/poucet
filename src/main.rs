use std::env;

use config::{Config, Environment, File};
use lazy_static::lazy_static;
use serenity::{
    async_trait,
    client::{Client, EventHandler},
    model::application::interaction::{Interaction, InteractionResponseType},
    model::gateway::Ready,
    model::id::GuildId,
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
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            info!("Received command interaction: {:#?}", command);

            let content = match command.data.name.as_str() {
                "ping" => "Coin !".to_string(),
                _ => "Not implemented (yet!)".to_string(),
            };

            if let Err(e) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                }).await
            {
                error!("Failed to respond to slash command: {}", e);
            }
        }
    }

    async fn ready(&self, ctx: Context, data: Ready) {
        info!("Authenticated as {}#{}", data.user.name, data.user.discriminator);

        let config = CONFIG.read().await.clone();
        let guild_id = config.get_string("discord.guild.id").expect("missing Discord guild ID");
        let guild_id = GuildId(guild_id
                               .parse()
                               .expect("Discord guild ID must be an integer"));

        let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command.name("ping").description("Check bot responsiveness")
                })
        })
        .await;

        info!("Registered guild slash commands: {:#?}", commands);
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
