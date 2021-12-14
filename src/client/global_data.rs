#![allow(dead_code)]
use crate::*;
use dashmap::{DashMap, DashSet};
use serenity::{
    prelude::{RwLock, TypeMap, TypeMapKey},
    Client,
};
use std::{error::Error, sync::Arc};

pub const HELP_MESSAGE: &str = "All of my commands are slash commands.
/ping: Pong!
/id: gives you the user id of the selected user
/blacklistedmarkov: lists out the users the bot will not learn from
/blacklistmarkov: blacklist yourself from the markov chain if you don't want the bot to store your messages and learn from them
/setbotchannel: for admins only, set the channel the bot will talk in, if you don't want users using the bot anywhere else you'll have to do it with roles
/createtag: create a tag that the bot will listen for and then respond to when it is said
/removetag: remove a tag
/tags: list out the current tags
/blacklistmefromtags: blacklist yourself from tags so the bot won't ping you if you trip off a tag
/version: Check the version of the bot";

pub struct ListenerResponse;
impl TypeMapKey for ListenerResponse {
    type Value = Arc<DashMap<String, String>>;
}
pub const LISTENER_RESPONSE_PATH: &str = "data/action response.json";

pub struct ListenerBlacklistedUsers;
impl TypeMapKey for ListenerBlacklistedUsers {
    type Value = Arc<DashSet<u64>>;
}
pub const LISTENER_BLACKLISTED_USERS_PATH: &str = "data/user listener blacklist.json";

///Server, Channel
pub struct BotChannelIds;
impl TypeMapKey for BotChannelIds {
    type Value = Arc<DashMap<u64, u64>>;
}
pub const BOT_CHANNEL_PATH: &str = "data/bot channel.json";

/// Initialize the global data for the client so it can be used from multiple threads.
///
/// If this is the first time the bot is run in the environment it will create the data files with initialized contents
pub async fn init_global_data_for_client(client: &Client) -> Result<(), Box<dyn Error>> {
    let mut data = client.data.write().await;

    let markov = if cfg!(debug_assertions) {
        println!("Debugging enabled");
        markov::init_debug()?
    } else {
        println!("Debugging disabled");
        markov::init()?
    };

    let blacklisted_channels_in_file: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(markov::MARKOV_BLACKLISTED_CHANNELS_PATH, "[]")?,
    )?)?;
    let blacklisted_users_in_file: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(markov::MARKOV_BLACKLISTED_USERS_PATH, "[]")?,
    )?)?;
    let action_response: DashMap<String, String> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(LISTENER_RESPONSE_PATH, "{}")?,
    )?)?;
    let user_listener_blacklist: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(LISTENER_BLACKLISTED_USERS_PATH, "[]")?,
    )?)?;
    let bot_channel: DashMap<u64, u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(BOT_CHANNEL_PATH, "{}")?,
    )?)?;

    data.insert::<markov::MarkovChain>(Arc::new(RwLock::new(markov)));
    data.insert::<markov::MarkovBlacklistedChannels>(Arc::new(blacklisted_channels_in_file));
    data.insert::<markov::MarkovBlacklistedUsers>(Arc::new(blacklisted_users_in_file));
    data.insert::<ListenerResponse>(Arc::new(action_response));
    data.insert::<ListenerBlacklistedUsers>(Arc::new(user_listener_blacklist));
    data.insert::<BotChannelIds>(Arc::new(bot_channel));

    Ok(())
}

pub async fn get_listener_response_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<DashMap<String, String>> {
    let listener_response_lock = data
        .read()
        .await
        .get::<ListenerResponse>()
        .expect("expected ListenerResponse in TypeMap")
        .clone();
    listener_response_lock
}

pub async fn get_listener_blacklisted_users_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<DashSet<u64>> {
    let listener_blacklisted_users_lock = data
        .read()
        .await
        .get::<ListenerBlacklistedUsers>()
        .expect("expected ListenerBlacklistedUsers in TypeMap")
        .clone();
    listener_blacklisted_users_lock
}

pub async fn get_bot_channel_id_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<DashMap<u64, u64>> {
    let bot_channel_ids_lock = data
        .read()
        .await
        .get::<BotChannelIds>()
        .expect("expected MarkovChain in TypeMap")
        .clone();
    bot_channel_ids_lock
}
