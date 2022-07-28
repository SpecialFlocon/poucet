use redis::Commands;
use serenity::model::channel::Channel;
use serenity::model::guild::Role;
use serenity::model::id::RoleId;
use tracing::error;

use crate::{Context, Error};
use crate::models::GuildConfiguration;

/// Am I responding? Use this command to find out!
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let serves_guild = ctx.data().serves_guild(ctx.guild_id().unwrap()).await?;

    if !serves_guild {
        poise::send_reply(ctx, |reply| {
            reply
                .content("I am not configured to work on this server! Have an admin configure me using the /setup command.")
                .ephemeral(true)
        }).await?;
    } else {
        ctx.say("Coin !").await?;
    }

    Ok(())
}

/// Configure onboarding in this guild
#[poise::command(slash_command)]
pub async fn onboarding(
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
        !ctx.author().has_role(&ctx.discord(), guild_id, admin_role).await? {
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

/// Configure Poucet to serve a guild
#[poise::command(slash_command)]
pub async fn setup(
    ctx: Context<'_>,
    #[description = "The role that is allowed to run restricted commands"] admin_role: Role,
    #[description = "Category in which to create private channels for member verification"] verification_category: Channel,
    #[description = "Channel in which to post a welcome message for new members"] welcome_channel: Channel,
    #[description = "Run setup for an already configured guild"] anew: Option<bool>,
) -> Result<(), Error> {
    let bot = ctx.data();
    let guild_id = ctx.guild_id().unwrap();
    let serves_guild = bot.serves_guild(guild_id).await?;

    if serves_guild && !anew.unwrap_or_default() {
        poise::send_reply(ctx, |reply| {
            reply
                .content("I'm already configured to serve this guild! Run this command again with the `anew` parameter set to `true` to reconfigure me.")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let guild_configuration = GuildConfiguration::new(admin_role, verification_category, welcome_channel).unwrap_or_else(|error| {
        error!("incorrect server configuration values: {}", error);

        GuildConfiguration::default()
    });

    if !guild_configuration.configured {
        poise::send_reply(ctx, |reply| {
            reply
                .content("Got incorrect configuration, please make sure the values you pass are of the right type!")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let guild_key = format!("guild:{}", guild_id);
    let mut database = bot.database.lock().await;

    database.hset(&guild_key, "configured", guild_configuration.configured)?;
    database.hset(&guild_key, "admin_role", guild_configuration.admin_role.unwrap().id.as_u64())?;
    database.hset(&guild_key, "verification_category", guild_configuration.verification_category.unwrap().id.as_u64())?;
    database.hset(&guild_key, "welcome_channel", guild_configuration.welcome_channel.unwrap().id.as_u64())?;

    ctx.say("🙌 All set! Poucet is now ready to use 🤖✨").await?;

    Ok(())
}
