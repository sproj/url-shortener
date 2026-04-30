use crate::{
    application::repository::RepositoryResult,
    domain::{
        errors::RepositoryError, models::short_url::ShortUrl, short_url_spec::ShortUrlSpec,
        traits::ShortUrlRepositoryTrait,
    },
    infrastructure::database::database_error::DatabaseError,
};
use chrono::Utc;
use deadpool_postgres::{GenericClient, Pool};
use metrics::gauge;
use tokio_postgres::types::{ToSql, Type};
use tracing::instrument;
use uuid::Uuid;

const SELECT_SHORT_URL_ROW: &str = "SELECT
id, 
uuid, 
code, 
long_url, 
expires_at,
user_id, 
created_at, 
updated_at, 
deleted_at 
FROM short_url
";

const WITHOUT_SOFT_DELETED: &str = "WHERE deleted_at IS NULL";

const UPDATE_SHORT_URL_ROW: &str = "UPDATE short_url SET
long_url = $1,
code = $2,
expires_at = $3
WHERE uuid = $4
RETURNING id, uuid, code, long_url, expires_at, user_id, created_at, updated_at, deleted_at";

pub struct PostgresShortUrlRepository {
    pool: Pool,
}

impl PostgresShortUrlRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ShortUrlRepositoryTrait for PostgresShortUrlRepository {
    #[instrument(skip(self))]
    async fn get_all(&self) -> RepositoryResult<Vec<ShortUrl>> {
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);
        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;

        let rows = client
            .query(
                format!("{} {}", SELECT_SHORT_URL_ROW, WITHOUT_SOFT_DELETED).as_str(),
                &[],
            )
            .await
            .map_err(DatabaseError::Query)?;

        rows.into_iter()
            .map(short_url_row_to_model)
            .collect::<Result<Vec<_>, _>>()
    }

    #[instrument(skip(self))]
    async fn get_by_uuid(&self, uuid: Uuid) -> RepositoryResult<Option<ShortUrl>> {
        tracing::debug!(%uuid, "get by uuid");
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);
        self.pool
            .get()
            .await
            .map_err(DatabaseError::Pool)?
            .query_opt(
                format!(
                    "{} {} {}",
                    SELECT_SHORT_URL_ROW, "WHERE uuid = $1", "AND deleted_at IS NULL"
                )
                .as_str(),
                &[&uuid],
            )
            .await
            .map_err(DatabaseError::Query)?
            .map(short_url_row_to_model)
            .transpose()
            .map_err(|e| RepositoryError::Internal(e.to_string()))
    }

    #[instrument(skip(self))]
    async fn get_by_code(&self, code: &str) -> RepositoryResult<Option<ShortUrl>> {
        tracing::debug!(%code, "get by code");

        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;

        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        client
            .query_opt(
                format!("{} WHERE code = $1", SELECT_SHORT_URL_ROW).as_str(),
                &[&code],
            )
            .await
            .map_err(DatabaseError::Query)?
            .map(short_url_row_to_model)
            .transpose()
            .map_err(|e| RepositoryError::Internal(e.to_string()))
    }

    #[instrument(skip(self))]
    async fn add_one(&self, spec: ShortUrlSpec) -> RepositoryResult<ShortUrl> {
        tracing::debug!(%spec, "insert short_url spec");

        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        let insert_long_url = client
        .prepare_typed(
            "INSERT INTO short_url (uuid, code, long_url, expires_at, user_id) \
        VALUES ($1, $2, $3, $4, $5) \
        RETURNING id, uuid, code, long_url, expires_at, created_at, updated_at, deleted_at, user_id",
            &[Type::UUID, Type::TEXT, Type::TEXT, Type::TIMESTAMPTZ, Type::INT8],
        )
        .await.map_err(DatabaseError::Query)?;

        let params: &[&(dyn ToSql + Sync); 5] = &[
            &spec.uuid,
            &spec.code,
            &spec.long_url,
            &spec.expires_at,
            &spec.user_id,
        ];

        let inserted_long_url_row = client
            .query_one(&insert_long_url, params)
            .await
            .map_err(DatabaseError::from)?;

        short_url_row_to_model(inserted_long_url_row)
    }

    #[instrument(skip(self))]
    async fn update_one_by_uuid(&self, spec: ShortUrlSpec) -> RepositoryResult<ShortUrl> {
        tracing::debug!(%spec, "update short_url spec");

        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        let update_statement = client
            .prepare(UPDATE_SHORT_URL_ROW)
            .await
            .map_err(DatabaseError::Query)?;

        let params: &[&(dyn ToSql + Sync); 4] =
            &[&spec.long_url, &spec.code, &spec.expires_at, &spec.uuid];

        let res = client
            .query_one(&update_statement, params)
            .await
            .map_err(DatabaseError::from)?;

        short_url_row_to_model(res)
    }

    #[instrument(skip(self))]
    async fn delete_one_by_uuid(&self, uuid: Uuid) -> RepositoryResult<bool> {
        tracing::debug!(%uuid, "delete short_url by uuid");
        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        let delete_statement = client
            .prepare("UPDATE short_url SET deleted_at = $1 WHERE uuid = $2")
            .await
            .map_err(DatabaseError::Query)?;

        let deleted_count = client
            .execute(&delete_statement, &[&Utc::now(), &uuid])
            .await
            .map_err(DatabaseError::from)?;

        tracing::debug!(%deleted_count);
        if deleted_count == 0 {
            Ok(false)
        } else {
            Ok(true)
        }
    }
}

fn short_url_row_to_model(row: tokio_postgres::Row) -> Result<ShortUrl, RepositoryError> {
    Ok(ShortUrl {
        id: row
            .try_get::<_, i64>("id")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        uuid: row
            .try_get::<_, uuid::Uuid>("uuid")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        code: row
            .try_get("code")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        long_url: row
            .try_get("long_url")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        expires_at: row
            .try_get("expires_at")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        user_id: row
            .try_get::<_, Option<i64>>("user_id")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        deleted_at: row
            .try_get("deleted_at")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
    })
}
