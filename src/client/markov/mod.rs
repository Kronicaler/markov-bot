mod file_operations;
mod global_data;
mod markov_chain;

use self::{
    file_operations::{import_chain_from_file, save_markov_blacklisted_users},
    global_data::{
        get_markov_blacklisted_channels_lock, get_markov_blacklisted_users_lock,
        get_markov_chain_lock, MARKOV_EXPORT_PATH,
    },
    markov_chain::filter_message_for_markov_file,
};
use super::file_operations::create_file_if_missing;
use dashmap::DashSet;
use markov_strings::Markov;
use serenity::{
    client::Context,
    model::{
        channel::Message, interactions::application_command::ApplicationCommandInteraction,
        prelude::User,
    },
    prelude::{RwLock, TypeMap},
};
use std::{error::Error, fs, sync::Arc};
use tokio::sync::RwLockWriteGuard;

pub async fn add_message_to_chain(msg: &Message, ctx: &Context) -> Result<bool, std::io::Error> {
    // if the message was not sent in a guild
    if msg
        .channel_id
        .to_channel(&ctx.http)
        .await
        .expect("Couldn't get channel")
        .guild()
        .is_none()
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
    if let Some(filtered_message) = filtered_message {
        file_operations::append_to_markov_file(&filtered_message)?;
        Ok(true)
    } else {
        Ok(false)
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
            message
        }
        Err(why) => match why {
            markov_strings::ErrorType::CorpusEmpty => "The corpus is empty, try again later!",
            markov_strings::ErrorType::TriesExceeded => "couldn't generate a sentence, try again!",
            markov_strings::ErrorType::CorpusNotEmpty => "Try again later.",
        }
        .to_owned(),
    }
}
/// Initializes the Markov chain from [`MARKOV_DATA_SET_PATH`][global_data::MARKOV_DATA_SET_PATH]
pub fn init() -> Result<Markov, Box<dyn Error>> {
    let mut markov_chain = Markov::new();
    markov_chain.set_state_size(3).expect("Will never fail");
    markov_chain.set_max_tries(1000);
    markov_chain.set_filter(|r| {
        if r.refs.len() >= 2 && r.score > 20 {
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

pub async fn add_user_to_blacklist(
    user: &User,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) {
    let blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;

    blacklisted_users.insert(user.id.0);

    let response = match save_markov_blacklisted_users(&*blacklisted_users) {
        Ok(_) => format!(
            "Added {} to data collection blacklist",
            match command.guild_id {
                Some(guild_id) => user
                    .nick_in(&ctx.http, guild_id)
                    .await
                    .or_else(|| Some(user.name.clone()))
                    .expect("Should always have Some value"),
                None => user.name.clone(),
            }
        ),
        Err(_) => "Something went wrong while adding you to the blacklist :(".to_owned(),
    };

    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content(response))
        })
        .await
        .expect("Error creating interaction response");
}

pub async fn remove_user_from_blacklist(
    user: &User,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) {
    let blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;

    blacklisted_users.remove(&user.id.0);
    let response = match save_markov_blacklisted_users(&*blacklisted_users) {
        Ok(_) => format!(
            "removed {} from data collection blacklist",
            match command.guild_id {
                Some(guild_id) => user
                    .nick_in(&ctx.http, guild_id)
                    .await
                    .or_else(|| Some(user.name.clone()))
                    .expect("Should always have Some value"),
                None => user.name.clone(),
            }
        ),
        Err(_) => "Something went wrong while removing you from the blacklist :(".to_owned(),
    };

    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content(response))
        })
        .await
        .expect("Error creating interaction response");
}

pub async fn blacklisted_users(ctx: Context, command: &ApplicationCommandInteraction) {
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
        command
            .create_interaction_response(ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.content("Currently there are no blacklisted users")
                })
            })
            .await
            .expect("Error creating interaction response");
        return;
    }

    let mut message = String::from("Blacklisted users: ");
    for user_name in blacklisted_usernames {
        message += &(user_name + ", ");
    }

    //remove the trailing comma and whitespace
    message.pop();
    message.pop();
    command
        .create_interaction_response(ctx.http, |r| {
            r.interaction_response_data(|d| d.content(message))
        })
        .await
        .expect("Error creating interaction response");
}

pub fn init_markov_data(
    data: &mut RwLockWriteGuard<TypeMap>,
    markov: markov_strings::Markov,
) -> Result<(), Box<dyn Error>> {
    let blacklisted_channels_in_file: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(global_data::MARKOV_BLACKLISTED_CHANNELS_PATH, "[]")?,
    )?)?;
    let blacklisted_users_in_file: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(global_data::MARKOV_BLACKLISTED_USERS_PATH, "[]")?,
    )?)?;
    data.insert::<global_data::MarkovChain>(Arc::new(RwLock::new(markov)));
    data.insert::<global_data::MarkovBlacklistedChannels>(Arc::new(blacklisted_channels_in_file));
    data.insert::<global_data::MarkovBlacklistedUsers>(Arc::new(blacklisted_users_in_file));
    Ok(())
}
