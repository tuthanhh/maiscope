//! Credentials (server-auth-github-oauth 02): access JWTs, opaque refresh
//! tokens, and the `AuthUser` extractor every write endpoint gates on.
//!
//! Two kinds of token, deliberately unlike each other. The access token is a
//! short-lived HS256 JWT that `AuthUser` verifies without touching the
//! database, so it stays authoritative for its whole lifetime — a role change
//! or a deletion takes up to [`ACCESS_TOKEN_TTL`] to bite (production-v2,
//! *Accepted, not solved*). The refresh token is random bytes the client
//! stores and the database holds only a SHA-256 hash of, so a leaked `users`
//! row is not a working credential.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::types::Uuid;

use crate::error::AppError;
use crate::state::AppState;

/// How long an access JWT is honoured. Short because nothing can revoke one.
pub const ACCESS_TOKEN_TTL: Duration = Duration::from_secs(15 * 60);

/// `users.role`. A closed set: a token carrying any other value fails to
/// decode and is rejected, rather than reaching a handler as an unknown role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Moderator,
    Admin,
}

/// The JWT payload. `sub` is the `users.id` UUID in its hyphenated form: RFC
/// 7519 makes `sub` a string, and the extractor parses it back.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: Role,
    pub iat: u64,
    pub exp: u64,
}

/// A request whose bearer token verified. Taking one as a handler argument is
/// the whole of authentication; the handler never sees an unverified id.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: Role,
}

/// Why a token was refused. Only the variant reaches the client, as the 401
/// message; the underlying `jsonwebtoken` error is not exposed.
#[derive(Debug, PartialEq)]
pub enum TokenError {
    Expired,
    Invalid,
}

pub fn issue_jwt(
    secret: &str,
    user_id: Uuid,
    role: Role,
) -> Result<String, jsonwebtoken::errors::Error> {
    issue_jwt_at(secret, user_id, role, unix_now())
}

/// [`issue_jwt`] with the clock passed in, so a test can mint a token that is
/// already expired.
fn issue_jwt_at(
    secret: &str,
    user_id: Uuid,
    role: Role,
    now: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        sub: user_id.to_string(),
        role,
        iat: now,
        exp: now + ACCESS_TOKEN_TTL.as_secs(),
    };
    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_jwt(secret: &str, token: &str) -> Result<AuthUser, TokenError> {
    // HS256 only: pinning the algorithm is what stops a token from choosing
    // its own (`none`, or an asymmetric one keyed with this secret).
    let mut validation = Validation::new(Algorithm::HS256);
    // Issuer and verifier are this server, so there is no clock skew to
    // forgive; the default 60s would stretch every token's life.
    validation.leeway = 0;
    validation.set_required_spec_claims(&["exp", "sub"]);

    let data = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        ErrorKind::ExpiredSignature => TokenError::Expired,
        _ => TokenError::Invalid,
    })?;

    // jsonwebtoken still accepts a token in the second where `now == exp`;
    // RFC 7519 §4.1.4 says from `exp` on it MUST NOT be.
    if data.claims.exp <= unix_now() {
        return Err(TokenError::Expired);
    }

    let id = Uuid::parse_str(&data.claims.sub).map_err(|_| TokenError::Invalid)?;
    Ok(AuthUser {
        id,
        role: data.claims.role,
    })
}

/// 32 random bytes from the thread-local CSPRNG, hex-encoded. Opaque on
/// purpose: a refresh token is looked up, never decoded, so it carries no
/// claims to forge.
pub fn generate_refresh_token() -> String {
    let bytes: [u8; 32] = rand::random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The value stored in `users.refresh_token_hash`. Unsalted SHA-256 is enough
/// here, unlike for a password: the input is 256 random bits, so there is no
/// dictionary to precompute.
pub fn hash_refresh_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before 1970")
        .as_secs()
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts).ok_or(AppError::Unauthorized("missing bearer token"))?;
        verify_jwt(state.config.jwt_secret.expose(), token).map_err(|e| match e {
            TokenError::Expired => AppError::InvalidToken("token expired"),
            TokenError::Invalid => AppError::InvalidToken("invalid token"),
        })
    }
}

/// The token from `Authorization: Bearer <token>`. The scheme is
/// case-insensitive (RFC 7235 §2.1); anything else — no header, another
/// scheme, an empty token — is `None`.
fn bearer_token(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use axum::http::Request;
    use axum::response::IntoResponse;
    use sqlx::postgres::PgPoolOptions;

    use crate::config::{Config, Secret};

    const SECRET: &str = "test-secret-test-secret-test-secret";

    fn user_id() -> Uuid {
        Uuid::parse_str("01a1e99a-d0f6-7e05-9534-21d2526b0462").unwrap()
    }

    #[test]
    fn a_fresh_token_round_trips() {
        let token = issue_jwt(SECRET, user_id(), Role::Moderator).unwrap();
        assert_eq!(
            verify_jwt(SECRET, &token),
            Ok(AuthUser {
                id: user_id(),
                role: Role::Moderator
            })
        );
    }

    #[test]
    fn a_token_past_its_lifetime_is_expired() {
        let issued = unix_now() - ACCESS_TOKEN_TTL.as_secs() - 1;
        let token = issue_jwt_at(SECRET, user_id(), Role::User, issued).unwrap();
        assert_eq!(verify_jwt(SECRET, &token), Err(TokenError::Expired));
    }

    #[test]
    fn a_tampered_payload_is_invalid() {
        // An admin payload under a user token's signature.
        let user = issue_jwt_at(SECRET, user_id(), Role::User, unix_now()).unwrap();
        let admin = issue_jwt_at(SECRET, user_id(), Role::Admin, unix_now()).unwrap();
        let (admin_signed, _) = admin.rsplit_once('.').unwrap();
        let (_, user_signature) = user.rsplit_once('.').unwrap();
        assert_eq!(
            verify_jwt(SECRET, &format!("{admin_signed}.{user_signature}")),
            Err(TokenError::Invalid)
        );
    }

    #[test]
    fn a_token_at_its_exact_expiry_is_expired() {
        let issued = unix_now() - ACCESS_TOKEN_TTL.as_secs();
        let token = issue_jwt_at(SECRET, user_id(), Role::User, issued).unwrap();
        assert_eq!(verify_jwt(SECRET, &token), Err(TokenError::Expired));
    }

    #[test]
    fn another_hmac_algorithm_under_the_right_secret_is_invalid() {
        // The case the algorithm pin exists for: a correctly keyed signature,
        // but not the algorithm this server issues.
        let claims = Claims {
            sub: user_id().to_string(),
            role: Role::Admin,
            iat: unix_now(),
            exp: unix_now() + 60,
        };
        for algorithm in [Algorithm::HS384, Algorithm::HS512] {
            let token = jsonwebtoken::encode(
                &Header::new(algorithm),
                &claims,
                &EncodingKey::from_secret(SECRET.as_bytes()),
            )
            .unwrap();
            assert_eq!(verify_jwt(SECRET, &token), Err(TokenError::Invalid));
        }
    }

    #[test]
    fn a_token_without_exp_or_sub_is_invalid() {
        let key = EncodingKey::from_secret(SECRET.as_bytes());
        let now = unix_now();
        for claims in [
            serde_json::json!({ "sub": user_id().to_string(), "role": "user", "iat": now }),
            serde_json::json!({ "role": "user", "iat": now, "exp": now + 60 }),
        ] {
            let token = jsonwebtoken::encode(&Header::default(), &claims, &key).unwrap();
            assert_eq!(verify_jwt(SECRET, &token), Err(TokenError::Invalid));
        }
    }

    #[test]
    fn a_token_signed_with_another_secret_is_invalid() {
        let token = issue_jwt_at(
            "another-secret-another-secret-another",
            user_id(),
            Role::User,
            unix_now(),
        )
        .unwrap();
        assert_eq!(verify_jwt(SECRET, &token), Err(TokenError::Invalid));
    }

    #[test]
    fn an_unsigned_token_is_invalid() {
        // {"alg":"none"} with a valid-looking payload and no signature.
        let header = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0";
        let real = issue_jwt_at(SECRET, user_id(), Role::Admin, unix_now()).unwrap();
        let payload = real.split('.').nth(1).unwrap();
        assert_eq!(
            verify_jwt(SECRET, &format!("{header}.{payload}.")),
            Err(TokenError::Invalid)
        );
    }

    #[test]
    fn an_unknown_role_or_a_non_uuid_subject_is_invalid() {
        let key = EncodingKey::from_secret(SECRET.as_bytes());
        let now = unix_now();
        let exp = now + 60;
        for claims in [
            serde_json::json!({ "sub": user_id().to_string(), "role": "root", "iat": now, "exp": exp }),
            serde_json::json!({ "sub": "42", "role": "user", "iat": now, "exp": exp }),
        ] {
            let token = jsonwebtoken::encode(&Header::default(), &claims, &key).unwrap();
            assert_eq!(verify_jwt(SECRET, &token), Err(TokenError::Invalid));
        }
    }

    #[test]
    fn refresh_tokens_differ_per_call_and_hash_deterministically() {
        let a = generate_refresh_token();
        let b = generate_refresh_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(hash_refresh_token(&a), hash_refresh_token(&a));
        assert_ne!(hash_refresh_token(&a), a);
        assert_eq!(hash_refresh_token(&a).len(), 64);
    }

    // The extractor is tested by calling it directly, like the handlers are.
    // The pool is lazy and never connects: `AuthUser` reads no database.
    fn state() -> AppState {
        AppState {
            pool: PgPoolOptions::new()
                .connect_lazy("postgres://unused")
                .unwrap(),
            config: Arc::new(Config {
                database_url: "postgres://unused".to_string(),
                database_max_connections: 1,
                port: 0,
                cors_allowed_origins: vec![],
                trusted_ip_headers: vec![],
                jwt_secret: Secret::for_test(SECRET),
                log_filter: "off".to_string(),
                log_json: false,
            }),
        }
    }

    async fn extract(authorization: Option<&str>) -> Result<AuthUser, AppError> {
        let mut request = Request::builder();
        if let Some(value) = authorization {
            request = request.header(AUTHORIZATION, value);
        }
        let (mut parts, ()) = request.body(()).unwrap().into_parts();
        AuthUser::from_request_parts(&mut parts, &state()).await
    }

    /// The 401's message, and its challenge header.
    async fn rejection(result: Result<AuthUser, AppError>) -> (String, String) {
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        let challenge = response.headers()[axum::http::header::WWW_AUTHENTICATE]
            .to_str()
            .unwrap()
            .to_string();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (body["message"].as_str().unwrap().to_string(), challenge)
    }

    #[tokio::test]
    async fn the_extractor_accepts_the_bearer_scheme_in_any_case() {
        let token = issue_jwt_at(SECRET, user_id(), Role::User, unix_now()).unwrap();
        for scheme in ["Bearer", "bearer", "BEARER"] {
            let user = extract(Some(&format!("{scheme} {token}"))).await.unwrap();
            assert_eq!(user.id, user_id());
        }
    }

    #[tokio::test]
    async fn the_extractor_rejects_missing_expired_and_invalid_tokens() {
        // RFC 6750 §3.1: no credential gets a bare challenge; a bad one gets
        // `error="invalid_token"`.
        let missing = ("missing bearer token".to_string(), "Bearer".to_string());
        assert_eq!(rejection(extract(None).await).await, missing);
        assert_eq!(
            rejection(extract(Some("Basic dXNlcjpwYXNz")).await).await,
            missing
        );
        assert_eq!(rejection(extract(Some("Bearer ")).await).await, missing);

        let invalid = r#"Bearer error="invalid_token""#.to_string();
        let issued = unix_now() - ACCESS_TOKEN_TTL.as_secs() - 1;
        let expired = issue_jwt_at(SECRET, user_id(), Role::User, issued).unwrap();
        assert_eq!(
            rejection(extract(Some(&format!("Bearer {expired}"))).await).await,
            ("token expired".to_string(), invalid.clone())
        );
        assert_eq!(
            rejection(extract(Some("Bearer not.a.jwt")).await).await,
            ("invalid token".to_string(), invalid)
        );
    }
}
