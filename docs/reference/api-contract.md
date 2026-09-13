# maiscope API Contract (v1)

Defines the HTTP surface the global backend (`apps/server`, Rust/Axum/Postgres)
exposes so the frontend (`apps/host/src/`) can drop its static `data.json`
dependency: live catalog, per-sheet chart text for the visualizer, and a
revision-probe sync tier. Auth (§4) and contributions (§5) are phase 2.

Each section carries its implementation status:

- **shipped** — routed in `apps/server/src/main.rs` and covered by handler tests
- **phase 2** — designed, not built; no route exists. See
  [ROADMAP](../ROADMAP.md).

No audio crosses this API. Audio hosting was cut by
[ADR-0002](../adr/0002-chart-data-only-no-audio-hosting.md); v1 plays charts on the
engine's silent, wall-clock-driven path.

- Base URL: `/api/v1`
- Content type: `application/json; charset=utf-8` unless noted (binary endpoints).
- Cross-tier identity key: **`sheetExpr`** = `` `${songId}|${type}|${difficulty}` ``
  (see `utils/sheet.ts:computeSheetExpr`). Path-encode it; `|` → `%7C`.
- Timestamps: RFC 3339 / ISO 8601 UTC.
- Auth: `Authorization: Bearer <jwt>` where required. Public reads need no token.

The frontend rehydrates a frozen, prototype-linked object graph in
`utils/data.ts:preprocessData`. **The server returns raw fields only** — it does
NOT send derived fields (`songNo`, `imageUrl`, `imageUrlM`, `sheetExpr`,
`notePercents`, `$canonicalSheet`). The client computes those, as it does today.

---

## 1. Catalog — replaces static `data.json`  *(shipped)*

Fills: `stores/data.ts:loadData`, which previously fetched a flat `data.json`
from CloudFront.

### `GET /catalog`

Full snapshot. Shape is **byte-identical to today's `data.json`** so
`preprocessData` is unchanged. Drives the offline-first first paint.

Query params (all optional):
| param | type | meaning |
|-------|------|---------|
| `region` | string | filter sheets to a region (else all) |

Freshness is negotiated with `If-None-Match`, not a `since` param — see Headers
below.

Response `200`:
```jsonc
{
  "songs": [ /* Song[] — see §1.1 */ ],
  "categories": [{ "category": "string" }],
  "versions":   [{ "version": "string", "abbr?": "string", "releaseDate?": "YYYY-MM-DD" }],
  "types":      [{ "type": "string", "name": "string", "abbr?": "string", "iconUrl?": "string", "iconHeight?": 0 }],
  "difficulties": [{ "difficulty": "string", "name": "string", "color?": "string", "iconUrl?": "string", "iconHeight?": 0 }],
  "regions":    [{ "region": "string", "name": "string" }],
  "updateTime": "YYYY-MM-DD"   // bumped whenever any canonical row changes;
                               // "0000-00-00" before the first catalog sync
}
```
> `sheets` is intentionally absent — `preprocessData` derives it from
> `songs[].sheets`. Keep it that way.

Headers: `ETag: "<sha256(revision:updateTime)>"` — the same value
`GET /sync/manifest` reports as `catalogHash` (§3). A matching `If-None-Match`
returns `304 Not Modified` with an empty body, repeating the `ETag` and
`Cache-Control` (RFC 7232 §4.1 — the client needs them to refresh the
freshness of the copy it already holds). `Cache-Control: public,
max-age=3600` — the catalog only changes on a `bin/ingest` run, so an hour
of unconditional client-side caching trades a small staleness window for
skipping the network round-trip entirely within it (`server-restructure`
issue 07).

### 1.1 `Song` / `Sheet` payload shape

Matches `types/Song.ts` and `types/Sheet.ts` (raw fields, no derived):
```jsonc
// Song
{
  "songId": "string|null",
  "category?": "string",
  "title?": "string",
  "artist?": "string",
  "bpm?": 0,
  "imageName?": "string",        // client resolves to imageUrl via dataSourceUrl
  "version?": "string",
  "releaseDate?": "YYYY-MM-DD",
  "isNew?": false,
  "isLocked?": false,
  "comment?": "string",
  "sheets": [ /* Sheet[] */ ]
}
// Sheet
{
  "type?": "string",
  "difficulty?": "string",
  "level?": "string",
  "levelValue?": 0,
  "internalLevel?": "string",
  "internalLevelValue?": 0,
  "noteDesigner?": "string",
  "noteCounts?": { "total": 0, "tap": 0, "hold": 0, "slide": 0, "touch": 0, "break": 0 },
  "regions?": { "intl": true, "jp": true },
  "regionOverrides?": { "<region>": { /* partial Sheet */ } },
  "isSpecial?": false,
  "hasChart": false    // true if a chart row exists for this sheetExpr
}
```
`hasChart` is the only addition vs. today — it lets `songs.vue`/`song.vue` show a
"visualize" affordance without a probe request. It is server truth, not derived,
so it belongs in the payload.

### `GET /songs/{songId}`
Single song with its sheets (same shape as §1.1). `404` if unknown.

### `GET /sheets/{sheetExpr}`
Single sheet, `sheetExpr` URL-encoded. `404` if no match — mirrors the client's
`makeDummySheet` fallback path in `utils/sheet.ts`.

### 1.2 `GET /sheets/search`

Filtered, paginated sheet list — powers the browse page's search/filter UI.
Unlike `GET /catalog` (full snapshot for the offline-first cache), this
endpoint does the filtering in SQL and returns only a page of results.

Query params (all optional — omitting all returns the full unfiltered,
paginated sheet list):

| param | type | meaning |
|-------|------|---------|
| `title` | string | substring match on song title (case-insensitive), or exact match if `matchExactTitle` is set |
| `matchExactTitle` | boolean | see above |
| `artist` | string | same substring/exact behavior as `title`, on artist |
| `matchExactArtist` | boolean | see above |
| `categories` | string[] | matches if any of `sheet.category`'s `\|`-delimited parts is in this list |
| `versions` | string[] | exact match |
| `types` | string[] | exact match |
| `difficulties` | string[] | exact match |
| `minLevelValue` / `maxLevelValue` | number | inclusive range on level value (or internal level value if `useInternalLevel`) |
| `useInternalLevel` | boolean | see above |
| `minBPM` / `maxBPM` | number | inclusive range |
| `noteDesigners` | string[] | exact match |
| `region` | string | prefix `!` excludes; otherwise includes. When combined with `useRegionOverride`, the region's override values (level/internalLevel/noteDesigner) substitute for the base sheet's before other filters evaluate. A sheet with a region-specific override counts as belonging to that region even without a separate availability row |
| `useRegionOverride` | boolean | see above |
| `page` | integer | 1-indexed, default 1 |
| `pageSize` | integer | default 22, max 100 |

Response `200`:
```jsonc
{
  "sheets": [ /* Sheet[] — same shape as GET /sheets/{sheetExpr}, see §1.1 */ ],
  "total": 0   // total matches before pagination, for computing page count
}
```

No `superFilter` equivalent — the client-side arbitrary-JS filter was removed
from the app (never had UI wiring); revisit if/when the app needs it again.

---

## 2. Charts — feeds the visualizer  *(shipped)*

Fills: `pages/visualizer.vue` + `composables/useEngine.ts`. Without it a user
must paste simai text by hand. This endpoint lets the engine load a real chart
for any sheet via `loadChart(chart)`.

### `GET /sheets/{songId}/chart?type={type}&difficulty={difficulty}`

Returns the raw chart source for the engine parser as **`text/plain`** — not a
JSON envelope. The engine wants the simai text itself, and wrapping it would
only cost the client a parse and an unwrap.

The path segment is the **`songId` alone**, not the full `sheetExpr`; `type`
and `difficulty` are query params. The server rebuilds `sheet_expr` from the
three (`main.rs:get_chart`), so the cross-tier key is still what is matched —
it is just not URL-encoded into one segment here.

| status | meaning |
|--------|---------|
| `200` | body is the raw simai text |
| `404` | no chart row for this sheet (`hasChart` is false) |
| `501` | a chart row exists but holds no inline text (blob-only charts are not served yet) |

Errors from this endpoint are plain text, not the §6 JSON error shape.

> Client wiring: when `sheet.hasChart`, fetch `/chart` → chart text →
> `loadChart(chart)`. Manual paste in `visualizer.vue` stays as a fallback.
> No audio endpoint exists or is planned for v1 (ADR-0002); the engine's
> `loadSong(chart, audioBytes)` audio-slaved path is unused.

---

## 3. Sync — revision probe for the client cache  *(shipped)*

Fills the client-side catalog cache: load cache → check manifest → delta sync →
atomic swap.

The cache is an **IndexedDB blob plus this revision probe**, not a relational
store. Because the client holds the whole catalog in memory and filters locally
([ADR-0007](../adr/0007-client-side-filtering.md)), a local SQLite mirror would
buy no query benefit and would only translate rows back into the JSON shape
`preprocessData` already wants. The earlier `src-tauri`-driven SQLite design
went with Tauri ([ADR-0003](../adr/0003-web-pwa-drop-tauri.md)).

### `GET /sync/manifest`
Cheap freshness probe.
```jsonc
{
  "updateTime": "YYYY-MM-DD", // "0000-00-00" before the first catalog sync
  "revision": 0,            // monotonic; cache stores last seen; 0 before the first sync
  "catalogHash": "string", // ETag for GET /catalog
  "counts": { "songs": 0, "sheets": 0, "charts": 0 }
}
```
Headers: same `ETag`/`If-None-Match`/`304` pairing as `GET /catalog` (§1) —
including repeating both validators on the `304` — but
`Cache-Control: no-cache` instead of a `max-age` — this endpoint's whole job
is telling the client whether the catalog changed, so it always revalidates
against the server rather than trusting a local cache blindly. The `304`
still saves the round-trip cost of a full body.

### `GET /sync/delta?since={revision}`
Rows changed since `revision`. `tombstones` carry deletions.
```jsonc
{
  "revision": 0,
  "songs":  [ /* changed Song (with sheets) */ ],
  "charts": [], // always empty for now — chart-meta delta tracking is a follow-up
  "tombstones": { "songIds": ["..."], "sheetExprs": ["..."] }
}
```
If `since` is too old to diff, respond `409` with
`{ "error": "snapshot_required" }` → client refetches `GET /catalog`.

> Precisely: `since` is "too old" when it predates the revision of the last
> full `bin/ingest` reload (`ingest` fully truncates and reloads canonical
> tables, so there is no stable row identity to diff across that boundary).
> A `song` is included whenever it or any of its sheets changed since
> `since`, so a sheet-only edit still surfaces its parent song (the response
> nests sheets under `songs`, so there's no other way to represent it).

---

## 4. Auth — GitHub OAuth, roles user/moderator/admin  *(phase 2)*

**Not implemented.** No auth route is mounted; the design below is the plan of
record for `docs/work/server-auth-github-oauth/`. Required to gate §5.

v1 has no accounts and holds no PII — a property worth preserving deliberately
rather than losing by accident (ADR-0002).

- `GET /auth/github/login` → `302` to GitHub OAuth.
- `GET /auth/github/callback?code=...` → sets session / returns
  `{ "token": "<jwt>", "user": { ... } }`.
- `POST /auth/refresh` → new JWT from refresh token.
- `GET /auth/me` → current user:
```jsonc
{ "id": "string", "login": "string", "avatarUrl": "string",
  "role": "user|moderator|admin" }
```
`401` when no/invalid token.

---

## 5. Contributions — submit → pending → moderate → merge  *(phase 2)*

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
  `not_found`, `snapshot_required` (§3), `rate_limited`. `bad_request` exists
  as an `AppError` variant but no shipped endpoint returns it — every query
  param today either parses or is optional.
  Exception: `GET /sheets/{songId}/chart` returns plain-text errors (§2).
- **Pagination**: `?page` (1-based) + `?pageSize` (default 22, max 100). The
  total is a `total` **field in the response body**, not an `X-Total-Count`
  header — the only paginated endpoint (`GET /sheets/search`) returns an
  envelope already, so a header would be a second place to look.
- **Rate limits**: per-IP (keyed on `Fly-Client-IP`, not per-user — there is
  no auth yet), generous burst with a slow refill (`server-restructure` issue
  08). `429` + `Retry-After` (always at least `1` — the underlying limiter
  reports whole seconds and would otherwise say `0`, i.e. "retry now"), same
  `{ "error": "rate_limited", "message": ... }` shape as every other error.
  `/healthcheck` is exempt. Phase 2 write
  endpoints (§5) will need their own, tighter, per-user limits — this ticket
  only covers the shipped public reads.
- **CORS**: allowlist built from `CORS_ALLOWED_ORIGINS` (`Config`, `server-restructure`
  issue 06) — no origin configured means no origin allowed, not a permissive
  fallback. Local dev sets it to the Vite dev origin (`.env.example`).
  Egress control, not a security control: `curl` ignores CORS entirely, so
  it doesn't gate access to public read-only data — the real cap on abuse is
  `server-restructure` issue 08's rate limiting. There is no native proxy
  tier — the browser talks to this API directly.
- **Versioning**: breaking changes → `/api/v2`. Additive fields are non-breaking;
  clients ignore unknowns (frontend already tolerates extra keys).

## 7. Frontend wiring map

| Contract | Frontend touch-point |
|----------|---------------------|
| `GET /catalog` | `stores/data.ts:loadData` (replaced `data.json`) |
| `hasChart` | `pages/songs.vue`, `pages/song.vue`, `MvSheetDialog.vue` (show visualize affordance) |
| §2 chart | `pages/visualizer.vue`, `composables/useEngine.ts` (auto-load real charts) |
| §3 sync | IndexedDB catalog cache + manifest revision probe |
| §4 auth *(phase 2)* | new login UI, token in browser storage |
| §5 contributions *(phase 2)* | new contribution UI (web) |
