use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use crate::*;
use markov_strings::{ImportExport, InputData};
use serenity::model::channel::Message;

pub fn save_user_listener_blacklist_to_file(blacklist: &HashSet<u64>) {
    fs::write(
        LISTENER_BLACKLISTED_USERS_PATH,
        serde_json::to_string(&blacklist).unwrap(),
    )
    .expect("Something went wrong while writing to file.");
}

pub fn save_listener_response_to_file(listener_response: &HashMap<String, String>) {
    fs::write(
        LISTENER_RESPONSE_PATH,
        serde_json::to_string(&listener_response).unwrap(),
    )
    .expect("Something went wrong while writing to file.");
}

///checks if a file exists and if it doesn't it initializes it
///otherwise it just returns the path back
pub fn create_file_if_missing<'a>(path: &'a str, contents: &str) -> &'a str {
    if !Path::new(path).exists() {
        fs::write(path, contents).expect("Something went wrong while writing to file.");
    }
    path
}

/// If the message filter changes it's helpful to call this function when the bot starts so the filtering is consistent across the file.
#[allow(dead_code)]
pub fn clean_markov_file(msg: &Message) {
    let file = fs::read_to_string(MARKOV_DATA_SET_PATH)
        .expect("Something went wrong while reading the file.");
    let messages = file
        .split("\n\n")
        .map(ToString::to_string)
        .collect::<Vec<String>>();
    fs::write(MARKOV_DATA_SET_PATH, "").expect("Something went wrong while writing to file.");

    let filtered_messages: Vec<String> = messages
        .into_par_iter()
        .map(|message| filter_message_for_markov_file(message, &msg))
        .collect();

    for message in filtered_messages {
        append_to_markov_file(&message);
    }
}

pub fn append_to_markov_file(str: &str) {
    if !str.is_empty() && str.split(' ').count() >= 5 {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(MARKOV_DATA_SET_PATH)
            .unwrap();

        if let Err(e) = writeln!(file, "{}\n", str) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }
}

#[allow(dead_code)]
pub fn export_to_markov_file(export: &ImportExport) -> Result<(), std::io::Error> {
    fs::write(MARKOV_EXPORT_PATH, serde_json::to_string(&export).unwrap())
}

pub fn import_chain_from_file() -> Vec<InputData> {
    let text_from_file =
        fs::read_to_string(create_file_if_missing(MARKOV_DATA_SET_PATH, "")).unwrap();
    let text_array: Vec<&str> = text_from_file.split("\n\n").collect();
    text_array
        .into_par_iter()
        .map(|message| InputData {
            text: message.to_owned(),
            meta: None,
        })
        .collect()
}

pub fn save_markov_blacklisted_users(
    blacklisted_users: &HashSet<u64>,
) -> Result<(), std::io::Error> {
    fs::write(
        MARKOV_BLACKLISTED_USERS_PATH,
        serde_json::to_string(blacklisted_users).unwrap(),
    )
}

pub fn save_bot_channel(bot_channels: &HashMap<u64, u64>) -> Result<(), std::io::Error> {
    fs::write(
        BOT_CHANNEL_PATH,
        serde_json::to_string(bot_channels).unwrap(),
    )
}
