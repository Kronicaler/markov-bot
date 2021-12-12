#![allow(dead_code)]
use crate::*;
use dashmap::{DashMap, DashSet};
use serenity::{
    prelude::{RwLock, TypeMap, TypeMapKey},
    Client,
};
use std::{error::Error, sync::Arc};

pub struct MarkovChain;
impl TypeMapKey for MarkovChain {
    type Value = Arc<RwLock<Markov>>;
}
pub const MARKOV_DATA_SET_PATH: &str = "data/markov data/markov data set.txt";
pub const MARKOV_EXPORT_PATH: &str = "data/markov data/markov export.json";

///user Ids that the bot will not learn from
pub struct MarkovBlacklistedUsers;
impl TypeMapKey for MarkovBlacklistedUsers {
    type Value = Arc<DashSet<u64>>;
}
pub const MARKOV_BLACKLISTED_USERS_PATH: &str = "data/markov data/blacklisted users.json";

///channel Ids that the bot will not learn from
pub struct MarkovBlacklistedChannels;
impl TypeMapKey for MarkovBlacklistedChannels {
    type Value = Arc<DashSet<u64>>;
}
pub const MARKOV_BLACKLISTED_CHANNELS_PATH: &str = "data/markov data/blacklisted channels.json";

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
pub async fn init_global_data_for_client(client: &Client) -> Result<(), Box<dyn Error>> {
    let mut data = client.data.write().await;

    let markov = if cfg!(debug_assertions) {
        println!("Debugging enabled");
        init_markov_debug()?
    } else {
        println!("Debugging disabled");
         init_markov()?
    };

    let blacklisted_channels_in_file: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(MARKOV_BLACKLISTED_CHANNELS_PATH, "[]")?,
    )?)?;
    let blacklisted_users_in_file: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(MARKOV_BLACKLISTED_USERS_PATH, "[]")?,
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

    data.insert::<MarkovChain>(Arc::new(RwLock::new(markov)));
    data.insert::<MarkovBlacklistedChannels>(Arc::new(blacklisted_channels_in_file));
    data.insert::<MarkovBlacklistedUsers>(Arc::new(blacklisted_users_in_file));
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

pub async fn get_markov_blacklisted_users_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<DashSet<u64>> {
    let markov_blacklisted_users_lock = data
        .read()
        .await
        .get::<MarkovBlacklistedUsers>()
        .expect("expected MarkovBlacklistedUsers in TypeMap")
        .clone();
    markov_blacklisted_users_lock
}

pub async fn get_markov_blacklisted_channels_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<DashSet<u64>> {
    let markov_blacklisted_channels_lock = data
        .read()
        .await
        .get::<MarkovBlacklistedChannels>()
        .expect("expected MarkovBlacklistedChannels in TypeMap")
        .clone();
    markov_blacklisted_channels_lock
}

pub async fn get_markov_chain_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<Markov>> {
    let markov_chain_lock = data
        .read()
        .await
        .get::<MarkovChain>()
        .expect("expected MarkovChain in TypeMap")
        .clone();
    markov_chain_lock
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
