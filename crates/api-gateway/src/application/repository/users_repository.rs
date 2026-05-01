use chrono::Utc;
use deadpool_postgres::Pool;
use metrics::gauge;
use tokio_postgres::types::{ToSql, Type};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    application::repository::RepositoryResult,
    domain::{
        errors::RepositoryError, models::user::User, traits::UsersRepositoryTrait,
        user_spec::UserSpec,
    },
    infrastructure::database::database_error::DatabaseError,
};

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

pub struct PostgresUsersRepository {
    pool: Pool,
}

impl PostgresUsersRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UsersRepositoryTrait for PostgresUsersRepository {
    #[instrument(skip(self))]
    async fn get_all(&self) -> RepositoryResult<Vec<User>> {
        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        let rows = client
            .query(
                format!("{}\n{}", SELECT_USER_ROW, WITHOUT_SOFT_DELETED).as_str(),
                &[],
            )
            .await
            .map_err(DatabaseError::Query)?;

        rows.into_iter()
            .map(user_row_to_model)
            .collect::<Result<_, _>>()
    }

    #[instrument(skip(self))]
    async fn get_user_by_uuid(&self, uuid: Uuid) -> RepositoryResult<Option<User>> {
        tracing::debug!(%uuid, "get by uuid");
        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        client
            .query_opt(
                format!(
                    "{}\n{}\n{}",
                    SELECT_USER_ROW, WITHOUT_SOFT_DELETED, "AND uuid = $1 ",
                )
                .as_str(),
                &[&uuid],
            )
            .await
            .map_err(DatabaseError::Query)?
            .map(user_row_to_model)
            .transpose()
    }

    #[instrument(skip(self))]
    async fn get_user_by_username(&self, username: &str) -> RepositoryResult<Option<User>> {
        tracing::debug!(%username, "finding user by username");
        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        client
            .query_opt(
                format!(
                    "{} {} {}",
                    SELECT_USER_ROW, WITHOUT_SOFT_DELETED, "\n AND username = $1"
                )
                .as_str(),
                &[&username],
            )
            .await
            .map_err(DatabaseError::Query)?
            .map(user_row_to_model)
            .transpose()
    }

    #[instrument(skip(self), fields(username = %spec.username, email = %spec.email, uuid = %spec.uuid))]
    async fn add_user(&self, spec: UserSpec) -> RepositoryResult<User> {
        tracing::debug!(%spec, "insert user spec");
        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);
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
            .await
            .map_err(DatabaseError::from)?;

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
            Ok(inserted) => user_row_to_model(inserted),
            Err(e) => {
                tracing::error!(%e, "database error on user insert");
                Err(RepositoryError::from(DatabaseError::from(e)))
            }
        }
    }

    #[instrument(skip(self))]
    async fn soft_delete_user_by_uuid(&self, uuid: Uuid) -> RepositoryResult<bool> {
        tracing::debug!(%uuid, "delete user by uuid");
        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        let delete_statement = client
            .prepare(DELETE_USER_BY_UUID)
            .await
            .map_err(DatabaseError::Query)?;

        let delete_user_result = client
            .execute(&delete_statement, &[&Utc::now(), &uuid])
            .await
            .map_err(DatabaseError::from)?;

        tracing::debug!(%delete_user_result, %uuid);

        Ok(delete_user_result != 0)
    }

    #[instrument(skip(self), fields(uuid = %uuid))]
    async fn update_password_by_uuid(
        &self,
        uuid: Uuid,
        hash: &str,
        salt: &str,
    ) -> RepositoryResult<bool> {
        tracing::debug!(%uuid, "update user by uuid");
        let client = self.pool.get().await.map_err(DatabaseError::Pool)?;
        let pool_status = self.pool.status();
        gauge!("db_connections_in_use").set(pool_status.size as f64 - pool_status.available as f64);

        let update_pass_statement = client
            .prepare(UPDATE_PASS_BY_USERID)
            .await
            .map_err(DatabaseError::Query)?;

        let update_pass_result = client
            .execute(&update_pass_statement, &[&hash, &salt, &uuid])
            .await
            .map_err(DatabaseError::from)?;

        tracing::debug!(%update_pass_result, %uuid);

        Ok(update_pass_result != 0)
    }
}

fn user_row_to_model(row: tokio_postgres::Row) -> Result<User, RepositoryError> {
    Ok(User {
        id: row
            .try_get::<_, i64>("id")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        uuid: row
            .try_get::<_, uuid::Uuid>("uuid")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        username: row
            .try_get("username")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        email: row
            .try_get("email")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        password_hash: row
            .try_get("password_hash")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        password_salt: row
            .try_get("password_salt")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        active: row
            .try_get("active")
            .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        roles: row
            .try_get("roles")
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
