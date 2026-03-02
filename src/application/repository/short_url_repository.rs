use deadpool_postgres::GenericClient;
use tokio_postgres::types::{ToSql, Type};

use crate::{
    application::{repository::RepositoryResult, state::SharedState},
    domain::models::short_url::{CreateShortUrlResponseDto, ShortUrl},
};

pub async fn get_all(state: SharedState) -> RepositoryResult<Vec<ShortUrl>> {
    let client = state.db_pool.get().await?;

    let rows = client.query("SELECT id, code, long_url, expires_at, created_at, updated_at, deleted_at FROM short_url", &[]).await?;

    rows.into_iter()
        .map(ShortUrl::try_from)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn get_by_id(state: SharedState, id: i64) -> RepositoryResult<Option<ShortUrl>> {
    println!("short url_repository::get_by_id called with {}", id);
    state
        .db_pool
        .get()
        .await?
        .query_opt("SELECT id, code, long_url, expires_at, created_at, updated_at, deleted_at FROM short_url WHERE id = $1", &[&id])
        .await?
        .map(ShortUrl::try_from)
        .transpose()
}

pub async fn add_one(state: SharedState, long_url: String) -> RepositoryResult<ShortUrl> {
    println!("short url_repository::add_one called with {}", long_url);
    let dto = CreateShortUrlResponseDto {
        code: bs58::encode(&long_url).into_string(),
        long_url,
        expires_at: None,
    };

    let client = state.db_pool.get().await?;

    let statement = client
        .prepare_typed(
            "INSERT INTO short_url (code, long_url, expires_at) \
            VALUES ($1, $2, $3) \
            RETURNING id, code, long_url, expires_at, created_at, updated_at, deleted_at",
            &[Type::TEXT, Type::TEXT, Type::TIMESTAMPTZ],
        )
        .await?;
    let params: &[&(dyn ToSql + Sync); 3] = &[&dto.code, &dto.long_url, &dto.expires_at];

    let row = client.query_one(&statement, params).await?;

    row.try_into()
}

pub async fn delete_one_by_id(state: SharedState, id: i64) -> RepositoryResult<Option<bool>> {
    println!("short url_repository::delete_one_by_id called with {}", id);
    let client = state.db_pool.get().await?;

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
