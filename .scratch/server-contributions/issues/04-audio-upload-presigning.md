# 04 — Presigned audio upload endpoint

**What to build:** `POST /contributions/audio` — any authenticated user gets a presigned S3-compatible PUT URL (R2/B2/etc, 15-minute expiry) to upload chart-contribution audio directly to storage, returning `{ uploadId, uploadUrl }`.

**Blocked by:** `server-auth-github-oauth/issues/02-jwt-and-authuser-extractor.md` (needs `AuthUser`).

**Status:** ready-for-agent

- [ ] `aws-sdk-s3`/`aws-config`/`uuid` deps added; `.env.example` documents `S3_ENDPOINT_URL`/`S3_BUCKET`/`S3_ACCESS_KEY_ID`/`S3_SECRET_ACCESS_KEY`/`S3_REGION`
- [ ] `POST /contributions/audio` returns a valid presigned PUT URL, 15-minute expiry, keyed `contributions/audio/{uploadId}`
- [ ] Manual smoke test against a real dev S3-compatible bucket: PUT a real audio file to the returned URL, confirm it lands in the bucket
