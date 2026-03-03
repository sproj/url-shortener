use url_shortener::api::error::ApiError;

pub fn pick_error_fields<'a>(
    err: &'a ApiError,
    details_code: &'a str,
    field: &'a str,
) -> Vec<&'a str> {
    err.detail
        .as_ref()
        .and_then(|d| d.get(details_code))
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get(field).and_then(|c| c.as_str()))
                .collect()
        })
        .unwrap_or_default()
}
