# 02 — Client identity off `sheetExpr`

**What to build:** the frontend indexes on the minted `publicId`, not on a
title-derived composite key.

`utils/sheet.ts:computeSheetExpr` builds `${songId}|${type}|${difficulty}`, and
`songId` is the title. Everything downstream — `$canonicalSheet`, the selected
sheets store, the sheet dialog, URL construction — inherits that.

**Blocked by:** [`song-public-id`](../../song-public-id/spec.md),
[`api-rewrite`](../../api-rewrite/spec.md).

**Status:** todo

- [ ] `publicId` replaces `sheetExpr` as the identity used for lookup and links
- [ ] `computeSheetExpr` either stops existing or is confined to whatever still
      genuinely needs the composite form
- [ ] Persisted client state keyed on the old value (selected sheets, any
      localStorage) migrates or is discarded deliberately — silently dropping a
      user's selection is the failure mode
- [ ] CLAUDE.md's "cross-tier key" convention is rewritten to match
