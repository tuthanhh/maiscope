//! GitHub sign-in (server-auth-github-oauth 03, contract §4).
//!
//! GitHub sends the browser back to the *frontend*, not here: the SPA and
//! this API sit on different registrable domains, so a JSON body returned to
//! a navigation on this origin would be unreadable by the SPA. The SPA picks
//! `state`, checks it when GitHub returns, and then fetches the callback
//! below with the code. This server keeps no OAuth state at all; binding the
//! response to the browser that started the flow is the SPA's half.

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{StatusCode, header::LOCATION},
    response::{IntoResponse, Response},
    routing::get,
};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet,
    EndpointSet, RedirectUrl, RequestTokenError, TokenResponse, TokenUrl,
};
use serde::Deserialize;

use crate::auth::{self, REFRESH_TOKEN_TTL_DAYS, Role};
use crate::config::GitHubConfig;
use crate::error::AppError;
use crate::queries;
use crate::state::AppState;
use crate::types::{SessionResponse, User};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/github/login", get(github_login))
        .route("/auth/github/callback", get(github_callback))
}

/// oauth2's client with an authorize and a token endpoint set, and nothing
/// else: the typestate makes calling a flow this app never configured a
/// compile error.
type GitHubClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

fn github_client(github: &GitHubConfig) -> Result<GitHubClient, AppError> {
    let auth_url = AuthUrl::new(format!("{}/login/oauth/authorize", github.oauth_base))
        .map_err(|_| AppError::Internal("github authorize url"))?;
    let token_url = TokenUrl::new(format!("{}/login/oauth/access_token", github.oauth_base))
        .map_err(|_| AppError::Internal("github token url"))?;
    let redirect_url = RedirectUrl::new(github.redirect_url.clone())
        .map_err(|_| AppError::Internal("github redirect url"))?;
    Ok(BasicClient::new(ClientId::new(github.client_id.clone()))
        .set_client_secret(ClientSecret::new(github.client_secret.expose().to_string()))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url)
        // GitHub documents the secret as a form field; oauth2 defaults to
        // HTTP Basic.
        .set_auth_type(AuthType::RequestBody))
}

/// oauth2 is built without its own HTTP client (Cargo.toml), so its requests
/// go through the server's reqwest. Both speak `http` 1.x, so this is a
/// conversion, not a translation.
async fn send(
    client: &reqwest::Client,
    request: oauth2::HttpRequest,
) -> Result<oauth2::HttpResponse, reqwest::Error> {
    let response = client.execute(request.try_into()?).await?;
    let mut builder = oauth2::http::Response::builder().status(response.status());
    for (name, value) in response.headers() {
        builder = builder.header(name, value);
    }
    Ok(builder
        .body(response.bytes().await?.to_vec())
        .expect("status and headers came from a response that parsed"))
}

#[derive(Deserialize)]
struct LoginParams {
    state: Option<String>,
}

/// A `state` the SPA could plausibly have generated: 16 to 128 URL-unreserved
/// characters. Refusing anything else means this route never builds a GitHub
/// URL around a missing or attacker-shaped value.
fn is_valid_state(state: &str) -> bool {
    (16..=128).contains(&state.len())
        && state
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
}

// GET /auth/github/login?state= — 302 to GitHub's consent page.
async fn github_login(
    State(state): State<AppState>,
    Query(params): Query<LoginParams>,
) -> Result<Response, AppError> {
    let csrf = params
        .state
        .filter(|s| is_valid_state(s))
        .ok_or_else(|| AppError::BadRequest("state must be 16-128 URL-safe characters".into()))?;

    // No scopes: the public profile is all this app reads, and GitHub grants
    // it to any token.
    let (url, _) = github_client(&state.config.github)?
        .authorize_url(|| CsrfToken::new(csrf))
        .url();
    // 302 rather than axum's `Redirect::to` (303): the contract names 302.
    Ok((StatusCode::FOUND, [(LOCATION, url.to_string())]).into_response())
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
}

/// The fields of `GET /user` this app stores. Anything else GitHub returns,
/// including an email, is never deserialised.
#[derive(Deserialize)]
struct GitHubProfile {
    id: i64,
    login: String,
    avatar_url: Option<String>,
}

// GET /auth/github/callback?code= — exchange, upsert, open a session.
async fn github_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> Result<Json<SessionResponse>, AppError> {
    let code = params
        .code
        .filter(|c| !c.is_empty())
        .ok_or_else(|| AppError::BadRequest("missing code".into()))?;

    // Each request's future owns a clone of the client (an `Arc` inside)
    // rather than borrowing it: a borrowed one makes the handler's future
    // `Send` only for one lifetime, and axum needs it for all.
    let http = |request: oauth2::HttpRequest| {
        let client = state.http.clone();
        async move { send(&client, request).await }
    };
    let token = github_client(&state.config.github)?
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&http)
        .await
        .map_err(|e| {
            if code_was_refused(&e) {
                tracing::warn!(error = %e, "github refused an authorization code");
                AppError::BadRequest("GitHub rejected the authorization code".into())
            } else {
                // Includes GitHub refusing *this server*: wrong client secret,
                // a redirect URL the app does not list. Those are ours to fix,
                // so they are logged as errors by `BadGateway`, not blamed on
                // the caller's code.
                tracing::warn!(error = %e, "github token exchange failed");
                AppError::BadGateway("github token exchange")
            }
        })?;

    let profile = fetch_profile(&state, token.access_token().secret()).await?;

    let refresh_token = auth::generate_refresh_token();
    let mut tx = state.pool.begin().await?;
    let user = queries::upsert_github_user(
        &mut tx,
        profile.id,
        &profile.login,
        profile.avatar_url.as_deref(),
    )
    .await?;
    queries::start_session(
        &mut tx,
        user.id,
        &auth::hash_refresh_token(&refresh_token),
        REFRESH_TOKEN_TTL_DAYS,
    )
    .await?;
    tx.commit().await?;

    let role = Role::from_db(&user.role).ok_or(AppError::Internal("users.role out of range"))?;
    let token = auth::issue_jwt(state.config.jwt_secret.expose(), user.id, role)
        .map_err(|_| AppError::Internal("jwt encoding"))?;

    Ok(Json(SessionResponse {
        token,
        refresh_token,
        user: User {
            id: user.id.to_string(),
            login: user.login,
            avatar_url: user.avatar_url,
            role,
        },
    }))
}

/// Whether a failed exchange was the caller's code being bad, expired or
/// already used — the one failure that is the caller's, not this server's.
/// GitHub reports it as a 200 whose body is `{"error":
/// "bad_verification_code"}`, which oauth2 cannot parse as a token and so
/// surfaces as `Parse`; a provider following RFC 6749 §5.2 would send
/// `invalid_grant` as a `ServerResponse`.
fn code_was_refused(
    e: &RequestTokenError<reqwest::Error, oauth2::basic::BasicErrorResponse>,
) -> bool {
    match e {
        RequestTokenError::ServerResponse(response) => matches!(
            response.error(),
            oauth2::basic::BasicErrorResponseType::InvalidGrant
        ),
        RequestTokenError::Parse(_, body) => serde_json::from_slice::<serde_json::Value>(body)
            .is_ok_and(|body| body["error"] == "bad_verification_code"),
        RequestTokenError::Request(_) | RequestTokenError::Other(_) => false,
    }
}

async fn fetch_profile(state: &AppState, access_token: &str) -> Result<GitHubProfile, AppError> {
    let response = state
        .http
        .get(format!("{}/user", state.config.github.api_base))
        .bearer_auth(access_token)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "github profile request failed");
            AppError::BadGateway("github profile")
        })?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "github profile refused");
        return Err(AppError::BadGateway("github profile"));
    }
    // Parsed by hand: this crate's reqwest is built without its `json`
    // feature.
    let body = response.bytes().await.map_err(|e| {
        tracing::warn!(error = %e, "github profile body failed");
        AppError::BadGateway("github profile")
    })?;
    serde_json::from_slice(&body).map_err(|e| {
        tracing::warn!(error = %e, "github profile did not parse");
        AppError::BadGateway("github profile")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Form,
        http::HeaderMap,
        routing::{get, post},
    };
    use serde_json::{Value, json};
    use sqlx::PgPool;

    use crate::test_support;

    /// A stand-in for GitHub on a local port. Stateless: the code names the
    /// profile (`ok:<github id>:<login>`), the access token carries it on to
    /// `/user`, so each test picks its user by the code it sends. `bad` is
    /// GitHub's 200-with-error answer to a spent code; `down` is a 500.
    async fn fake_github() -> String {
        async fn access_token(Form(form): Form<Vec<(String, String)>>) -> Response {
            let field = |name: &str| {
                form.iter()
                    .find(|(k, _)| k == name)
                    .map(|(_, v)| v.as_str())
            };
            // The exchange must carry the secret in the body and repeat the
            // redirect URL, or real GitHub refuses it.
            assert_eq!(field("client_id"), Some("test-client-id"));
            assert_eq!(field("client_secret"), Some("test-client-secret"));
            assert_eq!(
                field("redirect_uri"),
                Some("http://localhost:1420/auth/callback")
            );
            match field("code").unwrap() {
                "down" => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                // What GitHub says to a server with the wrong client secret,
                // also as a 200.
                "misconfigured" => Json(json!({
                    "error": "incorrect_client_credentials",
                    "error_description": "The client_id and/or client_secret passed are incorrect."
                }))
                .into_response(),
                "bad" => Json(json!({
                    "error": "bad_verification_code",
                    "error_description": "The code passed is incorrect or expired."
                }))
                .into_response(),
                code => Json(json!({
                    "access_token": format!("gho_{code}"),
                    "token_type": "bearer",
                    "scope": ""
                }))
                .into_response(),
            }
        }
        async fn user(headers: HeaderMap) -> Response {
            let token = headers["authorization"].to_str().unwrap();
            let profile = token.strip_prefix("Bearer gho_ok:").unwrap();
            let (id, login) = profile.split_once(':').unwrap();
            Json(json!({
                "id": id.parse::<i64>().unwrap(),
                "login": login,
                "avatar_url": format!("https://avatars.example/{login}"),
                "email": "never-stored@example.com"
            }))
            .into_response()
        }

        let app = Router::new()
            .route("/login/oauth/access_token", post(access_token))
            .route("/user", get(user));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        base
    }

    async fn state_against(pool: PgPool, github: &str) -> AppState {
        let mut config = test_support::config();
        config.github.oauth_base = github.to_string();
        config.github.api_base = github.to_string();
        test_support::state(pool, config)
    }

    async fn callback(state: &AppState, code: &str) -> Result<SessionResponse, AppError> {
        github_callback(
            State(state.clone()),
            Query(CallbackParams {
                code: Some(code.to_string()),
            }),
        )
        .await
        .map(|Json(body)| body)
    }

    async fn status_of(result: Result<SessionResponse, AppError>) -> StatusCode {
        result.unwrap_err().into_response().status()
    }

    #[sqlx::test]
    async fn login_redirects_to_github_with_the_callers_state(pool: PgPool) {
        let state = state_against(pool, "https://github.example").await;
        let response = github_login(
            State(state),
            Query(LoginParams {
                state: Some("abcdefghijklmnop-_.~".to_string()),
            }),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        let location = reqwest::Url::parse(response.headers()[LOCATION].to_str().unwrap()).unwrap();
        assert_eq!(
            location.as_str().split('?').next().unwrap(),
            "https://github.example/login/oauth/authorize"
        );
        let query: Vec<(String, String)> = location.query_pairs().into_owned().collect();
        for expected in [
            ("response_type", "code"),
            ("client_id", "test-client-id"),
            ("state", "abcdefghijklmnop-_.~"),
            ("redirect_uri", "http://localhost:1420/auth/callback"),
        ] {
            assert!(
                query.contains(&(expected.0.to_string(), expected.1.to_string())),
                "{expected:?} missing from {query:?}"
            );
        }
    }

    #[sqlx::test]
    async fn login_refuses_a_missing_or_malformed_state(pool: PgPool) {
        let state = state_against(pool, "https://github.example").await;
        for bad in [
            None,
            Some("too-short"),
            Some("has spaces in it, sixteen+"),
            Some("quote\"breaks-out-of-it"),
        ] {
            let result = github_login(
                State(state.clone()),
                Query(LoginParams {
                    state: bad.map(str::to_string),
                }),
            )
            .await;
            assert_eq!(
                result.unwrap_err().into_response().status(),
                StatusCode::BAD_REQUEST,
                "{bad:?} was accepted"
            );
        }
    }

    #[sqlx::test]
    async fn a_first_login_creates_the_user_and_a_session(pool: PgPool) -> sqlx::Result<()> {
        let github = fake_github().await;
        let state = state_against(pool.clone(), &github).await;

        let session = callback(&state, "ok:42:alice").await.unwrap();

        let claims = auth::verify_jwt(test_support::JWT_SECRET, &session.token).unwrap();
        assert_eq!(claims.id.to_string(), session.user.id);
        assert_eq!(claims.role, Role::User);
        assert_eq!(session.user.login, "alice");
        assert_eq!(
            session.user.avatar_url.as_deref(),
            Some("https://avatars.example/alice")
        );

        let stored = sqlx::query_scalar!(
            "SELECT refresh_token_hash FROM user_sessions WHERE user_id = $1",
            claims.id
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            stored,
            vec![auth::hash_refresh_token(&session.refresh_token)]
        );
        Ok(())
    }

    #[sqlx::test]
    async fn the_response_is_shaped_as_the_contract_says(pool: PgPool) {
        let github = fake_github().await;
        let state = state_against(pool, &github).await;
        let session = callback(&state, "ok:42:alice").await.unwrap();

        let body: Value = serde_json::to_value(&session).unwrap();
        let mut keys: Vec<&str> = body
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["refreshToken", "token", "user"]);
        let mut user_keys: Vec<&str> = body["user"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        user_keys.sort_unstable();
        assert_eq!(user_keys, ["avatarUrl", "id", "login", "role"]);
        assert_eq!(body["user"]["role"], "user");
    }

    #[sqlx::test]
    async fn a_returning_user_keeps_their_role_and_their_other_sessions(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let github = fake_github().await;
        let state = state_against(pool.clone(), &github).await;
        let first = callback(&state, "ok:42:alice").await.unwrap();
        sqlx::query!("UPDATE users SET role = 'moderator' WHERE github_id = 42")
            .execute(&pool)
            .await?;

        // Renamed on GitHub, signing in from a second device.
        let second = callback(&state, "ok:42:alice-renamed").await.unwrap();

        assert_eq!(second.user.id, first.user.id);
        assert_eq!(second.user.login, "alice-renamed");
        assert_eq!(second.user.role, Role::Moderator);
        let sessions = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM user_sessions s
               JOIN users u ON u.id = s.user_id WHERE u.github_id = 42"#
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(sessions, 2, "the first device must stay signed in");
        Ok(())
    }

    #[sqlx::test]
    async fn signing_in_reactivates_a_tombstone_without_its_old_role(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let tombstone = sqlx::query_scalar!(
            "INSERT INTO users (github_id, deleted_at) VALUES (42, now()) RETURNING id"
        )
        .fetch_one(&pool)
        .await?;
        let github = fake_github().await;
        let state = state_against(pool.clone(), &github).await;

        let session = callback(&state, "ok:42:alice").await.unwrap();

        assert_eq!(session.user.id, tombstone.to_string(), "no second account");
        assert_eq!(session.user.role, Role::User);
        let row = sqlx::query!(
            "SELECT login, deleted_at FROM users WHERE id = $1",
            tombstone
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(row.login.as_deref(), Some("alice"));
        assert!(row.deleted_at.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn signing_in_prunes_only_expired_sessions(pool: PgPool) -> sqlx::Result<()> {
        let user = sqlx::query_scalar!(
            "INSERT INTO users (github_id, login) VALUES (42, 'alice') RETURNING id"
        )
        .fetch_one(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO user_sessions (user_id, refresh_token_hash, expires_at)
             VALUES ($1, 'expired', now() - interval '1 second'),
                    ($1, 'live', now() + interval '1 day')",
            user
        )
        .execute(&pool)
        .await?;
        let github = fake_github().await;
        let state = state_against(pool.clone(), &github).await;

        callback(&state, "ok:42:alice").await.unwrap();

        let mut hashes = sqlx::query_scalar!(
            "SELECT refresh_token_hash FROM user_sessions WHERE user_id = $1",
            user
        )
        .fetch_all(&pool)
        .await?;
        hashes.sort();
        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains(&"live".to_string()));
        assert!(!hashes.contains(&"expired".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn a_refused_code_is_400_and_a_failing_github_is_502(pool: PgPool) -> sqlx::Result<()> {
        let github = fake_github().await;
        let state = state_against(pool.clone(), &github).await;

        assert_eq!(
            status_of(callback(&state, "bad").await).await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status_of(callback(&state, "down").await).await,
            StatusCode::BAD_GATEWAY
        );
        // GitHub refusing this server's credentials is not the caller's fault.
        assert_eq!(
            status_of(callback(&state, "misconfigured").await).await,
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            status_of(callback(&state, "").await).await,
            StatusCode::BAD_REQUEST
        );

        let unreachable = state_against(pool.clone(), "http://127.0.0.1:9").await;
        assert_eq!(
            status_of(callback(&unreachable, "ok:42:alice").await).await,
            StatusCode::BAD_GATEWAY
        );

        let users = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM users"#)
            .fetch_one(&pool)
            .await?;
        assert_eq!(users, 0, "a failed login writes nothing");
        Ok(())
    }
}
