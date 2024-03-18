#![deny(clippy::all, clippy::pedantic)]

//! A discord bot written in rust for fun

mod client;

use std::time::Duration;

use client::{file_operations, global_data, markov, start, tags, voice};
use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{SpanExporterBuilder, TonicExporterBuilder, WithExportConfig};
use opentelemetry_sdk::{
    runtime::Tokio,
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

    let provider = TracerProvider::builder()
        .with_batch_exporter(
            SpanExporterBuilder::Tonic(
                TonicExporterBuilder::default()
                    .with_timeout(Duration::from_millis(1000))
                    .with_endpoint(
                        dotenvy::var("OTLP_ENDPOINT")
                            .unwrap_or("http://localhost:4317".to_string()),
                    )
                    .with_protocol(opentelemetry_otlp::Protocol::Grpc),
            )
            .build_span_exporter()
            .unwrap(),
            Tokio,
        )
        .with_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "markov_bot",
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
