use serenity::builder::CreateInteractionResponseData;

pub fn ping<'a, 'b>(data: &'b mut CreateInteractionResponseData<'a>) -> &'b mut CreateInteractionResponseData<'a> {
    data.content("Coin !".to_string())
}
