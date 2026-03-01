use tokio_postgres::types::{ToSql, Type};

use crate::{
    application::{repository::RepositoryResult, state::SharedState},
    domain::models::short_url::{CreateShortUrlDto, ShortUrl},
};

pub async fn list(state: SharedState) -> RepositoryResult<Vec<ShortUrl>> {
    let client = state.db_pool.get().await?;

    let rows = client.query("SELECT * from short_url", &[]).await?;

    rows.into_iter()
        .map(ShortUrl::try_from)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn add(long_url: String, state: SharedState) -> RepositoryResult<ShortUrl> {
    let dto = CreateShortUrlDto {
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

    Ok(row.try_into()?)
}
