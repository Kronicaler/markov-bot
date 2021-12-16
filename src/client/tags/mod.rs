mod file_operations;
pub mod global_data;

use crate::{
    client::{tags::file_operations::save_user_tag_blacklist_to_file, Command},
    OWNER_ID,
};
use regex::Regex;
use serenity::{
    client::Context,
    model::{
        id::UserId,
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
            ApplicationCommandOptionType,
        },
        prelude::User,
    },
};
use {
    file_operations::{save_tag_response_channel, save_tag_to_file},
    global_data::{
        get_tag_response_channel_id_lock, get_tags_blacklisted_users_lock, get_tags_lock,
    },
};

pub async fn list_tags(ctx: &Context) -> String {
    let tag = get_tags_lock(&ctx.data).await;

    let mut message = String::new();

    for entry in tag.iter() {
        message += &format!("{}, ", entry.key());
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
        .expect("expected listener")
        .resolved
        .as_ref()
        .unwrap();
    let tag = get_tags_lock(&ctx.data).await;

    if let ApplicationCommandInteractionDataOptionValue::String(listener) = listener {
        if tag.contains_key(listener) {
            tag.remove(listener);
            save_tag_to_file(&tag);
            return "Successfully removed the tag".to_owned();
        }
        return "That tag doesn't exist".to_owned();
    }

    "Something went wrong".to_owned()
}

pub async fn create_tag(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let listener = command
        .data
        .options
        .get(0)
        .expect("expected listener")
        .resolved
        .as_ref()
        .unwrap();
    let response = command
        .data
        .options
        .get(1)
        .expect("expected response")
        .resolved
        .as_ref()
        .unwrap();

    if let ApplicationCommandInteractionDataOptionValue::String(listener) = listener {
        if let ApplicationCommandInteractionDataOptionValue::String(response) = response {
            let user_regex = Regex::new(r"<@!?(\d+)>").unwrap();
            let role_regex = Regex::new(r"<@&(\d+)>").unwrap();
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

            tags.insert(
                listener.to_lowercase().trim().to_owned(),
                response.trim().to_owned(),
            );
            save_tag_to_file(&tags);
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

/// Checks for all the listened words in the message
///
/// If a listened word is found it returns the response
pub async fn check_for_listened_words(
    ctx: &Context,
    words_in_message: &[String],
    user_id: UserId,
) -> Option<String> {
    let tags = get_tags_lock(&ctx.data).await;
    let tag_blacklisted_users = get_tags_blacklisted_users_lock(&ctx.data).await;

    if tag_blacklisted_users.contains(&user_id.0) {
        return None;
    }

    for entry in tags.iter() {
        let listener = entry.key();
        let response = entry.value();

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

    for entry in tags.iter() {
        let listener = entry.key();
        let response = entry.value();

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
    let member = command.member.as_ref().unwrap();
    let member_perms = member.permissions.unwrap();

    if !member_perms.administrator() && member.user.id != OWNER_ID {
        return "You need to have the Administrator permission to invoke this command".to_owned();
    }

    let guild_id = command.guild_id.unwrap().0;
    let channel_id = command.channel_id.0;
    let bot_channel_ids = get_tag_response_channel_id_lock(&ctx.data).await;
    bot_channel_ids.insert(guild_id, channel_id);
    match save_tag_response_channel(&bot_channel_ids) {
        Ok(_) => "Successfully set this channel as the bot channel".to_owned(),
        Err(_) => "Something went wrong setting bot channel".to_owned(),
    }
}
