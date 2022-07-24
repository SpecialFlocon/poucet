use redis::Commands;
use serenity::async_trait;
use serenity::client::EventHandler;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use tracing::{debug, error, info};

use crate::REDIS;
use crate::commands;

pub struct Handler;

impl Handler {
    fn guild_command(&self, command_name: &str) -> bool {
        commands::GUILD.contains(&command_name)
    }

    fn serves_guild(&self, guild_id: &GuildId) -> redis::RedisResult<bool> {
        REDIS.lock().unwrap().hget(format!("guild:{}", guild_id), "configured")
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        debug!("Received interaction: {:?}", interaction);

        match interaction {
            // Slash command
            Interaction::ApplicationCommand(interaction) => {
                let command_name = interaction.data.name.as_str();
                let guild_id = interaction.guild_id.unwrap();
                let serves_guild = self.serves_guild(&guild_id).unwrap_or_else(|error| {
                    error!("error querying database for bot configuration: {}", error);
                    false
                });

                debug!("Serves guild {}: {}", guild_id, serves_guild);

                // Do not handle commands on guilds for which the bot isn't configured.
                if self.guild_command(command_name) && !serves_guild {
                    if let Err(e) = interaction
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|data| {
                                    data
                                        .content("I am not configured to work on this server, I can't execute this command!")
                                        .ephemeral(true)
                                })
                        }).await
                    {
                        error!("Failed to respond negatively to slash command: {}", e);
                    }
                    return;
                }

                let command_result = match command_name {
                    "miniping" => commands::ping,
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

        // Register guild slash commands
        for guild in &data.guilds {
            let serves_guild = self.serves_guild(&guild.id).unwrap_or_else(|error| {
                error!("error querying database for bot configuration: {}", error);
                false
            });

            if !serves_guild {
                continue;
            }

            let result = GuildId::set_application_commands(&guild.id, &ctx.http, commands::guild_slash_commands).await;

            match result {
                Ok(commands) => {
                    info!("Registered {} slash commands in guild {}", commands.len(), guild.id);
                    debug!("Slash commands for guild {}: {:?}", guild.id, commands);
                },
                Err(e) => error!("Failed to register slash commands for guild {}: {}", guild.id, e),
            }
        }

        // Register global slash commands
        let result = Command::set_global_application_commands(&ctx.http, commands::global_slash_commands).await;

        match result {
            Ok(commands) => {
                info!("Registered {} global slash commands", commands.len());
                debug!("Global slash commands: {:?}", commands);
            },
            Err(e) => error!("Failed to register global slash commands: {}", e),
        }
    }
}
