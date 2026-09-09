# Spec — Repo hygiene

**Status:** planned
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Close the two pre-existing hazards that make this repository unsafe to publish.
Do this before anything else in v1.0 — both are live now, and one of them is a
single `git add .` away from being irreversible.

## Problem

| Hazard | State |
|---|---|
| `songs/` | 146MB of SEGA-owned `track.mp3` / `bg.png`. Untracked but **not ignored**. The directory was deleted on 2026-09-09, but the same layout returns whenever a song pack is unpacked locally. |
| Upstream licence | The frontend is a port of [`zetaraku/arcade-songs`](https://github.com/zetaraku/arcade-songs) and the README credits it in prose. If upstream is MIT or similar, the licence also requires retaining a copyright notice and licence text, which prose credit does not satisfy. |

Both are cheap to fix and expensive to fix late. Publishing SEGA assets from a
public repository is not undone by a later commit, and a licence violation
discovered after launch is a takedown rather than an edit.

## Scope

| # | Ticket |
|---|---|
| 01 | [Ignore `songs/`, verify upstream licence](issues/01-ignore-songs-and-verify-upstream-license.md) |

## Out of scope

The README rewrite itself — [`docs-restructure`](../docs-restructure/spec.md)
shipped that. This feature makes the README stop being the *only* compliance
mechanism; it does not touch its prose.

## Done when

`songs/` cannot be committed by accident, `git log --all -- songs/` confirms it
never was, the upstream licence is identified and recorded, and whatever that
licence requires exists as a file rather than as a sentence.
