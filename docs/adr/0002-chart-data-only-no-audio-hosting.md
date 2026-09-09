# ADR-0002 — Public hosted service, chart data only, no audio hosting

**Status:** accepted
**Date:** 2026-09-09

## Context

Three product shapes were considered: a public hosted service, a shipped app
with contributions as pull requests against a data repository, and a personal
tool. Audio is SEGA-owned; charts are community transcriptions of official
charts — cleaner, though not risk-free.

## Decision

Public hosted service. Chart text only. No audio crosses the server in v1.
Contributor-supplied audio is deferred to phase 2 and gets its own decision.

## Rejected alternatives

Hosting audio. The cost is not storage — it is moderation. A "community chart"
is the obvious route for laundering official audio, and the only defence is a
human reviewing every upload.

## Consequences

The engine's silent path (`load_chart`, wall-clock driven) is v1's playback
mode; the audio-slaved path (`load_song` → `sync_to_audio_position`) is unused.
Contract §5's audio upload and S3 presigning are struck. No object storage
enters the stack. No accounts and no PII, so no privacy policy is needed — a
property to preserve when adding client error reporting.
