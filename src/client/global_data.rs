#![allow(dead_code)]
use crate::*;
use dashmap::{DashMap, DashSet};
use serenity::{prelude::RwLock, Client};
use std::{error::Error, sync::Arc};

use super::tags::global_data::{
    TagBlacklistedUsers, TagResponseChannelIds, Tags, BLACKLISTED_USERS_PATH, BOT_CHANNEL_PATH,
    TAG_PATH,
};

pub const HELP_MESSAGE: &str = "All of my commands are slash commands.
/ping: Pong!
/id: gives you the user id of the selected user
/blacklisted-data: lists out the users the bot will not learn from
/stop-saving-my-messages: blacklist yourself if you don't want the bot to store your messages and learn from them
/continue-saving-my-messages: unblacklist yourself if you want the bot to save and learn from your messages
/create-tag: create a tag that the bot will listen for and then respond to when it is said
/remove-tag: remove a tag
/tags: list out the current tags
/blacklist-me-from-tags: blacklist yourself from tags so the bot won't ping you if you trip off a tag
/set-tag-response-channel: for admins only, set the channel the bot will talk in, if you don't want users using the bot anywhere else you'll have to do it with roles
/version: Check the version of the bot";

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
        create_file_if_missing(markov::global_data::MARKOV_BLACKLISTED_CHANNELS_PATH, "[]")?,
    )?)?;
    let blacklisted_users_in_file: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(markov::global_data::MARKOV_BLACKLISTED_USERS_PATH, "[]")?,
    )?)?;
    let tags: DashMap<String, String> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(TAG_PATH, "{}")?,
    )?)?;
    let user_tag_blacklist: DashSet<u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(BLACKLISTED_USERS_PATH, "[]")?,
    )?)?;
    let bot_channel: DashMap<u64, u64> = serde_json::from_str(&fs::read_to_string(
        create_file_if_missing(BOT_CHANNEL_PATH, "{}")?,
    )?)?;

    data.insert::<markov::global_data::MarkovChain>(Arc::new(RwLock::new(markov)));
    data.insert::<markov::global_data::MarkovBlacklistedChannels>(Arc::new(
        blacklisted_channels_in_file,
    ));
    data.insert::<markov::global_data::MarkovBlacklistedUsers>(Arc::new(blacklisted_users_in_file));
    data.insert::<Tags>(Arc::new(tags));
    data.insert::<TagBlacklistedUsers>(Arc::new(user_tag_blacklist));
    data.insert::<TagResponseChannelIds>(Arc::new(bot_channel));

    Ok(())
}
