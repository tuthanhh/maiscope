//! Extractors whose rejections are `AppError`s, so a malformed request gets
//! the contract's error shape rather than axum's plain-text default.

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

use crate::error::AppError;

/// `Path`, rejecting as `400 bad_request`. axum's own rejection (a segment
/// that is not UTF-8, `%FF`) would otherwise answer in plain text before the
/// handler runs. The chart route, which answers in plain text, takes
/// `Result<ApiPath<_>, AppError>` and renders the error itself.
pub struct ApiPath<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match Path::<T>::from_request_parts(parts, state).await {
            Ok(Path(value)) => Ok(ApiPath(value)),
            Err(rejection) => Err(AppError::BadRequest(rejection.body_text())),
        }
    }
}

/// Request input as it may be quoted back in an error message: at most 64
/// characters, so a long URL is not echoed whole.
pub fn quoted(input: &str) -> String {
    const LIMIT: usize = 64;
    match input.char_indices().nth(LIMIT) {
        Some((end, _)) => format!("{}…", &input[..end]),
        None => input.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_caps_long_input_on_a_character_boundary() {
        assert_eq!(quoted("short"), "short");
        let long = "協".repeat(100);
        let capped = quoted(&long);
        assert_eq!(capped.chars().count(), 65);
        assert!(capped.ends_with('…'));
    }
}
