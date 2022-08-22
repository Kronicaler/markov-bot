use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use serenity::prelude::{RwLock, TypeMap, TypeMapKey};
use std::sync::Arc;

#[derive(Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: u64,
    pub listener: String,
    pub response: String,
    pub creator_name: String,
    pub creator_id: u64,
}

pub struct TagBlacklistedUsers;
impl TypeMapKey for TagBlacklistedUsers {
    type Value = Arc<DashSet<u64>>;
}
pub const BLACKLISTED_USERS_PATH: &str = "data/user listener blacklist.json";

///Guild, Channel
pub struct TagResponseChannelIds;
impl TypeMapKey for TagResponseChannelIds {
    type Value = Arc<DashMap<u64, u64>>;
}
pub const BOT_CHANNEL_PATH: &str = "data/bot channel.json";

pub async fn get_tags_blacklisted_users_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<DashSet<u64>> {
    let tag_blacklisted_users_lock = data
        .read()
        .await
        .get::<TagBlacklistedUsers>()
        .expect("expected TagBlacklistedUsers in TypeMap")
        .clone();
    tag_blacklisted_users_lock
}

pub async fn get_tag_response_channel_id_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<DashMap<u64, u64>> {
    let bot_channel_ids_lock = data
        .read()
        .await
        .get::<TagResponseChannelIds>()
        .expect("expected MarkovChain in TypeMap")
        .clone();
    bot_channel_ids_lock
}
