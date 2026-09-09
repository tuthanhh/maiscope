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

- [ ] `songs/` added to `.gitignore` (whole directory; chart text lives in the
      private data repo per issue 17, not here)
- [ ] `git status` confirms `songs/` no longer appears as untracked
- [ ] Confirm nothing under `songs/` was ever committed (`git log --all -- songs/`)
- [ ] Upstream `zetaraku/arcade-songs` license identified and recorded here
- [ ] Whatever that license requires is satisfied — typically a `LICENSE-THIRD-PARTY`
      or an upstream copyright notice block retained alongside the MIT `LICENSE`
- [ ] README acknowledgment section kept, but no longer the *only* compliance mechanism
