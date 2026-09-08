# 03 — Document GET /sheets/search in api-contract.md

**What to build:** New §1.2 section in api-contract.md documenting every query param, response shape, and the explicit "no superFilter equivalent" note (removed from the app, not ported here).

**Blocked by:** 02 — search endpoint handler (docs describe what actually shipped).

**Status:** done

- [ ] New section added directly after the existing `GET /songs/{songId}`/`GET /sheets/{sheetExpr}` entries, full param table + response shape
- [ ] `grep -n "sheets/search" apps/server/docs/api-contract.md` finds the new section heading
