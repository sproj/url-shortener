use std::sync::Arc;

use crate::{
    application::{repository::RepositoryResult, service::short_url::ShortUrlSpec},
    domain::models::short_url::ShortUrl,
};
use deadpool_postgres::{GenericClient, Pool};
use tokio_postgres::types::{ToSql, Type};

pub struct ShortUrlRepository {
    pool: Arc<Pool>,
}

impl ShortUrlRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub async fn get_all(&self) -> RepositoryResult<Vec<ShortUrl>> {
        let client = self.pool.get().await?;

        let rows = client.query("SELECT id, uuid, code, long_url, expires_at, created_at, updated_at, deleted_at FROM short_url", &[]).await?;

        rows.into_iter()
            .map(ShortUrl::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn get_by_id(&self, id: i64) -> RepositoryResult<Option<ShortUrl>> {
        println!("short url_repository::get_by_id called with {}", id);
        self.pool
        .get()
        .await?
        .query_opt("SELECT id, uuid, code, long_url, expires_at, created_at, updated_at, deleted_at FROM short_url WHERE id = $1", &[&id])
        .await?
        .map(ShortUrl::try_from)
        .transpose()
    }

    pub async fn add_one(&self, spec: ShortUrlSpec) -> RepositoryResult<ShortUrl> {
        println!("short url_repository::add_one called with {:?}", spec);

        let client = self.pool.get().await?;

        let insert_long_url = client
            .prepare_typed(
                "INSERT INTO short_url (uuid, code, long_url, expires_at) \
        VALUES ($1, $2, $3, $4) \
        RETURNING id, uuid, code, long_url, expires_at, created_at, updated_at, deleted_at",
                &[Type::UUID, Type::TEXT, Type::TEXT, Type::TIMESTAMPTZ],
            )
            .await?;

        let params: &[&(dyn ToSql + Sync); 4] = &[
            &spec.uuid.expect("uuid missing"),
            &spec.code.expect("code missing"),
            &spec.long_url,
            &spec.expires_at,
        ];

        let inserted_long_url_row = client.query_one(&insert_long_url, params).await?;

        inserted_long_url_row.try_into()
    }

    pub async fn delete_one_by_id(&self, id: i64) -> RepositoryResult<Option<bool>> {
        println!("short url_repository::delete_one_by_id called with {}", id);
        let client = self.pool.get().await?;

        let delete_statement = client
            .prepare("DELETE from short_url WHERE id = $1")
            .await?;

        let deleted_count = client.execute(&delete_statement, &[&id]).await?;
        if deleted_count == 0 {
            Ok(None)
        } else {
            Ok(Some(true))
        }
    }
}
