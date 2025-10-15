#![warn(clippy::all, clippy::pedantic)]
#![feature(try_blocks)]

//! A discord bot written in rust for fun

mod client;
mod logging;

use std::process::Command;

use chrono::Duration;
use client::{file_operations, global_data, markov, start, tags, voice};
use logging::setup_logging;
use tokio::{spawn, time::interval};
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
        info_span!("updating_ytdlp").in_scope(|| {
            match Command::new("yt-dlp").args(["--update"]).output() {
                Ok(o) => {
                    let stdout = String::from_utf8(o.stdout).unwrap();
                    let stderr = String::from_utf8(o.stderr).unwrap();
                    if !stdout.is_empty() {
                        info!(stdout);
                    }
                    if !stderr.is_empty() {
                        error!(stderr);
                    }
                }
                Err(e) => error!(?e),
            }
        });
    }
}
