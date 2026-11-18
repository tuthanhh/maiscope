# Design direction

The visual system `apps/host` is built against: type, colour, spacing,
components and the shape of each screen. Tickets that touch the frontend
implement what is written here; a screen is done when it matches this
document, not when it looks right. Fields and data per screen are in
[`screen-data-requirements.md`](screen-data-requirements.md); this document
says what they look like.

Decided on 2026-11-17 against three references and one anti-reference:
acutic.net (three tones, colour only from artwork, mono for metadata),
sasakure.uk.com (artwork-led blocks separated by one rule and air),
Instagram's phone layout (an image grid, one bar of chrome), and
adxdls.saop.cc as the thing to avoid (five accents, seven radii, shadows,
badges on every card, navigation repeated three times).

## Summary

- **Quiet, then considered.** Two tones plus one grey, hairlines instead of
  boxes, no shadows, one radius, no colour except where it carries meaning.
- **Two faces.** A sans for titles and body, a mono for labels and numbers.
  Colour lives in the jackets and in the difficulty level; nothing else is
  coloured.
- **Two Catalog surfaces.** The grid with the song modal, and the chart page.
  The song page is gone; the modal has a URL.
- **One navigation.** Three destinations: Songs, Community, Play. Top bar
  on desktop, bottom bar on phones.
- **Community looks the same.** Same card, same grid, one tell: the
  designer line. Separation is structural, not visual.
- **Upload is one page**, reached from Community or from the chart page with
  the text prefilled. Rejection is inline and shows the failing token in
  context.

## Tokens

Three layers, as CSS custom properties on `:root`. Primitives are raw
values and never appear in a component. Semantic tokens are what
components read. Component tokens exist only where a component needs a
value no semantic token names. Dark mode swaps the semantic layer and
nothing else.

### Primitives

Neutral ramp, pure grey. Steps are chosen for the contrast pairs below,
not for evenness.

| Token | Value | Token | Value |
|---|---|---|---|
| `--n-0` | `#ffffff` | `--n-600` | `#767676` |
| `--n-50` | `#fafafa` | `--n-700` | `#444444` |
| `--n-100` | `#f4f4f4` | `--n-900` | `#111111` |
| `--n-200` | `#e6e6e6` | `--n-950` | `#0a0a0a` |
| `--n-300` | `#d4d4d4` | `--n-1000` | `#000000` |
| `--n-500` | `#8c8c8c` | | |

Type families:

| Token | Stack |
|---|---|
| `--f-sans` | `Inter, "Hiragino Sans", "Hiragino Kaku Gothic ProN", "Yu Gothic", "Noto Sans JP", "Noto Sans CJK JP", "Apple SD Gothic Neo", "Malgun Gothic", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif` |
| `--f-mono` | `"JetBrains Mono", ui-monospace, "SF Mono", Menlo, monospace` |
| `--f-display` | `"Space Grotesk"`, then the same CJK system faces and `system-ui` as `--f-sans` |

`--f-display` (2026-11-18, `frontend-redesign` 15) gives the brand, the
`heading`, `title` and `display` roles and the card title a face with more
character than Inter, where it is large or short enough to stay easy to
read; body text, controls and figures keep Inter and JetBrains Mono. Like
Inter it styles Latin only — a Japanese title shows in the system CJK face.

Inter ships as one Latin-subset variable file; the CJK names are system
fonts and nothing is downloaded for them. The `lang` attribute on `<html>`
follows the active locale so the browser picks the right CJK face for
Chinese, Japanese and Korean. Inter is a call, not a requirement: any
neutral grotesque with a Latin subset under 40 KB serves the same role.

Spacing on a 4px base, radius, line, motion:

| Token | Value | Token | Value |
|---|---|---|---|
| `--s-1` | 4px | `--s-8` | 32px |
| `--s-2` | 8px | `--s-12` | 48px |
| `--s-3` | 12px | `--s-16` | 64px |
| `--s-4` | 16px | `--r` | 4px |
| `--s-6` | 24px | `--r-round` | 50% |
| `--line-w` | 1px | `--dur` | 120ms |
| `--max-w` | 1200px | `--dur-slow` | 180ms |
| `--bp-md` | 768px | `--bp-lg` | 1100px |

One radius. `--r-round` is for avatars only. There is no shadow token and
no elevation scale; overlays separate from the page with a hairline and a
scrim.

### Semantic

| Token | Light | Dark | Role |
|---|---|---|---|
| `--bg` | `--n-0` | `--n-1000` | Page |
| `--bg-panel` | `--n-50` | `--n-950` | Drawer, modal body, table header |
| `--bg-sunken` | `--n-100` | `--n-950` | Jacket well, input background |
| `--fg` | `--n-900` | `--n-100` | Text, icons |
| `--fg-muted` | `--n-600` | `--n-500` | Secondary text; 4.5:1 or better on `--bg` |
| `--line` | `--n-200` | `--n-900` | Hairlines, dividers, card edges when needed |
| `--line-strong` | `--n-300` | `--n-700` | Input borders, focused hairline |
| `--inverse-bg` | `--n-900` | `--n-100` | Primary button, active chip |
| `--inverse-fg` | `--n-0` | `--n-1000` | Text on inverse |
| `--inverse-hover` | `--n-700` | `--n-300` | Primary button hover and active |
| `--scrim` | `rgb(255 255 255 / .7)` | `rgb(0 0 0 / .72)` | Behind modal and drawer |
| `--focus` | `--fg` | `--fg` | Focus outline |
| `--danger-fg` | `--n-900` | `--n-100` | Error text; errors are not red, they are framed |

Muted text passes AA in both modes (`#767676` on white is 4.54:1,
`#8c8c8c` on black is 6.1:1). Hairlines are deliberately below text
contrast; they divide, they do not inform.

**Difficulty colour is data, not a token.** It comes from the Catalog's
`difficulties[].color` and is set inline as `--diff` on the element that
shows a level. It is never used as a background for text. It appears as a
3px bar, a chip underline, or the colour of the level figure itself, so a
low-contrast value for one difficulty never makes a label unreadable.
`color` is optional: the 50 UTAGE difficulties carry none
([ADR-0022](../adr/0022-difficulties-derived-and-referenced.md)),
so `--diff` falls back to `--fg`, and the bar, underline and level figure
render in the text colour.

### Type scale

Seven roles replace the fourteen sizes in use today.

| Role | Family | Size / line | Weight | Tracking | Case | Used for |
|---|---|---|---|---|---|---|
| `label` | mono | 10 / 12 | 500 | 0.18em | upper | Section labels, field labels, table headers, the COMMUNITY mark |
| `meta` | mono | 12 / 16 | 400 | 0 | as is | Levels, BPM, note counts, dates, designer line, pager |
| `body` | sans | 14 / 21 | 400 | 0 | as is | Prose, card artist, inputs, buttons |
| `strong` | sans | 14 / 21 | 600 | 0 | as is | Card title, table song title |
| `heading` | display | 18 / 24 | 600 | -0.01em | as is | Page and drawer headings, Home subtitle |
| `title` | display | 24 / 28 | 600 | -0.015em | as is | Modal and chart page title |
| `display` | display | 72 / 72 | 700 | -0.03em | as is | Home brand only |

The card title is `strong` set in the display face. On phones `title`
drops to 20 and `display` to 52 / 56; nothing else changes.
Numbers are always mono, including inside sans lines ("174 BPM" sets the
figure in mono). Japanese text takes `font-feature-settings: "palt"` in
the sans and display roles so proportional kana does not gap.

### Component tokens

Only these; everything else reads the semantic layer directly.

| Token | Value |
|---|---|
| `--card-gap` | `--s-4` desktop, `--s-3` phone |
| `--control-h` | 36px (inputs, selects, buttons, chips are one height) |
| `--control-px` | `--s-3` |
| `--modal-w` | 540px |
| `--drawer-w` | 360px |
| `--nav-h` | 52px |
| `--jacket-bar` | 3px (the difficulty bar on a level pill) |

## Layout

- Page content sits in a column of `--max-w`, gutters `--s-4` on phones
  and `--s-6` from `--bp-md`.
- Sections are separated by one hairline and `--s-8` of air, never by a
  box. A section opens with a `label`.
- Two breakpoints. Below `--bp-md` is the phone layout; above `--bp-lg`
  the chart page gains its side column. Nothing else changes between.
- The grid: 2 columns on phones, 3 from `--bp-md`, 4 from `--bp-lg`, gap
  `--card-gap`, no card borders, no shadows.

## Interaction

- **Hover**: a border or background step, not opacity. Text links
  underline. Icon-only buttons may use opacity 0.7 since they have no
  border to move.
- **Focus**: `1px solid var(--focus)`, offset 2px, on `:focus-visible`
  only. No coloured ring.
- **Active chip or primary button**: inverse. That is the one strong
  state in the system; it is never used for two things on one screen.
- **Disabled**: opacity 0.4, `cursor: not-allowed`, no other change.
- **Motion**: colour and border over `--dur`; modal and drawer enter over
  `--dur-slow` with an 8px rise; `prefers-reduced-motion` removes the
  rise. Nothing else animates. The engine canvas is exempt.

## Components

State tables use: default, hover, active or selected, focus, disabled.
Blank means unchanged from default.

The components live in `apps/host/src/components/ui/`, one `Mv*` file each.
Their props follow one naming rule:

- **`label`** is the text that names the component. Where it is drawn
  (`MvField`, `MvError`) it is visible; where the component has no visible
  text of its own (an icon `MvButton`, a bare `MvTextInput` or `MvSelect`,
  `MvPager`, `MvModal` with nothing to point `labelledby` at, a `MvCard`
  whose text is not enough) it becomes the accessible name.
- **`<part>Label`** (`prevLabel`, `retryLabel`, `closeLabel`,
  `bookmarkLabel`) names a control the component draws inside itself.
- **`diff`** is a difficulty colour: a string, or `null` for none, which
  draws `--fg`. On `MvChip` alone, leaving it out (`undefined`) means the
  chip is a filter chip rather than a difficulty chip.
- **Dialogs** differ on purpose: `MvDrawer` is page state and takes
  `v-model`; `MvModal` is URL state, so it takes `open` and emits `close`
  for the owner to turn into a navigation.

### Button

Four kinds. Height `--control-h`, padding `0 var(--control-px)`, radius
`--r`, `body` type, mono allowed for a short technical label.

| Kind | Default | Hover | Active | Disabled | Use |
|---|---|---|---|---|---|
| primary | `--inverse-bg` / `--inverse-fg`, no border | background `--n-700` light, `--n-300` dark | same as hover | opacity 0.4 | One per screen: publish, load, resolve |
| ghost | transparent, `1px --line`, `--fg` | border `--line-strong` | background `--bg-panel` | opacity 0.4 | Everything secondary |
| text | transparent, no border, `--fg` | underline | | opacity 0.4 | Row actions: bookmark, play, publish, share |
| icon | 36 × 36, transparent, `--fg` | opacity 0.7 | background `--bg-panel` | opacity 0.4 | Close, theme, locale, bookmark on a jacket |

An icon button always has an accessible name. A primary button is never
inside a modal and a text button is never alone on a page.

### Chip

Filter toggle and difficulty selector. Height `--control-h`, padding
`0 var(--s-3)`, radius `--r`, `meta` type.

| State | Background | Border | Text |
|---|---|---|---|
| default | transparent | `--line` | `--fg` |
| hover | | `--line-strong` | |
| selected | `--inverse-bg` | none | `--inverse-fg` |
| selected, difficulty | transparent | `--line` | `--fg`, 2px underline in `--diff` |
| focus | | outline | |

Difficulty chips use the underline form so five selected colours never
sit as five filled blocks.

### Field

A `label` above a control, `--s-1` between, `--s-4` below the control.
Controls inside: text input, select, textarea, chip row, file picker.
Help or error text under the control in `meta`, `--fg-muted` for help,
`--fg` for error.

### Combobox

A text input over a list of known values (`MvCombobox`): Browse's title
search and the drawer's Artist. Focus or a click drops the whole list open, as a select would;
typing narrows it, names that start with the text first; arrows and Enter
choose, Escape closes the list and not the dialog around it. Free text
stays valid. The list is `--bg` with a `--line-strong` hairline, the active
row on `--bg-panel`, at most 200 rows with a line asking for more typing.

### Text input, select, textarea

Height `--control-h` (textarea 8 rows), padding `0 var(--control-px)`,
radius `--r`, `body` type, placeholder `--fg-muted`.

| State | Background | Border |
|---|---|---|
| default | `--bg-sunken` | `--line` |
| hover | | `--line-strong` |
| focus | `--bg` | `--line-strong` plus the focus outline |
| error | | `--fg` |
| disabled | `--bg-panel` | `--line`, opacity 0.4 |

Select shows a chevron icon in `--fg-muted`, native dropdown. Textarea
for chart text uses `--f-mono` at `meta` size with `line-height: 1.6`.

### Card

The grid item for both libraries.

```
┌──────────────┐
│              │
│   jacket     │   square, --bg-sunken behind, bookmark mark top-right
│              │
│         ▌14+ │   level pill, bottom-right
├──────────────┤
│ Title        │   strong, one line, ellipsis
│ Artist       │   body, --fg-muted, one line
│ ✎ designer   │   meta, --fg-muted; community cards only
└──────────────┘
```

No border, no background, no shadow. Hover lifts nothing; the title
underlines and the jacket gets a `--line-strong` hairline. The whole card
is one link. Jacket corners take `--r`.

**Level pill**: `meta` type at 600, `--fg` on `--bg`, padding `0 var(--s-2)`,
height 20px, radius `--r`, a `--jacket-bar` left bar in `--diff`, a 1px
`--bg` ring so it separates from any artwork.

**Jacket**: real cover when the song has one; the generated jacket seeded
from title and artist otherwise. A linked community song borrows its
official cover. No overlay text other than the pill and the bookmark mark.

### Table row

Reports queue and any future list of records. `meta` type for figures,
`body` for text, header row in `label` on `--bg-panel`, rows separated
by `--line`, row height 44px, no zebra striping. Hover sets the row
background to `--bg-panel`. One action per row at the right, as a text
button.

### Modal

The song modal. Width `--modal-w`, max 90vh, `--bg`, `1px --line`,
radius `--r`, over `--scrim`; closes on scrim tap, Escape and the icon
button. On phones it is a full-height sheet from the bottom with the same
content. Body padding `--s-4`. It has a URL (`?song=<id>`) so it can be
shared and so Back closes it.

### Drawer

Filters. On desktop a dialog floating in the middle of the screen,
`--modal-w` wide and at most 80vh, scrolling inside, with a hairline border
and `--r` corners; on phones full width from the bottom. `--bg-panel`, over
`--scrim`. (Until 2026-11-18 it was a `--drawer-w` panel against the right
edge; the maintainer preferred it centred, `frontend-redesign` 16.) Heading in
`heading`, a Reset text button, fields stacked, no footer. Applying is
live; there is no Apply button.

### Pager

One line of `meta`: `1 – 22 of 1 038` at the left, previous and next icon
buttons and `3 / 48` at the right. Hidden when there is one page.

### Empty state

A `body` sentence in `--fg-muted`, centred, `--s-12` of vertical padding,
no icon, no illustration, no border. Example: "No sheets match these
filters."

### Error state

Inline, never a toast. A block with `1px --fg` border, radius `--r`,
padding `--s-4`, a `label` reading what happened, then `body` text. Errors
are framed, not coloured. The upload rejection below is the full form of
this component; a fetch failure is the short form: one label, one
sentence, one text button to retry.

### Navigation

Desktop: a `--nav-h` top bar, hairline below. Left: the brand in `strong`.
Centre: Songs, Community, Play as text links, the active one in `--fg`
and the others in `--fg-muted`. Right: an icon button for theme and the
account avatar or a Sign in text button. Locale, About and Reports (for
moderators) live in the avatar menu.

Phone: the brand and the avatar in a slim top bar; a `--nav-h` bottom bar
with the three destinations as icon-above-label, `label` type, hairline
above. The bottom bar hides while the engine canvas is in view.

### Cover and jacket, loading

A jacket that has not loaded shows `--bg-sunken`; no spinner. There is no
page loading overlay: Browse, the one screen that needs the Catalog, shows
"Loading data…" in place of its empty state on first launch without a
cached Catalog. The chart page and a shared song link paint from their own
fetches and the reference tables, which an app-wide overlay would have
hidden (frontend-redesign 12).

## Screens

### Browse

```
┌─────────────────────────────────────────────────────────────────┐
│ maiscope          Songs   Community   Play              ◐  ◯    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ 🔍 Search title…                                   Filters ≡    │
│ [BASIC] [ADVANCED] [EXPERT] [MASTER] [Re:MASTER]                 │
│ LEVEL  [ 13 ▾ ] – [ 14+ ▾ ]              1 038 SHEETS  ☆ MY LIST │
│                                                                 │
│ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐                             │
│ │      │ │      │ │      │ │      │                             │
│ │  ▌14+│ │  ▌13 │ │  ▌14 │ │  ▌13+│                             │
│ ├──────┤ ├──────┤ ├──────┤ ├──────┤                             │
│ │Title │ │Title │ │Title │ │Title │                             │
│ │Artist│ │Artist│ │Artist│ │Artist│                             │
│ └──────┘ └──────┘ └──────┘ └──────┘                             │
│   …                                                             │
│                                                                 │
│ 1 – 22 of 1 038                                ‹  3 / 48  ›     │
└─────────────────────────────────────────────────────────────────┘
```

Surface: search, difficulty chips, level range, result count, My list
toggle, the grid/list toggle, the Filters button. Drawer: category, version, designer, region
and its override toggle, type chips, BPM range, exact-match toggles,
Reset. "Draw a random sheet" becomes a text button in the result line.
The list view, dropped at first, is back since 2026-11-18 (`frontend-redesign`
17): rows of a 48px jacket, the title in the display face over the artist,
type and difficulty in `meta`, the level pill and the bookmark mark; on
phones type and difficulty sit under the artist. The choice is remembered
per browser.

### Song modal

```
┌─────────────────────────────────────┐
│ jacket                       ✕      │
├─────────────────────────────────────┤
│ CATEGORY                   174 BPM  │
│ Title                               │
│ Artist                              │
│                                     │
│ [BAS] [ADV] [EXP] [MAS] [Re:MAS]    │   selected chip underlined in --diff
│                                     │
│ MASTER  14+  (14.7)                 │   level in --diff, figures mono
│ NOTE DESIGNER      someone          │
│ RELEASE            2025/09/11       │
│ TYPE               DX               │
│ TOTAL 1000 · TAP 500 · HOLD 100 …   │
│                                     │
│ ☆ bookmark   ▶ play                 │   text buttons, equal weight
└─────────────────────────────────────┘
```

Information first. Play plays the selected difficulty. No primary button,
no video search, no lock or globe icons. The song page that used to sit
behind this modal no longer exists; the modal's URL is the shareable link.

### Chart page

The playback surface (canvas, transport, speed sliders, the frame-pacing
overlay behind `?debug`) does not change. Everything around it does.

```
┌─────────────────────────────────────────────┬───────────────────┐
│                                             │ CHART TEXT        │
│                 canvas                      │ ┌───────────────┐ │
│                                             │ │ &title=…      │ │
│  ⟲  ▶     note speed ──●──  play speed ──●──│ │ (1)           │ │
├─────────────────────────────────────────────┤ │ …             │ │
│ Title                       MASTER 14+ (14.7)│ │               │ │
│ Artist · CATEGORY · 174 BPM                 │ └───────────────┘ │
│                                             │ [ load ]          │
│ ☆ bookmark   ⇡ share   ⇡ publish            │                   │
│                                             │                   │
│ ┌ jacket ┐ NOTE DESIGNER someone            │                   │
│ │        │ TOTAL 1000 · TAP 500 · HOLD 100 …│                   │
│ └────────┘ DX · PRiSM · 2025/09/11          │                   │
│                                             │                   │
│ OTHER DIFFICULTIES                          │                   │
│   EXPERT 12 · 720 notes                     │                   │
└─────────────────────────────────────────────┴───────────────────┘
```

Desktop from `--bp-lg`: the chart text panel is a side column the height
of the canvas, textarea plus a primary Load button. Below `--bp-lg` the
panel stacks under the title block. Title block in `title` and `meta`;
level in `--diff`. Actions are text buttons. Publish appears when a pasted
chart has loaded; signed-out users get the sign-in on tap.

Community variant: the designer line reads the chart's designer with the
uploader after it; the meta line gains "fan chart of X" when linked; no
version, release date or category.

### Community browse

The browse grid with a COMMUNITY label as the page heading, an Upload
ghost button beside it, the search box, and the designer line on every
card. No difficulty chips or level range on the surface; the filters are
title, artist and official song, in the drawer.

### Upload

One page, one column, `--max-w` 640px.

```
UPLOAD A CHART

MAIDATA.TXT
[ choose file ]   name.txt · 38 KB

TITLE
[ parsed from the file, editable            ]
ARTIST
[ parsed from the file, editable            ]

OFFICIAL SONG                                optional
[ search the catalog…                        ]
┌ jacket ┐ Title · Artist                 ✕

[ publish ]
```

Arriving from the chart page prefills the file field with the pasted text
(shown as "pasted chart · 38 KB"). Over 512 KB is refused before upload
with the short error form. Title and artist are editable so the file's
header can be corrected; the upload sends both as overrides, two
optional fields `community-charts` issue 02 gains.

### Upload rejection

The full error form, inline above the publish button, replacing nothing.

```
┌ REJECTED ──────────────────────────────────────────┐
│ inote_4 (MASTER), token 212                        │
│                                                    │
│   …,1h[4:1],2/5,│3$x│,4b,…                         │
│                                                    │
│ unknown modifier '$' after tap                     │
│ Fix the chart and choose the file again.           │
└────────────────────────────────────────────────────┘
```

Label REJECTED; first line in `meta` naming difficulty and token index;
the token in context in a mono block, four tokens either side, the failing
one boxed with `1px --fg`; the cause in `body`; one sentence of next step.
Nothing is red.

### Reports queue

A table. Columns: target (song, and chart when set), reporter, reason
truncated to one line, age in `meta`, and a Resolve text button. Open
reports first. Resolve opens a one-field inline row with a note input and
a primary button; resolving does not take anything down, and the row
stays, struck through in `--fg-muted`, when the resolved filter is on.

### Home

A picture, then the brand in `display`, the subtitle in `heading`
`--fg-muted`, and two buttons, primary for Songs and ghost for Play.
Community is reached from the bar. The picture is the maintainer's
image when `apps/host/src/assets/home/` holds a `home-hero` file, and the
maiscope logo otherwise. The logo (`assets/brand/logo.svg`) is the one
place the shell carries colour besides the difficulty colours: it is the
brand, drawn by the maintainer, and it reads on both themes as drawn. It
also marks the top bar beside the wordmark and is the favicon and app
icon.

## Worked example: browse in the new system

The browse screen, element by element, as a checklist for the restyle
of the existing screens.

| Element | Spec |
|---|---|
| Page column | `--max-w`, gutters `--s-6` desktop, `--s-4` phone |
| Search box | text input, full width on phone, 360px on desktop, search icon in `--fg-muted` inside |
| Filters button | ghost, with a sliders icon, opens the drawer |
| Difficulty chips | chip row, difficulty form, `--s-2` gaps, wraps on phone |
| Level range | two selects at 96px with an en dash in `--fg-muted` between, under a LEVEL label |
| Result line | `label` count at left, My list text button with the bookmark icon (the card's mark; one concept, one icon) and the bookmark count, Draw text button, right |
| Grid | 2 / 3 / 4 columns, `--card-gap`, cards as specified, 24 a page (whole rows at every column count) |
| Card title | `strong`, ellipsis |
| Card artist | `body`, `--fg-muted`, ellipsis |
| Level pill | as specified, `--diff` from the sheet's difficulty |
| Bookmark mark | icon button 28px on the jacket's top-right, filled when bookmarked |
| Empty | "No sheets match these filters." or "Your list is empty." |
| Pager | as specified |
| Modal | opens on card tap, URL updates, Back closes |

## Not changing

- The visualizer's playback surface: canvas, transport, speed sliders,
  the `?debug` overlay. The page is split into the chart page
  layout for legibility; nothing here restyles playback.
- The engine's own rendering: notes, judgement ring, sprites.
- The generated jacket algorithm; it is the identity of standalone fan
  songs.

## Copy and locales

Every label is multiplied by nine locales. New strings this direction
introduces, so locale work can be sized: Community, Play, Upload, Publish,
Filters, Reset, My list, Draw, Chart text, Load, Share, Rejected, Fix the
chart and choose the file again, Official song, optional, fan chart of,
Other difficulties, Reports, Resolve, Sign in, deleted user, No sheets
match these filters, Your list is empty. Strings removed: Advanced search,
Search on YouTube, Sheet data, the list and grid toggle labels.

## Decisions recorded here

| Decision | Chosen | Rejected |
|---|---|---|
| Where style comes from | Sans and mono pairing, monochrome | Mono only; one accent colour |
| Sans coverage | Latin webfont, system CJK | CJK webfont; system-only stack |
| Browse view | Grid default with list kept (reversed 2026-11-18; was grid only) | Grid only |
| Filters | Search plus difficulty and level on the surface, rest in a drawer | Search only; full panel collapsed |
| Card tap | Song modal, information first | Song page; two taps on the card |
| Song page | Removed; modal has a URL | Kept as the comparison table |
| Chart page | Watch-page layout, chart text as a side column | Side card plus paste; canvas only |
| Navigation | Three destinations, bottom bar on phones | Five items; two items plus avatar |
| Community look | Same card, designer line as the tell | Tinted surface; distinct card |
| Upload from the player | Navigate to the one upload page prefilled | A sheet over the player |
| Token structure | Three layers, state tables | Two layers; plus a style page now |
| Muted text | Raised to pass AA | Kept as is |
| Display face (2026-11-18) | Space Grotesk for brand, headings, titles; Inter body | One rounded face everywhere; a serif display face |
| Brand colour (2026-11-18) | The maintainer's coloured logo, the shell stays monochrome | A monochrome mark |
| Filters container (2026-11-18) | Centred dialog on desktop, bottom sheet on phones | Edge drawer |
| Artist filter (2026-11-18) | Combobox: full list on focus, narrowed by typing | `<datalist>` (no open-on-focus, unstyled); plain select of every artist |
