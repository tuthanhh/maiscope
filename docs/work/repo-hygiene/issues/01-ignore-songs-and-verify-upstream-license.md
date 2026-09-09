# 01 — Repo hygiene: ignore `songs/`, verify upstream license

**What to build:** Close two pre-existing hazards before any other work. `songs/`
is 146MB of SEGA-owned `track.mp3` / `bg.png` that is untracked but **not
ignored** — one `git add .` publishes it from a repo that is about to become
public. Separately, the frontend is a port of `zetaraku/arcade-songs` and the
README credits it in prose; if upstream is MIT (or similar) the license also
requires retaining their copyright notice and license text, which prose credit
does not satisfy.

**Blocked by:** None — do this first.

**Status:** done

- [x] `songs/` added to `.gitignore` **pre-emptively** — the directory was deleted
      on 2026-09-09, but the same layout returns whenever a song pack is unpacked
      locally, and it must never be committed
- [x] Confirm nothing under `songs/` was ever committed (`git log --all -- songs/`) —
      empty, clean
- [x] Upstream `zetaraku/arcade-songs` license identified and recorded here — MIT,
      copyright (c) 2022 Raku Zeta (via `gh api repos/zetaraku/arcade-songs/license`)
- [x] Whatever that license requires is satisfied — added `LICENSE-THIRD-PARTY` at
      repo root with the upstream copyright notice and full MIT text
- [x] README acknowledgment section kept, but no longer the *only* compliance
      mechanism — License section now links `LICENSE-THIRD-PARTY` alongside prose
