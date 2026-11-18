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
    body::Bytes,
    extract::{Query, State},
    http::{StatusCode, header::LOCATION},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet,
    EndpointSet, RedirectUrl, RequestTokenError, TokenResponse, TokenUrl,
};
use serde::Deserialize;

use crate::auth::{self, AuthUser, REFRESH_TOKEN_TTL_DAYS, Role};
use crate::config::GitHubConfig;
use crate::error::AppError;
use crate::queries;
use crate::state::AppState;
use crate::types::{SessionResponse, TokenPair, User, UserRow};

/// The two routes that reach GitHub. Split from [`session_router`] so the
/// caller can give them their own, tighter rate limit: each callback costs an
/// outbound exchange and a transaction.
pub fn github_router() -> Router<AppState> {
    Router::new()
        .route("/auth/github/login", get(github_login))
        .route("/auth/github/callback", get(github_callback))
}

pub fn session_router() -> Router<AppState> {
    Router::new()
        .route("/auth/refresh", post(refresh))
        .route("/auth/me", get(me).delete(delete_me))
}

/// Applied to every auth route: tokens and personal data must not be stored
/// by a browser or anything between (RFC 6749 §5.1 for token responses).
pub async fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
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
                // oauth2 displays a transport failure as only "Request failed";
                // the reqwest error says which (timeout, connect, TLS).
                match &e {
                    RequestTokenError::Request(cause) => {
                        tracing::warn!(error = %e, cause = %cause, "github token exchange failed")
                    }
                    _ => tracing::warn!(error = %e, "github token exchange failed"),
                }
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
    // Everything that can still fail happens before the commit, so a failure
    // rolls the session back rather than leaving one the client never got.
    let token = access_token(&state, user.id, role_from_db(&user.role)?)?;
    tx.commit().await?;

    Ok(Json(SessionResponse {
        token,
        refresh_token,
        user: user_from_row(user)?,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshBody {
    refresh_token: String,
}

// POST /auth/refresh — trade a refresh token for a new pair (rotation).
async fn refresh(State(state): State<AppState>, body: Bytes) -> Result<Json<TokenPair>, AppError> {
    // Parsed here rather than by axum's `Json`, whose rejection is plain text
    // rather than the contract's error shape.
    let RefreshBody { refresh_token } = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("body must be {\"refreshToken\": string}".into()))?;

    let new_refresh_token = auth::generate_refresh_token();
    let mut tx = state.pool.begin().await?;
    // Unknown, expired, already rotated, or a deleted account: one answer for
    // all of them. Each leaves the client the same move, signing in again,
    // and telling them apart would only inform someone holding a stolen one.
    let (user_id, role) = queries::rotate_session(
        &mut tx,
        &auth::hash_refresh_token(&refresh_token),
        &auth::hash_refresh_token(&new_refresh_token),
        REFRESH_TOKEN_TTL_DAYS,
    )
    .await?
    .ok_or(AppError::InvalidToken("invalid refresh token"))?;

    // As in the callback: the old token dies only if the new pair is ready.
    let token = access_token(&state, user_id, role_from_db(&role)?)?;
    tx.commit().await?;
    Ok(Json(TokenPair {
        token,
        refresh_token: new_refresh_token,
    }))
}

// GET /auth/me — the caller, read fresh from `users`.
async fn me(State(state): State<AppState>, caller: AuthUser) -> Result<Json<User>, AppError> {
    let mut conn = state.pool.acquire().await?;
    let row = queries::fetch_active_user(&mut conn, caller.id)
        .await?
        .ok_or(AppError::InvalidToken("account deleted"))?;
    Ok(Json(user_from_row(row)?))
}

// DELETE /auth/me — identity erasure, not erasure: the row stays, and so does
// everything the account published.
//
// Acts on the verified caller only; there is no id in the path to get wrong.
// The caller's outstanding JWT keeps verifying until it expires, up to 15
// minutes (production-v2, *Accepted, not solved*), but no session survives to
// refresh it, and `/auth/me` refuses it at once.
async fn delete_me(
    State(state): State<AppState>,
    caller: AuthUser,
) -> Result<StatusCode, AppError> {
    let mut tx = state.pool.begin().await?;
    queries::tombstone_user(&mut tx, caller.id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `users.role` as the enum. The column's `CHECK` makes a miss unreachable,
/// so one is a 500 and a log line, never a guessed role.
fn role_from_db(role: &str) -> Result<Role, AppError> {
    Role::from_db(role).ok_or(AppError::Internal("users.role out of range"))
}

fn access_token(
    state: &AppState,
    user_id: sqlx::types::Uuid,
    role: Role,
) -> Result<String, AppError> {
    auth::issue_jwt(state.config.jwt_secret.expose(), user_id, role)
        .map_err(|_| AppError::Internal("jwt encoding"))
}

fn user_from_row(row: UserRow) -> Result<User, AppError> {
    Ok(User {
        id: row.id.to_string(),
        role: role_from_db(&row.role)?,
        login: row.login,
        avatar_url: row.avatar_url,
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
            if token == "Bearer gho_noprofile" {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
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
    async fn signing_in_reactivates_a_tombstone_as_the_same_account(
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
    async fn a_failure_after_the_upsert_rolls_the_whole_login_back(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        // Make the session insert fail, after the user row is written.
        sqlx::query(
            "CREATE FUNCTION refuse() RETURNS trigger LANGUAGE plpgsql
             AS $$ BEGIN RAISE EXCEPTION 'refused'; END $$",
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TRIGGER refuse BEFORE INSERT ON user_sessions
             FOR EACH ROW EXECUTE FUNCTION refuse()",
        )
        .execute(&pool)
        .await?;
        let github = fake_github().await;
        let state = state_against(pool.clone(), &github).await;

        assert_eq!(
            status_of(callback(&state, "ok:42:alice").await).await,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        let users = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM users"#)
            .fetch_one(&pool)
            .await?;
        assert_eq!(users, 0, "the upsert must roll back with the session");
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

        // Code accepted, profile refused.
        assert_eq!(
            status_of(callback(&state, "noprofile").await).await,
            StatusCode::BAD_GATEWAY
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

#[cfg(test)]
mod session_tests {
    use super::*;
    use sqlx::PgPool;

    use crate::test_support;

    /// A live user with one session whose refresh token is `token`.
    async fn signed_in(pool: &PgPool, github_id: i64, token: &str) -> sqlx::types::Uuid {
        let user = sqlx::query_scalar!(
            "INSERT INTO users (github_id, login, avatar_url, role)
             VALUES ($1, 'alice', 'https://avatars.example/alice', 'moderator') RETURNING id",
            github_id
        )
        .fetch_one(pool)
        .await
        .unwrap();
        add_session(pool, user, token, "1 day").await;
        user
    }

    async fn add_session(pool: &PgPool, user: sqlx::types::Uuid, token: &str, lifetime: &str) {
        sqlx::query!(
            "INSERT INTO user_sessions (user_id, refresh_token_hash, expires_at)
             VALUES ($1, $2, now() + $3::text::interval)",
            user,
            auth::hash_refresh_token(token),
            lifetime
        )
        .execute(pool)
        .await
        .unwrap();
    }

    /// Polls until `n` backends in this test's database are waiting on a lock,
    /// for up to two seconds. Counting the whole database is safe because
    /// `#[sqlx::test]` gives every test its own.
    async fn lock_waiters_reach(pool: &PgPool, n: i64) -> sqlx::Result<bool> {
        for _ in 0..200 {
            let waiting = sqlx::query_scalar!(
                r#"SELECT count(*) AS "n!" FROM pg_stat_activity
                   WHERE datname = current_database() AND wait_event_type = 'Lock'"#
            )
            .fetch_one(pool)
            .await?;
            if waiting == n {
                return Ok(true);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok(false)
    }

    async fn refresh_with(pool: &PgPool, token: &str) -> Result<TokenPair, AppError> {
        let body = serde_json::json!({ "refreshToken": token }).to_string();
        refresh(
            State(test_support::state(pool.clone(), test_support::config())),
            Bytes::from(body),
        )
        .await
        .map(|Json(pair)| pair)
    }

    fn status(result: Result<impl std::fmt::Debug, AppError>) -> StatusCode {
        result.unwrap_err().into_response().status()
    }

    #[sqlx::test]
    async fn refresh_rotates_and_the_old_token_dies(pool: PgPool) -> sqlx::Result<()> {
        let user = signed_in(&pool, 42, "first").await;

        let pair = refresh_with(&pool, "first").await.unwrap();

        let claims = auth::verify_jwt(test_support::JWT_SECRET, &pair.token).unwrap();
        assert_eq!(claims.id, user);
        assert_eq!(claims.role, Role::Moderator, "role comes from the row");
        assert_eq!(
            status(refresh_with(&pool, "first").await),
            StatusCode::UNAUTHORIZED
        );
        assert!(refresh_with(&pool, &pair.refresh_token).await.is_ok());

        let days_left = sqlx::query_scalar!(
            r#"SELECT extract(day FROM expires_at - now())::int AS "d!"
               FROM user_sessions WHERE user_id = $1"#,
            user
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(
            days_left,
            REFRESH_TOKEN_TTL_DAYS - 1,
            "expiry restarts at 30 days"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn refresh_leaves_the_users_other_devices_alone(pool: PgPool) -> sqlx::Result<()> {
        let user = signed_in(&pool, 42, "phone").await;
        add_session(&pool, user, "desktop", "1 day").await;

        refresh_with(&pool, "phone").await.unwrap();

        assert!(refresh_with(&pool, "desktop").await.is_ok());
        Ok(())
    }

    #[sqlx::test]
    async fn two_concurrent_refreshes_with_one_token_yield_one_pair(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        signed_in(&pool, 42, "contested").await;

        // Force the overlap rather than hope for it: hold the session row's
        // lock, start both refreshes, and release only once Postgres shows
        // both waiting on it. Then they race for the row for real, and the
        // loser must take the re-check path.
        let mut holder = pool.begin().await?;
        sqlx::query!(
            "SELECT id FROM user_sessions WHERE refresh_token_hash = $1 FOR UPDATE",
            auth::hash_refresh_token("contested")
        )
        .fetch_one(&mut *holder)
        .await?;

        let racers: Vec<_> = (0..2)
            .map(|_| {
                let pool = pool.clone();
                tokio::spawn(async move { refresh_with(&pool, "contested").await })
            })
            .collect();

        assert!(
            lock_waiters_reach(&pool, 2).await?,
            "both refreshes must be blocked on the row"
        );
        holder.commit().await?;

        let mut outcomes = Vec::new();
        for racer in racers {
            outcomes.push(racer.await.unwrap());
        }
        let winners = outcomes.iter().filter(|r| r.is_ok()).count();
        assert_eq!(winners, 1, "a refresh token is single-use");
        let loser = outcomes.into_iter().find(Result::is_err).unwrap();
        assert_eq!(status(loser), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[sqlx::test]
    async fn refresh_refuses_expired_unknown_and_deleted(pool: PgPool) -> sqlx::Result<()> {
        let user = signed_in(&pool, 42, "live").await;
        add_session(&pool, user, "stale", "-1 second").await;
        assert_eq!(
            status(refresh_with(&pool, "stale").await),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status(refresh_with(&pool, "never-issued").await),
            StatusCode::UNAUTHORIZED
        );

        // A tombstone with a session left over (deletion removes them; this
        // proves refresh does not rely on that).
        sqlx::query!(
            "UPDATE users SET deleted_at = now(), login = NULL, avatar_url = NULL,
                              role = 'user' WHERE id = $1",
            user
        )
        .execute(&pool)
        .await?;
        assert_eq!(
            status(refresh_with(&pool, "live").await),
            StatusCode::UNAUTHORIZED
        );
        Ok(())
    }

    #[sqlx::test]
    async fn refresh_rejects_a_malformed_body_as_400(pool: PgPool) {
        for body in ["", "{}", "not json", r#"{"refreshToken": 42}"#] {
            let result = refresh(
                State(test_support::state(pool.clone(), test_support::config())),
                Bytes::from(body),
            )
            .await;
            assert_eq!(status(result), StatusCode::BAD_REQUEST, "{body:?}");
        }
    }

    #[sqlx::test]
    async fn me_returns_the_caller_as_stored(pool: PgPool) {
        let user = signed_in(&pool, 42, "t").await;
        let Json(me) = me(
            State(test_support::state(pool.clone(), test_support::config())),
            AuthUser {
                id: user,
                // A stale JWT role: `me` reports the row's.
                role: Role::User,
            },
        )
        .await
        .unwrap();
        assert_eq!(me.id, user.to_string());
        assert_eq!(me.login, "alice");
        assert_eq!(me.role, Role::Moderator);
    }

    #[sqlx::test]
    async fn me_refuses_a_deleted_account_while_its_jwt_still_verifies(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let user = sqlx::query_scalar!(
            "INSERT INTO users (github_id, deleted_at) VALUES (42, now()) RETURNING id"
        )
        .fetch_one(&pool)
        .await?;
        let result = me(
            State(test_support::state(pool.clone(), test_support::config())),
            AuthUser {
                id: user,
                role: Role::User,
            },
        )
        .await;
        assert_eq!(status(result), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    async fn delete(pool: &PgPool, user: sqlx::types::Uuid) -> StatusCode {
        delete_me(
            State(test_support::state(pool.clone(), test_support::config())),
            AuthUser {
                id: user,
                role: Role::Moderator,
            },
        )
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn deleting_strips_identity_and_sessions_and_keeps_the_row(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let user = signed_in(&pool, 42, "phone").await;
        add_session(&pool, user, "desktop", "1 day").await;

        assert_eq!(delete(&pool, user).await, StatusCode::NO_CONTENT);

        let row = sqlx::query!(
            "SELECT github_id, login, avatar_url, role, deleted_at FROM users WHERE id = $1",
            user
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(row.github_id, 42, "github_id is what a later sign-in finds");
        assert!(row.login.is_none() && row.avatar_url.is_none());
        assert_eq!(row.role, "user", "a moderator's privileges do not survive");
        assert!(row.deleted_at.is_some());

        for token in ["phone", "desktop"] {
            assert_eq!(
                status(refresh_with(&pool, token).await),
                StatusCode::UNAUTHORIZED,
                "{token} still refreshes"
            );
        }
        let me_after = me(
            State(test_support::state(pool.clone(), test_support::config())),
            AuthUser {
                id: user,
                role: Role::Moderator,
            },
        )
        .await;
        assert_eq!(status(me_after), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[sqlx::test]
    async fn deleting_twice_succeeds_and_keeps_the_first_timestamp(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let user = signed_in(&pool, 42, "t").await;
        delete(&pool, user).await;
        let first = sqlx::query_scalar!("SELECT deleted_at FROM users WHERE id = $1", user)
            .fetch_one(&pool)
            .await?;

        assert_eq!(delete(&pool, user).await, StatusCode::NO_CONTENT);

        let second = sqlx::query_scalar!("SELECT deleted_at FROM users WHERE id = $1", user)
            .fetch_one(&pool)
            .await?;
        assert_eq!(first, second);
        Ok(())
    }

    #[sqlx::test]
    async fn deleting_touches_no_other_account(pool: PgPool) -> sqlx::Result<()> {
        let user = signed_in(&pool, 42, "mine").await;
        let other = sqlx::query_scalar!(
            "INSERT INTO users (github_id, login) VALUES (43, 'bob') RETURNING id"
        )
        .fetch_one(&pool)
        .await?;
        add_session(&pool, other, "theirs", "1 day").await;

        delete(&pool, user).await;

        assert!(refresh_with(&pool, "theirs").await.is_ok());
        let login = sqlx::query_scalar!("SELECT login FROM users WHERE id = $1", other)
            .fetch_one(&pool)
            .await?;
        assert_eq!(login.as_deref(), Some("bob"));
        Ok(())
    }

    #[sqlx::test]
    async fn a_refresh_racing_a_deletion_cannot_outlive_it(pool: PgPool) -> sqlx::Result<()> {
        let user = signed_in(&pool, 42, "racing").await;

        // The deletion runs up to its commit and holds there, its `DELETE`
        // holding the session row.
        let mut deletion = pool.begin().await?;
        queries::tombstone_user(&mut deletion, user).await?;
        let racer = {
            let pool = pool.clone();
            tokio::spawn(async move { refresh_with(&pool, "racing").await })
        };
        assert!(
            lock_waiters_reach(&pool, 1).await?,
            "the refresh must be blocked on the deletion"
        );
        deletion.commit().await?;

        assert_eq!(status(racer.await.unwrap()), StatusCode::UNAUTHORIZED);
        let sessions = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM user_sessions WHERE user_id = $1"#,
            user
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(sessions, 0);
        Ok(())
    }

    #[sqlx::test]
    async fn a_repeat_deletion_changes_nothing(pool: PgPool) -> sqlx::Result<()> {
        let user = signed_in(&pool, 42, "t").await;
        delete(&pool, user).await;
        let before = sqlx::query!(
            "SELECT deleted_at, updated_at FROM users WHERE id = $1",
            user
        )
        .fetch_one(&pool)
        .await?;

        delete(&pool, user).await;

        let after = sqlx::query!(
            "SELECT deleted_at, updated_at FROM users WHERE id = $1",
            user
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(before.deleted_at, after.deleted_at);
        assert_eq!(before.updated_at, after.updated_at);
        Ok(())
    }
}
