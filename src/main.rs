//#![windows_subsystem = "windows"]
#![deny(missing_docs, clippy::all)]

//! A discord bot written in rust for fun

mod client;
mod unit_tests;

use client::*;
use serenity::model::{id::GuildId, interactions::*};

#[tokio::main]
async fn main() {
    file_operations::create_data_folders();

    dotenv::dotenv().expect(
        "Failed to load .env file\n
        Create a .env file in the same folder as the executable and type in the following without the braces or any whitespace:\n\n
        DISCORD_TOKEN={your discord token here}\nAPPLICATION_ID={your application id here}\n\n");

    start_client().await;
}
