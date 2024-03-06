pub mod commands;
mod create_tag;
mod data_access;
mod model;
mod remove_tag;

pub use create_tag::create_tag;
use regex::Regex;
pub use remove_tag::remove_tag;
use tokio::task;
use tracing::{info_span, Instrument};

use self::data_access::{
    create_tag_blacklisted_user, create_tag_channel, delete_tag_blacklisted_user,
    get_tag_blacklisted_user, get_tag_channel, update_tag_channel,
};
use super::ComponentIds;
pub use model::Tag;
use serenity::{
    all::{ButtonStyle, CommandInteraction, CreateInteractionResponseMessage, GuildChannel, User},
    builder::{
        CreateActionRow, CreateAllowedMentions, CreateButton, CreateInteractionResponse,
        CreateMessage, EditMessage,
    },
    client::Context,
    model::{
        channel::Message,
        guild::Guild,
        id::{ChannelId, UserId},
    },
    prelude::Mentionable,
};
use sqlx::{MySql, MySqlPool, Pool};
use std::{fmt::Write, time::Duration};

#[tracing::instrument(skip(ctx))]
pub async fn list_tags(ctx: &Context, command: &CommandInteraction, pool: &Pool<MySql>) {
    let tags = data_access::get_tags_by_server_id(command.guild_id.unwrap().get(), pool).await;

    let mut message = String::new();

    for tag in tags {
        write!(&mut message, "{}, ", tag.listener).unwrap();
    }
    message.pop();
    message.pop();

    if message.is_empty() {
        message = "There are no tags in this server".to_string();
    }

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(message),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

#[tracing::instrument(skip(ctx))]
pub async fn blacklist_user_from_tags_command(
    ctx: &Context,
    user: &User,
    command: &CommandInteraction,
    pool: &MySqlPool,
) {
    let response = blacklist_user(user, pool).await;

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(response),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

#[tracing::instrument(skip(pool))]
pub async fn blacklist_user(user: &User, pool: &MySqlPool) -> String {
    let is_user_blacklisted = get_tag_blacklisted_user(user.id.get(), pool)
        .await
        .is_some();

    if is_user_blacklisted {
        delete_tag_blacklisted_user(user.id.get(), pool).await;

        "I will now ping you when you trip off a tag".to_string()
    } else {
        create_tag_blacklisted_user(user.id.get(), pool).await;

        "I won't ping you anymore when you trip off a tag".to_string()
    }
}

/// Checks for all the tag [`Listeners`][L] in the message
///
/// If a [`Listener`][L] is found it returns the response for that [`Listener`][L]
///
/// [L]: self::global_data::Listener
pub async fn check_for_tag_listeners(
    words_in_message: &[String],
    user_id: UserId,
    server_id: u64,
    pool: &Pool<MySql>,
) -> Option<String> {
    let alphanumeric_regex = Regex::new(r"[^A-Za-z0-9 ]").expect("Invalid regular expression");
    let words_in_message: Vec<String> = words_in_message
        .iter()
        .map(|w| alphanumeric_regex.replace_all(w, " ").to_string())
        .collect();

    let tags = data_access::get_tags_by_server_id(server_id, pool).await;
    let is_user_blacklisted = get_tag_blacklisted_user(user_id.get(), pool)
        .await
        .is_some();

    if is_user_blacklisted {
        return None;
    }

    for tag in &tags {
        let listener = &tag.listener;
        let response = &tag.response;

        let listener_words = listener
            .split(' ')
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        let mut listener_iterator = listener_words.iter();

        if listener_words.len() > 1 {
            let mut count = 0;
            for message_word in &words_in_message {
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

    for tag in tags {
        let listener = &tag.listener;
        let response = &tag.response;

        let listener_words = listener.split(' ').map(ToString::to_string);

        if words_in_message.contains(listener) && listener_words.count() < 2 {
            return Some(response.clone());
        }
    }

    None
}

#[tracing::instrument(skip(ctx))]
pub async fn set_tag_response_channel(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
) {
    let Some(guild_id) = command.guild_id else {
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("You can only use this command in a server"),
                ),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Error creating interaction response");
        return;
    };

    let tag_channel = get_tag_channel(guild_id.get(), pool).await;

    match tag_channel {
        Some(t) => {
            update_tag_channel(t.server_id, command.channel_id.get(), pool).await;
        }
        None => {
            create_tag_channel(guild_id.get(), command.channel_id.get(), pool).await;
        }
    }

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Successfully set this channel as the tag response channel"),
            ),
        )
        .instrument(info_span!("Sending message"))
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
#[tracing::instrument(skip(ctx, pool))]
pub async fn respond_to_tag(ctx: &Context, msg: &Message, message: &str, pool: &MySqlPool) {
    let tag_channel = get_tag_channel(msg.guild_id.unwrap().get(), pool).await;

    //If the guild has a tag response channel send the response there
    if let Some(tag_channel) = tag_channel {
        send_response_in_tag_channel(ctx, tag_channel.channel_id, msg, message).await;
        return;
    }

    //Try sending a message to the channel the tag listener was tripped off
    if msg
        .channel_id
        .say(&ctx.http, message)
        .instrument(info_span!("Sending message"))
        .await
        .is_err()
    {
        //If sending a message fails iterate through the guild channels until it manages to send a message
        let channels: Vec<GuildChannel> = msg
            .guild(&ctx.cache)
            .expect("Couldn't retrieve guild from cache")
            .channels
            .values()
            .cloned()
            .collect();

        for channel in channels {
            match tag_response(channel, ctx, msg, message).await {
                Ok(mut msg) => {
                    let http = ctx.http.clone();
                    task::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        msg.edit(&http, EditMessage::new().components(vec![]))
                            .await
                            .unwrap();
                    });

                    break;
                }
                Err(_) => continue,
            }
        }
    }
}

async fn tag_response(
    channel: GuildChannel,
    ctx: &Context,
    msg: &Message,
    message: &str,
) -> Result<Message, serenity::Error> {
    channel
        .id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
                    ComponentIds::BlacklistMeFromTags.to_string(),
                )
                .label("Stop pinging me")
                .style(ButtonStyle::Primary)])])
                .allowed_mentions(CreateAllowedMentions::new().all_users(true))
                .content(msg.author.mention().to_string() + " " + message),
        )
        .instrument(info_span!("Sending message"))
        .await
}

async fn send_response_in_tag_channel(
    ctx: &Context,
    channel_id: u64,
    msg: &Message,
    message: &str,
) {
    let mut tag_response_channel = ctx.cache.channel(channel_id).map(|c| c.to_owned());

    if tag_response_channel.is_none() {
        let guild_channels = Guild::get(&&ctx.http, msg.guild_id.unwrap())
            .await
            .expect("Couldn't fetch guild")
            .channels(&ctx.http)
            .await
            .unwrap();

        tag_response_channel = guild_channels
            .get(&ChannelId::from(channel_id))
            .map(std::clone::Clone::clone);
    }

    let Some(tag_response_channel) = tag_response_channel else {
        return;
    };

    let mut tag_response = CreateMessage::new();

    let response_content = if msg.channel_id == tag_response_channel.id {
        message.to_owned()
    } else {
        // Create this button only if the user is pinged
        tag_response = stop_pinging_me_button(tag_response);

        msg.author.mention().to_string() + " " + message
    };

    tag_response = tag_response
        .allowed_mentions(CreateAllowedMentions::new().all_users(true))
        .content(response_content);

    match tag_response_channel
        .send_message(&ctx.http, tag_response)
        .instrument(info_span!("Sending message"))
        .await
    {
        Ok(mut msg) => {
            let http = ctx.http.clone();
            task::spawn(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                msg.edit(&http, EditMessage::new().components(vec![]))
                    .await
                    .unwrap();
            });
        }
        Err(err) => eprintln!("{err}"),
    };
}

fn stop_pinging_me_button(tag_response: CreateMessage) -> CreateMessage {
    tag_response.components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
        ComponentIds::BlacklistMeFromTags.to_string(),
    )
    .label("Stop pinging me")
    .style(ButtonStyle::Primary)])])
}
