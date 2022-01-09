//#![windows_subsystem = "windows"]
#![deny(missing_docs, clippy::all)]

//! A discord bot written in rust for fun

mod client;
mod unit_tests;

use client::*;
use serenity::model::{id::GuildId, interactions::*};
use std::{collections::HashSet, panic};

#[tokio::main]
async fn main() {
    create_data_folders();

    dotenv::dotenv().expect("Failed to load .env file\nCreate a .env file in the same folder as the executable and type in the following without the braces or any whitespace:\n\nDISCORD_TOKEN={your discord token here}\nAPPLICATION_ID={your application id here}\n\n");

    start_client().await;
}
