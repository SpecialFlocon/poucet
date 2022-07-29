use redis::Commands;
use serenity::model::guild::Role;
use serenity::model::id::RoleId;

use crate::{Context, Error};

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
