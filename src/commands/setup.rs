use serenity::builder::{CreateComponents, CreateInteractionResponseData};
use serenity::model::application::component::ButtonStyle;
use serenity::model::channel::ReactionType;
use serenity::utils::Colour;

pub fn home<'a, 'b>(data: &'b mut CreateInteractionResponseData<'a>) -> &'b mut CreateInteractionResponseData<'a> {
    data.embed(|embed| {
        embed
            .title("Configuration")
            .description("Let's get Poucet ready to work on this server!")
            .colour(Colour::DARK_TEAL)
            .field("👋 Onboarding", "Setup onboarding module, to welcome new members", false)
            .field("👍 Done", "Exit setup", false)
    }).components(|components| {
        components.create_action_row(|row| {
            row.create_button(|button| {
                button
                    .custom_id("setup_onboarding")
                    .emoji('👋')
                    .style(ButtonStyle::Primary)
            }).create_button(|button| {
                button
                    .custom_id("setup_done")
                    .emoji('👍')
                    .style(ButtonStyle::Success)
            })
        })
    })
}

pub fn onboarding<'a, 'b>(data: &'b mut CreateInteractionResponseData<'a>) -> &'b mut CreateInteractionResponseData<'a> {
    data.embed(|embed| {
        embed
            .title("👋 Onboarding")
            .colour(Colour::DARK_TEAL)
    }).components(|components| {
        components.create_action_row(|row| {
            row.create_button(|button| {
                button
                    .custom_id("setup_go_back")
                    .emoji(ReactionType::Unicode("↩️".to_string()))
            })
        })
    })
}

pub fn done<'a, 'b>(data: &'b mut CreateInteractionResponseData<'a>) -> &'b mut CreateInteractionResponseData<'a> {
    data
        .content("🙌 All set! Poucet is now ready to use 🤖✨")
        .set_embeds(Vec::new())
        .set_components(CreateComponents::default())
}
