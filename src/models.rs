use serenity::model::channel::{Channel, ChannelCategory, GuildChannel};
use serenity::model::guild::Role;

use crate::Error;

/// Guild configuration for the bot.
#[derive(Default)]
pub struct GuildConfiguration {
    pub configured: bool,
    pub admin_role: Option<Role>,
    pub validated_role: Option<Role>,
    pub introductions_channel: Option<GuildChannel>,
    pub role_assignment_channel: Option<GuildChannel>,
    pub validation_category: Option<ChannelCategory>,
    pub welcome_channel: Option<GuildChannel>,
}

impl GuildConfiguration {
    pub fn new(admin_role: Role, validated_role: Role, introductions_channel: Channel, role_assignment_channel: Channel, validation_category: Channel, welcome_channel: Channel) -> Result<Self, Error> {
        let introductions_channel = match introductions_channel {
            Channel::Guild(c) => Some(c),
            _ => { return Err(Error::from(format!("given introductions channel (id: {}) is not a guild channel", introductions_channel.id()))); },
        };

        let role_assignment_channel = match role_assignment_channel {
            Channel::Guild(c) => Some(c),
            _ => { return Err(Error::from(format!("given role assignment channel (id: {}) is not a guild channel", role_assignment_channel.id()))); },
        };

        let validation_category = match validation_category {
            Channel::Category(c) => Some(c),
            _ => { return Err(Error::from(format!("given validation channel (id: {}) is not a category channel", validation_category.id()))); },
        };

        let welcome_channel = match welcome_channel {
            Channel::Guild(c) => Some(c),
            _ => { return Err(Error::from(format!("given welcome channel (id: {}) is not a guild channel", welcome_channel.id()))); },
        };

        Ok(Self {
            configured: true,
            admin_role: Some(admin_role),
            validated_role: Some(validated_role),
            introductions_channel,
            role_assignment_channel,
            validation_category,
            welcome_channel
        })
    }
}
