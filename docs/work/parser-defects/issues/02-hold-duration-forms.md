# 02 — HOLD accepts every duration form

**What to build:** let a HOLD carry any duration bracket the notation defines,
not just `[N:M]`.

[The HOLD section](../../../reference/simai-notation.md#hold) gives three forms:

| Notation | Meaning |
|---|---|
| `5h[2:1],` | one half note — **works today** |
| `4h[#5.678],` | exactly 5.678 seconds — rejected |
| `4h[150#2:1],` | one half note at 150 BPM — rejected |

`TAP_TOUCH_RE` hard-codes `\[(\d+):(\d+)\]`, so the other two never match and
the whole note is an "Invalid note syntax" error.

`parse_duration_bracket` already handles `[150#2:1]`. The regex simply never
gives it the chance. `[#5.678]` needs a small fix there too: the `#` sits at
index 0, so `bpm_part` is empty and `"".parse::<f32>()` fails.

**Blocked by:** None.

**Status:** done

- [x] Widen `TAP_TOUCH_RE` to capture the whole bracket, then delegate to
      `parse_duration_bracket` — `hold_duration` changes from
      `Option<(usize, usize)>` to `Option<Duration>`
- [x] `parse_duration_bracket` handles a leading `#` as "absolute seconds", a
      form it currently rejects
- [x] `build_button_note` and `build_touch_note` stop rebuilding
      `Duration::Simple` from a tuple; the pseudo-hold fallback stays `[1280:1]`
- [x] `note::tests::hold_rejects_non_simple_durations` inverts into a test that
      asserts both forms parse
- [x] `duration::tests::absolute_seconds_bracket_is_unsupported` likewise
