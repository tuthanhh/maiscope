# 02 — POST /contributions and GET /contributions, GET /contributions/{id}

**What to build:** `POST /contributions` (any authenticated user submits a proposal), `GET /contributions?status=&mine=` (moderators see all, plain users always restricted to their own regardless of the `mine` param — enforced server-side, not just a client filter), `GET /contributions/{id}` (same ownership rule, 404 not 403 to avoid leaking existence).

**Blocked by:** 01 — contributions tables migration, `server-auth-github-oauth/issues/02-jwt-and-authuser-extractor.md` (needs `AuthUser`).

**Status:** ready-for-agent

- [ ] `POST /contributions` inserts a row, returns `201` with `{ id, status: "pending" }`; invalid `kind` returns `400` not `500`
- [ ] `GET /contributions` — plain `user` role always restricted to own contributions server-side (not just an optional `mine` filter a caller could omit); moderators see all or filter by `mine`
- [ ] `GET /contributions/{id}` — plain `user` accessing someone else's contribution gets `404`, not `403`
- [ ] Manual smoke test: submit as a real user, list as that user, confirm ownership restriction holds
