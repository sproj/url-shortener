use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{
    api::handlers::short_url::input_validation_rules::{
        url_cannot_expire_in_the_past, validate_url_input, validate_vanity_code,
    },
    domain::{errors::ShortUrlError, validation_issue::ValidationIssue},
};

#[derive(Deserialize, Debug, Clone)]
pub struct UpdateShortUrlRequest {
    long_url: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    code: Option<String>,
}

impl TryFrom<UpdateShortUrlRequest> for ValidatedUpdateShortUrlRequest {
    type Error = ShortUrlError;

    fn try_from(value: UpdateShortUrlRequest) -> Result<Self, Self::Error> {
        if value.long_url.is_none() && value.expires_at.is_none() && value.code.is_none() {
            return Err(ShortUrlError::InvalidInput(vec![ValidationIssue {
                field: "all".to_string(),
                code: "all_empty",
                message: "update request is empty".to_string(),
            }]));
        }

        let mut issues: Vec<ValidationIssue> = Vec::new();

        if let Some(time) = value.expires_at {
            url_cannot_expire_in_the_past(&time, &mut issues);
        }

        // do not bother parsing and applying url validation issues to obviously crap input.
        if !issues.is_empty() {
            return Err(ShortUrlError::InvalidInput(issues));
        }

        if let Some(long_url) = value.long_url.clone() {
            let long_url_input = long_url.trim();
            validate_url_input(long_url_input, "long_url", &mut issues)?;
        }

        if !issues.is_empty() {
            return Err(ShortUrlError::InvalidInput(issues));
        }

        if let Some(code) = value.code.clone() {
            let code_input = code.trim();

            validate_vanity_code(code_input, &mut issues);
        }

        if !issues.is_empty() {
            return Err(ShortUrlError::InvalidInput(issues));
        }

        Ok(Self {
            long_url: value.long_url,
            code: value.code,
            expires_at: value.expires_at,
        })
    }
}

#[derive(Debug)]
pub struct ValidatedUpdateShortUrlRequest {
    pub long_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub code: Option<String>,
}
