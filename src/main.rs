#![warn(clippy::all, clippy::pedantic)]

//! A discord bot written in rust for fun

mod client;
mod logging;

use chrono::Duration;
use client::{file_operations, global_data, markov, start, tags, voice};
use logging::setup_logging;
use serenity::model::id::GuildId;
use tokio::{process::Command, spawn, time::interval};
use tracing::{error, info, info_span};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    _ = dotenvy::dotenv();
    setup_logging();
    file_operations::create_data_folders();

    spawn(update_ytdlp_loop());

    start().await;
}

async fn update_ytdlp_loop() {
    let mut interval = interval(Duration::days(1).to_std().unwrap());
    loop {
        interval.tick().await;
        info_span!("updating_ytdlp")
            .in_scope(async || {
                match Command::new("yt-dlp")
                    .args(["yt-dlp", "--update"])
                    .output()
                    .await
                {
                    Ok(o) => {
                        info!("{:?}", String::from_utf8(o.stdout));
                        error!("{:?}", String::from_utf8(o.stderr));
                    }
                    Err(e) => error!(?e),
                }
            })
            .await;
    }
}
