#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};

use serenity::{client::Context, prelude::RwLock};

use super::*;

pub struct MarkovChain;
impl TypeMapKey for MarkovChain {
    type Value = Arc<RwLock<Markov>>;
}
pub const MARKOV_DATA_SET_PATH: &str = "data/markov data/markov data set.txt";
pub const MARKOV_EXPORT_PATH: &str = "data/markov data/markov export.json";

///user Ids that the bot will not learn from
pub struct MarkovBlacklistedUsers;
impl TypeMapKey for MarkovBlacklistedUsers {
    type Value = Arc<RwLock<HashSet<u64>>>;
}
pub const BLACKLISTED_USERS_PATH: &str = "data/markov data/blacklisted users.json";

///channel Ids that the bot will not learn from
pub struct MarkovBlacklistedChannels;
impl TypeMapKey for MarkovBlacklistedChannels {
    type Value = Arc<RwLock<HashSet<u64>>>;
}
pub const BLACKLISTED_CHANNELS_PATH: &str = "data/markov data/blacklisted channels.json";

pub struct ListenerResponse;
impl TypeMapKey for ListenerResponse {
    type Value = Arc<RwLock<HashMap<String, String>>>;
}
pub const LISTENER_RESPONSE_PATH: &str = "data/action response.json";

pub struct ListenerBlacklistedUsers;
impl TypeMapKey for ListenerBlacklistedUsers {
    type Value = Arc<RwLock<HashSet<u64>>>;
}
pub const USER_LISTENER_BLACKLIST_PATH: &str = "data/user listener blacklist.json";

pub async fn init_global_data_for_client(client: &Client) {
    let mut data = client.data.write().await;

    let markov;
    if cfg!(debug_assertions) {
        println!("Debugging enabled");
        markov = init_markov_debug();
    } else {
        println!("Debugging disabled");
        markov = init_markov();
    }

    let blacklisted_channels_in_file: HashSet<u64> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(BLACKLISTED_CHANNELS_PATH, "[]"))
            .expect("couldn't read file"),
    )
    .unwrap();
    let blacklisted_users_in_file: HashSet<u64> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(BLACKLISTED_USERS_PATH, "[]"))
            .expect("couldn't read file"),
    )
    .unwrap();
    let action_response: HashMap<String, String> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(LISTENER_RESPONSE_PATH, "{}")).unwrap(),
    )
    .unwrap();
    let user_listener_blacklist: HashSet<u64> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(USER_LISTENER_BLACKLIST_PATH, "[]"))
            .expect("couldn't read file"),
    )
    .unwrap();

    data.insert::<MarkovChain>(Arc::new(RwLock::new(markov)));
    data.insert::<MarkovBlacklistedChannels>(Arc::new(RwLock::new(blacklisted_channels_in_file)));
    data.insert::<MarkovBlacklistedUsers>(Arc::new(RwLock::new(blacklisted_users_in_file)));
    data.insert::<ListenerResponse>(Arc::new(RwLock::new(action_response)));
    data.insert::<ListenerBlacklistedUsers>(Arc::new(RwLock::new(user_listener_blacklist)));
}

pub async fn get_listener_response_lock(ctx: &Context) -> Arc<RwLock<HashMap<String, String>>> {
    let listener_response_lock = ctx
        .data
        .read()
        .await
        .get::<ListenerResponse>()
        .expect("expected ListenerResponse in TypeMap")
        .clone();
    listener_response_lock
}

pub async fn get_listener_blacklisted_users_lock(ctx: &Context) -> Arc<RwLock<HashSet<u64>>> {
    let listener_blacklisted_users_lock = ctx
        .data
        .read()
        .await
        .get::<ListenerBlacklistedUsers>()
        .expect("expected ListenerBlacklistedUsers in TypeMap")
        .clone();
    listener_blacklisted_users_lock
}

pub async fn get_markov_blacklisted_users_lock(ctx: &Context) -> Arc<RwLock<HashSet<u64>>> {
    let markov_blacklisted_users_lock = ctx
        .data
        .read()
        .await
        .get::<MarkovBlacklistedUsers>()
        .expect("expected MarkovBlacklistedUsers in TypeMap")
        .clone();
    markov_blacklisted_users_lock
}

pub async fn get_markov_blacklisted_channels_lock(ctx: &Context) -> Arc<RwLock<HashSet<u64>>> {
    let markov_blacklisted_channels_lock = ctx
        .data
        .read()
        .await
        .get::<MarkovBlacklistedChannels>()
        .expect("expected MarkovBlacklistedChannels in TypeMap")
        .clone();
    markov_blacklisted_channels_lock
}

pub async fn get_markov_chain_lock(ctx: &Context) -> Arc<RwLock<Markov>> {
    let markov_chain_lock = ctx
        .data
        .read()
        .await
        .get::<MarkovChain>()
        .expect("expected MarkovChain in TypeMap")
        .clone();
    markov_chain_lock
}
