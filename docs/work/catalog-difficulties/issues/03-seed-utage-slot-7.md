# 03 — Seed UTAGE charts (maidata slot 7)

**What to build:** stop silently skipping every UTAGE chart in `seed_songs`.

Three separate mismatches, all of which have to be handled:

| | maidata | catalog |
|---|---|---|
| slot | `&inote_7=` — not in `SLOT_DIFFICULTIES` | — |
| label | `&lv_7=協` | `【協】` |
| title | `[宴/バディ/1P] エイリアンエイリアン [13?]` | `(宴) エイリアンエイリアン` |

172 local maidata files carry `inote_7`. The catalog holds 176 songs with a
UTAGE sheet, so the two sets nearly agree — the seeder simply never matches them.

**Blocked by:** None, though 01 makes the label question easier to answer.

**Status:** todo

- [ ] Slot 7 maps to a UTAGE difficulty rather than being dropped
- [ ] The `協` / `【協】` normalisation happens in one place, named, not inline
- [ ] Title matching handles the `[宴/…] X [13?]` form against `(宴) X` and
      `[好]X`. This is the hard part and may not be fully solvable — matching is
      already too strict (`prod-data-and-infra` issue 03), and a wrong match
      attaches a chart to the wrong song, which is worse than skipping
- [ ] Unmatched UTAGE titles are *reported*, not silently skipped — the current
      behaviour hid 172 files
- [ ] Tests over real maidata fixtures for each of the three mismatches

> Do not make matching looser to raise the hit rate. A missed chart is a chart
> nobody can view; a mismatched chart is the wrong chart rendered as if correct.
