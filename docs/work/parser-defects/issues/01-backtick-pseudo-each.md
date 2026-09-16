# 01 — Backtick pseudo-EACH

**What to build:** support the `` ` `` separator, which places the following
note one millisecond after the preceding one.

Per [the notation doc](../../../reference/simai-notation.md#pseudo-each),
`` 1`2, `` is BUTTON-1 then BUTTON-2 a millisecond later — deliberately *not* an
EACH, so the two do not turn yellow and do not count as an EACH in results.
`` 1`2`3/4, `` chains it: 2 is 1ms after 1, and the EACH of 3+4 is 1ms after 2.

The character appears in neither `TAP_TOUCH_RE` nor `SLIDE_PATTERN_RE`, so the
whole token fails and — because of issue 05 — vanishes silently.

This is not theoretical. `engine/tests/fixtures/local/BIRTH.txt` uses it three
times: `` E6`B5 ``, `` B3`E4 ``, `` B2`E2 ``.

**Blocked by:** None.

**Status:** done

- [x] `parse_chart` splits a token on `` ` `` into ordered sub-groups, each of
      which is then split on `/` as today
- [x] Decide how the 1ms offset is represented — `ChartEvent::NoteGroup` carries
      no time, so this likely needs either a new event or a per-group offset
      field that `compute_timestamps` adds to `current_time`
- [x] The offset does not advance the beat grid: a token is still one comma
      regardless of how many `` ` `` it contains
- [x] Notes in a pseudo-EACH are not an EACH — whatever marks EACH-ness for
      rendering must not treat them as simultaneous
- [x] `note::tests::pseudo_each_backtick_is_unsupported` becomes a test that
      asserts the parse, renamed accordingly
- [x] `BIRTH.txt` parses with zero warnings

## Outcome

Represented as `Note.offset_ms: u32` — a sub-comma delay carried by the note
itself, rather than a new event or a nested group. `parse_chart` splits a token
on `` ` `` first, then each group on `/`, and stamps the group index onto every
note it produced. All of it stays in one `NoteGroup`, so the beat grid is
untouched.

`compute_timestamps` needed no change: it advances once per `NoteGroup`, and
there is still exactly one per token.

The EACH consequence turned out to be live. `spawning.rs` computed
`is_paired = notes.len() >= 2` for the whole group, which would have rendered a
pseudo-EACH as a pair — precisely what the notation says it is not. Pairing is
now counted per offset.

**Not done:** the 1ms delay is parsed, stored and used for pairing, but not
applied to spawn time. At 1ms it is two orders of magnitude below a frame, so
nothing could render differently; applying it would mean threading a sub-frame
offset through the spawn path for no visible gain. Revisit only if judgement
ever moves off the frame clock.
