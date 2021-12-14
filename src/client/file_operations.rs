use std::{error::Error, fs, path::Path};

use crate::*;
use dashmap::{DashMap, DashSet};

pub fn save_user_listener_blacklist_to_file(blacklist: &DashSet<u64>) {
    fs::write(
        LISTENER_BLACKLISTED_USERS_PATH,
        serde_json::to_string(&blacklist).unwrap(),
    )
    .expect("Something went wrong while writing to file.");
}

pub fn save_listener_response_to_file(listener_response: &DashMap<String, String>) {
    fs::write(
        LISTENER_RESPONSE_PATH,
        serde_json::to_string(&listener_response).unwrap(),
    )
    .expect("Something went wrong while writing to file.");
}

/// Checks if a file exists and if it doesn't it initializes it.
/// Otherwise it just returns the path back
pub fn create_file_if_missing<'a>(
    path: &'a str,
    contents: &str,
) -> Result<&'a str, Box<dyn Error>> {
    if !Path::new(path).exists() {
        fs::write(path, contents)?;
    }
    Ok(path)
}

pub fn save_bot_channel(bot_channels: &DashMap<u64, u64>) -> Result<(), std::io::Error> {
    fs::write(
        BOT_CHANNEL_PATH,
        serde_json::to_string(bot_channels).unwrap(),
    )
}

pub fn create_data_folders() {
    if !Path::new("data").exists() {
        fs::create_dir("data").unwrap();
    };
    if !Path::new("data/markov data").exists() {
        fs::create_dir("data/markov data").unwrap();
    };
}
