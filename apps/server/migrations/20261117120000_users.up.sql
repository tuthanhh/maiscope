-- server-auth-github-oauth 01: GitHub identities, the attribution every write
-- endpoint needs.
--
-- `id` is a minted UUID v7 like `songs.id` (production-v2 rule 4): it is the
-- JWT `sub` and what an upload is attributed to, so it is not a sequence that
-- counts accounts.
--
-- No email and no password. `refresh_token_hash` is a SHA-256 hash of the
-- current refresh token, never the token: a leaked row must not be a working
-- credential. UNIQUE because a refresh presents only the token, so the hash is
-- the lookup key; NULLs do not collide.
--
-- Deletion is a tombstone (issue 01's comments): `login`, `avatar_url` and the
-- refresh token are nulled, `role` drops to `user` and `deleted_at` is set.
-- `github_id` is kept and never cleared, so re-authenticating reactivates this
-- row instead of minting a second account.
--
-- No trigger maintains `updated_at`; every UPDATE sets it, as elsewhere in
-- this schema.
CREATE TABLE users (
    id                       UUID PRIMARY KEY DEFAULT uuidv7(),
    github_id                BIGINT NOT NULL UNIQUE,
    login                    TEXT,
    avatar_url               TEXT,
    role                     TEXT NOT NULL DEFAULT 'user'
                             CHECK (role IN ('user', 'moderator', 'admin')),
    refresh_token_hash       TEXT UNIQUE,
    refresh_token_expires_at TIMESTAMPTZ,
    deleted_at               TIMESTAMPTZ,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- A token without an expiry would be a credential that never lapses.
    CONSTRAINT users_refresh_token_has_expiry
        CHECK ((refresh_token_hash IS NULL) = (refresh_token_expires_at IS NULL)),
    -- An active account has a login; a tombstone has no identity, no session
    -- and no privileges. Login is the only path back, and it refills `login`.
    CONSTRAINT users_tombstone_shape CHECK (
        CASE WHEN deleted_at IS NULL THEN login IS NOT NULL
             ELSE login IS NULL AND avatar_url IS NULL
                  AND refresh_token_hash IS NULL AND role = 'user'
        END
    )
);
