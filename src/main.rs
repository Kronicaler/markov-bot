#![warn(clippy::all, clippy::pedantic)]

//! A discord bot written in rust for fun

mod client;
mod logging;

use client::{file_operations, global_data, markov, start, tags, voice};
use logging::setup_logging;
use serenity::model::id::GuildId;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    _ = dotenvy::dotenv();
    setup_logging();
    file_operations::create_data_folders();

    start().await;
}
