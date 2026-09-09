# 01 — Fix stale docs: drop hasAudio/audio, fix schema.md planned-tables list

**What to build:** `apps/server/docs/api-contract.md` and `apps/server/docs/schema.md` no longer describe the cut audio-serving feature (`hasAudio` field, `GET /sheets/{sheetExpr}/audio`), and `schema.md`'s "Planned tables" list no longer lists `charts` (already migrated) or `assets` (dropped with the audio feature). This is a text-truth precondition the later typed-query tasks build on.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] `hasAudio` removed from the Sheet JSON payload shape in api-contract.md §1.1, with the following sentence updated to reference only `hasChart`
- [ ] `GET /sheets/{sheetExpr}/audio` section (§2) deleted entirely
- [ ] `grep -n -i "hasaudio\|sheets/{sheetExpr}/audio" apps/server/docs/api-contract.md` returns no output
- [ ] `schema.md`'s "Planned tables" table no longer lists `charts` or `assets`
- [ ] `grep -n "^| \`charts\`\|^| \`assets\`" apps/server/docs/schema.md` returns no output
