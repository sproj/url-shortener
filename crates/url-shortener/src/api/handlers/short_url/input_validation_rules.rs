use chrono::{DateTime, Utc};

use crate::domain::{
    errors::ShortUrlError,
    validation_issue::{ValidationIssue, ValidationRule},
};

pub fn url_cannot_expire_in_the_past(input_expiry: &DateTime<Utc>, out: &mut Vec<ValidationIssue>) {
    if input_expiry < &Utc::now() {
        let issue = ValidationIssue {
            field: "expires_at".to_string(),
            code: "in_past",
            message: "cannot set expiry in the past".to_string(),
        };

        out.push(issue)
    }
}

pub fn validate_url_input(
    input: &str,
    field_name: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Result<(), ShortUrlError> {
    for rule in TARGET_URL_INPUT_VALIDATION_RULES {
        rule(input, field_name, issues);
    }

    if !issues.is_empty() {
        return Err(ShortUrlError::InvalidInput(issues.to_vec()));
    }

    let target_url = url::Url::parse(input).map_err(|e| {
        ShortUrlError::InvalidInput(vec![ValidationIssue {
            field: field_name.to_string(),
            code: "parse_url",
            message: e.to_string(),
        }])
    })?;

    for rule in TARGET_URL_VALIDATION_RULES {
        rule(&target_url, field_name, issues);
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(ShortUrlError::InvalidInput(issues.to_vec()))
    }
}

pub fn validate_vanity_code(input: &str, issues: &mut Vec<ValidationIssue>) {
    if input.is_empty() {
        issues.push(ValidationIssue {
            field: "vanity_url".to_string(),
            code: "empty",
            message: "vanity code must not be empty".to_string(),
        });
        return;
    }
    if input.len() > 64 {
        issues.push(ValidationIssue {
            field: "vanity_url".to_string(),
            code: "too_long",
            message: "vanity code must not exceed 64 characters".to_string(),
        });
    }
    if !input
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        issues.push(ValidationIssue {
            field: "vanity_url".to_string(),
            code: "invalid_characters",
            message:
                "vanity code may only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
        });
    }
}

pub const TARGET_URL_INPUT_VALIDATION_RULES: &[ValidationRule<str>] = &[
    long_url_input_must_not_be_empty,
    long_url_input_must_not_be_too_long,
];

pub const TARGET_URL_VALIDATION_RULES: &[ValidationRule<url::Url>] =
    &[scheme_must_be_hypertext, no_passwords];

fn long_url_input_must_not_be_empty(input: &str, field: &str, out: &mut Vec<ValidationIssue>) {
    if input.is_empty() {
        let issue = ValidationIssue {
            field: field.to_string(),
            code: "empty",
            message: "empty string is not a valid url".to_string(),
        };

        out.push(issue);
    }
}

fn long_url_input_must_not_be_too_long(input: &str, field: &str, out: &mut Vec<ValidationIssue>) {
    if input.len() > 2048 {
        let issue = ValidationIssue {
            field: field.to_string(),
            code: "too_long",
            message: "more than 2048 characters for a url is too many characters".to_string(),
        };

        out.push(issue);
    }
}

fn scheme_must_be_hypertext(url: &url::Url, field: &str, out: &mut Vec<ValidationIssue>) {
    let scheme = url.scheme();
    if !(scheme == "http" || scheme == "https") {
        let issue = ValidationIssue {
            field: field.to_string(),
            code: "scheme",
            message: "scheme must be `http` or `https`".to_string(),
        };
        out.push(issue);
    }
}

fn no_passwords(url: &url::Url, field: &str, out: &mut Vec<ValidationIssue>) {
    if url.password().is_some() {
        let issue = ValidationIssue {
            field: field.to_string(),
            code: "password",
            message: "directing traffic to a url containing a password is foolish and forbidden and you should feel bad about it".to_string(),
        };
        out.push(issue);
    }
}
