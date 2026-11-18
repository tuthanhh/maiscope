# maiscope API Contract

The HTTP surface `apps/server` (Rust/Axum/Postgres) exposes to the frontend
(`apps/host/src/`): the official Catalog and its reference tables, one song,
one sheet, a sheet's chart text, the freshness probe (§0), and accounts (§4).
§5 is superseded by
[ADR-0013](../adr/0013-community-charts-beside-the-catalog.md).

Each section carries its implementation status:

- **shipped** — routed in `apps/server/src/routes/` and covered by handler tests
- **phase 2** — designed, not built; no route exists. See
  [ROADMAP](../ROADMAP.md).

No audio crosses this API. Audio hosting was cut by
[ADR-0002](../adr/0002-chart-data-only-no-audio-hosting.md); the engine plays
charts on its silent, wall-clock-driven path.

- Base URL: `/api`, with no version segment (`api-rewrite`; §6 *Versioning*).
- Content type: `application/json; charset=utf-8` unless noted (the chart text
  is `text/plain`).
- Identity: a song is its minted UUID; a sheet is that id plus its type and
  difficulty code — `/api/songs/{id}/sheets/{type}/{code}`
  ([ADR-0026](../adr/0026-v2-api-carries-codes-and-read-fields-only.md)).
  Nothing is addressed by a title.
- Timestamps: RFC 3339 / ISO 8601 UTC.
- Auth: `Authorization: Bearer <jwt>` where required. Public reads need no token.

**The server returns raw fields a screen reads, nothing else.** The client
derives `imageUrl` and its graph references (`sheet.song`, `sheet.key`) in
`utils/data.ts`, and resolves names and colours through the reference tables.

---

## 0. Reads — `/api`  *(shipped)*

Authored from scratch (`api-rewrite`). Payloads carry raw fields that a
screen reads and nothing else, and name a difficulty by its `code`
([ADR-0026](../adr/0026-v2-api-carries-codes-and-read-fields-only.md)); the
client resolves names and colours through the reference tables and derives
`imageUrl` itself. Errors use the §6 envelope.

### `GET /api/healthcheck`

`{ "status": "good" }`. Outside every rate limiter.

### `GET /api/reference`

The five reference tables, each in its table's order — the boot fetch that
lets any screen resolve names without the Catalog
([ADR-0021](../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md)):

```jsonc
{ "categories":   [{ "category": "POPS＆アニメ" }],
  "versions":     [{ "version": "MAGiCAL", "abbr": "MAGiCAL" }],
  "types":        [{ "type": "dx", "name": "DX（でらっくす）", "abbr": "DX" }],
  "difficulties": [{ "code": "master", "name": "MASTER", "color": "#9e45e2" },
                   { "code": "協", "name": "【協】", "color": null }],
  "regions":      [{ "region": "jp", "name": "日本版" }] }
```

`abbr` and `color` are omitted when absent, as every absent v2 field is. The
tables change only with the Catalog, so
they share its `ETag` (`304` on `If-None-Match`) and its
`Cache-Control: public, max-age=3600`.

### `GET /api/catalog`

The whole official library — the snapshot browse filters in memory — with the
five reference arrays of `GET /api/reference` beside `songs`:

```jsonc
{ "songs": [{
    "id": "0195f3c2-…",            // the song's UUID; the only song key
    "category": "POPS＆アニメ", "title": "…", "artist": "…", "bpm": 150,
    "imageName": "….png", "version": "MAGiCAL", "releaseDate": "2024-01-02",
    "isLocked": false, "comment": "…",
    "sheets": [{
      "type": "dx", "difficulty": "master",   // difficulties.code; UTAGE: "協"
      "level": "13+", "levelValue": 13.7,
      "internalLevel": "13.8", "internalLevelValue": 13.8,
      "noteDesigner": "…",
      "noteCounts": { "total": 900, "tap": 500, … },
      "regions": { "jp": true, "intl": false },
      "regionOverrides": { "intl": { "level": "13", "levelValue": 13.0 } },
      "hasChart": true }] }],
  "categories": […], "versions": […], "types": […],
  "difficulties": […], "regions": […] }
```

Songs and sheets are in upstream order; songs and sheets upstream no longer
lists are not served (they keep their rows and come back if upstream does —
ADR-0026). Absent fields are omitted, not `null`;
`hasChart` is always present. Inside `noteCounts` a key may be `null`: the
count is unknown for that note kind, which a screen shows differently from
`0`. The songs, sheets, reference tables and `ETag` come from one database
snapshot, so they always agree. Not sent: `songId`, `isNew`, `isSpecial`,
`updateTime`, and every field the client derives (`imageUrl`, …). No
`?region=` parameter: region filtering is the client's. `ETag`, `304` and
`Cache-Control: public, max-age=3600`.

### `GET /api/songs/{id}`

One song with its sheets, shaped exactly as an element of the Catalog's
`songs` — the song modal opened by a shared link, without the snapshot.
`400 bad_request` when `{id}` is not a UUID in its canonical spelling —
lowercase, hyphenated — so a song has one URL; `404 not_found` for an
unknown song.

### `GET /api/songs/{id}/sheets/{type}/{difficulty}`

One sheet: its own fields as in the Catalog, plus its song under `song`
(the song's fields, without `sheets`):

```jsonc
{ "type": "utage", "difficulty": "協", "level": "13?", …, "hasChart": true,
  "song": { "id": "0195f3c2-…", "title": "…", "artist": "…", … } }
```

`{type}` is `std`, `dx` or `utage`; `{difficulty}` is `difficulties.code`,
percent-encoded where it is not ASCII (`/sheets/utage/%E5%8D%94`) and
case-sensitive (`X` is not `x`). The bracketed name (`【協】`) is not an
address. `400` for a malformed id; `404` for an unknown song, type or code —
the lookup is the validation.

### `GET /api/songs/{id}/sheets/{type}/{difficulty}/chart`

The sheet's simai text, `200 text/plain; charset=utf-8`, for the engine.
Errors are **plain text** too — the engine reads this body as text whatever
the status — but they are the same errors as everywhere else, with the same
statuses and the same redaction: `400` malformed id, `404` no such sheet
(`sheet '…' not found`) or no chart for it (`chart '…' not found`), `501` a
chart row without inline text, `500` with the body `internal error` and
nothing more. v1's handler put a database error's own text in the body; this
one cannot. The body is the message alone — no `error` code — so the `501`
reads `chart for '…' has no inline simai content`.

### `GET /api/sync/manifest`

The freshness probe: `{ "revision": 42 }`, `0` before the first sync. The
client compares it with the revision its cached Catalog was fetched under;
nothing else is sent (v1's `updateTime`, `catalogHash` and `counts` had no
reader). `Cache-Control: no-cache`, `ETag` and `304` as the Catalog. There is no
`/api/sync/delta`
([ADR-0026](../adr/0026-v2-api-carries-codes-and-read-fields-only.md)): a
changed revision means refetching the whole Catalog. **Refetch with
`cache: 'no-cache'`** (and the reference tables likewise): the Catalog is
`max-age=3600`, so a plain fetch can be answered from the browser's cache with
the previous revision's body for up to an hour. The `ETag`'s `304` saves bytes
only on a repeat visit at an unchanged revision.

The manifest, the Catalog and the reference tables send the same `ETag`
value — one validator for one Catalog revision. Caches key by URL, so this is
not an identity; do not compare it across routes.

Under `/api/…`, a path that matches no route is `404 not_found` and a route
asked with a method it does not serve is `405 method_not_allowed`, both in
the §6 envelope; a path or parameter that cannot be parsed is
`400 bad_request`. `/api/v1/…` is gone: its paths are misses like any other.

---

## 1–3. Removed: `/api/v1`

`/api/v1` — the Catalog keyed by `sheetExpr`, `GET /sheets/search`, the v1
chart route and `/sync/delta` — was deleted by
[`api-rewrite` 07](../work/api-rewrite/issues/07-delete-v1.md) on 2026-11-18.
§0 is the API. The numbering of §4–§7 is kept so references to them stay
valid; the v1 text is in this file's history.

---

## 4. Auth — GitHub OAuth, roles user/moderator/admin  *(phase 2)*

**Implemented on the v2 branch** (`docs/work/server-auth-github-oauth/`); no
screen calls it yet (`frontend-redesign` 05). Required to gate §5.

The API before v2 had no accounts and held no PII. v2 ends that deliberately rather than by
accident: the stored set is `github_id`, `login`, `avatar_url`, `role`, a
refresh-token **hash** per signed-in device, and timestamps. **No email address**, and adding one
later is a decision about privacy obligations rather than a schema convenience.

**Sign-in flow.** GitHub sends the browser back to the frontend, not to this
API: a JSON body returned to a navigation on the API's origin is unreadable
by the SPA on another domain.

1. The SPA generates a random `state`, keeps it in `sessionStorage`, and
   navigates to `GET /api/auth/github/login?state=<state>`.
2. The API answers `302` to GitHub's consent page, carrying that `state` and
   the frontend's `/auth/callback` as `redirect_uri`. No scopes are requested.
3. GitHub sends the browser to `/auth/callback?code=…&state=…` on the
   frontend, which compares `state` with its stored copy and stops on a
   mismatch. That check is the login-CSRF defence; the API keeps no OAuth
   state.
4. The SPA calls `GET /api/auth/github/callback?code=…`.

- `GET /api/auth/github/login?state=…` → `302` to GitHub. `state` must be
  16–128 characters from `A–Z a–z 0–9 - . _ ~`; otherwise `400 bad_request`.
- `GET /api/auth/github/callback?code=...` → exchanges the code and returns
  `{ "token": "<jwt>", "refreshToken": "<opaque>", "user": { ... } }`, the
  user shaped as in `GET /api/auth/me` below. Each call opens a new session:
  the caller's other devices stay signed in. Re-authenticating a deleted
  account **reactivates** it (see below), as `user`.
  `400 bad_request` for a missing code or one GitHub refuses (bad, expired or
  already used); `502 upstream_error` when GitHub cannot be reached or answers
  unexpectedly. A failed callback writes nothing.
- `POST /api/auth/refresh` → `{ "refreshToken": "<opaque>" }` in,
  `{ "token": "<jwt>", "refreshToken": "<opaque>" }` out. Refresh tokens rotate
  on every use: the presented one stops working as the new one is issued, and
  of two concurrent refreshes with one token exactly one succeeds. A token
  lives 30 days from issue or last rotation. Rotation touches only the
  caller's own session; the user's other devices are unaffected.
  `401` (`invalid refresh token`) for a token that is unknown, expired,
  already rotated, or belongs to a deleted account — one answer for all four;
  `400 bad_request` for a body that is not `{ "refreshToken": string }`.
- `GET /api/auth/me` → current user, read from the database, so `role` is
  current even if the JWT's is not:
```jsonc
{ "id": "string", "login": "string", "avatarUrl": "string|null",
  "role": "user|moderator|admin" }
```
`401` when no/invalid token, or when the account has been deleted — `/me`
reads the row, so it sees a deletion at once, unlike routes that need only
the JWT.

- `DELETE /api/auth/me` → `204`, identity removed from the caller's account
  (see below). Acts on the token's user only. Repeating it while the token
  still verifies is `204` and changes nothing; once that token expires there
  is no way to obtain another for a deleted account, so it is `401`. `401`
  without a valid token.

**401 responses.** Body `{ "error": "unauthorized", "message": … }` with a
`WWW-Authenticate` challenge (RFC 6750 §3.1): bare `Bearer` and the message
`missing bearer token` when no credential was sent; `Bearer
error="invalid_token"` when one was refused, with the message `token expired`
(refresh and retry; if the refresh is refused too, sign in again),
`invalid token`, `invalid refresh token` or `account deleted` (sign in
again).
**Every `/api/auth/*` response carries `Cache-Control: no-store`** — tokens
and personal data are not for a browser cache or anything between (RFC 6749
§5.1). The request log records the path only, never the query string: a
callback's `code` could still be exchanged by anyone reading it.

**Tokens live in client storage, not a cookie.** The frontend and API are on
different registrable domains (`pages.dev` and `fly.dev` are both Public Suffix
List entries), so an API-set cookie is third-party and browsers block it. Hence
`Authorization: Bearer` and an opaque refresh token the client stores itself.

**Deleting an account strips identity; it does not delete published work.** The
row is kept with `login` and `avatar_url` nulled, `role` reset to `user`,
`deleted_at` set and every session revoked. `github_id` is retained so a later login reaches the same
row rather than minting a second one. Community uploads stay published,
attributed to a deleted user. Describe it to users as identity removal, not
deletion.

**The JWT is authoritative for its 15-minute lifetime.** `AuthUser` verifies the
token without reading the database, so a role change or a deletion takes up to
one token lifetime to take effect.

---

## 5. Contributions — submit → pending → moderate → merge  *(phase 2, superseded)*

> **Superseded, not yet rewritten.** This section describes a moderation queue
> that merges proposals into the canonical tables.
> [ADR-0013](../adr/0013-community-charts-beside-the-catalog.md) replaced it with
> a separate community library: `/community/*`, published on upload, reports instead
> of approvals, and no writes to `songs`/`sheets`/`charts` at all. The plan of
> record is now [`community-charts`](../work/community-charts/spec.md); this
> section is rewritten by its ticket 07, when there is shipped behaviour to
> describe. Read what follows as history.
>
> §4 (auth) is unaffected and remains a prerequisite.

**Not implemented.** No contribution route is mounted; the design below is the
plan of record for `docs/work/server-contributions/`. No direct writes to
canonical; everything goes through a review queue. Needs auth (§4).

### `POST /contributions`  *(role: user+)*
Propose a new sheet, a chart, or an edit.
```jsonc
{
  "kind": "song|sheet|chart|edit",
  "sheetExpr": "string|null",   // target for chart/edit; null for new song
  "payload": { /* Song | Sheet | { format, chart } */ },
  "note": "string"              // contributor message to moderators
}
```
`201` → `{ "id": "string", "status": "pending" }`.

> **Audio upload is struck.** An earlier draft specified
> `POST /contributions/audio` returning a presigned PUT to S3-compatible
> storage. [ADR-0002](../adr/0002-chart-data-only-no-audio-hosting.md) cut it:
> the cost is not storage but moderation — a "community chart" is the obvious
> route for laundering official audio, and the only defence is a human
> reviewing every upload. No object storage enters the stack. Contributor
> audio, if it ever returns, needs its own decision record.

### `GET /contributions?status=&mine=`  *(user sees own; moderator+ sees all)*
List with `{ id, kind, sheetExpr, status, author, createdAt }[]`, paginated the
same way as §1.2 — `?page=&pageSize=` with a `total` field in the envelope.

### `GET /contributions/{id}`
Full record incl. `payload`, `diff`, review history.

### `POST /contributions/{id}/approve`  *(role: moderator+)*
Merges into canonical, bumps `updateTime`/`revision`. `200` → updated record.

### `POST /contributions/{id}/reject`  *(role: moderator+)*
Body `{ "reason": "string" }`. `200` → updated record.

Status enum: `pending | approved | rejected | merged`.

---

## 6. Conventions

- **Errors** (non-2xx): `{ "error": "snake_case_code", "message": "human text" }`.
  Codes in use: `database_error`, `internal_error` (both `500`, both with an
  opaque message — the real cause is logged server-side, never returned),
  `not_found`, `bad_request`, `method_not_allowed`, `unauthorized`,
  `rate_limited`, and `upstream_error` (`502`, opaque, for a failing GitHub —
  §4).
  Exception: the chart endpoint (`…/chart`, §0) returns plain-text errors —
  `AppError`s rendered as text: same status, same redaction, body the message
  alone.
  `not_implemented` (`501`) is the code a chart without inline text would
  carry in the envelope; the chart endpoint's text body omits it.
- **Pagination**: no route pages today; `community-charts` browse is the
  first. It takes `?page` (1-based) + `?pageSize` and returns the total as an
  **`X-Total-Count` response header**
  ([ADR-0015](../adr/0015-pagination-total-as-a-response-header.md)), which
  must then be added to `Access-Control-Expose-Headers` — or a cross-origin
  browser cannot read it, silently. The CORS layer exposes nothing until then.
- **Rate limits**: per-IP (keyed on the client IP from the configured
  trusted headers — `Fly-Client-IP` on Fly, the socket address elsewhere,
  [ADR-0019](../adr/0019-trusted-ip-headers-are-configuration.md) — not
  per-user; there is no auth yet), generous burst with a slow refill
  (`server-restructure` issue 08). `429` + `Retry-After` (always at least `1` — the underlying limiter
  reports whole seconds and would otherwise say `0`, i.e. "retry now"), same
  `{ "error": "rate_limited", "message": ... }` shape as every other error.
  `/healthcheck` is exempt. The two GitHub sign-in routes have their own,
  tighter bucket (burst 10, then one per 6 s), since each callback is an
  outbound exchange and a transaction; refresh and `me` share the main one. Phase 2 write endpoints (§5) reuse this same
  limiter: a per-user quota was considered and **deferred**, so the only
  write-specific bound is a 512 KB body limit on the upload route (see
  `production-v2`'s *Accepted, not solved*).
- **CORS**: allowlist built from `CORS_ALLOWED_ORIGINS` (`Config`, `server-restructure`
  issue 06) — no origin configured means no origin allowed, not a permissive
  fallback. `.env.example` names the Vite dev origin, `http://localhost:1420`;
  `apps/host/vite.config.ts` pins that port because it is allowlisted.
  Methods `GET`, `POST`, `DELETE`; request headers `Authorization` and
  `Content-Type`, which an authenticated or JSON request needs to pass the
  browser's preflight. No `Access-Control-Allow-Credentials`: credentials
  travel in a header the page sets, never in cookies.
  Egress control, not a security control: `curl` ignores CORS entirely, so
  it doesn't gate access to public read-only data — the real cap on abuse is
  `server-restructure` issue 08's rate limiting. There is no native proxy
  tier — the browser talks to this API directly.
- **Versioning**: *retired.* This rule said breaking changes go to `/api/v2`.
  With one first-party consumer and no second API ever served, the version
  segment named a distinction that did not exist —
  [`api-rewrite`](../work/api-rewrite/spec.md) drops it and cuts over on a
  branch instead. Additive fields remain non-breaking; clients ignore unknowns.

## 7. Frontend wiring map

| Contract | Frontend touch-point |
|----------|---------------------|
| `GET /catalog` | `stores/data.ts:loadData`, behind the IndexedDB cache |
| `GET /reference` | `stores/data.ts:loadReference`, at boot; names and colours before or without the Catalog |
| `GET /songs/{id}/sheets/{type}/{code}` | `composables/useEngine.ts:fetchSheetInfo` (the chart page's title block) |
| `GET /songs/{id}` | `components/dialogs/SongModal.vue` (a shared `?song=` link, before the Catalog loads) |
| `hasChart` | `components/dialogs/SongModal.vue` (Play is enabled only with a chart) |
| `GET /songs/{id}/sheets/{type}/{code}/chart` | `composables/useEngine.ts:fetchChart`, through `useChartLoader` (the chart page's deep link) |
| `GET /sync/manifest` | `stores/data.ts:loadData`: the revision probe before refetching the Catalog |
| §4 auth | not called yet; the sign-in UI is `frontend-redesign` 05 |
| §5 contributions *(superseded)* | replaced by the community library — [ADR-0013](../adr/0013-community-charts-beside-the-catalog.md) |
