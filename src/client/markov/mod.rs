pub mod commands;
mod data_access;
mod file_operations;
mod markov_chain;
mod model;

use self::{
    data_access::{
        create_markov_blacklisted_channel, create_markov_blacklisted_server,
        create_markov_blacklisted_user, delete_markov_blacklisted_channel,
        delete_markov_blacklisted_server, delete_markov_blacklisted_user,
        get_markov_blacklisted_channel, get_markov_blacklisted_server, get_markov_blacklisted_user,
    },
    file_operations::{export_corpus_to_file, import_corpus_from_file, import_messages_from_file},
    markov_chain::filter_message_for_markov_file,
    model::{get_markov_chain_lock, replace_markov_chain_lock, MARKOV_EXPORT_PATH},
};
use markov_strings::{ImportExport, Markov};
use serenity::{
    all::{CommandInteraction, CreateInteractionResponseMessage, User},
    builder::CreateInteractionResponse,
    client::Context,
    model::channel::Message,
    prelude::{RwLock, TypeMap},
};
use sqlx::{MySql, MySqlPool, Pool};
use std::{error::Error, sync::Arc};
use tokio::sync::RwLockWriteGuard;
use tracing::{info_span, instrument, Instrument};

pub async fn add_message_to_chain(
    msg: &Message,
    ctx: &Context,
    pool: &Pool<MySql>,
) -> Result<bool, std::io::Error> {
    // if the message was not sent in a guild
    let Some(guild_id) = msg.guild_id else {
        return Ok(false);
    };

    let markov_blacklisted_user = get_markov_blacklisted_user(msg.author.id.get(), pool).await;
    let markov_blacklisted_channel =
        get_markov_blacklisted_channel(msg.channel_id.get(), pool).await;
    let markov_blacklisted_server = get_markov_blacklisted_server(guild_id.get(), pool).await;

    if markov_blacklisted_server.is_some()
        || markov_blacklisted_channel.is_some()
        || markov_blacklisted_user.is_some()
        || msg.mentions_me(&ctx.http).await.unwrap_or(false)
    {
        return Ok(false);
    }

    let filtered_message = filter_message_for_markov_file(msg);
    if let Some(filtered_message) = filtered_message {
        file_operations::append_to_markov_file(&filtered_message)?;

        let data = ctx.data.clone();

        if rand::random::<f32>() < 0.001 {
            tokio::spawn(async move {
                replace_markov_chain_lock(&data).await;
            });
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

#[tracing::instrument(skip(ctx))]
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

#[instrument]
/// Initializes the Markov chain from [`MARKOV_EXPORT_PATH`][model::MARKOV_EXPORT_PATH]
pub fn init() -> Result<Markov, Box<dyn Error>> {
    let mut markov_chain = create_default_chain();

    if !std::path::Path::new(MARKOV_EXPORT_PATH).exists() {
        let input_data = import_messages_from_file()?;

        info_span!("Add markov data to corpus").in_scope(|| {
            markov_chain.add_to_corpus(input_data);
        });

        export_corpus_to_file(&markov_chain.export())?;
    }

    markov_chain = create_default_chain_from_export(import_corpus_from_file()?);

    Ok(markov_chain)
}

pub const MARKOV_STATE_SIZE: usize = 4;
pub const MARKOV_MAX_TRIES: u16 = 5000;

#[instrument]
fn create_default_chain() -> Markov {
    let mut markov_chain = Markov::new();
    markov_chain
        .set_state_size(MARKOV_STATE_SIZE)
        .expect("Will never fail");
    markov_chain.set_max_tries(MARKOV_MAX_TRIES);
    markov_chain.set_filter(markov_filter);
    markov_chain
}

#[instrument(skip(export))]
fn create_default_chain_from_export(export: ImportExport) -> Markov {
    let mut markov_chain = Markov::from_export(export);
    markov_chain.set_max_tries(MARKOV_MAX_TRIES);
    markov_chain.set_filter(markov_filter);
    markov_chain
}

fn markov_filter(r: &markov_strings::MarkovResult) -> bool {
    if r.score >= 10 {
        return true;
    }
    false
}

#[tracing::instrument(skip(ctx))]
pub async fn add_user_to_blacklist(
    user: &User,
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
) {
    let markov_blacklisted_user = get_markov_blacklisted_user(user.id.get(), pool).await;

    if markov_blacklisted_user.is_some() {
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("I'm already not saving your messages"),
                ),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Error creating interaction response");
        return;
    }

    let response = match create_markov_blacklisted_user(user.id.get(), pool).await {
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

#[tracing::instrument(skip(ctx))]
pub async fn remove_user_from_blacklist(
    user: &User,
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
) {
    let response = match delete_markov_blacklisted_user(user.id.get(), pool).await {
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

#[tracing::instrument(skip(ctx))]
pub async fn stop_saving_messages_channel(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &Pool<MySql>,
) {
    let markov_blacklisted_channel =
        get_markov_blacklisted_channel(command.channel_id.get(), pool).await;

    if let Some(c) = markov_blacklisted_channel {
        delete_markov_blacklisted_channel(c.channel_id, pool)
            .await
            .unwrap();
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Continuing message saving in this channel"),
                ),
            )
            .instrument(info_span!("Sending message"))
            .await
            .unwrap();
    } else {
        create_markov_blacklisted_channel(command.channel_id.get(), pool)
            .await
            .unwrap();
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Stopping message saving in this channel"),
                ),
            )
            .instrument(info_span!("Sending message"))
            .await
            .unwrap();
    }
}

#[tracing::instrument(skip(ctx))]
pub async fn stop_saving_messages_server(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &Pool<MySql>,
) {
    let Some(guild_id) = command.guild_id else {
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("This command can only be used in a server"),
                ),
            )
            .instrument(info_span!("Sending message"))
            .await
            .unwrap();
        return;
    };

    let markov_blacklisted_server = get_markov_blacklisted_server(guild_id.into(), pool).await;

    if let Some(s) = markov_blacklisted_server {
        delete_markov_blacklisted_server(s.server_id, pool)
            .await
            .unwrap();
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Continuing message saving in this server"),
                ),
            )
            .instrument(info_span!("Sending message"))
            .await
            .unwrap();
    } else {
        create_markov_blacklisted_server(guild_id.into(), pool)
            .await
            .unwrap();
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Stopping message saving in this server"),
                ),
            )
            .instrument(info_span!("Sending message"))
            .await
            .unwrap();
    }
}

#[instrument(skip(data))]
pub fn init_markov_data(data: &mut RwLockWriteGuard<TypeMap>) -> Result<(), Box<dyn Error>> {
    let markov = init()?;

    data.insert::<model::MarkovChain>(Arc::new(RwLock::new(markov)));
    Ok(())
}
