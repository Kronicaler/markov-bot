use std::{env, time::Duration};

use anyhow::Error;
use dotenvy::dotenv_override;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{ExporterBuildError, WithExportConfig};
use opentelemetry_sdk::Resource;
use tokio::time::interval;
use tracing::{Level, error, level_filters::LevelFilter, subscriber::set_global_default};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::Filtered;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{
    filter::Targets,
    fmt::{format::FmtSpan, time::ChronoLocal},
    layer::SubscriberExt,
    reload,
};

pub fn setup_logging() {
    let otl_address: String =
        env::var("OTLP_ENDPOINT").unwrap_or("http://localhost:4317".to_owned());
    let log_level: Level = env::var("LOG_LEVEL")
        .unwrap_or("info".to_owned())
        .parse()
        .expect("invalid LOG_LEVEL");
    let lib_log_level: Level = env::var("LIB_LOG_LEVEL")
        .unwrap_or("error".to_owned())
        .parse()
        .expect("invalid LIB_LOG_LEVEL");

    let crate_filter = Targets::new().with_target("markov_bot", log_level);
    let lib_filter = Targets::new()
        .with_default(lib_log_level)
        .with_target("markov_bot", LevelFilter::OFF);

    let (crate_layer, crate_handle) = reload::Layer::new(
        tracing_subscriber::fmt::layer()
            .compact()
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_file(true)
            .with_line_number(true)
            .with_target(false)
            .with_timer(ChronoLocal::new("%F %T%.3f".to_string()))
            .with_filter(crate_filter.clone()),
    );

    let (lib_layer, lib_handle) = reload::Layer::new(
        tracing_subscriber::fmt::layer()
            .compact()
            .with_timer(ChronoLocal::new("%F %T%.3f".to_string()))
            .with_filter(lib_filter),
    );

    let tracer_provider = init_tracer_provider(&otl_address)
        .expect("Failed to initialize tracer provider.")
        .tracer("markov_bot");
    let (otl_layer, otl_handle) = reload::Layer::new(
        tracing_opentelemetry::layer()
            .with_tracer(tracer_provider)
            .with_filter(crate_filter),
    );

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let res: Result<(), Error> = reload_log_levels(&crate_handle, &lib_handle, &otl_handle);

            if let Err(err) = res {
                error!(?err);
            }
        }
    });

    let subscriber = tracing_subscriber::registry()
        .with(crate_layer)
        .with(lib_layer)
        .with(otl_layer);

    set_global_default(subscriber).unwrap();
}

fn reload_log_levels<T, U, V, A, B, C, D, E, F>(
    crate_handle: &Handle<Filtered<T, Targets, A>, D>,
    lib_handle: &Handle<Filtered<U, Targets, B>, E>,
    otl_handle: &Handle<Filtered<V, Targets, C>, F>,
) -> Result<(), Error> {
    _ = dotenv_override();

    let log_level: Level = env::var("LOG_LEVEL").unwrap_or("info".to_owned()).parse()?;
    let lib_log_level: Level = env::var("LIB_LOG_LEVEL")
        .unwrap_or("error".to_owned())
        .parse()?;
    let crate_filter = Targets::new().with_target("markov_bot", log_level);
    let lib_filter = Targets::new()
        .with_default(lib_log_level)
        .with_target("markov_bot", LevelFilter::OFF);
    crate_handle.modify(|h| {
        *h.filter_mut() = crate_filter.clone();
    })?;
    otl_handle.modify(|h| {
        *h.filter_mut() = crate_filter;
    })?;
    lib_handle.modify(|h| {
        *h.filter_mut() = lib_filter;
    })?;
    Ok(())
}

pub fn init_tracer_provider(
    otl_address: &str,
) -> Result<opentelemetry_sdk::trace::SdkTracerProvider, ExporterBuildError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otl_address)
        .build()?;

    Ok(opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name("markov_bot").build())
        .build())
}
