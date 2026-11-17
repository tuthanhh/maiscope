-- server-auth-github-oauth 01: GitHub identities, the attribution every write
-- endpoint needs, and the sessions that keep them signed in.
--
-- `id` is a minted UUID v7 like `songs.id` (production-v2 rule 4): it is the
-- JWT `sub` and what an upload is attributed to, so it is not a sequence that
-- counts accounts.
--
-- No email and no password. Deletion is a tombstone (issue 01's comments):
-- `login` and `avatar_url` are nulled, `role` drops to `user`, `deleted_at` is
-- set and every session row goes. `github_id` is kept and never cleared, so
-- re-authenticating reactivates this row instead of minting a second account.
--
-- No trigger maintains `updated_at`; every UPDATE sets it, as elsewhere in
-- this schema.
CREATE TABLE users (
    id         UUID PRIMARY KEY DEFAULT uuidv7(),
    github_id  BIGINT NOT NULL UNIQUE,
    login      TEXT,
    avatar_url TEXT,
    role       TEXT NOT NULL DEFAULT 'user'
               CHECK (role IN ('user', 'moderator', 'admin')),
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- An active account has a login; a tombstone has no identity and no
    -- privileges. Login is the only path back, and it refills `login`.
    CONSTRAINT users_tombstone_shape CHECK (
        CASE WHEN deleted_at IS NULL THEN login IS NOT NULL
             ELSE login IS NULL AND avatar_url IS NULL AND role = 'user'
        END
    )
);

-- One row per signed-in device, so a phone and a desktop each hold their own
-- refresh token (maintainer decision, issue 01's comments). A refresh rotates
-- its own row only.
--
-- `refresh_token_hash` is a SHA-256 hash of the token, never the token: a
-- leaked row must not be a working credential. UNIQUE because a refresh
-- presents only the token, so the hash is the lookup key. `expires_at` is
-- NOT NULL because rotation alone bounds nothing: a stolen token that is
-- never used is never rotated.
CREATE TABLE user_sessions (
    id                 UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id            UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    refresh_token_hash TEXT NOT NULL UNIQUE,
    expires_at         TIMESTAMPTZ NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX user_sessions_user_idx ON user_sessions (user_id);
