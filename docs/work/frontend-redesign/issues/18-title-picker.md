# 18 — Title as a searchable list

**What to build:** the Browse search box becomes the same combobox as the
Artist filter (16), over every song title (maintainer's ask, 2026-11-18).

**Blocked by:** 16.

**Status:** done

- [x] The search box lists every Catalog title, sorted, on focus; typing
      narrows it (prefix matches first); a chosen title fills the box
- [x] Free text still filters by substring, as before; Exact match still
      applies from the drawer
- [x] `MvCombobox` takes a leading `icon`, as `MvTextInput` does

## Comments

**2026-11-18 — built.** `MvTextInput` is no longer used by Browse's
search; it stays for the artist-less fields (BPM). Covered by the browser
checks that already exercise the combobox, plus a title-specific one.
