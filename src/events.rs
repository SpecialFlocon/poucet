use redis::Commands;
use serenity::builder::CreateMessage;
use serenity::http::StatusCode;
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType};
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member};
use serenity::model::id::ChannelId;
use serenity::model::permissions::Permissions;
use serenity::prelude::SerenityError;
use serenity::utils::Colour;
use tracing::{debug, error, info};

use crate::{Bot, Error};

const ONBOARDING_START: &str = "onboarding_start";

async fn setup_member_verification(ctx: &serenity::client::Context, bot: &Bot, member: &Member) -> Result<(), Error> {
    let guild_id = member.guild_id;
    let serves_guild = bot.serves_guild(guild_id).await?;

    if !serves_guild {
        return Ok(());
    }

    let guild_key = format!("guild:{}", guild_id);
    let mut database = bot.database.lock().await;
    let verification_category = database.hget(&guild_key, "verification_category")?;
    let verification_category = ChannelId(verification_category);

    let member_channel = guild_id.create_channel(&ctx.http, |channel| {
        channel
            .kind(ChannelType::Text)
            .name(member.user.tag().replace('#', "-"))
            .category(verification_category)
            .permissions(vec![PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY | Permissions::SEND_MESSAGES,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(member.user.id),
            }])
    }).await?;

    member_channel.send_message(&ctx.http, |message| new_member_wait_notice(member, message)).await?;

    Ok(())
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
                        .custom_id(ONBOARDING_START)
                        .style(ButtonStyle::Primary)
                        .emoji('🚪')
                        .label("Enter")
                })
            })
        })
}

fn new_member_wait_notice<'a, 'b>(member: &Member, message: &'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {
    message
        .content(format!("{}", member))
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

// Event dispatcher
pub async fn listener(ctx: &serenity::client::Context, event: &poise::Event<'_>, _framework: poise::FrameworkContext<'_, Bot, Error>, bot: &Bot) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => ready(data_about_bot),
        poise::Event::GuildCreate { guild, is_new } => guild_create(ctx, bot, guild, is_new).await?,
        poise::Event::InteractionCreate { interaction } => interaction_create(ctx, bot, interaction).await?,
        _ => {},
    }

    Ok(())
}

// Event handlers
fn ready(data: &Ready) {
    info!("Authenticated as {}#{}", data.user.name, data.user.discriminator);
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

async fn interaction_create(ctx: &serenity::client::Context, bot: &Bot, interaction: &Interaction) -> Result<(), Error> {
    if let Interaction::MessageComponent(interaction) = interaction {
        if interaction.data.custom_id == ONBOARDING_START {
            interaction.create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::DeferredUpdateMessage)
                    .interaction_response_data(|data| data) // No-op
            }).await?;
            setup_member_verification(ctx, bot, interaction.member.as_ref().unwrap()).await?;
        }
    }

    Ok(())
}
