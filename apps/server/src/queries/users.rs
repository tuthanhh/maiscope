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

/// Rotates one session's refresh token, if it is live: known, unexpired, and
/// belonging to an account that is not a tombstone. Returns the user it
/// belongs to, or `None` for any refusal.
///
/// One statement on purpose. Two refreshes racing with the same token both
/// reach the row; the second waits on the first's row lock, re-checks the
/// `WHERE` against the hash the first wrote, and matches nothing. That is
/// what makes a refresh token single-use without an explicit lock.
pub async fn rotate_session(
    conn: &mut PgConnection,
    old_hash: &str,
    new_hash: &str,
    ttl_days: i32,
) -> Result<Option<(Uuid, String)>, sqlx::Error> {
    let row = sqlx::query!(
        "UPDATE user_sessions s
            SET refresh_token_hash = $2,
                expires_at = now() + make_interval(days => $3),
                updated_at = now()
           FROM users u
          WHERE s.refresh_token_hash = $1
            AND s.expires_at > now()
            AND u.id = s.user_id
            AND u.deleted_at IS NULL
         RETURNING u.id, u.role",
        old_hash,
        new_hash,
        ttl_days
    )
    .fetch_optional(conn)
    .await?;
    Ok(row.map(|r| (r.id, r.role)))
}

/// A user as `/auth/me` shows them, or `None` for a tombstone: the JWT may
/// still be valid for minutes after a deletion, but this read is not.
pub async fn fetch_active_user(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as!(
        UserRow,
        r#"SELECT id, login AS "login!", avatar_url, role
           FROM users WHERE id = $1 AND deleted_at IS NULL"#,
        id
    )
    .fetch_optional(conn)
    .await
}

/// Turns an account into a tombstone (server-auth-github-oauth 06): identity
/// and privileges cleared, every session revoked, the row and `github_id`
/// kept so uploads stay attributed and a later sign-in finds it. Idempotent:
/// a second call rewrites the same tombstone and keeps the first
/// `deleted_at`.
pub async fn tombstone_user(conn: &mut PgConnection, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users
            SET deleted_at = COALESCE(deleted_at, now()),
                login = NULL,
                avatar_url = NULL,
                role = 'user',
                updated_at = now()
          WHERE id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM user_sessions WHERE user_id = $1", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
