# Spec — Difficulties as a real set

**Status:** planned
**Milestone:** phase 2

Make the `difficulties` table the authoritative set of difficulty values, with
`sheets.difficulty` referencing it. Fixes a live bug and unblocks the API
rewrite's `Difficulty` type.

## Problem

`/catalog` ships five difficulties while its sheets use fifty-five.

```
difficulties table:                    basic, advanced, expert, master, remaster
distinct sheets.difficulty values:     5 standard + 50 UTAGE labels
labels used by sheets, absent there:   50
```

`schema.md` notes the frontend builds index maps from that array because its
order is significant. Every UTAGE sheet's difficulty indexes into nothing.

Separately, the seeder never loads a UTAGE chart at all:

```rust
// bin/seed_songs.rs:35
const SLOT_DIFFICULTIES: &[(&str, &str)] = &[
    ("2","basic"), ("3","advanced"), ("4","expert"), ("5","master"), ("6","remaster"),
];
```

maidata puts UTAGE on **slot 7**, which is not in that map, so every such chart
is silently skipped — 172 local files. That is a real contributor to the
`291 songs skipped entirely` from the 2026-09-14 production seed run
(`prod-data-and-infra` issue 04).

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Difficulties table covers UTAGE](issues/01-difficulties-cover-utage.md) | Migration + `catalog_sync` |
| 02 | [`sheets.difficulty` foreign key](issues/02-sheets-difficulty-fk.md) | Blocked by 01 |
| 03 | [Seed UTAGE charts (slot 7)](issues/03-seed-utage-slot-7.md) | Independent of 01/02 |

## Decisions that constrain this work

- **The table is the source of truth, not a list in Rust.** A hardcoded set
  drifts the first time upstream adds a label, and nothing would stop a sheet
  holding a value the code rejects. A foreign key makes that impossible.
- **Synthesised rows must survive the sync.** `catalog_sync` currently `DELETE`s
  the lookup tables and re-inserts them from upstream, which does not send UTAGE
  difficulty rows. Whatever this adds has to be reconstructed each run rather
  than wiped.
- **`catalog_sync` is the riskiest module in the server** — 1,897 lines, 34
  tests, and the piece most worth leaving alone. This touches it deliberately
  and minimally.

## Out of scope

The `Difficulty` Rust type and any API shape — those belong to
[`api-rewrite`](../api-rewrite/spec.md). This feature only makes the data
correct enough for that type to be validated against.

## Done when

`/catalog`'s `difficulties` array covers every value its sheets use, a sheet
cannot hold a difficulty the table does not know, and `seed_songs` loads UTAGE
charts instead of skipping them.
