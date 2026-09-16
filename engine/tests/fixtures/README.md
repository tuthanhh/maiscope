# Parser fixtures

Hand-written simai chart text, one `.txt` per fixture, consumed by
[`../corpus.rs`](../corpus.rs).

A fixture is the **note-data body only** — what follows `&inote_N=` in a real
`maidata.txt`. `parse_chart` never sees the `&title=` / `&artist=` header keys.

## Rules

- **Chart text only.** Never an mp3, never a bg image. The original sample data
  was deleted with its audio on 2026-09-09 and is not coming back here.
- **Hand-written, not copied.** These are synthetic charts authored for this
  repo, so they are committable without an upstream licensing question. Real
  `maidata.txt` stays in the gitignored `songs/` at the repo root, or in
  `local/` (see below).
- **Small and legible.** A fixture exists to make a snapshot diff readable. If
  you cannot tell at a glance which line moved the count, it is too big.
- **One concern per file.** A fixture that mixes slides and BPM changes tells
  you a count moved but not why.

## `local/` — real charts, never committed

`local/` is gitignored. Drop real `maidata.txt` *bodies* there and
`every_fixture_parses_without_panicking` sweeps them alongside the committed
fixtures; a missing directory is not an error, so CI and fresh clones simply
skip them.

They are deliberately excluded from the snapshot: committed expectations must
not depend on files most checkouts will not have.

This is worth doing. `BIRTH.txt` sitting here is what surfaced the unsupported
backtick pseudo-EACH — synthetic fixtures only cover syntax someone thought to
write.

## Snapshot

`../corpus-snapshot.txt` holds one line per fixture: `name: key=n key=n ...`,
counting events by kind. It sits *outside* this directory on purpose — anything
`*.txt` in here is globbed as a chart, so a snapshot stored here would fixture
itself. Regenerate with:

```sh
UPDATE_SNAPSHOT=1 cargo test -p engine --test corpus
```

Read the resulting diff before committing it.

## Suggested coverage

One file per row, named after the row:

| Fixture | Covers |
|---|---|
| `taps.txt` | bare taps, EACH shorthand (`12`), `/` groups |
| `holds.txt` | `h[N:M]`, pseudo-hold `h`, break/ex holds |
| `touches.txt` | every zone A–E, `C` with and without index, touch-holds |
| `slides_simple.txt` | one file covering each single-segment shape |
| `slides_chained.txt` | chained segments, shared vs. per-segment duration |
| `slides_star.txt` | `*` chains, including a chained path inside one |
| `timing.txt` | mid-chart BPM changes, `{N}` resolution, `{#S}` absolute length |
| `modifiers.txt` | `b` / `x` / `f`, before and after the bracket, combined |
