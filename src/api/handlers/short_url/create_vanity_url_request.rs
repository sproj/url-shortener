use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::handlers::short_url::{
        ValidatedCreateShortUrlRequest,
        input_validation_rules::{url_cannot_expire_in_the_past, validate_url_input},
    },
    domain::{errors::ShortUrlError, validation_issue::ValidationIssue},
};

#[derive(Deserialize, Debug, Clone)]
pub struct CreateVanityUrlRequest {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub vanity_url: String,
}

impl TryFrom<(CreateVanityUrlRequest, Uuid)> for ValidatedCreateShortUrlRequest {
    type Error = ShortUrlError;
    fn try_from(tuple: (CreateVanityUrlRequest, Uuid)) -> Result<Self, Self::Error> {
        let value = tuple.0;
        let user_uuid = tuple.1;

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

        let vanity_url_input: &str = value.vanity_url.trim();

        // validate_url_input(vanity_url_input, "vanity_url", &mut issues)?;

        Ok(Self {
            long_url: value.long_url.trim().to_string(),
            expires_at: value.expires_at,
            code: Some(vanity_url_input.to_string()),
            user_uuid: Some(user_uuid),
        })
    }
}
