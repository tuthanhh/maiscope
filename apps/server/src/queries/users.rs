//! `users` and `user_sessions` (server-auth-github-oauth). Each function takes
//! a connection rather than the pool, so a login's upsert and its new session
//! commit together.

use sqlx::PgConnection;
use sqlx::types::Uuid;

use crate::types::UserRow;

/// Inserts a first-time GitHub identity, or refreshes a known one's profile.
/// A tombstoned row comes back to life here and only here: `deleted_at` is
/// cleared and `login`/`avatar_url` refilled. `role` is never written, so a
/// returning moderator stays one, and a reactivated tombstone is `user`
/// because deletion reset it.
pub async fn upsert_github_user(
    conn: &mut PgConnection,
    github_id: i64,
    login: &str,
    avatar_url: Option<&str>,
) -> Result<UserRow, sqlx::Error> {
    sqlx::query_as!(
        UserRow,
        r#"INSERT INTO users (github_id, login, avatar_url)
           VALUES ($1, $2, $3)
           ON CONFLICT (github_id) DO UPDATE
              SET login = EXCLUDED.login,
                  avatar_url = EXCLUDED.avatar_url,
                  deleted_at = NULL,
                  updated_at = now()
           RETURNING id, login AS "login!", avatar_url, role"#,
        github_id,
        login,
        avatar_url
    )
    .fetch_one(conn)
    .await
}

/// Opens a session for one device. The user's other sessions stay, except
/// those already past their expiry, which are pruned here: nothing else ever
/// deletes them, and a login is the natural moment.
pub async fn start_session(
    conn: &mut PgConnection,
    user_id: Uuid,
    refresh_token_hash: &str,
    ttl_days: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM user_sessions WHERE user_id = $1 AND expires_at <= now()",
        user_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "INSERT INTO user_sessions (user_id, refresh_token_hash, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
        user_id,
        refresh_token_hash,
        ttl_days
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}
