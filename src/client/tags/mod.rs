mod file_operations;
mod global_data;

pub use global_data::Tag;
use std::{error::Error, fs, sync::Arc};

use crate::client::{tags::file_operations::save_user_tag_blacklist_to_file, Command};
use dashmap::{DashMap, DashSet};
use regex::Regex;
use serenity::{
    builder::ParseValue,
    client::Context,
    model::{
        channel::{GuildChannel, Message},
        id::UserId,
        interactions::{
            application_command::{
                ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
                ApplicationCommandOptionType,
            },
            message_component::ButtonStyle,
        },
        prelude::User,
    },
    prelude::{Mentionable, TypeMap},
};
use tokio::sync::RwLockWriteGuard;

use self::global_data::{
    TagBlacklistedUsers, TagResponseChannelIds, TagsContainer, BLACKLISTED_USERS_PATH,
    BOT_CHANNEL_PATH, TAG_PATH,
};

use super::{create_file_if_missing, ButtonIds};
use {
    file_operations::{save_tag_response_channel, save_tags_to_file},
    global_data::{
        get_tag_response_channel_id_lock, get_tags_blacklisted_users_lock, get_tags_lock,
    },
};

pub async fn list_tags(ctx: &Context) -> String {
    let tag = get_tags_lock(&ctx.data).await;

    let mut message = String::new();

    for tag in tag.iter() {
        message += &format!("{}, ", tag.listener);
    }
    message.pop();
    message.pop();

    message
}

pub async fn remove_tag(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let listener = command
        .data
        .options
        .get(0)
        .expect("Expected listener option")
        .resolved
        .as_ref()
        .expect("Expected listener value");
    let tags = get_tags_lock(&ctx.data).await;

    if let ApplicationCommandInteractionDataOptionValue::String(listener) = listener {
        for tag in tags.as_ref().clone().iter() {
            if &tag.listener == listener {
                tags.remove(&tag);
                save_tags_to_file(&tags);
                println!("{} removed tag {}", command.user.name, tag.listener);
                return "Successfully removed the tag".to_owned();
            }
        }
        return "Couldn't find the tag".to_owned();
    }

    "Something went wrong".to_owned()
}

pub async fn create_tag(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let listener = command
        .data
        .options
        .get(0)
        .expect("Expected listener option")
        .resolved
        .as_ref()
        .expect("Expected listener value");
    let response = command
        .data
        .options
        .get(1)
        .expect("Expected response option")
        .resolved
        .as_ref()
        .expect("Expected response value");

    if let ApplicationCommandInteractionDataOptionValue::String(listener) = listener {
        if let ApplicationCommandInteractionDataOptionValue::String(response) = response {
            let user_regex = Regex::new(r"<@!?(\d+)>").expect("Invalid regular expression");
            let role_regex = Regex::new(r"<@&(\d+)>").expect("Invalid regular expression");
            if user_regex.is_match(response)
                || user_regex.is_match(listener)
                || role_regex.is_match(response)
                || role_regex.is_match(listener)
                || response.contains("@everyone")
                || response.contains("@here")
            {
                return "can't add a mention".to_owned();
            }

            let tags = get_tags_lock(&ctx.data).await;

            let tag = Tag {
                listener: listener.to_lowercase().trim().to_owned(),
                response: response.trim().to_owned(),
                creator_name: command.user.name.to_owned(),
                creator_id: command.user.id.0,
            };

            tags.insert(tag);
            save_tags_to_file(&tags);
            return "Set tag".to_owned();
        }
    }
    "Couldn't set tag".to_owned()
}

pub async fn blacklist_user_from_tags(ctx: &Context, user: &User) -> String {
    let users_blacklisted_from_tags = get_tags_blacklisted_users_lock(&ctx.data).await;

    if users_blacklisted_from_tags.contains(&user.id.0) {
        users_blacklisted_from_tags.remove(&user.id.0);
        save_user_tag_blacklist_to_file(&users_blacklisted_from_tags);
        format!("Removed {} from the blacklist", &user.name)
    } else {
        users_blacklisted_from_tags.insert(user.id.0);
        save_user_tag_blacklist_to_file(&users_blacklisted_from_tags);
        format!("Added {} to the tag blacklist", &user.name)
    }
}

/// Checks for all the tag [`Listeners`][L] in the message
///
/// If a [`Listener`][L] is found it returns the response for that [`Listener`][L]
///
/// [L]: self::global_data::Listener
pub async fn check_for_tag_listeners(
    ctx: &Context,
    words_in_message: &[String],
    user_id: UserId,
) -> Option<String> {
    let tags = get_tags_lock(&ctx.data).await;
    let tag_blacklisted_users = get_tags_blacklisted_users_lock(&ctx.data).await;

    if tag_blacklisted_users.contains(&user_id.0) {
        return None;
    }

    for tag in tags.iter() {
        let listener = &tag.listener;
        let response = &tag.response;

        let listener_words = listener
            .split(' ')
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        let mut listener_iterator = listener_words.iter();

        if listener_words.len() > 1 {
            let mut count = 0;
            for message_word in words_in_message.iter() {
                if message_word == listener_iterator.next()? {
                    count += 1;
                } else {
                    count = 0;
                    listener_iterator = listener_words.iter();
                }

                if count == listener_words.len() {
                    return Some(response.to_owned());
                }
            }
        }
    }

    for tag in tags.iter() {
        let listener = &tag.listener;
        let response = &tag.response;

        let listener_words = listener.split(' ').map(ToString::to_string);

        if words_in_message.contains(listener) && listener_words.count() < 2 {
            return Some(response.to_owned());
        }
    }

    None
}
/// Create the tag slash commands
pub fn create_tag_commands(
    commands: &mut serenity::builder::CreateApplicationCommands,
) -> &mut serenity::builder::CreateApplicationCommands {
    commands.create_application_command(|command| {
            command.name(Command::createtag).description(
                "Create a tag for a word or list of words and a response whenever someone says that word",
            )
            .create_option(|option|{
                option.name("tag").description("What word to listen for").kind(ApplicationCommandOptionType::String).required(true)
            })
            .create_option(|option|{
                option.name("response").description("What the response should be when the tag is said")
                .kind(ApplicationCommandOptionType::String)
                .required(true)
            })
        })
        .create_application_command(|command| {
            command.name(Command::removetag).description("Remove a tag").create_option(|option|{
                option.name("tag").description("The tag to remove").kind(ApplicationCommandOptionType::String).required(true)
            })
        })
        .create_application_command(|command|{
            command.name(Command::tags).description("List all of the tags")
        })
        .create_application_command(|command|{
            command.name(Command::blacklistmefromtags).description("The bot won't respond to your messages if you trip off a tag")
        })
}

pub async fn set_tag_response_channel(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> String {
    let guild_id = match command.guild_id {
        Some(guild_id) => guild_id,
        None => return "You can only use this command in a server".to_owned(),
    };

    let member = command.member.as_ref().expect("Expected member");
    let member_perms = member.permissions.expect("Couldn't get member permissions");

    if !member_perms.administrator()
        && member.user.id
            != ctx
                .http
                .get_current_application_info()
                .await
                .expect("Couldn't fetch the owner id")
                .owner
                .id
    {
        return "You need to have the Administrator permission to invoke this command".to_owned();
    }

    let channel_id = command.channel_id.0;
    let bot_channel_ids = get_tag_response_channel_id_lock(&ctx.data).await;
    bot_channel_ids.insert(guild_id.0, channel_id);
    match save_tag_response_channel(&bot_channel_ids) {
        Ok(_) => "Successfully set this channel as the tag response channel".to_owned(),
        Err(_) => "Something went wrong setting the tag response channel".to_owned(),
    }
}

/// It first checks if a tag response channel exists for the guild the message is in.
///
/// If there is it sends the response there.
///
/// If there is no tag response channel set then it first tries to send a message in the same channel.
/// If that fails then it sends the message to the tag response channel if one is set
/// If that fails then it iterates through every channel in the guild until it finds one it can send a message in
pub async fn respond_to_tag(ctx: &Context, msg: &Message, message: &str) {
    let tag_response_channels = get_tag_response_channel_id_lock(&ctx.data).await;
    let tag_response_channel_id =
        tag_response_channels.get(&msg.guild_id.expect("Couldn't get the guild id").0);

    //If the guild has a tag response channel send the response there
    if let Some(channel_id) = tag_response_channel_id {
        let tag_response_channel = ctx.cache.guild_channel(*channel_id).await;
        if let Some(tag_response_channel) = tag_response_channel {
            tag_response_channel
                .send_message(&ctx.http, |m| {
                    let response_content = if msg.channel_id == tag_response_channel.id {
                        message.to_owned()
                    } else {
                        // Create this button only if the user is pinged
                        if rand::random::<f32>() < 0.05 {
                            m.components(|c| {
                                c.create_action_row(|a| {
                                    a.create_button(|b| {
                                        b.label("Stop pinging me")
                                            .style(ButtonStyle::Primary)
                                            .custom_id(ButtonIds::BlacklistMeFromTags)
                                    })
                                })
                            });
                        }

                        msg.author.mention().to_string() + " " + message
                    };

                    m.allowed_mentions(|m| m.parse(ParseValue::Users))
                        .content(response_content)
                })
                .await
                .expect("Couldn't send message");
        }
        return;
    }

    //Try sending a message to the channel the tag listener was tripped off
    if msg.channel_id.say(&ctx.http, message).await.is_err() {
        //If sending a message fails iterate through the guild channels until it manages to send a message
        let channels: Vec<GuildChannel> = msg
            .guild(&ctx.cache)
            .await
            .expect("Couldn't retrieve guild from cache")
            .channels
            .iter()
            .map(|(_, channel)| channel.clone())
            .collect();
        for channel in channels {
            match channel
                .id
                .send_message(&ctx.http, |m| {
                    m.components(|c| {
                        c.create_action_row(|a| {
                            a.create_button(|b| {
                                b.label("Stop pinging me")
                                    .style(ButtonStyle::Primary)
                                    .custom_id(ButtonIds::BlacklistMeFromTags)
                            })
                        })
                    })
                    .allowed_mentions(|m| m.parse(ParseValue::Users))
                    .content(msg.author.mention().to_string() + " " + message)
                })
                .await
            {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }
}

pub fn init_tags_data(mut data: RwLockWriteGuard<TypeMap>) -> Result<(), Box<dyn Error>> {
    let tags: DashSet<Tag> = serde_json::from_str(&fs::read_to_string(create_file_if_missing(
        TAG_PATH, "[]",
    )?)?)?;
    let user_tag_blacklist: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(BLACKLISTED_USERS_PATH, "[]")?,
    )?)?;
    let bot_channel: DashMap<u64, u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(BOT_CHANNEL_PATH, "{}")?,
    )?)?;
    data.insert::<TagsContainer>(Arc::new(tags));
    data.insert::<TagBlacklistedUsers>(Arc::new(user_tag_blacklist));
    data.insert::<TagResponseChannelIds>(Arc::new(bot_channel));
    Ok(())
}
