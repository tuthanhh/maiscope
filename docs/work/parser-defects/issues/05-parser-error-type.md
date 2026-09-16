# 05 — Parser error type; refuse unparseable charts

**What to build:** replace `io::Error` with a parser-specific error, and stop
dropping tokens that fail to parse.

Today an unparseable token is logged to stderr and *no event is pushed*:

```rust
Err(e) => eprintln!("Warning: Error parsing note at token {}: '{}' - {}", idx, token, e),
```

Every note after it lands one beat early. Worse, `parse_chart("@@@")` returns
`Ok(vec![])` — a chart that parsed to nothing is indistinguishable from an empty
chart, so no caller can tell.

**Decision:** propagate. A chart containing notes we cannot parse is a chart we
cannot render correctly, and saying so beats rendering it subtly wrong. The only
caller (`systems/mod.rs:80`) already logs and skips on `Err`, so a rejected
chart degrades to "this song does not load" — never a panic, which in wasm would
take the whole canvas down.

`io::Error` goes with it. Nothing here touches IO; it was only ever a
placeholder. See
[ADR-0012](../../../adr/0012-one-marker-of-each-kind-per-simai-token.md)'s
consequences, which flagged these two as one job.

**Blocked by:** 01, 02, 03, 04 — **this must land last.**

Tightening the error path before the notation is supported would turn a silent
wrong render into a loud total failure, for charts whose syntax is perfectly
valid simai. `BIRTH.txt` is the proof: it would stop loading entirely until
issue 01 ships.

**Status:** done

- [x] A `ParseError` type carrying the token index, the token text, and the
      cause — the three things needed to diagnose from one log line
- [x] `parse_chart` returns `Result<Vec<ChartEvent>, ParseError>`
- [x] A token whose note fails to parse propagates instead of being skipped
- [x] The duplicate-marker error from ADR-0012 moves onto the same type
- [x] No `eprintln!` or `println!` left in the parser — `parse_chart` currently
      prints `"Parsed N events"` on every call, which is noise in tests and in
      the browser console
- [x] `chart::tests::unparseable_token_drops_the_event_and_shifts_the_chart`
      inverts into a rejection test
- [x] `parse_chart_never_panics` keeps passing unchanged — `Err` was always an
      acceptable outcome there; panicking never was
