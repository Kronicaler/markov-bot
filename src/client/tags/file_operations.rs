use std::fs;

use dashmap::{DashMap, DashSet};

use super::global_data::{BLACKLISTED_USERS_PATH, BOT_CHANNEL_PATH, TAG_PATH};

pub fn save_user_tag_blacklist_to_file(blacklist: &DashSet<u64>) {
    fs::write(
        BLACKLISTED_USERS_PATH,
        serde_json::to_string(&blacklist).expect("Serialization failed"),
    )
    .expect("Something went wrong while writing to file.");
}

pub fn save_tag_to_file(tag: &DashMap<String, String>) {
    fs::write(
        TAG_PATH,
        serde_json::to_string(&tag).expect("Serialization failed"),
    )
    .expect("Something went wrong while writing to file.");
}

pub fn save_tag_response_channel(bot_channels: &DashMap<u64, u64>) -> Result<(), std::io::Error> {
    fs::write(
        BOT_CHANNEL_PATH,
        serde_json::to_string(bot_channels).expect("Serialization failed"),
    )
}
