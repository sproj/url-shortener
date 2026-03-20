use chrono::Utc;
use deadpool_postgres::Pool;
use tokio_postgres::types::{ToSql, Type};
use uuid::Uuid;

use crate::{
    application::{repository::RepositoryResult, service::user::user_spec::UserSpec},
    domain::models::user::User,
    infrastructure::database::database_error::DatabaseError,
};

pub struct UsersRepository {
    pool: Pool,
}

const SELECT_USER_ROW: &str = "
SELECT 
    id, 
    uuid,
    username,
    email,
    password_hash,
    password_salt,
    active,
    roles,
    created_at,
    updated_at,
    deleted_at 
    FROM
    users";

const INSERT_USER: &str = "
INSERT INTO users (
uuid,
username,
email,
password_hash,
password_salt,
active,
roles
) 
VALUES ($1, $2, $3, $4, $5, $6, $7) 
RETURNING id, uuid, username, password_hash, password_salt, email, active, roles, created_at, updated_at, deleted_at";

const DELETE_USER_BY_UUID: &str = "UPDATE users SET deleted_at = $1 WHERE uuid = $2";

const UPDATE_PASS_BY_USERID: &str =
    "UPDATE users SET password_hash = $1, password_salt = $2 WHERE uuid = $3";

const WITHOUT_SOFT_DELETED: &str = "\n WHERE deleted_at IS NULL";

impl UsersRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> RepositoryResult<Vec<User>> {
        let client = self.pool.get().await?;

        let rows = client
            .query(
                format!("{}\n{}", SELECT_USER_ROW, WITHOUT_SOFT_DELETED).as_str(),
                &[],
            )
            .await?;

        rows.into_iter()
            .map(User::try_from)
            .collect::<Result<_, _>>()
    }

    pub async fn get_user_by_uuid(&self, uuid: Uuid) -> RepositoryResult<Option<User>> {
        tracing::debug!(%uuid, "get by uuid");
        self.pool
            .get()
            .await?
            .query_opt(
                format!(
                    "{}\n{}\n{}",
                    SELECT_USER_ROW, WITHOUT_SOFT_DELETED, "AND uuid = $1 ",
                )
                .as_str(),
                &[&uuid],
            )
            .await?
            .map(User::try_from)
            .transpose()
    }

    pub async fn add_user(&self, spec: UserSpec) -> RepositoryResult<User> {
        tracing::debug!(%spec, "insert user spec");
        let client = self.pool.get().await?;

        let insert_user = client
            .prepare_typed(
                INSERT_USER,
                &[
                    Type::UUID,
                    Type::TEXT,
                    Type::TEXT,
                    Type::TEXT,
                    Type::TEXT,
                    Type::BOOL,
                    Type::TEXT,
                ],
            )
            .await?;

        let params: &[&(dyn ToSql + Sync); 7] = &[
            &spec.uuid,
            &spec.username,
            &spec.email,
            &spec.password_hash,
            &spec.password_salt,
            &spec.active,
            &spec.roles,
        ];

        match client.query_one(&insert_user, params).await {
            Ok(inserted) => inserted.try_into(),
            Err(e) => {
                tracing::error!(%e, "database error on user insert");
                Err(DatabaseError::from(e))
            }
        }
    }

    pub async fn soft_delete_user_by_uuid(&self, uuid: Uuid) -> RepositoryResult<bool> {
        tracing::debug!(%uuid, "delete user by uuid");
        let client = self.pool.get().await?;

        let delete_statement = client.prepare(DELETE_USER_BY_UUID).await?;

        let delete_user_result = client
            .execute(&delete_statement, &[&Utc::now(), &uuid])
            .await?;

        tracing::debug!(%delete_user_result, %uuid);

        Ok(delete_user_result != 0)
    }

    pub async fn update_password_by_uuid(
        &self,
        uuid: Uuid,
        hash: &str,
        salt: &str,
    ) -> RepositoryResult<bool> {
        tracing::debug!(%uuid, "update user by uuid");
        let client = self.pool.get().await?;

        let update_pass_statement = client.prepare(UPDATE_PASS_BY_USERID).await?;

        let update_pass_result = client
            .execute(&update_pass_statement, &[&hash, &salt, &uuid])
            .await?;

        tracing::debug!(%update_pass_result, %uuid);

        Ok(update_pass_result != 0)
    }
}
