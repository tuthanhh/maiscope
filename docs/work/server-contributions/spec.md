# Spec — Server contributions

**Status:** superseded
**Milestone:** —
**Superseded by:** [`community-charts`](../community-charts/spec.md) and
[ADR-0013](../../adr/0013-community-charts-beside-the-catalog.md)

This feature never had a spec. It is written now only to carry the status line
and the record of why the design was dropped — the six tickets stay in git as the
account of what was considered.

## What it was

A contribution queue. A user proposes a new song, sheet, chart or edit; it sits
`pending`; a moderator approves or rejects; an approval merges into the canonical
`songs`/`sheets`/`charts` tables and bumps the revision. `api-contract.md` §5 is
its design, and six tickets described the build.

## Why it was dropped

**It cannot hold a fan chart.** The merge target is a canonical row, so a chart
for a song the official catalog does not contain has nowhere to go. The model
wanted — anyone uploads, anyone plays — is a second library, not a stream of
edits to the first.

**The queue needs a moderator who is always available.** On a solo project that
is one person, and upload-to-visible latency becomes however long until they next
open the queue.

**The catalog's constraints forbid the content anyway.** `charts` is
`UNIQUE (sheet_id, format)` — one chart per sheet, so two people cannot chart the
same master no matter what the queue decides.

Full reasoning and the rejected alternatives are in
[ADR-0013](../../adr/0013-community-charts-beside-the-catalog.md).

## Ticket disposition

| # | Was | Now |
|---|---|---|
| 01 | contributions / audit_log / chart_revisions tables | dropped — wrong shape. `chart_revisions` was built separately and is live |
| 02 | submit and list contributions | dropped — replaced by `community-charts` 02, 03 |
| 03 | approve and reject | dropped — there is no approval |
| 04 | presigned audio upload | dropped — already contradicted [ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md) before this decision, and must not be revived by it |
| 05 | contract and schema doc sync | survives in spirit as `community-charts` 07 |
| 06 | bump revision on approve | dropped — community content deliberately stays out of the revision stream |

Nothing here is blocked or waiting. `server-auth-github-oauth` is unaffected and
remains a prerequisite for its replacement.
