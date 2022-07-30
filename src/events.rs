use redis::{Commands, Connection};
use serenity::builder::{CreateMessage, CreateInteractionResponseFollowup};
use serenity::http::StatusCode;
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType};
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member, Role};
use serenity::model::id::{ChannelId, GuildId, RoleId, UserId};
use serenity::model::permissions::Permissions;
use serenity::model::user::User;
use serenity::prelude::{Mentionable, SerenityError};
use serenity::utils::Colour;
use tokio::sync::MutexGuard;
use tracing::{debug, error, info};

use crate::{Bot, Error};
use crate::identifiers;

// Event dispatcher
pub async fn listener(ctx: &serenity::client::Context, event: &poise::Event<'_>, _framework: poise::FrameworkContext<'_, Bot, Error>, bot: &Bot) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => ready(data_about_bot),
        poise::Event::GuildCreate { guild, is_new } => guild_create(ctx, bot, guild, is_new).await,
        poise::Event::GuildMemberRemoval { guild_id, user, member_data_if_available: _ } => guild_member_removal(ctx, bot, guild_id, user).await,
        poise::Event::InteractionCreate { interaction } => interaction_create(ctx, bot, interaction).await,
        _ => Ok(()),
    }
}

// Event handlers
fn ready(data: &Ready) -> Result<(), Error> {
    info!("Authenticated as {}#{}", data.user.name, data.user.discriminator);

    Ok(())
}

async fn guild_create(ctx: &serenity::client::Context, bot: &Bot, guild: &Guild, is_new: &bool) -> Result<(), Error> {
    debug!("guild_create event fired for {}", guild.id);

    if *is_new {
        return Ok(());
    }

    let serves_guild = bot.serves_guild(guild.id).await?;

    if !serves_guild {
        return Ok(());
    }

    let guild_key = format!("guild:{}", guild.id);
    let mut database = bot.database.lock().await;
    let welcome_channel = database.hget(&guild_key, "welcome_channel")?;
    let welcome_channel = ChannelId(welcome_channel);

    if database.hexists(&guild_key, "welcome_message")? {
        let welcome_message: u64 = database.hget(&guild_key, "welcome_message")?;
        let welcome_message = welcome_channel.message(&ctx.http, welcome_message).await;

        if welcome_message.is_ok() {
            return Ok(());
        }

        let error = welcome_message.err().unwrap();

        if let SerenityError::Http(error) = error {
            if let Some(status_code) = error.status_code() {
                if status_code == StatusCode::NOT_FOUND {
                    let new_welcome_message = welcome_channel.send_message(&ctx.http, welcome_instructions).await?;

                    database.hset(&guild_key, "welcome_message", new_welcome_message.id.as_u64())?;

                    return Ok(());
                }
            }

            error!("{}", error);
        }
    } else {
        let new_welcome_message = welcome_channel.send_message(&ctx.http, welcome_instructions).await?;

        database.hset(&guild_key, "welcome_message", new_welcome_message.id.as_u64())?;
    }

    Ok(())
}

async fn guild_member_removal(ctx: &serenity::client::Context, bot: &Bot, guild_id: &GuildId, user: &User) -> Result<(), Error> {
    onboarding_member_removal(ctx, bot, guild_id, user).await?;

    Ok(())
}

async fn interaction_create(ctx: &serenity::client::Context, bot: &Bot, interaction: &Interaction) -> Result<(), Error> {
    if let Interaction::MessageComponent(interaction) = interaction {
        let guild_id = interaction.guild_id.unwrap();
        let serves_guild = bot.serves_guild(guild_id).await?;

        if !serves_guild {
            return Ok(());
        }

        match interaction.data.custom_id.as_str() {
            identifiers::ONBOARDING_ARCHIVE => onboarding_archive(ctx, bot, interaction).await?,
            identifiers::ONBOARDING_DELETE => onboarding_delete(ctx, bot, interaction).await?,
            identifiers::ONBOARDING_START => onboarding_start(ctx, bot, interaction).await?,
            _ => (),
        }
    }

    Ok(())
}

// Onboarding actions
async fn onboarding_archive(ctx: &serenity::client::Context, bot: &Bot, interaction: &MessageComponentInteraction) -> Result<(), Error> {
    interaction.create_interaction_response(&ctx.http, |response| {
        response
            .kind(InteractionResponseType::DeferredUpdateMessage)
            .interaction_response_data(|data| data)
    }).await?;

    let guild_id = interaction.guild_id.unwrap();
    let validation_user_to_channel_key = "validation_user_to_channel";
    let validation_channel_to_user_key = "validation_channel_to_user";
    let mut database = bot.database.lock().await;

    let user_id: u64 = database.hget(validation_channel_to_user_key, interaction.channel_id.as_u64())?;
    let user_id = UserId(user_id);
    let validation_key = format!("validation:{}:{}", guild_id, user_id);
    let validation_channel = pending_validation(&mut database, &guild_id, &user_id).await?;

    if let Some(validation_channel) = validation_channel {
        // Detach validation instance from the database so that the bot stops managing it
        database.hdel(&validation_key, "channel")?;
        database.hdel(&validation_user_to_channel_key, user_id.as_u64())?;
        database.hdel(&validation_channel_to_user_key, validation_channel.as_u64())?;

        let mut guild_channels = guild_id.channels(&ctx.http).await?;
        let validation_guild_channel = guild_channels.get_mut(&validation_channel).ok_or_else(|| {
            Error::from(format!("channel {} is set as the validation channel for user {}, but it was not found in the server.", validation_channel, user_id))
        })?;

        // Rename channel to indicate it is archived
        let channel_name = format!("📦-{}", validation_guild_channel.name);

        validation_guild_channel.edit(&ctx, |channel| {
            channel.name(&channel_name)
        }).await?;

        info!("archived validation channel {} in guild {}", validation_channel, guild_id);

        interaction.edit_original_interaction_response(&ctx.http, |response| {
            response.components(|components| {
                components.create_action_row(|row| {
                    row
                        .create_button(|button| {
                            button
                                .custom_id(identifiers::ONBOARDING_ARCHIVE)
                                .style(ButtonStyle::Primary)
                                .disabled(true)
                                .label("Archive")
                        })
                        .create_button(|button| {
                            button
                                .custom_id(identifiers::ONBOARDING_DELETE)
                                .style(ButtonStyle::Danger)
                                .label("Delete")
                        })
                })
            })
        }).await?;
    }

    Ok(())
}

async fn onboarding_delete(ctx: &serenity::client::Context, bot: &Bot, interaction: &MessageComponentInteraction) -> Result<(), Error> {
    let guild_id = interaction.guild_id.unwrap();
    let validation_user_to_channel_key = "validation_user_to_channel";
    let validation_channel_to_user_key = "validation_channel_to_user";
    let mut database = bot.database.lock().await;

    if database.hexists(validation_channel_to_user_key, interaction.channel_id.as_u64())? {
        let user_id: u64 = database.hget(validation_channel_to_user_key, interaction.channel_id.as_u64())?;
        let user_id = UserId(user_id);
        let validation_key = format!("validation:{}:{}", interaction.guild_id.unwrap(), user_id);
        let validation_channel = pending_validation(&mut database, &guild_id, &user_id).await?;

        if let Some(validation_channel) = validation_channel {
            // Detach validation instance from the database so that the bot stops managing it
            database.hdel(&validation_key, "channel")?;
            database.hdel(&validation_user_to_channel_key, user_id.as_u64())?;
            database.hdel(&validation_channel_to_user_key, validation_channel.as_u64())?;
        }
    }

    interaction.channel_id.delete(&ctx.http).await?;

    Ok(())
}

async fn onboarding_member_removal(ctx: &serenity::client::Context, bot: &Bot, guild_id: &GuildId, user: &User) -> Result<(), Error> {
    debug!("Member {} left the server, prompting staff to decide what to do with the validation channel", &user.id);

    let mut database = bot.database.lock().await;
    let validation_channel = pending_validation(&mut database, guild_id, &user.id).await?;

    if let Some(validation_channel) = validation_channel {
        validation_channel.send_message(&ctx.http, |message| {
            message
                .embed(|embed| {
                    embed
                        .colour(Colour::DARK_RED)
                        .title("Member left")
                        .description(format!("{} ({}#{}) has left the server", user, user.name, user.discriminator))
                })
                .components(|components| {
                    components.create_action_row(|row| {
                        row
                            .create_button(|button| {
                                button
                                    .custom_id(identifiers::ONBOARDING_ARCHIVE)
                                    .style(ButtonStyle::Primary)
                                    .label("Archive")
                            })
                            .create_button(|button| {
                                button
                                    .custom_id(identifiers::ONBOARDING_DELETE)
                                    .style(ButtonStyle::Danger)
                                    .label("Delete")
                            })
                    })
                })
        }).await?;
    }

    Ok(())
}

async fn onboarding_start(ctx: &serenity::client::Context, bot: &Bot, interaction: &MessageComponentInteraction) -> Result<(), Error> {
    interaction.create_interaction_response(&ctx.http, |response| {
        response
            .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            .interaction_response_data(|data| data.ephemeral(true))
    }).await?;

    let guild_id = interaction.guild_id.unwrap();
    let mut database = bot.database.lock().await;

    let member = interaction.member.as_ref().unwrap();
    let validation_channel = pending_validation(&mut database, &guild_id, &member.user.id).await?;

    if validation_channel.is_none() {
        setup_member_verification(ctx, &mut database, member).await?;
    }

    interaction.create_followup_message(&ctx.http, |message| {
        reply_to_join_request(validation_channel, message)
    }).await?;

    Ok(())
}

// Utility functions
async fn pending_validation<'a>(database: &mut MutexGuard<'a, Connection>, guild_id: &GuildId, user_id: &UserId) -> Result<Option<ChannelId>, Error> {
    let user_id = user_id.as_u64();
    let validation_key = format!("validation:{}:{}", guild_id, user_id);

    if !database.hexists(&validation_key, "channel")? {
        return Ok(None);
    }

    Ok(Some(ChannelId(database.hget(&validation_key, "channel")?)))
}

async fn setup_member_verification<'a>(ctx: &serenity::client::Context, database: &mut MutexGuard<'a, Connection>, member: &Member) -> Result<(), Error> {
    let guild_id = member.guild_id;
    let guild_key = format!("guild:{}", guild_id);
    let onboarding_key = format!("onboarding:{}", guild_id);
    let validation_key = format!("validation:{}:{}", guild_id, member.user.id.as_u64());
    let validation_user_to_channel_key = "validation_user_to_channel";
    let validation_channel_to_user_key = "validation_channel_to_user";
    let roles = guild_id.roles(&ctx.http).await?;
    let notify_role = database.hget(&onboarding_key, "notify_role")?;
    let notify_role = roles.get(&RoleId(notify_role)).ok_or_else(|| {
        Error::from(format!("role {} is configured as the notify role for onboarding, but it doesn't exist in the guild", notify_role))
    })?;
    let validation_category = database.hget(&guild_key, "validation_category")?;
    let validation_category = ChannelId(validation_category);

    let member_channel = guild_id.create_channel(&ctx.http, |channel| {
        channel
            .kind(ChannelType::Text)
            .name(member.user.tag().replace('#', "-"))
            .category(validation_category)
    }).await?;

    member_channel.create_permission(&ctx.http, &PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY | Permissions::SEND_MESSAGES,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(member.user.id),
    }).await?;

    database.hset(&validation_key, "channel", member_channel.id.as_u64())?;
    database.hset(validation_channel_to_user_key, member_channel.id.as_u64(), member.user.id.as_u64())?;
    database.hset(validation_user_to_channel_key, member.user.id.as_u64(), member_channel.id.as_u64())?;

    member_channel.send_message(&ctx.http, |message| new_member_wait_notice(member, notify_role, message)).await?;

    Ok(())
}

fn reply_to_join_request<'a, 'b>(pending_validation_channel: Option<ChannelId>, followup_message: &'b mut CreateInteractionResponseFollowup<'a>) -> &'b mut CreateInteractionResponseFollowup<'a> {
    match pending_validation_channel {
        Some(channel) => {
            followup_message
                .content(format!("You already asked to join the server! Your request is being discussed in {}.", channel.mention()))
        },
        None => {
            followup_message
                .content("Your request to join the server has been received. Follow the ping!")
        }
    }
}

fn welcome_instructions<'a, 'b>(message: &'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {
    message
        .embed(|embed| {
            embed
                .colour(Colour::BLITZ_BLUE)
                .title("Welcome to Transpouce! 👋")
                .description(
                    "This server is a safe space for discussion and exchange among trans and/or questioning people who are living in the Netherlands. It is open to 18+ people only, and is not tied to any existing organization, association or group.

The main language of the server is English.

To keep our space safe and gezellig, we have a simple verification process for new members in place. By clicking the button below, you'll be added to a private channel with the server staff, where we'll ask you some questions and get to know each other a little! <:transkitty:1000713242236178442>"
                )
        })
        .components(|components| {
            components.create_action_row(|row| {
                row.create_button(|button| {
                    button
                        .custom_id(identifiers::ONBOARDING_START)
                        .style(ButtonStyle::Primary)
                        .emoji('🚪')
                        .label("Enter")
                })
            })
        })
}

fn new_member_wait_notice<'a, 'b>(member: &Member, notify_role: &Role, message: &'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {
    message
        .content(format!("{} {}", member, notify_role))
        .embed(|embed| {
            embed
                .colour(Colour::BLITZ_BLUE)
                .description(
                    "**Hey there! 👋**

Hold on, a staff member will be with you soon to help you get started.

To speed up the validation process, can you already tell us a few words about you, how or where you found out about this server, what brings you here, etc.? Thank you! 😁"
                )
        })
}
