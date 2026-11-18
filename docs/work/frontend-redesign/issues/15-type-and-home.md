# 15 — Display face, logo and Home

**What to build:** the maintainer's three asks of 2026-11-18 about type and
Home. A display face with character for the brand, headings and titles,
still easy to read; the maiscope logo as the favicon, the app icons and
the top bar's brand; a Home with a picture (the maintainer's own, later)
and larger type.

**Blocked by:** 09, 13.

**Status:** done

- [x] `--f-display` (Space Grotesk) on `heading`, `title`, `display`, the
      brand and the card title; Inter and JetBrains Mono unchanged elsewhere
- [x] The maintainer's logo is `assets/brand/logo.svg`; favicon (SVG and
      ICO), Apple touch icon, Android and maskable icons rendered from it
- [x] Home: a picture slot (`assets/home/home-hero.*`, the logo until one is
      there), `display` at 72 (52 on phones), the subtitle at `heading`
- [x] The upstream arcade-songs icons and logos in `public/`, and the unused
      `src/static/` copy of them, are gone
- [x] design-direction records the type and colour decisions

## Comments

**2026-11-18 — built.** Picked by the maintainer from labelled options
(display face plus Inter; the favicon and a Home visual; logo supplied as
SVG). The logo's C2PA metadata block was left out of the repository copy:
it carries no drawing, and a hand-copied signed manifest would only be a
corrupt one; the maintainer can drop the original over the file. The
maskable and Apple icons put the logo on white inside the platforms' safe
zone; the others keep its transparency. Dark mode still does not persist
across a reload — it never did — and is outside this ticket.
