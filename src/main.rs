#![deny(clippy::pedantic, warnings)]

//! A discord bot written in rust for fun

mod client;

use client::{file_operations, global_data, markov, start, tags, voice};
use serenity::model::id::GuildId;

#[tokio::main]
async fn main() {
    file_operations::create_data_folders();

    dotenv::dotenv().expect(
        "Failed to load .env file\n
        Create a .env file in the same folder as the executable and type in the following without the braces or any whitespace:\n\n
        DISCORD_TOKEN={your discord token here}\nAPPLICATION_ID={your application id here}\n\n");

    start().await;
}
