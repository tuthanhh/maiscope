# 01 — Repo hygiene: ignore `songs/`, verify upstream license

**What to build:** Close two pre-existing hazards before any other work. `songs/`
is 146MB of SEGA-owned `track.mp3` / `bg.png` that is untracked but **not
ignored** — one `git add .` publishes it from a repo that is about to become
public. Separately, the frontend is a port of `zetaraku/arcade-songs` and the
README credits it in prose; if upstream is MIT (or similar) the license also
requires retaining their copyright notice and license text, which prose credit
does not satisfy.

**Blocked by:** None — do this first.

**Status:** todo

- [ ] `songs/` added to `.gitignore` **pre-emptively** — the directory was deleted
      on 2026-09-09, but the same layout returns whenever a song pack is unpacked
      locally, and it must never be committed
- [ ] Confirm nothing under `songs/` was ever committed (`git log --all -- songs/`)
- [ ] Upstream `zetaraku/arcade-songs` license identified and recorded here
- [ ] Whatever that license requires is satisfied — typically a `LICENSE-THIRD-PARTY`
      or an upstream copyright notice block retained alongside the MIT `LICENSE`
- [ ] README acknowledgment section kept, but no longer the *only* compliance mechanism
