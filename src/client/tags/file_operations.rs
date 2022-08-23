use super::global_data::BOT_CHANNEL_PATH;
use dashmap::DashMap;
use std::fs;

pub fn save_tag_response_channel(bot_channels: &DashMap<u64, u64>) -> Result<(), std::io::Error> {
    fs::write(
        BOT_CHANNEL_PATH,
        serde_json::to_string(bot_channels).expect("Serialization failed"),
    )
}
