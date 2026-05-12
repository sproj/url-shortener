use common::{
    events::redirect_event::{RedirectEvent, RedirectType},
    repository_error::RepositoryError,
};
use deadpool_postgres::Pool;
use tokio_postgres::types::{ToSql, Type};

use crate::{
    application::repository::redirect_event_repository_trait::RedirectRepositoryTrait,
    infrastructure::database::database_error::DatabaseError,
};

pub struct RedirectEventRepository {
    db_pool: Pool,
}

impl RedirectEventRepository {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

const INSERT_REDIRECT_EVENT: &str = "
INSERT INTO redirect_events 
(code, long_url, redirect_type, timestamp, event_id) 
VALUES 
($1, $2, $3, $4, $5)
ON CONFLICT (event_id) DO NOTHING;";
#[async_trait::async_trait]
impl RedirectRepositoryTrait for RedirectEventRepository {
    async fn save(&self, event: &RedirectEvent) -> Result<(), RepositoryError> {
        let client = self
            .db_pool
            .get()
            .await
            .map_err(|e| RepositoryError::Internal(DatabaseError::Pool(e).to_string()))?;

        let insert_redirect_event = client
            .prepare_typed(
                INSERT_REDIRECT_EVENT,
                &[
                    Type::TEXT,
                    Type::TEXT,
                    Type::TEXT,
                    Type::TIMESTAMPTZ,
                    Type::UUID,
                ],
            )
            .await
            .map_err(|e| RepositoryError::Internal(DatabaseError::from(e).to_string()))?;

        let params: &[&(dyn ToSql + Sync); 5] = &[
            &event.code,
            &event.long_url,
            match &event.redirect_type {
                RedirectType::Permanent => &"permanent",
                RedirectType::Temporary => &"temporary",
            },
            &event.timestamp,
            &event.event_id,
        ];

        let insert_result = client
            .execute(&insert_redirect_event, params)
            .await
            .map_err(|e| RepositoryError::Internal(DatabaseError::from(e).to_string()))?;

        if insert_result == 0 {
            tracing::debug!(%event.event_id, %event.code, "deduplicated event (already seen)")
        } else {
            tracing::debug!(%insert_result, "inserted event");
        }

        Ok(())
    }
}
