use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{
    api::handlers::short_url::input_validation_rules::{
        url_cannot_expire_in_the_past, validate_url_input, validate_vanity_code,
    },
    domain::{errors::ShortUrlError, validation_issue::ValidationIssue},
};

// Serde cannot distinguish a missing field from an explicit `null` for `Option<T>` — both
// deserialize as `None`. For `expires_at` we need that distinction: a missing field means
// "don't touch the existing value", while `null` means "clear the expiry". This deserializer
// is applied only when the field is present in the JSON payload (`#[serde(default)]` handles
// the missing-field case), wrapping the inner `Option` in an outer `Some` so the service layer
// can tell the two cases apart.
fn deserialize_double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpdateShortUrlRequest {
    long_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    expires_at: Option<Option<DateTime<Utc>>>,
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
