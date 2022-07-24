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
use crate::commands;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        debug!("Received interaction: {:?}", interaction);

        match interaction {
            // Slash command
            Interaction::ApplicationCommand(interaction) => {
                let command_result = match interaction.data.name.as_str() {
                    "ping" => commands::ping,
                    "setup" => commands::setup::home,
                    _ => commands::fallback,
                };

                if let Err(e) = interaction
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(command_result)
                    }).await
                {
                    error!("Failed to respond to slash command: {}", e);
                }
            },
            // Component interaction (button click, etc.)
            Interaction::MessageComponent(interaction) => {
                let followup_action = match interaction.data.custom_id.as_str() {
                    "setup_onboarding" => commands::setup::onboarding,
                    "setup_go_back" => commands::setup::home,
                    "setup_done" => commands::setup::done,
                    _ => {
                        if let Err(e) = interaction.delete_original_interaction_response(&ctx.http).await {
                            error!("Failed to delete original interaction response: {}", e);
                        };
                        return;
                    },
                };

                if let Err(e) = interaction
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::UpdateMessage)
                            .interaction_response_data(followup_action)
                    }).await
                {
                    error!("Failed to respond to component interaction: {}", e)
                }
            },
            _ => (),
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
        let result = GuildId::set_application_commands(&guild_id, &ctx.http, commands::guild_slash_commands).await;

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
            GuildId::set_application_commands(&guild_id, &ctx.http, commands::global_slash_commands).await
        } else {
            Command::set_global_application_commands(&ctx.http, commands::global_slash_commands).await
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

