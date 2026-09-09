# 07 — Rewire GET /catalog onto typed queries + add the region filter

**What to build:** `GET /catalog`'s handler in `main.rs` stops reading `v_song`/etc via `json_build_object` and instead assembles a typed `Catalog` from `queries::fetch_all_songs` (new, added in this ticket) + `queries::fetch_all_sheets` + the lookup-table fetches, returning `Json<Catalog>`. Adds the already-documented-but-missing `?region=` query param: every song stays in the response, only that song's `sheets[]` is filtered to region-available sheets (empty array allowed, song never dropped).

**Blocked by:** 04 — typed lookup-table queries, 05 — typed sheet queries.

**Status:** done

- [ ] `fetch_all_songs(pool) -> Vec<SongRow>` added to `queries.rs`, ordered by `source_index`, with its own `#[sqlx::test]`
- [ ] `catalog()` handler takes `Query<CatalogQuery { region: Option<String> }>`, returns `Json<Catalog>`
- [ ] `region` filter behavior: song with zero region-matching sheets still appears with `sheets: []`, never dropped from the response
- [ ] Integration test: region filter that excludes all of a song's sheets still returns that song with an empty `sheets` array
- [ ] Manual smoke test: `curl /api/v1/catalog` and `curl /api/v1/catalog?region=<x>` both return valid contract-shaped JSON
