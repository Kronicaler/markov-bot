//#![windows_subsystem = "windows"]
#![deny(missing_docs, clippy::all)]

//! A discord bot written in rust for fun

mod client;
mod gui;
mod system_tray;
mod unit_tests;

use client::*;
use crossbeam::channel::{Receiver, Sender};
use druid::ExtEventSink;
use gui::*;
use markov_strings::Markov;
use rayon::prelude::*;
use serenity::model::{id::GuildId, interactions::*};
use system_tray::*;
use tokio::select;

use std::{collections::HashSet, fs, panic};

const KRONI_ID: u64 = 594_772_815_283_093_524;

#[tokio::main]
async fn main() {
    fs::create_dir("data/markov data").ok();
    dotenv::dotenv().expect("Failed to load .env file\nCreate a .env file in the same folder as the executable and type in the following without the braces or any whitespace:\n\nDISCORD_TOKEN={your discord token here}\nAPPLICATION_ID={your application id here}\n\n");

    let (tx, rx): (Sender<ExtEventSink>, Receiver<ExtEventSink>) = crossbeam::channel::unbounded();
    let (export_and_quit_sender, export_and_quit_receiver): (Sender<bool>, Receiver<bool>) =
        crossbeam::channel::unbounded();

    let senders_to_client = SendersToClient {
        export_and_quit: export_and_quit_sender,
    };

    std::thread::spawn(move || start_gui(&tx, senders_to_client));

    let event_sink = rx.recv().unwrap();

    let front_channel = FrontChannelStruct {
        event_sink,
        export_and_quit_receiver,
    };
    select! {
        _=start_client(front_channel) =>{println!("client exited")},
        _=start_tray() => {println!("tray exited")}
    }
}
