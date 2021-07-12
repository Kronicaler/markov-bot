use serenity::{
    client::Context,
    model::{
        channel::{GuildChannel, Message},
        interactions::{
            ApplicationCommandInteractionData, ApplicationCommandInteractionDataOptionValue,
        },
        prelude::User,
    },
    prelude::Mentionable,
};

pub fn id_command(data: &ApplicationCommandInteractionData) -> String {
    let options = data
        .options
        .get(0)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");
    if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
        format!("{}'s id is {}", user, user.id)
    } else {
        "Please provide a valid user".to_string()
    }
}

pub fn get_first_mentioned_user(msg: &Message) -> Option<&User> {
    for user in &msg.mentions {
        if user.bot {
            continue;
        }
        return Option::Some(user);
    }
    return None;
}

///iterates through every channel in the guild starting with the one from which the message came from until it finds one it can send a message in
pub async fn send_message_to_first_available_channel(ctx: &Context, msg: &Message, message: &str) {
    match msg.channel_id.say(&ctx.http, message).await {
        Ok(_) => return,
        Err(_) => {
            let guild = msg.guild(&ctx.cache).await.unwrap();
            let channels: Vec<GuildChannel> = guild
                .channels
                .iter()
                .map(|(_, channel)| channel.clone())
                .collect();
            for channel in channels {
                match channel
                    .id
                    .say(&ctx.http, msg.author.mention().to_string() + " " + message)
                    .await
                {
                    Ok(_) => break,
                    Err(_) => continue,
                }
            }
        }
    }
}
