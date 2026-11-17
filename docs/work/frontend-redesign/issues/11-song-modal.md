# 11 — Song modal with a URL

**What to build:** the *Song modal*: card tap opens it,
`?song=<id>` in the URL so it can be shared and Back closes it, information
first, difficulty chips in the underline form, bookmark and play as text
buttons. The song page and the old sheet dialog are deleted.

**Blocked by:** 08, 10.

**Status:** todo

- [ ] Opening a `?song=<id>` link without the Catalog loaded fetches `/api/songs/{id}` (screen-data-requirements: a shared link must not cost the snapshot)
- [ ] Full-height sheet from the bottom on phones
- [ ] No video search, no lock or globe icons
- [ ] `pages/song.vue` and `MvSheetDialog.vue` removed; their routes redirect to the modal
