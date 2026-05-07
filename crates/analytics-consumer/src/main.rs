mod api;
mod application;
mod infrastructure;

use lapin::options::ExchangeDeclareOptions;
use lapin::types::FieldTable;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace as sdktrace};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::application::app::App;
use crate::application::config;
use crate::application::startup_error::StartupError;
use crate::infrastructure::database::postgres::Database;
use crate::infrastructure::messaging;

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    let provider = init_tracer();

    let tracer = provider.tracer("url-shortener");

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new("debug,opentelemetry_sdk=off,opentelemetry=off,h2=off,hyper=off")
            }),
        ))
        .with(
            tracing_opentelemetry::layer()
                .with_tracer(tracer)
                .with_filter(filter_fn(|meta| {
                    !meta.target().starts_with("h2")
                        && !meta.target().starts_with("tower")
                        && !meta.target().starts_with("hyper")
                })),
        )
        .try_init()
        .map_err(|e| StartupError::TracingSubscriber(e.to_string()))?;

    let app = build().await?;
    let consumer = app.state().analytics.clone();
    tokio::select! {
        r = consumer.run() => r?,
        r = app.start() => r?
    }

    provider
        .shutdown()
        .expect("tracer provider shutdown failed");

    Ok(())
}

async fn build() -> Result<App, StartupError> {
    let cfg = config::load()?;
    let db_pool = Database::connect(&cfg.db)?;
    Database::migrate(&db_pool).await?;

    let channel = messaging::connect::connect(&cfg.rabbitmq).await?;
    channel
        .exchange_declare(
            cfg.rabbitmq.rabbitmq_exchange.as_str().into(),
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(|e| StartupError::RabbitMqConnection(e.to_string()))?;

    App::builder(cfg.clone(), db_pool, channel).build().await
}

fn init_tracer() -> sdktrace::SdkTracerProvider {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic() // gRPC, needs the grpc-tonic feature
        .with_endpoint(endpoint) // default Jaeger OTLP gRPC port
        .build()
        .expect("failed to build OTLP exporter");

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name("analytics").build())
        .build();

    // This sets the global OTel provider (optional but useful)
    opentelemetry::global::set_tracer_provider(provider.clone());

    provider
}
