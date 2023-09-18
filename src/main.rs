#![deny(clippy::all, clippy::pedantic)]

//! A discord bot written in rust for fun

mod client;

use std::time::Duration;

use client::{file_operations, global_data, markov, start, tags, voice};
use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{ExportConfig, TonicConfig};
use opentelemetry_sdk::{
    trace::{Config, TracerProvider},
    Resource,
};
use serenity::model::id::GuildId;
use tracing_subscriber::{
    filter::LevelFilter, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
    EnvFilter, Registry,
};

#[tokio::main]
async fn main() {
    file_operations::create_data_folders();

    _ = dotenvy::dotenv();

    let exporter_config = ExportConfig {
        endpoint: "http://jaeger:4317".to_string(),
        protocol: opentelemetry_otlp::Protocol::Grpc,
        timeout: Duration::from_millis(500),
    };

    let provider = TracerProvider::builder()
        .with_simple_exporter(
            opentelemetry_otlp::SpanExporter::new_tonic(exporter_config, TonicConfig::default())
                .unwrap(),
        )
        .with_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "markov-bot",
            )])),
        )
        .build();

    let tracer = provider.tracer("markov-bot");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let _subscriber = Registry::default().with(telemetry).with(env_filter).init();

    start().await;
}
