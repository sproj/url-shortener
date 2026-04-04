use chrono::{DateTime, Utc};
use serde::Deserialize;

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
    pub long_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub expires_at: Option<Option<DateTime<Utc>>>,
    pub code: Option<String>,
}
