pub mod commands;
mod data_access;
mod file_operations;
mod global_data;
mod markov_chain;

use self::{
    data_access::{
        create_markov_blacklisted_server, create_markov_blacklisted_user,
        delete_markov_blacklisted_server, delete_markov_blacklisted_user,
        get_markov_blacklisted_server, get_markov_blacklisted_user,
    },
    file_operations::{
        export_corpus_to_file, generate_new_corpus_from_msg_file, import_corpus_from_file,
        import_messages_from_file,
    },
    global_data::{
        get_markov_blacklisted_channels_lock, get_markov_chain_lock, MARKOV_EXPORT_PATH,
    },
    markov_chain::filter_message_for_markov_file,
};
use super::file_operations::create_file_if_missing;
use dashmap::DashSet;
use markov_strings::Markov;
use serenity::{
    client::Context,
    model::{
        channel::Message,
        prelude::{interaction::application_command::ApplicationCommandInteraction, User},
    },
    prelude::{RwLock, TypeMap},
};
use sqlx::{MySql, MySqlPool, Pool};
use std::{error::Error, fs, sync::Arc};
use tokio::sync::RwLockWriteGuard;

pub async fn add_message_to_chain(
    msg: &Message,
    ctx: &Context,
    pool: &Pool<MySql>,
) -> Result<bool, std::io::Error> {
    // if the message was not sent in a guild
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(false),
    };

    let markov_blacklisted_user = get_markov_blacklisted_user(msg.author.id.0, pool).await;
    let markov_blacklisted_channels = get_markov_blacklisted_channels_lock(&ctx.data).await;

    if get_markov_blacklisted_server(guild_id.0, pool)
        .await
        .is_none()
        || markov_blacklisted_channels.contains(&msg.channel_id.0)
        || markov_blacklisted_user.is_some()
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

        let markov_chain_lock = get_markov_chain_lock(&ctx.data).await;

        if rand::random::<f32>() < 0.005 {
            std::thread::spawn(move || {
                let corpus = generate_new_corpus_from_msg_file().unwrap();

                let mut markov_chain = markov_chain_lock.blocking_write();

                *markov_chain = Markov::from_export(corpus);
            });
        }

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
/// Initializes the Markov chain from [`MARKOV_EXPORT_PATH`][global_data::MARKOV_EXPORT_PATH]
pub fn init() -> Result<Markov, Box<dyn Error>> {
    let mut markov_chain = create_default_chain();

    if !std::path::Path::new(MARKOV_EXPORT_PATH).exists() {
        let input_data = import_messages_from_file()?;
        markov_chain.add_to_corpus(input_data);

        export_corpus_to_file(&markov_chain.export())?;
    }

    markov_chain = Markov::from_export(import_corpus_from_file()?);

    Ok(markov_chain)
}

fn create_default_chain() -> Markov {
    let mut markov_chain = Markov::new();
    markov_chain.set_state_size(3).expect("Will never fail");
    markov_chain.set_max_tries(1000);
    markov_chain.set_filter(|r| {
        if r.refs.len() >= 2 && r.score > 20 {
            return true;
        }
        false
    });
    markov_chain
}

pub async fn add_user_to_blacklist(
    user: &User,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    pool: &MySqlPool,
) {
    let markov_blacklisted_user = get_markov_blacklisted_user(user.id.0, pool).await;

    if markov_blacklisted_user.is_some() {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("I'm already not saving your messages"))
            })
            .await
            .expect("Error creating interaction response");
        return;
    }

    let response = match create_markov_blacklisted_user(user.id.0, pool).await {
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
    pool: &MySqlPool,
) {
    let response = match delete_markov_blacklisted_user(user.id.0, pool).await {
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

pub async fn stop_saving_messages_server(
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
                        d.content("This command can only be used in a server")
                    })
                })
                .await
                .unwrap();
            return;
        }
    };

    let markov_blacklisted_server = get_markov_blacklisted_server(guild_id.into(), pool).await;

    match markov_blacklisted_server {
        Some(s) => {
            delete_markov_blacklisted_server(s.server_id, pool)
                .await
                .unwrap();
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Continuing message saving in this server")
                    })
                })
                .await
                .unwrap();
        }
        None => {
            create_markov_blacklisted_server(guild_id.into(), pool)
                .await
                .unwrap();
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Stopping message saving in this server")
                    })
                })
                .await
                .unwrap();
        }
    }
}

pub fn init_markov_data(data: &mut RwLockWriteGuard<TypeMap>) -> Result<(), Box<dyn Error>> {
    let markov = init()?;

    let blacklisted_channels_in_file: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(global_data::MARKOV_BLACKLISTED_CHANNELS_PATH, "[]")?,
    )?)?;
    data.insert::<global_data::MarkovChain>(Arc::new(RwLock::new(markov)));
    data.insert::<global_data::MarkovBlacklistedChannels>(Arc::new(blacklisted_channels_in_file));
    Ok(())
}
