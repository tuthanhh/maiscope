# 22 — JetBrains Mono everywhere

**What to build:** the maintainer's ask of 2026-11-18: "the JetBrains Mono
Nerd Font as before" — one mono face for all text, as the front page
already read after 21.

**Blocked by:** 21.

**Status:** done

- [x] `--f-sans` and `--f-display` point at `--f-mono`; Inter is no longer
      fetched
- [x] `JetBrainsMono Nerd Font` (and its `NF` name) lead the stack for
      machines with it installed; the JetBrains Mono webfont serves the rest
- [x] No horizontal overflow at 390px with the wider face (Songs, Home)
- [x] design-direction's type families and decisions updated

## Comments

**2026-11-18 — built.** The Nerd Font build adds icon glyphs to the
private-use area and is not served as a webfont; nothing in the UI uses
those glyphs, so it only changes the face where it is installed locally.
