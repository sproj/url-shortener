use thiserror::Error;
use tokio_postgres::error::SqlState;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("database connection pool error")]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error("database query error")]
    Query(tokio_postgres::Error),
    #[error("database row mapping error: {0}")]
    Mapping(String),
    #[error("database conflict: state={state:?}, constraint={constraint:?}, message={message}")]
    Conflict {
        state: SqlState,
        constraint: Option<String>,
        message: String,
    },
}

impl From<tokio_postgres::Error> for DatabaseError {
    fn from(err: tokio_postgres::Error) -> Self {
        if let Some(db) = err.as_db_error() {
            let state = db.code().clone();

            if state == SqlState::UNIQUE_VIOLATION || state == SqlState::EXCLUSION_VIOLATION {
                return DatabaseError::Conflict {
                    state,
                    constraint: db.constraint().map(str::to_owned),
                    message: db.message().to_owned(),
                };
            }
        }

        DatabaseError::Query(err)
    }
}
