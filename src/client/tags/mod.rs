pub mod commands;
mod data_access;
mod file_operations;
mod global_data;

use self::global_data::{
    TagBlacklistedUsers, TagResponseChannelIds, BLACKLISTED_USERS_PATH, BOT_CHANNEL_PATH,
};
use super::{create_file_if_missing, ButtonIds};
use crate::client::tags::file_operations::save_user_tag_blacklist_to_file;
use dashmap::{DashMap, DashSet};
pub use global_data::Tag;
use regex::Regex;
use serenity::{
    builder::ParseValue,
    client::Context,
    model::{
        channel::{Channel, Message},
        guild::Guild,
        id::{ChannelId, UserId},
        prelude::{
            component::ButtonStyle,
            interaction::application_command::{
                ApplicationCommandInteraction, CommandDataOptionValue,
            },
            User,
        },
    },
    prelude::{Mentionable, TypeMap},
};
use sqlx::{MySql, Pool};
use std::fmt::Write;
use std::{error::Error, fs, sync::Arc};
use tokio::sync::RwLockWriteGuard;
use {
    file_operations::save_tag_response_channel,
    global_data::{get_tag_response_channel_id_lock, get_tags_blacklisted_users_lock},
};

pub async fn list(ctx: &Context, command: &ApplicationCommandInteraction, pool: &Pool<MySql>) {
    let tags = data_access::get_tags_by_server_id(command.guild_id.unwrap().0, pool).await;

    let mut message = String::new();

    for tag in tags.iter() {
        write!(&mut message, "{}, ", tag.listener).unwrap();
    }
    message.pop();
    message.pop();

    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content(message))
        })
        .await
        .expect("Error creating interaction response");
}

pub async fn remove_tag(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    pool: &Pool<MySql>,
) {
    let listener = command
        .data
        .options
        .get(0)
        .unwrap()
        .options
        .get(0)
        .expect("Expected listener option")
        .resolved
        .as_ref()
        .expect("Expected listener value");

    let listener = if let CommandDataOptionValue::String(listener) = listener {
        listener
    } else {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("Something went wrong"))
            })
            .await
            .expect("Error creating interaction response");
        return;
    };

    let tag = data_access::get_tag_by_listener(
        listener,
        command
            .guild_id
            .expect("This command can't be called outside guilds")
            .0,
        pool,
    )
    .await;

    match tag {
        Some(tag) => {
            data_access::delete_tag(tag.id, pool).await;

            println!(
                "{} removed tag {} in server {}",
                command.user.name, tag.listener, tag.server_id
            );
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content(format!("Removed the tag {}", &tag.listener))
                    })
                })
                .await
                .expect("Error creating interaction response");
            return;
        }
        None => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content(format!("Couldn't find the tag {}", listener))
                    })
                })
                .await
                .expect("Error creating interaction response");
            return;
        }
    }
}

pub async fn create_tag(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    pool: &Pool<MySql>,
) {
    let guild_id = match command.guild_id {
        Some(g) => g,
        None => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Can't create a tag outside of a server")
                    })
                })
                .await
                .expect("Error creating interaction response");
            return;
        }
    };

    let (listener, response) = get_listener_and_response(command);

    if !is_tag_valid(response, listener) {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("can't add a mention"))
            })
            .await
            .expect("Error creating interaction response");
        return;
    }

    match data_access::create_tag(
        listener.to_lowercase().trim().to_owned(),
        response.trim().to_owned(),
        command.user.name.clone(),
        command.user.id.0,
        guild_id.0,
        pool,
    )
    .await
    {
        Ok(tag) => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content(format!("Created tag {}", tag.listener))
                    })
                })
                .await
                .expect("Error creating interaction response");
        }
        Err(e) => match e {
            data_access::CreateTagError::TagWithSameListenerExists => {
                command
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| {
                            d.content(format!("The tag \"{}\" already exists", listener))
                        })
                    })
                    .await
                    .expect("Error creating interaction response");
                return;
            }
        },
    };
}

fn is_tag_valid(response: &str, listener: &str) -> bool {
    let user_regex = Regex::new(r"<@!?(\d+)>").expect("Invalid regular expression");
    let role_regex = Regex::new(r"<@&(\d+)>").expect("Invalid regular expression");

    !(user_regex.is_match(response)
        || user_regex.is_match(listener)
        || role_regex.is_match(response)
        || role_regex.is_match(listener)
        || response.contains("@everyone")
        || response.contains("@here"))
}

fn get_listener_and_response(command: &ApplicationCommandInteraction) -> (&String, &String) {
    let listener = command
        .data
        .options
        .get(0)
        .unwrap()
        .options
        .get(0)
        .expect("Expected listener option")
        .resolved
        .as_ref()
        .expect("Expected listener value");
    let response = command
        .data
        .options
        .get(0)
        .unwrap()
        .options
        .get(1)
        .expect("Expected response option")
        .resolved
        .as_ref()
        .expect("Expected response value");

    let listener = match listener {
        CommandDataOptionValue::String(s) => s,
        _ => panic!("Expected listener to be a string"),
    };

    let response = match response {
        CommandDataOptionValue::String(s) => s,
        _ => panic!("Expected listener to be a string"),
    };

    (listener, response)
}

pub async fn blacklist_user_from_tags_command(
    ctx: &Context,
    user: &User,
    command: &ApplicationCommandInteraction,
) {
    let users_blacklisted_from_tags = get_tags_blacklisted_users_lock(&ctx.data).await;

    let response = if users_blacklisted_from_tags.contains(&user.id.0) {
        users_blacklisted_from_tags.remove(&user.id.0);
        save_user_tag_blacklist_to_file(&users_blacklisted_from_tags);

        "I will now ping you when you trip off a tag"
    } else {
        users_blacklisted_from_tags.insert(user.id.0);
        save_user_tag_blacklist_to_file(&users_blacklisted_from_tags);

        "I won't ping you anymore when you trip off a tag"
    };

    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content(response))
        })
        .await
        .expect("Error creating interaction response");
}

pub async fn blacklist_user(ctx: &Context, user: &User) -> String {
    let users_blacklisted_from_tags = get_tags_blacklisted_users_lock(&ctx.data).await;

    if users_blacklisted_from_tags.contains(&user.id.0) {
        users_blacklisted_from_tags.remove(&user.id.0);
        save_user_tag_blacklist_to_file(&users_blacklisted_from_tags);
        "I won't ping you anymore when you trip off a tag".to_string()
    } else {
        users_blacklisted_from_tags.insert(user.id.0);
        save_user_tag_blacklist_to_file(&users_blacklisted_from_tags);
        "I will now ping you when you trip off a tag".to_string()
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
    server_id: u64,
    pool: &Pool<MySql>,
) -> Option<String> {
    let tags = data_access::get_tags_by_server_id(server_id, pool).await;
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
                    return Some(response.clone());
                }
            }
        }
    }

    for tag in tags.iter() {
        let listener = &tag.listener;
        let response = &tag.response;

        let listener_words = listener.split(' ').map(ToString::to_string);

        if words_in_message.contains(listener) && listener_words.count() < 2 {
            return Some(response.clone());
        }
    }

    None
}

pub async fn set_tag_response_channel(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = if let Some(guild_id) = command.guild_id {
        guild_id
    } else {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.content("You can only use this command in a server")
                })
            })
            .await
            .expect("Error creating interaction response");
        return;
    };

    let member = command.member.as_ref().expect("Expected member");
    let member_perms = member.permissions.expect("Couldn't get member permissions");

    let response;
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
        response = "You need to have the Administrator permission to invoke this command";
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content(response))
            })
            .await
            .expect("Error creating interaction response");
        return;
    }

    let channel_id = command.channel_id.0;
    let bot_channel_ids = get_tag_response_channel_id_lock(&ctx.data).await;
    bot_channel_ids.insert(guild_id.0, channel_id);

    response = match save_tag_response_channel(&bot_channel_ids) {
        Ok(_) => "Successfully set this channel as the tag response channel",
        Err(_) => "Something went wrong setting the tag response channel",
    };

    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content(response))
        })
        .await
        .expect("Error creating interaction response");
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
        send_response_in_tag_channel(ctx, channel_id, msg, message).await;
        return;
    }

    //Try sending a message to the channel the tag listener was tripped off
    if msg.channel_id.say(&ctx.http, message).await.is_err() {
        //If sending a message fails iterate through the guild channels until it manages to send a message
        let channels: Vec<Channel> = msg
            .guild(&ctx.cache)
            .expect("Couldn't retrieve guild from cache")
            .channels
            .iter()
            .map(|(_, channel)| channel.clone())
            .collect();
        for channel in channels {
            match channel
                .id()
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

async fn send_response_in_tag_channel(
    ctx: &Context,
    channel_id: dashmap::mapref::one::Ref<'_, u64, u64>,
    msg: &Message,
    message: &str,
) {
    let mut tag_response_channel = ctx.cache.guild_channel(*channel_id);
    if tag_response_channel.is_none() {
        let guild_channels = Guild::get(&&ctx.http, *channel_id.key())
            .await
            .expect("Couldn't fetch guild")
            .channels(&ctx.http)
            .await
            .unwrap();

        tag_response_channel = guild_channels
            .get(&ChannelId::from(*channel_id.value()))
            .cloned();
    }
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
}

pub fn init_tags_data(mut data: RwLockWriteGuard<TypeMap>) -> Result<(), Box<dyn Error>> {
    let user_tag_blacklist: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(BLACKLISTED_USERS_PATH, "[]")?,
    )?)?;
    let bot_channel: DashMap<u64, u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(BOT_CHANNEL_PATH, "{}")?,
    )?)?;
    data.insert::<TagBlacklistedUsers>(Arc::new(user_tag_blacklist));
    data.insert::<TagResponseChannelIds>(Arc::new(bot_channel));
    Ok(())
}
