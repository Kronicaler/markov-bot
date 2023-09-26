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

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    file_operations::create_data_folders();

    _ = dotenvy::dotenv();

    let exporter_config = ExportConfig {
        endpoint: dotenvy::var("OTLP_ENDPOINT").unwrap_or("http://localhost:4317".to_string()),
        protocol: opentelemetry_otlp::Protocol::Grpc,
        timeout: Duration::from_millis(500),
    };

    let provider = TracerProvider::builder()
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::new_tonic(exporter_config, TonicConfig::default())
                .unwrap(),
            opentelemetry::runtime::Tokio,
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

    Registry::default().with(telemetry).with(env_filter).init();

    start().await;
}
