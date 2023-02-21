use redis::Commands;
use serenity::model::application::component::ButtonStyle;
use serenity::model::channel::PermissionOverwriteType;
use serenity::model::guild::Role;
use serenity::model::id::{ChannelId, RoleId, UserId};
use serenity::prelude::Mentionable;

use crate::{Context, Error};
use crate::identifiers;

/// Configure onboarding in this guild
#[poise::command(
    slash_command,
    subcommands("configure", "approve", "deny"),
)]
pub async fn onboarding(_: Context<'_>) -> Result<(), Error>{
    Ok(())
}

/// Configure onboarding in this guild
#[poise::command(slash_command)]
async fn configure(
    ctx: Context<'_>,
    #[description = "The staff role to notify when a new member requests access to the server."] notify_role: Role,
) -> Result<(), Error> {
    let bot = ctx.data();
    let guild_id = ctx.guild_id().unwrap();
    let serves_guild = bot.serves_guild(guild_id).await?;

    if !serves_guild {
        poise::send_reply(ctx, |reply| {
            reply
                .content("I am not configured to work on this server! Have an admin configure me using the /setup command.")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let guild_key = format!("guild:{}", guild_id);
    let mut database = bot.database.lock().await;

    let admin_role = database.hget(&guild_key, "admin_role")?;
    let admin_role = RoleId(admin_role);

    if ctx.author().id != ctx.guild().unwrap().owner_id &&
        !ctx.author().has_role(&ctx, guild_id, admin_role).await? {
        poise::send_reply(ctx, |reply| {
            reply
                .content("This is an admin command, you do not have the required rights to run it!")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let onboarding_key = format!("onboarding:{}", guild_id);

    database.hset(&onboarding_key, "notify_role", notify_role.id.as_u64())?;

    ctx.say(format!("✅ Set {} as the staff role to notify when new members join", notify_role)).await?;

    Ok(())
}

/// Approve a member's request to join the server
#[poise::command(slash_command)]
async fn approve(ctx: Context<'_>) -> Result<(), Error> {
    let bot = ctx.data();
    let guild_id = ctx.guild_id().unwrap();
    let serves_guild = bot.serves_guild(guild_id).await?;

    if !serves_guild {
        poise::send_reply(ctx, |reply| {
            reply
                .content("I am not configured to work on this server! Have an admin configure me using the /setup command.")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let guild = ctx.guild().unwrap();
    let guild_key = format!("guild:{}", guild_id);
    let mut database = bot.database.lock().await;

    let admin_role = database.hget(&guild_key, "admin_role")?;
    let admin_role = RoleId(admin_role);

    if ctx.author().id != guild.owner_id &&
        !ctx.author().has_role(&ctx, guild_id, admin_role).await? {
            poise::send_reply(ctx, |reply| {
                reply
                    .content("This is an admin command, you do not have the required rights to run it!")
                    .ephemeral(true)
            }).await?;

            return Ok(());
    }

    let channel_id = ctx.channel_id();
    let validation_channel_to_user_key = "validation_channel_to_user";

    if !database.hexists(validation_channel_to_user_key, channel_id.as_u64())? {
        poise::send_reply(ctx, |reply| {
            reply
                .content("The member approval command must be run in the corresponding validation channel.")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let user_id: u64 = database.hget(validation_channel_to_user_key, channel_id.as_u64())?;
    let user_id = UserId(user_id);

    // Remove approved member's access to the validation channel
    channel_id.delete_permission(&ctx, PermissionOverwriteType::Member(user_id)).await?;

    let mut member = guild_id.member(&ctx, user_id).await?;
    let validated_role = database.hget(&guild_key, "validated_role")?;
    let validated_role = RoleId(validated_role);

    member.add_role(ctx, validated_role).await?;

    poise::send_reply(ctx, |reply| {
        reply
            .content(format!("Approved {} ({}#{})", member.user, member.user.name, member.user.discriminator))
            .components(|components| {
                components.create_action_row(|row| {
                    row
                        .create_button(|button| {
                            button
                                .custom_id(identifiers::ONBOARDING_ARCHIVE)
                                .style(ButtonStyle::Secondary)
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

    if let Some(system_channel) = guild.system_channel_id {
        let introductions_channel = database.hget(&guild_key, "introductions_channel")?;
        let introductions_channel = ChannelId(introductions_channel);
        let role_assignment_channel = database.hget(&guild_key, "role_assignment_channel")?;
        let role_assignment_channel = ChannelId(role_assignment_channel);

        system_channel.send_message(&ctx, |message| {
            message.content(format!(
                "👋 Welcome {} to Transpouce! Feel free to grab some roles in {}, and to write a few words about yourself in {} if you like. Have a pleasant stay here! 🤗",
                member.mention(),
                role_assignment_channel.mention(),
                introductions_channel.mention(),
            ))
        }).await?;
    }

    Ok(())
}

/// Deny a member's request to join the server.
#[poise::command(slash_command)]
async fn deny(ctx: Context<'_>) -> Result<(), Error> {
    let bot = ctx.data();
    let guild_id = ctx.guild_id().unwrap();
    let serves_guild = bot.serves_guild(guild_id).await?;

    if !serves_guild {
        poise::send_reply(ctx, |reply| {
            reply
                .content("I am not configured to work on this server! Have an admin configure me using the /setup command.")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let guild_key = format!("guild:{}", guild_id);
    let mut database = bot.database.lock().await;

    let admin_role = database.hget(&guild_key, "admin_role")?;
    let admin_role = RoleId(admin_role);

    if ctx.author().id != ctx.guild().unwrap().owner_id &&
        !ctx.author().has_role(&ctx, guild_id, admin_role).await? {
        poise::send_reply(ctx, |reply| {
            reply
                .content("This is an admin command, you do not have the required rights to run it!")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let channel_id = ctx.channel_id();
    let validation_channel_to_user_key = "validation_channel_to_user";

    if !database.hexists(validation_channel_to_user_key, channel_id.as_u64())? {
        poise::send_reply(ctx, |reply| {
            reply
                .content("The member denial command must be run in the corresponding validation channel.")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let user_id: u64 = database.hget(validation_channel_to_user_key, channel_id.as_u64())?;
    let user_id = UserId(user_id);
    let member = guild_id.member(&ctx, user_id).await?;

    member.kick_with_reason(&ctx, "Denied at validation").await?;

    poise::send_reply(ctx, |reply| {
        reply.content(format!("Denied {} ({}#{})", member.user, member.user.name, member.user.discriminator))
    }).await?;

    Ok(())
}
