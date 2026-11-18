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

**2026-11-18 — review.** Applied: the hero box has a fixed height, so the
picture loading moves nothing; with several `home-hero` files the README's
order decides and development warns; the service worker precaches `avif`
and `jpg` too, so any accepted picture works offline; the unused Space
Grotesk 500 weight is no longer requested; the `palt` note names the
display roles; `index.html` says `favicon.svg` is a copy of the logo.
Kept: the hero is decorative (`alt=""`) — the page's heading names it —
and the subtitle stays in `heading`, as asked ("the font is quite
small"). Open, for the maintainer: the logo's handle (`#3B4A54`) is about
2.3:1 on the dark theme's black; a dark-theme variant with a lighter
handle is theirs to approve. Not new: the webfonts (Inter and JetBrains
Mono too) come from Google Fonts and are not cached for offline use;
self-hosting them is its own change.
