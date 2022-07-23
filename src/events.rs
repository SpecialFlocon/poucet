use serenity::async_trait;
use serenity::client::EventHandler;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use tracing::{debug, error, info};

use crate::{CONFIG, DEV};
use crate::commands::{global_slash_commands, guild_slash_commands};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            debug!("Received command interaction: {:?}", command);

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

    async fn guild_member_addition(&self, _ctx: Context, member: Member) {
        debug!("New member {}#{}: {:?}", member.user.name, member.user.discriminator, member);
    }

    async fn ready(&self, ctx: Context, data: Ready) {
        info!("Authenticated as {}#{}", data.user.name, data.user.discriminator);

        let config = CONFIG.read().unwrap().clone();
        let guild_id = config.get_string("discord.guild.id").expect("missing Discord guild ID");
        let guild_id = GuildId(guild_id
                               .parse()
                               .expect("Discord guild ID must be an integer"));

        // Register guild slash commands
        let result = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| guild_slash_commands(commands)).await;

        match result {
            Ok(commands) => {
                info!("Registered {} guild slash commands", commands.len());
                debug!("Guild slash commands: {:?}", commands);
            },
            Err(e) => error!("Failed to register guild slash commands: {}", e),
        }

        // Register global slash commands
        //
        // When running in development mode, register commands as guild slash commands to avoid
        // caching and for quicker access.
        let result = if *DEV {
            GuildId::set_application_commands(&guild_id, &ctx.http, |commands| global_slash_commands(commands)).await
        } else {
            Command::set_global_application_commands(&ctx.http, |commands| global_slash_commands(commands)).await
        };

        match result {
            Ok(commands) => {
                info!("Registered {} global slash commands", commands.len());
                debug!("Global slash commands: {:?}", commands);
            },
            Err(e) => error!("Failed to register global slash commands: {}", e),
        }
    }
}

