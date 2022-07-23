mod ping;

use serenity::builder::CreateApplicationCommands;

pub use ping::ping;

pub fn fallback() -> String {
    "Not implemented (yet!)".to_string()
}

pub fn global_slash_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    commands
        .create_application_command(|command| {
            command.name("ping").description("Am I responding? Use this command to find out!")
        })
}

pub fn guild_slash_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    commands
}
