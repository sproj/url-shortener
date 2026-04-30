use chrono::{DateTime, Utc};

use crate::{
    api::handlers::short_url::UpdateShortUrlRequest,
    api::handlers::short_url::input_validation_rules::{
        url_cannot_expire_in_the_past, validate_url_input, validate_vanity_code,
    },
    domain::{errors::ShortUrlError, validation_issue::ValidationIssue},
};

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
        if let Some(Some(time)) = value.expires_at {
            url_cannot_expire_in_the_past(&time, &mut issues);
        };

        // do not bother parsing and applying url validation issues to obviously crap input.
        if !issues.is_empty() {
            return Err(ShortUrlError::InvalidInput(issues));
        }
        let long_url_input = match value.long_url {
            Some(long) => {
                let s = long.trim();
                validate_url_input(s, "long_url", &mut issues)?;
                Some(s.to_string())
            }
            None => None,
        };

        if !issues.is_empty() {
            return Err(ShortUrlError::InvalidInput(issues));
        }

        let code_input = match value.code {
            Some(input) => {
                let s = input.trim();
                validate_vanity_code(s, &mut issues);
                Some(s.to_string())
            }
            None => None,
        };

        if !issues.is_empty() {
            return Err(ShortUrlError::InvalidInput(issues));
        }

        Ok(Self {
            long_url: long_url_input,
            code: code_input,
            expires_at: value.expires_at,
        })
    }
}

#[derive(Debug)]
pub struct ValidatedUpdateShortUrlRequest {
    pub long_url: Option<String>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
    pub code: Option<String>,
}
