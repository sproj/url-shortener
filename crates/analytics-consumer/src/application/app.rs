use std::sync::Arc;

use crate::application::repository::redirect_event_repository::RedirectEventRepository;
use crate::application::service::analytics_consumer_trait::AnalyticsConsumerTrait;
use crate::application::service::analytics_service::AnalyticsService;
use crate::application::startup_error::StartupError;
use crate::application::state::{AppState, SharedState};
use crate::infrastructure::messaging::rabbitmq::RabbitMqConsumer;
use crate::{api::server, application::config::Config};

use deadpool_postgres::Pool;
use lapin::Channel;

pub struct App {
    config: Config,
    state: SharedState,
}

impl App {
    pub fn builder(config: Config, db_pool: Pool, channel: Channel) -> AppBuilder {
        AppBuilder::builder(config, db_pool, channel)
    }

    pub fn state(&self) -> &SharedState {
        &self.state
    }

    pub async fn start(self) -> Result<(), StartupError> {
        tracing::info!(
            "Starting server on: {}:{}",
            self.config.app.host,
            self.config.app.port
        );

        server::start(self.config, self.state).await
    }
}

pub struct AppBuilder {
    config: Config,
    db_pool: Pool,
    rabbitmq: Channel,
}

impl AppBuilder {
    pub fn builder(config: Config, db_pool: Pool, channel: Channel) -> Self {
        Self {
            config,
            db_pool,
            rabbitmq: channel,
        }
    }

    pub async fn build(self) -> Result<App, StartupError> {
        let cfg = self.config.clone();
        let analytics_consumer: Arc<dyn AnalyticsConsumerTrait> = Arc::new(
            RabbitMqConsumer::new(
                self.rabbitmq,
                cfg.rabbitmq.rabbitmq_exchange.clone(),
                cfg.analytics_queue_name.clone(),
            )
            .setup(&cfg.rabbitmq.redirect_event_routing_key.clone())
            .await
            .map_err(|e| StartupError::RabbitMqConnection(e.to_string()))?,
        );

        let redirect_event_repository =
            Arc::new(RedirectEventRepository::new(self.db_pool.clone()));

        let analytics_service =
            AnalyticsService::new(analytics_consumer, redirect_event_repository);
        let state = Arc::new(AppState {
            db_pool: self.db_pool,
            analytics: Arc::new(analytics_service),
        });
        Ok(App {
            config: self.config,
            state,
        })
    }
}
