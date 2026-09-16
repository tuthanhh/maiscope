# 04 — Reject partially bracketed slide chains

**What to build:** make a chained slide that brackets *some* of its segments an
error, as the notation requires.

[Chaining SLIDE](../../../reference/simai-notation.md#chaining-slide) defines
exactly two legal shapes:

- **One bracket, on the last segment.** `1-4q7-2[1:2],` — one constant speed
  over the whole path, computed from the total length.
- **A bracket on every segment.** `1-4[2:1]q7[2:1]-2[1:1],` — per-segment
  speeds. "When doing this, *every* sub-track needs its own length — omitting
  one causes an error."

`1-4[2:1]q7-2[1:1],` is neither. Today `parse_chained_slide_segments` fills the
bare segment from the last one and marks the note `shared_duration: true`, so it
traces at a speed nobody wrote, silently.

Same rule as
[ADR-0012](../../../adr/0012-one-marker-of-each-kind-per-simai-token.md):
contradictory input is rejected, not guessed at.

**Blocked by:** None.

**Status:** done

- [x] `parse_chained_slide_segments` distinguishes "only the last is bracketed"
      from "some are bracketed" — currently both collapse into
      `segments.iter().any(|s| s.duration.is_none())`
- [x] The all-but-last-bare case stays legal and keeps `shared_duration: true`
- [x] The every-segment case stays legal with `shared_duration: false`
- [x] Anything between is an error naming the segment that lacks a bracket
- [x] A single-segment slide is unaffected
- [x] `slide::tests::partially_bracketed_chain_silently_inherits` inverts into a
      rejection test
