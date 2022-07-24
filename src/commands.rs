mod ping;
pub mod setup;

use serenity::builder::{CreateApplicationCommands, CreateInteractionResponseData};
use serenity::model::application::command::CommandOptionType;

pub use ping::ping;

pub fn fallback<'a, 'b>(data: &'b mut CreateInteractionResponseData<'a>) -> &'b mut CreateInteractionResponseData<'a> {
    data.content("Not implemented (yet!)".to_string())
}

pub fn global_slash_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    commands
        .create_application_command(|command| {
            command.name("ping").description("Am I responding? Use this command to find out!")
        })
        .create_application_command(|command| {
            command.name("setup").description("Setup Poucet for this server")
                .create_option(|option| {
                    option
                        .name("anew")
                        .description("Setup an already configured server from scratch")
                        .kind(CommandOptionType::Boolean)
                })
        })
}

pub fn guild_slash_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    commands
}
