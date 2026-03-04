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

        let mut issues: Vec<ValidationIssue> = Vec::new();

        for rule in URL_INPUT_VALIDATION_RULES {
            rule(input, &mut issues);
        }

        if let Some(time) = value.expires_at {
            cannot_expire_in_the_past(&time, &mut issues);
        }

        // do not bother parsing and applying url validation issues to obviously crap input.
        if !issues.is_empty() {
            return Err(ShortUrlError::InvalidInput(issues));
        }

        let url = url::Url::try_from(input).map_err(|e| {
            ShortUrlError::InvalidInput(vec![ValidationIssue {
                field: "long_url",
                code: "parse_url",
                message: e.to_string(),
            }])
        })?;

        for rule in URL_VALIDATION_RULES {
            rule(&url, &mut issues);
        }

        if issues.is_empty() {
            Ok(Self {
                long_url: value.long_url.trim().to_string(),
                expires_at: value.expires_at,
            })
        } else {
            Err(ShortUrlError::InvalidInput(issues))
        }
    }
}

const URL_INPUT_VALIDATION_RULES: &[ValidationRule<str>] = &[
    long_url_input_must_not_be_empty,
    long_url_input_must_not_be_too_long,
];
const URL_VALIDATION_RULES: &[ValidationRule<url::Url>] = &[scheme_must_be_hypertext, no_passwords];

fn long_url_input_must_not_be_empty(input: &str, out: &mut Vec<ValidationIssue>) {
    if input.is_empty() {
        let issue = ValidationIssue {
            field: "long_url",
            code: "empty",
            message: "empty string is not a valid url".to_string(),
        };

        out.push(issue);
    }
}

fn long_url_input_must_not_be_too_long(input: &str, out: &mut Vec<ValidationIssue>) {
    if input.len() > 2048 {
        let issue = ValidationIssue {
            field: "long_url",
            code: "too_long",
            message: "more than 2048 characters for a url is too many characters".to_string(),
        };

        out.push(issue);
    }
}

fn scheme_must_be_hypertext(url: &url::Url, out: &mut Vec<ValidationIssue>) {
    let scheme = url.scheme();
    if !(scheme == "http" || scheme == "https") {
        let issue = ValidationIssue {
            field: "long_url",
            code: "scheme",
            message: "scheme must be `http` or `https`".to_string(),
        };
        out.push(issue);
    }
}

fn no_passwords(url: &url::Url, out: &mut Vec<ValidationIssue>) {
    if url.password().is_some() {
        let issue = ValidationIssue {
            field: "long_url",
            code: "password",
            message: "directing traffic to a url containing a password is foolish and forbidden and you should feel bad about it".to_string(),
        };
        out.push(issue);
    }
}

fn cannot_expire_in_the_past(input_expiry: &DateTime<Utc>, out: &mut Vec<ValidationIssue>) {
    if input_expiry < &Utc::now() {
        let issue = ValidationIssue {
            field: "expires_at",
            code: "in_past",
            message: "cannot set expiry in the past".to_string(),
        };

        out.push(issue)
    }
}
