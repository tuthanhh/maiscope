# 05 — Community library screens

**What to build:** browse, detail, upload, and the reports queue for the second
library.

[ADR-0013](../../../adr/0013-community-charts-beside-the-catalog.md) keeps the two
libraries visibly separate: official pages show official charts only, and a fan
chart is never rendered where it could be mistaken for the arcade's.

**Blocked by:** [`community-charts`](../../community-charts/spec.md), 04.

**Status:** todo

- [ ] Community browse and detail, distinct from the official browse
- [ ] Upload: a `maidata.txt` picker, the official-song link chooser, and a
      rejection path that shows *which* difficulty and *which* token failed —
      `ParseError` carries all three, so the UI should not flatten it to
      "invalid chart"
- [ ] Reports: submit from a chart, and a moderator queue
- [ ] Nothing community-shaped appears on an official song or sheet page
