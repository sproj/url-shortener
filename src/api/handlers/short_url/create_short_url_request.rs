use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{
    api::handlers::short_url::ShortUrlError,
    domain::{
        models::short_url::NewShortUrlDto,
        validation_issue::{ValidationIssue, ValidationRule},
    },
};

#[derive(Deserialize, Debug, Clone)]
pub struct CreateShortUrlRequest {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl TryFrom<CreateShortUrlRequest> for NewShortUrlDto {
    type Error = ShortUrlError;
    fn try_from(value: CreateShortUrlRequest) -> Result<Self, Self::Error> {
        let input: &str = value.long_url.trim();

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

        let mut issues: Vec<ValidationIssue> = Vec::new();
        if let Some(time) = value.expires_at {
            cannot_expire_in_the_past(&time, &mut issues);
        }

        let url = url::Url::try_from(input)
            .map_err(|e| ShortUrlError::UnprocessableInput(e.to_string()))?;

        for rule in URL_INPUT_VALIDATION_RULES {
            rule(&url, &mut issues);
        }

        if issues.is_empty() {
            Ok(Self {
                long_url: value.long_url.trim().to_string(),
                expires_at: value.expires_at,
            })
        } else {
            Err(ShortUrlError::InvalidLongUrl(issues))
        }
    }
}

const URL_INPUT_VALIDATION_RULES: &[ValidationRule<url::Url>] =
    &[scheme_must_be_hypertext, host_must_be_present, no_passwords];

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
