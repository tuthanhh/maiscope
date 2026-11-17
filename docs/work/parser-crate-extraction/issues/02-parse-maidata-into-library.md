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

**Status:** done (2026-11-17)

- [x] `simai::maidata::parse(text) -> Result<Maidata, ParseError>` returning
      song fields (`title`, `artist`, `wholebpm`, `genre`, `version`) plus a
      `Vec` of difficulty entries (slot, `lv`, `des`, `inote`)
- [x] Unknown `&key=` pairs are preserved, not dropped — the format has keys this
      app does not read (`&cabinet=`, `&chartconverter=`, `&shortid=`) and
      discarding them silently loses contributor data
- [x] An empty or whitespace-only `inote_N` is absent, not an empty chart
      (`seed_songs.rs:187` already does this)
- [x] `bin/seed_songs.rs` calls it and keeps its existing behaviour, including
      the `--allow-unmatched` exit codes
- [x] Tests cover: a real multi-difficulty file, a file with gaps in the slot
      numbering, CRLF line endings, and a missing `&title=`

## Comments

**2026-11-17: done.** `simai::maidata::parse(text) -> Result<Maidata,
MaidataError>`. The error is `MaidataError`, not `ParseError`, so it cannot be
confused with the chart parser's `simai::ParseError`; its one case is
`MissingTitle`. The slot table became `maidata::standard_difficulty(slot)`.

Details that keep the seeder's behaviour exact:

- `inote` is kept as written, lines joined by `\n`, so chart hashes match
  what earlier seeds stored. Scalar fields are trimmed.
- Unknown keys land in `other` untrimmed and in file order, repeats
  included. A known key that repeats keeps its last value, as before.
- A leading byte-order mark is stripped here now, so the seeder's own
  BOM handling and its test moved into the library.
- A folder is UTAGE when slot 7 has a level or a chart. A bare `&des_7=` is
  not enough, matching the old check on `lv_7`/`inote_7`.

The test fixtures copy a real file's shape (BOM, mixed CRLF and LF, extra
keys) with hand-written chart bodies; real chart text is not committed.
Checked on dev: a full reseed wrote nothing (7065 already up to date) with
every report count unchanged, and without `--allow-unmatched` it still
exits 1. `apps/server` now depends on `simai` without the `bevy` feature;
ticket 03 verifies the tree.
