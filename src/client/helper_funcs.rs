use crate::*;
use serenity::{client::Context, model::{channel::{GuildChannel, Message}, interactions::application_command::{ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue}, prelude::User}, prelude::Mentionable};

pub fn id_command(command: &ApplicationCommandInteraction) -> String {
    let options = command
        .data
        .options
        .get(0)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");
    if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
        format!("{}'s id is {}", user, user.id)
    } else {
        "Please provide a valid user".to_owned()
    }
}

pub fn get_first_mentioned_user(msg: &Message) -> Option<&User> {
    for user in &msg.mentions {
        if user.bot {
            continue;
        }
        return Option::Some(user);
    }
    None
}
/**
It first tries to send a message in the same channel.

If that fails then it sends the message to the bot channel if one is set

If that fails then it iterates through every channel in the guild until it finds one it can send a message in
*/
pub async fn send_message_to_first_available_channel(ctx: &Context, msg: &Message, message: &str) {
    let bot_channels_lock = get_bot_channel_id_lock(&ctx.data).await;
    let bot_channels = bot_channels_lock.read().await;
    let bot_channel_id = bot_channels.get(&msg.guild_id.unwrap().0);

    if msg.channel_id.say(&ctx.http, message).await.is_err() {
        //try sending message to bot channel
        if let Some(channel_id) = bot_channel_id {
            let bot_channel = ctx.cache.guild_channel(*channel_id).await;
            if let Some(channel) = bot_channel {
                channel
                    .say(&ctx.http, msg.author.mention().to_string() + " " + message)
                    .await
                    .unwrap();
                return;
            }
        }

        //iterate until it managages to send a message
        let channels: Vec<GuildChannel> = msg
            .guild(&ctx.cache)
            .await
            .unwrap()
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

pub async fn set_bot_channel(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let member = command.member.as_ref().unwrap();
    let member_perms = member.permissions.unwrap();

    if !member_perms.administrator() && member.user.id != KRONI_ID {
        return "You need to have the Administrator permission to invoke this command".to_owned();
    }

    let guild_id = command.guild_id.unwrap().0;
    let channel_id = command.channel_id.0;
    let response_channel_lock = get_bot_channel_id_lock(&ctx.data).await;
    let mut bot_channel_ids = response_channel_lock.write().await;
    bot_channel_ids.insert(guild_id, channel_id);
    match save_bot_channel(&bot_channel_ids.clone()) {
        Ok(_) => "Succesfully set this channel as the bot channel".to_owned(),
        Err(_) => "Something went wrong setting bot channel".to_owned(),
    }
}
