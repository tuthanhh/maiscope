# 02 — `parse_maidata` into the library

**What to build:** move maidata-file parsing out of `apps/server/src/bin/seed_songs.rs`
into `crates/simai`, so the seeder and the future upload endpoint share one
implementation.

Today `bin/seed_songs.rs` holds:

- `parse_maidata` (line 62) — splits `&key=` pairs, handling `inote` bodies that
  span many lines with no `&` prefix
- the inote-slot → difficulty-code map (line 34), mirroring `sheets.difficulty`
- `nfc` (line 43) — Unicode normalisation for title matching

The first two are format knowledge and belong beside the chart parser. `nfc` and
the title-matching are the seeder's own concern and stay put.

A `maidata.txt` is one song plus up to five difficulties, so the natural return
type is a struct, not the current `HashMap<String, String>` — the caller should
not have to know that `&lv_4=` pairs with `&inote_4=`.

**Blocked by:** 01 — extract `crates/simai`.

**Status:** todo

- [ ] `simai::maidata::parse(text) -> Result<Maidata, ParseError>` returning
      song fields (`title`, `artist`, `wholebpm`, `genre`, `version`) plus a
      `Vec` of difficulty entries (slot, `lv`, `des`, `inote`)
- [ ] Unknown `&key=` pairs are preserved, not dropped — the format has keys this
      app does not read (`&cabinet=`, `&chartconverter=`, `&shortid=`) and
      discarding them silently loses contributor data
- [ ] An empty or whitespace-only `inote_N` is absent, not an empty chart
      (`seed_songs.rs:187` already does this)
- [ ] `bin/seed_songs.rs` calls it and keeps its existing behaviour, including
      the `--allow-unmatched` exit codes
- [ ] Tests cover: a real multi-difficulty file, a file with gaps in the slot
      numbering, CRLF line endings, and a missing `&title=`
