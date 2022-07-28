use serenity::model::channel::{Channel, ChannelCategory, GuildChannel};

use crate::Error;

/// Guild configuration for the bot.
#[derive(Default)]
pub struct GuildConfiguration {
    pub configured: bool,
    pub verification_category: Option<ChannelCategory>,
    pub welcome_channel: Option<GuildChannel>,
}

impl GuildConfiguration {
    pub fn new(verification_category: Channel, welcome_channel: Channel) -> Result<Self, Error> {
        let verification_category = match verification_category {
            Channel::Category(c) => Some(c),
            _ => { return Err(Error::from(format!("given verification channel (id: {}) is not a category channel", verification_category.id()))); },
        };

        let welcome_channel = match welcome_channel {
            Channel::Guild(c) => Some(c),
            _ => { return Err(Error::from(format!("given welcome channel (id: {}) is not a guild channel", welcome_channel.id()))); },
        };

        Ok(Self { configured: true, verification_category, welcome_channel })
    }
}
