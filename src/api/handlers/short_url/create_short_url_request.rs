use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{
    api::handlers::short_url::input_validation_rules::{
        url_cannot_expire_in_the_past, validate_url_input,
    },
    application::service::short_url::ValidatedCreateShortUrlRequest,
    domain::{errors::ShortUrlError, validation_issue::ValidationIssue},
};

#[derive(Deserialize, Debug, Clone)]
pub struct CreateShortUrlRequest {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl TryFrom<CreateShortUrlRequest> for ValidatedCreateShortUrlRequest {
    type Error = ShortUrlError;
    fn try_from(value: CreateShortUrlRequest) -> Result<Self, Self::Error> {
        let target_url_input: &str = value.long_url.trim();

        let mut issues: Vec<ValidationIssue> = Vec::new();

        if let Some(time) = value.expires_at {
            url_cannot_expire_in_the_past(&time, &mut issues);
        }

        // do not bother parsing and applying url validation issues to obviously crap input.
        if !issues.is_empty() {
            return Err(ShortUrlError::InvalidInput(issues));
        }

        validate_url_input(target_url_input, "long_url", &mut issues)?;

        if issues.is_empty() {
            Ok(Self {
                long_url: value.long_url.trim().to_string(),
                expires_at: value.expires_at,
                code: None,
                user_uuid: None,
            })
        } else {
            Err(ShortUrlError::InvalidInput(issues))
        }
    }
}
