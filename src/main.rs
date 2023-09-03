#![deny(clippy::all, clippy::pedantic)]

//! A discord bot written in rust for fun

mod client;

use client::{file_operations, global_data, markov, start, tags, voice};
use serenity::model::id::GuildId;

#[tokio::main]
async fn main() {
    file_operations::create_data_folders();

    _ = dotenvy::dotenv();

    start().await;
}
