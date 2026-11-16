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

**Blocked by:** 01 — and this is a hard block, not a convenience.

Chart text is keyed by `sheet_expr = songId|type|difficulty`, so the difficulty
representation is baked into every chart row this ticket creates. Today that
costs nothing to change: `charts` holds 6849 rows and **zero** are UTAGE,
precisely because this bug skips them. The moment this ticket runs, changing the
representation means re-keying real chart text. 01 settles it first.

**Status:** todo

- [ ] Slot 7 maps to a UTAGE difficulty rather than being dropped
- [ ] Lookup goes through `difficulties.code` — 01 stores `協` there
      specifically so the seeder matches maidata without transforming anything
- [ ] Title matching handles the `[宴/…] X [13?]` form against `(宴) X` and
      `[好]X`. This is the hard part and may not be fully solvable — matching is
      already too strict (`prod-data-and-infra` issue 03), and a wrong match
      attaches a chart to the wrong song, which is worse than skipping
- [ ] Unmatched UTAGE titles are *reported*, not silently skipped — the current
      behaviour hid 172 files
- [ ] Tests over real maidata fixtures for each of the three mismatches

> Do not make matching looser to raise the hit rate. A missed chart is a chart
> nobody can view; a mismatched chart is the wrong chart rendered as if correct.

## Comments

**The seeder now names these folders instead of failing on them
(2026-11-17, `prod-data-and-infra` 04).** A folder with `&lv_7=` or
`&inote_7=` is counted in the run summary as `N UTAGE folders not seedable
yet` and skipped before title matching, so the 164 local ones no longer sit
inside "unmatched titles". The resolver's catalog index deliberately loads no
`type = 'utage'` sheet; this ticket extends it. Facts gathered for that:
catalog UTAGE sheets carry `difficulty = '【X】'` (the kanji tag in full-width
brackets, 50 distinct tags) and the song title is the tag plus the title with
no space (`[X]人マニア`), while the sample folder is `[X] 人マニア` and its
`&title=` is `[宴] 人マニア [14?]`. So the mapping is: folder name with the
space after the first `]` removed → title; `&lv_7=X` → `【X】`; slot 7 →
that sheet. Buddy charts (`[宴/バディ/1P]`) have their own folders per side.
