use serenity::builder::CreateApplicationCommands;

pub fn global_slash_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    commands
        .create_application_command(|command| {
            command.name("ping").description("Am I responding? Use this command to find out!")
        })
}

pub fn guild_slash_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    commands
}
