# ADR-0027 — maiscope is GPL-3.0-or-later

**Status:** accepted
**Date:** 2026-11-18

## Context

The visualizer's playfield is to look like MajdataPlay's: its note, hold,
slide, touch and effect art, and slide tracks laid out from its per-shape
prefab data rather than computed geometry
([spec](../work/visualizer-majdata/spec.md), piece 3). MajdataPlay is licensed
under the GPL-3.0, so its art and layouts ship in maiscope's web bundle. Until
now maiscope was MIT. Its frontend is a port of `zetaraku/arcade-songs` (MIT),
and its engine sprites are derived from SEGA's maimai art (no licence
asserted; see `LICENSE-THIRD-PARTY`).

## Decision

maiscope as a whole is licensed under the **GNU General Public License,
version 3 or (at your option) any later version** (`GPL-3.0-or-later`). The
change lands in the same commit as the first MajdataPlay file
(`engine/assets/majdata/`, copied by `scripts/extract-majdata.py` at a pinned
commit). MajdataPlay's files keep their own GPL-3.0 terms; the maintainer
chose `-or-later` for maiscope's own code (2026-11-18).

## Rejected alternatives

- **Keep MIT, with MajdataPlay's files in a GPL folder.** The published bundle
  combines them with the engine either way, so it is GPL in practice; an MIT
  label on the repository would mislead.
- **Imitate the look with our own art.** It would avoid the licence change,
  but the slide layouts are the hard part, and hand-traced ones would be
  guesswork.
- **`GPL-3.0-only`.** It matches MajdataPlay's file exactly. `-or-later` is the
  FSF's usual form and stays compatible with MajdataPlay's terms for the
  combined work; the maintainer chose it.

## Consequences

- `LICENSE` is the GPL-3.0 text; `LICENSE-THIRD-PARTY` and the web bundle's
  `THIRD_PARTY_NOTICES.txt` name MajdataPlay; `package.json` and every crate's
  `Cargo.toml` say `GPL-3.0-or-later`.
- Anyone who distributes maiscope, including the deployed web app, must offer
  its source under the GPL. The repository is public, which already does.
- Contributions are accepted under the same licence.
- The SEGA notice stays: the MajdataPlay skin is likely derived from the same
  art, and the GPL does not change SEGA's position.
- Removing MajdataPlay's files later would not by itself undo this decision:
  code released under the GPL stays available under it.
