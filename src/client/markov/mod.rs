mod file_operations;
pub mod global_data;
mod markov_chain;
use markov_strings::Markov;
use serenity::{
    client::Context,
    model::{channel::Message, prelude::User},
};
use std::{error::Error, fs};

use self::{
    file_operations::{import_chain_from_file, save_markov_blacklisted_users},
    global_data::{
        get_markov_blacklisted_channels_lock, get_markov_blacklisted_users_lock,
        get_markov_chain_lock, MARKOV_EXPORT_PATH,
    },
    markov_chain::filter_message_for_markov_file,
};

use super::file_operations::create_file_if_missing;

pub async fn add_message_to_chain(msg: &Message, ctx: &Context) -> Result<bool, std::io::Error> {
    // if the message was not sent in a guild
    if !msg
        .channel_id
        .to_channel(&ctx.http)
        .await
        .expect("Couldn't get channel")
        .guild()
        .is_some()
    {
        return Ok(false);
    }

    let markov_blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;
    let markov_blacklisted_channels = get_markov_blacklisted_channels_lock(&ctx.data).await;

    if markov_blacklisted_channels.contains(&msg.channel_id.0)
        || markov_blacklisted_users.contains(&msg.author.id.0)
        || msg
            .mentions_me(&ctx.http)
            .await
            .expect("Couldn't fetch mention from cache")
    {
        return Ok(false);
    }

    let filtered_message = filter_message_for_markov_file(msg);
    if !filtered_message.is_empty() {
        file_operations::append_to_markov_file(&filtered_message)?;
        return Ok(true);
    } else {
        return Ok(false);
    }
}

pub async fn generate_sentence(ctx: &Context) -> String {
    let markov_lock = get_markov_chain_lock(&ctx.data).await;

    let markov_chain = markov_lock.read().await;

    match markov_chain.generate() {
        Ok(markov_result) => {
            let mut message = markov_result.text;
            if cfg!(debug_assertions) {
                message += " --debug";
            }
            return message;
        }
        Err(why) => {
            return match why {
                markov_strings::ErrorType::CorpusEmpty => "The corpus is empty, try again later!",
                markov_strings::ErrorType::TriesExceeded => {
                    "couldn't generate a sentence, try again!"
                }
                _ => "Try again later.",
            }
            .to_owned();
        }
    };
}
/// Initializes the Markov chain from [`MARKOV_DATA_SET_PATH`]
pub fn init() -> Result<Markov, Box<dyn Error>> {
    let mut markov_chain = Markov::new();
    markov_chain.set_state_size(3).expect("Will never fail");
    markov_chain.set_max_tries(200);
    markov_chain.set_filter(|r| {
        if r.text.split(' ').count() >= 5 && r.refs.len() >= 2 {
            return true;
        }
        false
    });
    let input_data = import_chain_from_file()?;
    markov_chain.add_to_corpus(input_data);
    Ok(markov_chain)
}
/// Initializes the Markov chain from [`MARKOV_EXPORT_PATH`]
pub fn init_debug() -> Result<Markov, Box<dyn Error>> {
    let mut markov: Markov = serde_json::from_str(&fs::read_to_string(create_file_if_missing(
        MARKOV_EXPORT_PATH,
        &serde_json::to_string(&Markov::new().export())?,
    )?)?)?;
    markov.set_max_tries(200);
    markov.set_filter(|r| {
        if r.text.split(' ').count() >= 5 && r.refs.len() >= 2 {
            return true;
        }
        false
    });
    Ok(markov)
}

pub async fn add_user_to_blacklist(user: &User, ctx: &Context) -> Result<(), std::io::Error> {
    let blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;

    blacklisted_users.insert(user.id.0);
    save_markov_blacklisted_users(&*blacklisted_users)
}

pub async fn remove_user_from_blacklist(user: &User, ctx: &Context) -> Result<(), std::io::Error> {
    let blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;

    blacklisted_users.remove(&user.id.0);
    save_markov_blacklisted_users(&*blacklisted_users)
}

pub async fn blacklisted_users(ctx: &Context) -> String {
    let mut blacklisted_usernames = Vec::new();
    let blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;

    for user_id in blacklisted_users.iter() {
        blacklisted_usernames.push(
            ctx.http
                .get_user(*user_id)
                .await
                .expect("Couldn't get user")
                .name,
        );
    }

    if blacklisted_usernames.is_empty() {
        return "Currently there are no blacklisted users".to_owned();
    }

    let mut message = String::from("Blacklisted users: ");
    for user_name in blacklisted_usernames {
        message += &(user_name + ", ");
    }

    //remove the trailing comma and whitespace
    message.pop();
    message.pop();
    message
}
