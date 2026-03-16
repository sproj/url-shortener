use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::application::{
    repository::short_url_repository::ShortUrlRepository,
    service::short_url::{
        code_generator::{CodeGenerator, RandomCodeGenerator},
        redirect_cache_trait::{NoopRedirectCache, RedirectCache},
        short_url_service::ShortUrlService,
    },
};

pub type SharedState = Arc<AppState>;
pub struct AppState {
    pub short_url: Arc<ShortUrlService>,
    pub db_pool: Pool,
}

pub struct AppStateBuilder {
    code_generator: Arc<dyn CodeGenerator>,
    redirect_cache: Arc<dyn RedirectCache>,
    max_retries: u8,
}

impl AppStateBuilder {
    pub fn builder() -> Self {
        Self::default()
    }
    
    pub fn with_code_generator(mut self, code_generator: Arc<dyn CodeGenerator>) -> Self {
        self.code_generator = code_generator;
        self
    }

    pub fn with_redirect_cache(mut self, redirect_cache: Arc<dyn RedirectCache>) -> Self {
        self.redirect_cache = redirect_cache;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u8) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn build(self, db_pool: Pool) -> AppState {
        let short_url_repository = ShortUrlRepository::new(db_pool.clone());

        let short_url_service = Arc::new(ShortUrlService::new(
            Arc::new(short_url_repository),
            self.code_generator,
            self.max_retries,
            self.redirect_cache,
        ));

        AppState {
            short_url: short_url_service,
            db_pool,
        }
    }
}

impl Default for AppStateBuilder {
    fn default() -> Self {
        Self {
            code_generator: Arc::new(RandomCodeGenerator),
            max_retries: 5,
            redirect_cache: Arc::new(NoopRedirectCache),
        }
    }
}
