use redis::Commands;
use serenity::model::channel::Channel;
use tracing::{debug, error};

use crate::{Context, Error};

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

/// Configure Poucet to serve a guild
#[poise::command(slash_command)]
pub async fn setup(
    ctx: Context<'_>,
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

    if matches!(verification_category, Channel::Guild(_) | Channel::Private(_)) || matches!(welcome_channel, Channel::Private(_) | Channel::Category(_)) {
        error!("Setup in guild {} was called with incorrect channel types", &guild_id);
        debug!("given values are: verification category {}, welcome channel {}", verification_category.id(), welcome_channel.id());
        poise::send_reply(ctx, |reply| {
            reply
                .content("The channels you gave me are of incorrect type! Welcome channel needs to be a guild channel, verification channel needs to be a category.")
                .ephemeral(true)
        }).await?;

        return Ok(());
    }

    let guild_key = format!("guild:{}", guild_id);
    let mut database = bot.database.lock().await;

    database.hset(&guild_key, "configured", true)?;
    database.hset(&guild_key, "verification_category", &verification_category.id().as_u64())?;
    database.hset(&guild_key, "welcome_channel", &welcome_channel.id().as_u64())?;

    ctx.say("🙌 All set! Poucet is now ready to use 🤖✨").await?;

    Ok(())
}
