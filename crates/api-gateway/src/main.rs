use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace as sdktrace};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use api_gateway::application::{app::App, config, startup_error::StartupError};
use api_gateway::infrastructure::{database::postgres::Database, redis::connect};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    let provider = init_tracer();

    let tracer = provider.tracer("api-gateway");

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

    let result = run().await;

    provider
        .shutdown()
        .expect("tracer provider shutdown failed");

    if let Err(e) = result {
        eprintln!("startup error: {e}");
        std::process::exit(1);
    } else {
        Ok(())
    }
}

async fn run() -> Result<(), StartupError> {
    let cfg = config::load()?;
    let db_pool = Database::connect(&cfg.db)?;
    Database::migrate(&db_pool).await?;
    let redis = connect::connect(&cfg.redis).await?;
    App::builder(cfg, db_pool)
        .with_redis(redis)
        .build()
        .await?
        .start()
        .await
}

fn init_tracer() -> sdktrace::SdkTracerProvider {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("failed to build OTLP exporter");

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name("api-gateway")
                .build(),
        )
        .build();

    opentelemetry::global::set_tracer_provider(provider.clone());

    provider
}
