#![warn(clippy::all, clippy::pedantic)]
#![feature(try_blocks)]

//! A discord bot written in rust for fun

mod client;
mod logging;

use std::process::Command;

use chrono::Duration;
use client::{file_operations, global_data, markov, start, tags};
use logging::setup_logging;
use tokio::{spawn, time::interval};
use tracing::{error, info, info_span};

#[derive(Debug, Default, Clone)]
pub struct PeriodUnit;
#[derive(Debug, Default, Clone)]
pub struct ProcessingData;

#[derive(Debug, Default, Clone)]
pub struct First {
    state: AggregationState,
}
#[derive(Debug, Default, Clone)]
pub struct Last {
    state: AggregationState,
}

#[derive(Debug, Default, Clone)]
pub struct AggregationState {
    pub signal_id: i32,
    pub period_unit: PeriodUnit,
    pub period_value: i32,
    pub fill_missing_periods: bool,
    pub current_value: f64,
}

pub trait Aggregation {
    fn calc_from_good_value(&self, data: ProcessingData) -> ProcessingData;
    fn calc_from_non_good_value(&self, data: ProcessingData) -> ProcessingData;
    fn init_next_period_value(&self, new_input: ProcessingData);

    fn get_state(&self) -> &AggregationState;
    fn get_value_to_fill_missing_periods(&self) -> f64 {
        let state = self.get_state();

        state.current_value
    }
}

impl Aggregation for First {
    fn calc_from_good_value(&self, data: ProcessingData) -> ProcessingData {
        todo!()
    }
    fn calc_from_non_good_value(&self, data: ProcessingData) -> ProcessingData {
        todo!()
    }
    fn init_next_period_value(&self, new_input: ProcessingData) {
        todo!()
    }
    fn get_state(&self) -> &AggregationState {
        &self.state
    }
}

pub fn calc_from_aggr(aggr: impl Aggregation) -> ProcessingData {
    aggr.calc_from_good_value(ProcessingData {})
}

pub fn calc_from_aggr2(aggr: &impl Aggregation) -> ProcessingData {
    aggr.calc_from_non_good_value(ProcessingData {})
}

pub fn calc_from_aggrs(aggrs: &Vec<impl Aggregation>) {
    for aggr in aggrs {
        aggr.calc_from_non_good_value(ProcessingData {});
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // let first = First::default();
    // let aggregation_first = first;

    // calc_from_aggr2(&aggregation_first);
    // calc_from_aggr(aggregation_first);

    _ = dotenvy::dotenv();
    setup_logging();
    file_operations::create_data_folders();

    spawn(update_ytdlp_loop());

    start().await;
}

async fn update_ytdlp_loop() -> ! {
    let mut interval = interval(Duration::days(1).to_std().unwrap());
    loop {
        interval.tick().await;
        info_span!("updating_ytdlp").in_scope(|| {
            match Command::new("yt-dlp").args(["--update"]).output() {
                Ok(o) => {
                    let stdout =
                        String::from_utf8(o.stdout).unwrap_or(String::from("invalid stdout bytes"));
                    let stderr =
                        String::from_utf8(o.stderr).unwrap_or(String::from("invalid stderr bytes"));
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
