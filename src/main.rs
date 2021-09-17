//#![windows_subsystem = "windows"]
#![deny(missing_docs, clippy::all)]

//! A discord bot written in rust for fun

mod client;
mod unit_tests;

use client::*;
use markov_strings::Markov;
use rayon::prelude::*;
use serenity::model::{id::GuildId, interactions::*};
use tokio::select;

use std::{collections::HashSet, fs, panic};

const KRONI_ID: u64 = 594_772_815_283_093_524;

#[tokio::main]
async fn main() {
    fs::create_dir("data/markov data").ok();
    dotenv::dotenv().expect("Failed to load .env file\nCreate a .env file in the same folder as the executable and type in the following without the braces or any whitespace:\n\nDISCORD_TOKEN={your discord token here}\nAPPLICATION_ID={your application id here}\n\n");

    select! {
        _=start_client() =>{println!("client exited")},
        //_=start_tray() => {println!("tray exited")}
    }
}
