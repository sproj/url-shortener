use crate::{
    application::{repository::RepositoryResult, service::short_url::ShortUrlSpec},
    domain::models::short_url::ShortUrl,
};
use chrono::Utc;
use deadpool_postgres::{GenericClient, Pool};
use tokio_postgres::types::{ToSql, Type};
use uuid::Uuid;

pub async fn get_all(pool: &Pool) -> RepositoryResult<Vec<ShortUrl>> {
    let client = pool.get().await?;

    let rows = client.query("SELECT id, uuid, code, long_url, expires_at, created_at, updated_at, deleted_at FROM short_url WHERE deleted_at is NULL", &[]).await?;

    rows.into_iter()
        .map(ShortUrl::try_from)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn get_by_uuid(pool: &Pool, uuid: Uuid) -> RepositoryResult<Option<ShortUrl>> {
    tracing::debug!(%uuid, "get by uuid");
    pool
        .get()
        .await?
        .query_opt("SELECT id, uuid, code, long_url, expires_at, created_at, updated_at, deleted_at FROM short_url WHERE uuid = $1", &[&uuid])
        .await?
        .map(ShortUrl::try_from)
        .transpose()
}

pub async fn get_by_code(pool: &Pool, code: &str) -> RepositoryResult<Option<ShortUrl>> {
    tracing::debug!(%code, "get by code");
    pool.get().await?
        .query_opt("SELECT id, uuid, code, long_url, expires_at, created_at, updated_at, deleted_at FROM short_url WHERE code = $1", &[&code])
        .await?
        .map(ShortUrl::try_from)
        .transpose()
}

pub async fn add_one(pool: &Pool, spec: ShortUrlSpec) -> RepositoryResult<ShortUrl> {
    tracing::debug!(%spec, "insert short_url spec");

    let client = pool.get().await?;

    let insert_long_url = client
        .prepare_typed(
            "INSERT INTO short_url (uuid, code, long_url, expires_at) \
        VALUES ($1, $2, $3, $4) \
        RETURNING id, uuid, code, long_url, expires_at, created_at, updated_at, deleted_at",
            &[Type::UUID, Type::TEXT, Type::TEXT, Type::TIMESTAMPTZ],
        )
        .await?;

    let params: &[&(dyn ToSql + Sync); 4] =
        &[&spec.uuid, &spec.code, &spec.long_url, &spec.expires_at];

    let inserted_long_url_row = client.query_one(&insert_long_url, params).await?;

    inserted_long_url_row.try_into()
}

pub async fn delete_one_by_uuid(pool: &Pool, uuid: Uuid) -> RepositoryResult<bool> {
    tracing::debug!(%uuid, "delete short_url by uuid");
    let client = pool.get().await?;

    let delete_statement = client
        .prepare("UPDATE short_url SET deleted_at = $1 WHERE uuid = $2")
        .await?;

    let deleted_count = client
        .execute(&delete_statement, &[&Utc::now(), &uuid])
        .await?;

    tracing::debug!(%deleted_count);
    if deleted_count == 0 {
        Ok(false)
    } else {
        Ok(true)
    }
}
