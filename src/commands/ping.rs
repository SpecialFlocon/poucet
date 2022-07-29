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
