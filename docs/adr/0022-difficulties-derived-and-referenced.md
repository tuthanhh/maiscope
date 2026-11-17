# ADR-0022 — Difficulties are derived, referenced, and matched by code

**Status:** accepted
**Date:** 2026-11-17

## Context

Upstream's `data.json` lists five difficulty rows. Its UTAGE sheets use 50
more values, each a one-character tag in full-width brackets (`【協】`), that
it never lists. `/catalog` therefore shipped five difficulties for 55
values in use, and nothing stopped a sheet holding a value no row
described.

maidata names the same UTAGE difficulty by the bare tag (`&lv_7=協`) and
puts the chart in slot 7, which `seed_songs` did not read. So no UTAGE chart
was ever seeded.

[ADR-0010](0010-differential-catalog-sync.md) made the sync differential
but kept all five lookup tables replaced wholesale on every run.
`catalog-difficulties` (tickets 01 to 03) changes that for `difficulties`.

## Decision

**`difficulties` holds every value `sheets.difficulty` holds.** The sync
keeps upstream's rows in upstream order and derives one row per other
value, from the payload's sheets and the stored ones alike, in byte order.
A derived row has no colour or icon.

**Two columns: `name` for display, `code` for matching and URLs.** `name`
is the bracketed form the sheets store; `code` is the bare tag maidata
emits, `UNIQUE`. For upstream rows `code` equals the difficulty.

**`sheets.difficulty` references `difficulties` `ON DELETE RESTRICT`,
checked at the statement.** The sync upserts `difficulties` and prunes
only rows no sheet uses, before writing sheets. The other four lookup
tables are still replaced wholesale. This narrows ADR-0010's scope clause
to those four.

**The migration that adds `code` backfills the derived rows by the same
rule.** A deploy runs the foreign key migration straight after it, before
any sync could add the rows.

**`seed_songs` resolves a UTAGE folder by `difficulties.code = &lv_7=`,
pooling the candidates from every title form the folder offers.** Only a
single sheet seeds. Both sides of a buddy chart are set aside and
reported. An unmatched UTAGE folder fails the run like any unmatched chart.
A sheet that several folders resolve to seeds none of them.

## Rejected alternatives

**One column, deriving the other form on demand.** It saves a column.
Against: a strip-or-add step on every path (parser output, URL parsing,
display, upload validation), and one missed application makes a chart
unfindable.

**A hard-coded set of UTAGE tags in Rust.** No database work. Against: it
drifts the first time upstream adds a tag, and nothing stops a sheet
holding a value the code rejects.

**Derived rows in first-seen order.** It follows the catalog's own order.
Against: the lookup digest compares runs, so a reordered payload would
advance the revision on a no-op sync and every client would refetch.

**A deferred foreign key, keeping the wholesale replace.** It leaves the
sync untouched. Against: a violation surfaces at commit with no statement
to blame, and the sync stays able to drop a row mid-transaction.

**Taking the first title form that matches.** Simpler, and it seeds more.
Against: the catalog lists 16 UTAGE songs twice, once as `(宴) X` and once
as `[tag]X`, and the folder names neither. First-hit picks one silently,
and a wrong chart renders as if correct.

**Seeding one side of a buddy chart.** It loads 48 more folders. Against:
the catalog holds one sheet for both sides, so half the chart would render
as the whole.

## Consequences

**Positive.** `/catalog` describes every difficulty a sheet uses, and the
database rejects any other. 93 UTAGE charts seed from the local sample,
where none did. `code` is ready to be the URL component the API rewrite
needs.

**Negative.** `difficulties` is no longer a mirror: part of it is computed,
so its rows can differ from upstream's file. The bracket-strip rule exists
twice, in the migration and in `catalog_sync`; a dev sync after the
migration left the table byte-identical, but nothing tests the two
together. A future upstream key equal to a derived `code` fails the sync
until resolved. Buddy charts and the 31 unmatched UTAGE folders stay
unseeded. With 12 unmatched standard titles as well, the production seed
needs `--allow-unmatched` to pass.

**Neutral.** Test fixtures that insert a sheet now seed its difficulty row
first. The browse filter lists only the difficulties standard and DX
sheets use; UTAGE stays reachable through its type chip.
