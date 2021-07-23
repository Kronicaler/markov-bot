#![allow(dead_code)]
use crate::*;
use druid::Target;
use serenity::{
    prelude::{RwLock, TypeMap, TypeMapKey},
    Client,
};
use std::{collections::HashMap, sync::Arc, usize};

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
pub const MARKOV_BLACKLISTED_USERS_PATH: &str = "data/markov data/blacklisted users.json";

///channel Ids that the bot will not learn from
pub struct MarkovBlacklistedChannels;
impl TypeMapKey for MarkovBlacklistedChannels {
    type Value = Arc<RwLock<HashSet<u64>>>;
}
pub const MARKOV_BLACKLISTED_CHANNELS_PATH: &str = "data/markov data/blacklisted channels.json";

pub struct ListenerResponse;
impl TypeMapKey for ListenerResponse {
    type Value = Arc<RwLock<HashMap<String, String>>>;
}
pub const LISTENER_RESPONSE_PATH: &str = "data/action response.json";

pub struct ListenerBlacklistedUsers;
impl TypeMapKey for ListenerBlacklistedUsers {
    type Value = Arc<RwLock<HashSet<u64>>>;
}
pub const LISTENER_BLACKLISTED_USERS_PATH: &str = "data/user listener blacklist.json";

///Server, Channel
pub struct BotChannelIds;
impl TypeMapKey for BotChannelIds {
    type Value = Arc<RwLock<HashMap<u64, u64>>>;
}
pub const BOT_CHANNEL_PATH: &str = "data/bot channel.json";

pub struct MessageCount;
impl TypeMapKey for MessageCount {
    type Value = Arc<RwLock<usize>>;
}

pub struct FrontChannel;
impl TypeMapKey for FrontChannel {
    type Value = Arc<RwLock<FrontChannelStruct>>;
}
pub struct FrontChannelStruct {
    pub event_sink: ExtEventSink,
    pub export_and_quit_receiver: Receiver<bool>,
}

pub async fn init_global_data_for_client(client: &Client, front_channel: FrontChannelStruct) {
    let mut data = client.data.write().await;

    let markov;
    let mut num_of_messages = 10;
    if cfg!(debug_assertions) {
        println!("Debugging enabled");
        markov = init_markov_debug();
    } else {
        println!("Debugging disabled");
        let init = init_markov();
        markov = init.0;
        num_of_messages = init.1;
    }
    front_channel
        .event_sink
        .submit_command(
            SET_MESSAGE_COUNT,
            num_of_messages,
            Target::Widget(ID_MESSAGE_COUNT),
        )
        .unwrap();

    let blacklisted_channels_in_file: HashSet<u64> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(
            MARKOV_BLACKLISTED_CHANNELS_PATH,
            "[]",
        ))
        .expect("couldn't read file"),
    )
    .unwrap();
    let blacklisted_users_in_file: HashSet<u64> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(MARKOV_BLACKLISTED_USERS_PATH, "[]"))
            .expect("couldn't read file"),
    )
    .unwrap();
    let action_response: HashMap<String, String> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(LISTENER_RESPONSE_PATH, "{}")).unwrap(),
    )
    .unwrap();
    let user_listener_blacklist: HashSet<u64> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(
            LISTENER_BLACKLISTED_USERS_PATH,
            "[]",
        ))
        .expect("couldn't read file"),
    )
    .unwrap();
    let bot_channel: HashMap<u64, u64> = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(BOT_CHANNEL_PATH, "{}"))
            .expect("couldn't read file"),
    )
    .unwrap();

    data.insert::<MarkovChain>(Arc::new(RwLock::new(markov)));
    data.insert::<MarkovBlacklistedChannels>(Arc::new(RwLock::new(blacklisted_channels_in_file)));
    data.insert::<MarkovBlacklistedUsers>(Arc::new(RwLock::new(blacklisted_users_in_file)));
    data.insert::<ListenerResponse>(Arc::new(RwLock::new(action_response)));
    data.insert::<ListenerBlacklistedUsers>(Arc::new(RwLock::new(user_listener_blacklist)));
    data.insert::<BotChannelIds>(Arc::new(RwLock::new(bot_channel)));
    data.insert::<MessageCount>(Arc::new(RwLock::new(num_of_messages)));
    data.insert::<FrontChannel>(Arc::new(RwLock::new(front_channel)));
}

pub async fn get_listener_response_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<RwLock<HashMap<String, String>>> {
    let listener_response_lock = data
        .read()
        .await
        .get::<ListenerResponse>()
        .expect("expected ListenerResponse in TypeMap")
        .clone();
    listener_response_lock
}

pub async fn get_listener_blacklisted_users_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<RwLock<HashSet<u64>>> {
    let listener_blacklisted_users_lock = data
        .read()
        .await
        .get::<ListenerBlacklistedUsers>()
        .expect("expected ListenerBlacklistedUsers in TypeMap")
        .clone();
    listener_blacklisted_users_lock
}

pub async fn get_markov_blacklisted_users_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<RwLock<HashSet<u64>>> {
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
) -> Arc<RwLock<HashSet<u64>>> {
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

pub async fn get_bot_channel_id_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<RwLock<HashMap<u64, u64>>> {
    let bot_channel_ids_lock = data
        .read()
        .await
        .get::<BotChannelIds>()
        .expect("expected MarkovChain in TypeMap")
        .clone();
    bot_channel_ids_lock
}

pub async fn get_message_count_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<usize>> {
    let message_count_lock = data
        .read()
        .await
        .get::<MessageCount>()
        .expect("expected MessageCount in TypeMap")
        .clone();
    message_count_lock
}

pub async fn get_front_channel_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<RwLock<FrontChannelStruct>> {
    let event_sink_lock = data
        .read()
        .await
        .get::<FrontChannel>()
        .expect("expected EventSink in TypeMap")
        .clone();
    event_sink_lock
}
