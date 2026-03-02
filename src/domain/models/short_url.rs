use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

use crate::{
    api::handlers::short_url_handlers::ShortUrlError,
    application::repository::database_error::DatabaseError,
    domain::validation_issue::{ValidationIssue, ValidationRule},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortUrl {
    pub id: i64,
    pub code: String,
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ShortUrl {
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|ts| ts <= Utc::now())
    }
}

impl TryFrom<Row> for ShortUrl {
    type Error = DatabaseError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Self::try_from(&row)
    }
}

impl TryFrom<&Row> for ShortUrl {
    type Error = DatabaseError;
    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row
                .try_get::<_, i64>("id")
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
}

#[derive(Deserialize, Debug, Clone)]
pub struct CreateShortUrlDto {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct CreateShortUrlResponseDto {
    pub code: String,
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl CreateShortUrlDto {
    pub fn validate(&self) -> Result<(), ShortUrlError> {
        let input = self.long_url.trim();

        if input.len() > 2048 {
            return Err(ShortUrlError::UnprocessableInput(
                "2048 characters for a url is too many characters".to_string(),
            ));
        }
        if input.is_empty() {
            return Err(ShortUrlError::UnprocessableInput(
                "empty string is not a valid url".to_string(),
            ));
        }

        let mut issues = Vec::new();
        if let Some(time) = self.expires_at {
            cannot_expire_in_the_past(&time, &mut issues);
        }

        let url = url::Url::try_from(input)
            .map_err(|e| ShortUrlError::UnprocessableInput(e.to_string()))?;

        let url_rules: &[ValidationRule<url::Url>] =
            &[scheme_must_be_hypertext, host_must_be_present, no_passwords];

        for rule in url_rules {
            rule(&url, &mut issues);
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(ShortUrlError::InvalidLongUrl(issues))
        }
    }
}

fn scheme_must_be_hypertext(url: &url::Url, out: &mut Vec<ValidationIssue>) {
    let scheme = url.scheme();
    if !(scheme == "http" || scheme == "https") {
        let issue = ValidationIssue {
            field: "long_url",
            code: "dur?",
            message: "scheme must be `http` or `https`".to_string(),
        };
        out.push(issue);
    }
}
fn host_must_be_present(url: &url::Url, out: &mut Vec<ValidationIssue>) {
    if url.host().is_none() {
        let issue = ValidationIssue {
            field: "long_url",
            code: "dur?",
            message: "url must have a host".to_string(),
        };
        out.push(issue);
    }
}
fn no_passwords(url: &url::Url, out: &mut Vec<ValidationIssue>) {
    if url.password().is_some() {
        let issue = ValidationIssue {
            field: "long_url",
            code: "dur?",
            message: "directing traffic to a url containing a password is foolish and forbidden and you should feel bad about it".to_string(),
        };
        out.push(issue);
    }
}
fn cannot_expire_in_the_past(input_expiry: &DateTime<Utc>, out: &mut Vec<ValidationIssue>) {
    if input_expiry < &Utc::now() {
        let issue = ValidationIssue {
            field: "expires_at",
            code: "dur?",
            message: "cannot set expiry in the past".to_string(),
        };

        out.push(issue)
    }
}
