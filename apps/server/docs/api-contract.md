# maiscope API Contract (v1)

Status: **proposed**. Defines the HTTP surface the global backend (`apps/server`,
Rust/Axum/Postgres) must expose so the frontend (`apps/host/src/`) can drop its
static `data.json` dependency and gain the features it has stubs for but no
backend: live catalog, per-sheet chart/audio for the visualizer, offline-first
delta sync, and open community contributions with auth.

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

## 1. Catalog — replaces static `data.json`

Fills: `stores/data.ts:loadData` and tauri `load_chart_data`, which today fetch a
flat `data.json` from CloudFront.

### `GET /catalog`

Full snapshot. Shape is **byte-identical to today's `data.json`** so
`preprocessData` is unchanged. Drives the offline-first first paint.

Query params (all optional):
| param | type | meaning |
|-------|------|---------|
| `region` | string | filter sheets to a region (else all) |
| `since` | RFC3339 | if set, server MAY 304 when nothing newer (see §3) |

Response `200`:
```jsonc
{
  "songs": [ /* Song[] — see §1.1 */ ],
  "categories": [{ "category": "string" }],
  "versions":   [{ "version": "string", "abbr?": "string", "releaseDate?": "YYYY-MM-DD" }],
  "types":      [{ "type": "string", "name": "string", "abbr?": "string", "iconUrl?": "string", "iconHeight?": 0 }],
  "difficulties": [{ "difficulty": "string", "name": "string", "color?": "string", "iconUrl?": "string", "iconHeight?": 0 }],
  "regions":    [{ "region": "string", "name": "string" }],
  "updateTime": "YYYY-MM-DD"   // bumped whenever any canonical row changes
}
```
> `sheets` is intentionally absent — `preprocessData` derives it from
> `songs[].sheets`. Keep it that way.

Headers: `ETag: "<updateTime-or-hash>"`, `Cache-Control: public, max-age=...`.
Honors `If-None-Match` → `304 Not Modified`.

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
  "hasChart": false,   // NEW: true if §2 chart/audio exists for this sheetExpr
  "hasAudio": false    // NEW
}
```
`hasChart`/`hasAudio` are the only additions vs. today — they let `songs.vue`/
`song.vue` show a "visualize" affordance without a probe request. They are server
truth, not derived, so they belong in the payload.

### `GET /songs/{songId}`
Single song with its sheets (same shape as §1.1). `404` if unknown.

### `GET /sheets/{sheetExpr}`
Single sheet, `sheetExpr` URL-encoded. `404` if no match — mirrors the client's
`makeDummySheet` fallback path in `utils/sheet.ts`.

---

## 2. Charts & audio — feeds the visualizer

Fills: `pages/visualizer.vue` + `composables/useEngine.ts`. Today a user must
paste simai text and pick an audio file by hand. These endpoints let the engine
load a real chart for any sheet via `loadSong(chart, audioBytes)` /
`loadChart(chart)`.

### `GET /sheets/{sheetExpr}/chart`
Returns the chart source for the engine parser.
```jsonc
{
  "sheetExpr": "string",
  "format": "simai",          // first target format; "ma2" reserved
  "chart": "string",          // raw simai text → loadChart()/loadSong() arg
  "audioUrl": "string|null",  // null if no audio (copyright); → GET below
  "version": 1,               // chart revision, bumps on re-merge
  "updatedAt": "RFC3339"
}
```
`404` if `hasChart` is false.

### `GET /sheets/{sheetExpr}/audio`
Binary audio stream for `loadSong`'s `Uint8Array` arg.
- `200` body = raw bytes, `Content-Type: audio/*`, supports `Range`.
- May `302` to S3-compatible storage (R2/B2) instead of streaming.
- `404` / `451` if unavailable (not hosted / copyright-withheld).

> Client wiring: when `sheet.hasChart`, fetch `/chart`; if `audioUrl`, fetch
> `/audio` → bytes → `loadSong(chart, bytes)`, else `loadChart(chart)`. Manual
> paste in `visualizer.vue` stays as a fallback.

---

## 3. Sync — offline-first local SQLite cache

Fills the planned tier-2 cache (driven from `src-tauri/`): load cache → check
manifest → delta sync → atomic swap. No frontend code yet; this is its contract.

### `GET /sync/manifest`
Cheap freshness probe.
```jsonc
{
  "updateTime": "YYYY-MM-DD",
  "revision": 0,            // monotonic; cache stores last seen
  "catalogHash": "string", // ETag for GET /catalog
  "counts": { "songs": 0, "sheets": 0, "charts": 0 }
}
```

### `GET /sync/delta?since={revision}`
Rows changed since `revision`. `tombstones` carry deletions.
```jsonc
{
  "revision": 0,
  "songs":  [ /* changed Song (with sheets) */ ],
  "charts": [ /* changed chart meta, see §2 (no chart body) */ ],
  "tombstones": { "songIds": ["..."], "sheetExprs": ["..."] }
}
```
If `since` is too old to diff, respond `409` with
`{ "error": "snapshot_required" }` → client refetches `GET /catalog`.

---

## 4. Auth — GitHub OAuth, roles user/moderator/admin

Fills: nothing in `src/` today (no login). Required to gate §5.

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

## 5. Contributions — submit → pending → moderate → merge

Fills: the open-contribution flow. No direct writes to canonical; everything goes
through a review queue. Needs auth (§4).

### `POST /contributions`  *(role: user+)*
Propose a new sheet, a chart, or an edit.
```jsonc
{
  "kind": "song|sheet|chart|edit",
  "sheetExpr": "string|null",   // target for chart/edit; null for new song
  "payload": { /* Song | Sheet | { format, chart, audioUploadId? } */ },
  "note": "string"              // contributor message to moderators
}
```
`201` → `{ "id": "string", "status": "pending" }`.

### Audio upload (for chart contributions)
- `POST /contributions/audio` → `{ "uploadId": "string", "uploadUrl": "string" }`
  (presigned PUT to S3-compatible storage). Client PUTs bytes, then references
  `audioUploadId` in the contribution payload.

### `GET /contributions?status=&mine=`  *(user sees own; moderator+ sees all)*
List with `{ id, kind, sheetExpr, status, author, createdAt }[]`, paginated
(`?page=&perPage=`, `X-Total-Count` header).

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
- **Pagination**: `?page` (1-based) + `?perPage` (default 50, max 200);
  `X-Total-Count` response header.
- **Rate limits**: write endpoints (§5) limited per user; `429` +
  `Retry-After`.
- **CORS**: the Tauri client routes writes through `src-tauri` (reqwest) to hold
  the token and dodge CORS; browser dev build needs `Access-Control-Allow-Origin`.
- **Versioning**: breaking changes → `/api/v2`. Additive fields are non-breaking;
  clients ignore unknowns (frontend already tolerates extra keys).

## 7. Frontend wiring map

| Contract | Frontend touch-point |
|----------|---------------------|
| `GET /catalog` | `stores/data.ts:loadData`, tauri `data.rs:load_chart_data` (swap `data.json`) |
| `hasChart/hasAudio` | `pages/songs.vue`, `pages/song.vue`, `MvSheetDialog.vue` (show visualize affordance) |
| §2 chart/audio | `pages/visualizer.vue`, `composables/useEngine.ts` (auto-load real charts) |
| §3 sync | new `src-tauri` cache layer (tier 2) |
| §4 auth | new login UI + `src-tauri` token store |
| §5 contributions | new contribution UI (desktop and/or web — open question) |
