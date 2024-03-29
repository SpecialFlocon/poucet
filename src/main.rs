mod commands;
mod events;
mod identifiers;
mod models;

use std::env;
use std::error;

use config::{Config, Environment, File};
use derivative::Derivative;
use poise::{Framework, FrameworkError, FrameworkOptions};
use poise::builtins;
use redis::{Commands, RedisResult};
use serenity::model::application::command::Command;
use serenity::model::gateway::GatewayIntents;
use serenity::model::id::GuildId;
use serenity::prelude::Mutex;
use tracing::{debug, error, info};

type Error = Box<dyn error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Bot, Error>;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Bot {
    #[derivative(Debug="ignore")]
    database: Mutex<redis::Connection>,
    run_mode: String,
}

impl Bot {
    async fn serves_guild(&self, guild_id: GuildId) -> RedisResult<bool> {
        let mut database = self.database.lock().await;

        database.hget(format!("guild:{}", guild_id), "configured")
    }
}

async fn error_handler(error: FrameworkError<'_, Bot, Error>) {
    match error {
        FrameworkError::Setup { error, .. } => error!("error setting up bot framework: {:?}", error),
        FrameworkError::EventHandler { error, ctx: _, event, framework: _ } => error!("error while handling event {}: {:?}", event.name(), error),
        FrameworkError::Command { error, ctx } => error!("error while running command {}: {:?}", ctx.command().name, error),
        FrameworkError::ArgumentParse { error, input, ctx } => {
            let incorrect_input = input.unwrap_or_default();

            error!("error while parsing argument: {:?}, incorrect input: {}", error, incorrect_input);
            poise::send_reply(ctx, |reply| {
                reply
                    .content(format!("Sorry, I didn't understand your command! Can you double-check your input?\nFor the record, here's the part I didn't understand (if any): {}", incorrect_input))
                    .ephemeral(true)
            }).await.ok();
        },
        _ => error!("discord API error: {:?}", error),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "dev".into());

    // Load configuration
    let configuration = Config::builder()
        // Load configuration file for desired run mode
        .add_source(File::with_name(&format!("config.{}.toml", run_mode)).required(false))
        // Load configuration from environment variables
        .add_source(Environment::with_prefix("poucet").separator("_"))
        // Build final configuration object
        .build()
        .expect("configuration error");

    debug!("Loaded configuration: {:?}", configuration);

    // Connect to Redis
    let redis_address = configuration.get_string("redis.address").unwrap_or_else(|_| "127.0.0.1:6379".into());
    let redis_username = configuration.get_string("redis.username").unwrap_or_default();
    let redis_password = configuration.get_string("redis.password").unwrap_or_default();

    let auth_info = if redis_username.is_empty() && redis_password.is_empty() {
        vec![]
    } else if redis_username.is_empty() {
        vec![redis_password]
    } else {
        vec![redis_username, redis_password]
    };

    let client = redis::Client::open(format!("redis://{}", redis_address)).expect("redis client creation error");
    let mut database = client.get_connection().expect("connecting to redis failed");

    // Authenticate to Redis with provided credentials, if applicable
    if !auth_info.is_empty() {
        redis::cmd("AUTH").arg(&auth_info).execute(&mut database);
    }

    let database = Mutex::new(database);

    // Create bot instance to be passed as context to command functions
    let bot = Bot { database, run_mode };

    // Connect to Discord and run bot framework
    let bot_token = configuration.get_string("discord.bot.token").expect("missing or incorrect discord bot token");
    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;
    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                commands::ping(),
                commands::setup(),
                commands::onboarding(),
            ],
            event_handler: |ctx, event, framework, user_data| {
                Box::pin(events::listener(ctx, event, framework, user_data))
            },
            on_error: |error| Box::pin(error_handler(error)),
            ..Default::default()
        })
        .token(&bot_token)
        .intents(intents)
        .setup(move |ctx, ready, framework| Box::pin(async move {
            let mut created_commands: serenity::Result<Vec<Command>>;

            // Register slash commands as guild commands if running in dev mode,
            // so that they're available immediately, which is easier for debugging.
            if bot.run_mode == "dev" {
                created_commands = Ok(vec![]);

                for guild in &ready.guilds {
                    created_commands = GuildId::set_application_commands(&guild.id, ctx, |builder| {
                        *builder = builtins::create_application_commands(&framework.options().commands);

                        builder
                    }).await;
                }
            } else {
                created_commands = Command::set_global_application_commands(ctx, |builder| {
                    *builder = builtins::create_application_commands(&framework.options().commands);

                    builder
                }).await;
            }

            match created_commands {
                Ok(commands) => {
                    info!("registered {} slash commands", commands.len());
                    debug!("slash commands: {:?}", commands);
                },
                Err(error) => error!("error registering slash commands: {}", error),
            }

            Ok(bot)
        }));

    framework.run().await.unwrap();
}
