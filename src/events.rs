use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use tracing::{debug, info};

use crate::{Bot, Error};

pub async fn listener(_ctx: &serenity::client::Context, event: &poise::Event<'_>, _framework: poise::FrameworkContext<'_, Bot, Error>, _user_data: &Bot) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => ready(data_about_bot),
        poise::Event::GuildMemberAddition { new_member } => guild_member_addition(new_member),
        _ => {},
    }

    Ok(())
}

fn ready(data: &Ready) {
    info!("Authenticated as {}#{}", data.user.name, data.user.discriminator);
}

fn guild_member_addition(member: &Member) {
    debug!("New member: {}#{}: {:?}", member.user.name, member.user.discriminator, member);
}
