# Screen data requirements

What each screen of `apps/host` renders, field by field, which of those
fields the client derives rather than receives, and what each screen needs
delivered. This is the consumer specification the next API payload is
designed against: it says what a screen *shows*, not what an endpoint
returns. Endpoint shape is decided in the API contract, which reads this
document.

Grounded against the Vue pages, components and composables under
`apps/host/src/` and the wire shapes in `apps/server/src/types.rs`. The
community screens have no code yet; their section is grounded in the
`community-charts` tickets and marked as such. The delivery decisions are
recorded in [ADR-0021](../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md);
the rest of the decisions below were taken on 2026-11-17 and are reversible.

> **Read as v1 as-built, 2026-11-17.** "Official screens", "Derived fields:
> where they stand" and their tables describe the screens this document was
> written against — the song page, the sheet dialog, in-memory bookmarks
> keyed by `sheetExpr`, a 22-sheet pager. `frontend-redesign` replaced them
> (2026-11-18): the song modal with a URL, a `sheet.key` of song id, type
> and difficulty code, bookmarks persisted in `localStorage`, 24 cards a
> page. "What each screen needs delivered" and "Community screens" are
> still the current requirements; the ticket files under
> `docs/work/frontend-redesign/issues/` record what was built.

## Summary

- Of the five derived fields, **two are rendered** (`imageUrl`, and
  `sheetExpr` as a list key and re-resolution key) and **three are never
  read** (`songNo`, `notePercents`, `imageUrlM`).
- The prototype chain (`$canonicalSheet`, sheet inherits song) is what lets
  every row show a title. It is replaced by an explicit `song` reference on
  each Sheet; the Catalog keeps nesting on the wire.
- **Browse and the upload chooser need the whole Catalog.** Every other
  screen needs one song, one sheet, one community song, or a page.
- **Reference tables load at boot on their own**, so a screen that opens
  first can resolve names without the 3MB snapshot.
- `difficulties[].color` becomes the one source of difficulty colour. The
  other fields shipped but never rendered (`isNew`, `isSpecial`,
  `versions[].releaseDate`, type and difficulty icons) are not required by
  any screen.

## Vocabulary

Domain terms follow the root [`CONTEXT.md`](../../CONTEXT.md).

| Term | Meaning here |
|---|---|
| Raw field | Sent by the server as stored |
| Derived field | Computed by the client after fetch, in `utils/data.ts` |
| Inherited | Read off a Sheet but stored on its Song; reaches the Sheet through the prototype chain today |
| Snapshot | The whole Catalog held in memory and in IndexedDB |
| Page | One request returning a bounded list for one view of the results |

## Official screens

### App shell

Navigation, menu, dark-mode toggle. Renders nothing from the Catalog and
waits on nothing: it fetches the reference tables (`GET /api/reference`)
and starts the Catalog load at boot. Browse shows its own loading line while
the Catalog is in flight.

### Home

Static: brand, subtitle, two links. No Catalog fields.

### About

Static prose and links. No Catalog fields.

### Browse

The default landing screen for data. One search box, an advanced filter
panel, a result bar, a list or grid of sheets, and a pager.

**Filter options are aggregates over the whole Catalog.** The panel is
populated before any filter is applied:

| Control | Built from |
|---|---|
| Category select | `categories[]`, in Catalog order |
| Version select | `versions[]`, newest first, labelled by `abbr` falling back to `version` |
| Note designer select | Every sheet's `noteDesigner`, grouped, labelled with its sheet count, ordered by count |
| Level min and max selects | Every sheet's distinct `levelValue`, labelled by `level`, excluding levels ending in `?` |
| Region select | `regions[]`, each twice: available in, and not available in |
| Type chips | `types[]`, labelled by `abbr` |
| Difficulty chips | `difficulties[]`, labelled by `name` |

**Filtering reads these sheet fields**: `title`, `artist`, `category` (split
on `|`, any part may match), `version`, `type`, `difficulty`, `levelValue`,
`internalLevelValue`, `bpm`, `noteDesigner`, `regions`. With "use region
override" on and a region selected, the sheet's `regionOverrides[region]`
supplies `level`, `levelValue`, `internalLevel`, `internalLevelValue` and
`noteDesigner` for the checks (the filter compares the numeric values).

**Each result renders:**

| Element | Field | Source |
|---|---|---|
| Cover | `imageUrl`; `title` and `artist` seed the generated fallback jacket | Derived, inherited |
| Level | `level` | Raw |
| Title | `title` | Inherited |
| Artist | `artist` | Inherited |
| Category (list only) | `category` | Inherited |
| Type (list only) | `type` resolved to `abbr` through the type reference table | Raw plus reference |
| BPM (list only) | `bpm` | Inherited |
| Bookmark state | Membership in the bookmark list | Client state |
| Row key | `sheetExpr` | Derived |

The result count, the pager (22 per page, sliced client-side) and "draw a
random sheet" all operate on the filtered array in memory.

**Bookmarks** are an in-memory list of sheet objects, not persisted. When a
fresh Catalog replaces the graph, each bookmark is re-resolved by `sheetExpr`
and dropped if no sheet carries that key any more.

### Song detail

Reached by `?id=<songId>` and found by scanning the Catalog's song list; a
miss logs a warning and renders an empty page. Renders:

| Element | Field |
|---|---|
| Heading | `title` |
| Cover | `imageUrl` (derived), `title` as alt text |
| Info table | `category`, `title`, `artist`, `bpm`, `releaseDate` (hyphens shown as slashes), `version` |

Then one row per sheet of the song:

| Column | Field |
|---|---|
| Type | `type` resolved to `abbr` |
| Difficulty | `difficulty` resolved to `name`; opens the sheet dialog |
| Level | `level` |
| Internal level | `internalLevel` |
| Total, tap, hold, slide, touch, break | `noteCounts[key]` |
| Warning mark on total | Fires when the five parts do not sum to `total` |
| Note designer | `noteDesigner` |

### Sheet dialog

Opened from browse or song detail. Finds the sheet's siblings by scanning the
Catalog for the song with the same `songId`, so difficulty chips can switch
within the song. Renders:

| Element | Field |
|---|---|
| Cover | `imageUrl`, with `title` and `artist` as fallback seed |
| Category badge and label | `category` |
| BPM | `bpm` |
| Title, artist | `title`, `artist` |
| Difficulty chips | Each sibling's `difficulty` resolved to `name` |
| Level line | `difficulty` name, `level`, `internalLevelValue` to one decimal |
| Meta rows | `noteDesigner`, `releaseDate`, `type` resolved to `name`, `comment` |
| Lock icon | `isLocked` |
| Globe icon | Static; reads no field |
| Bookmark state | Membership in the bookmark list |
| Video search | Builds a search URL from the game title, `title`, and the difficulty name; a placeholder sheet may carry `searchUrl` instead |
| Visualizer button | `hasChart` chooses the ready style and tooltip; the link carries `songId`, `type`, `difficulty` |

### Visualizer

Two inputs: a deep link `?songId=&type=&difficulty=`, or pasted chart text.
The deep link fetches two things independently: the sheet's metadata from
the per-sheet endpoint and the chart text from the chart endpoint. A pasted
chart has no metadata card.

The per-sheet response already flattens song fields into the sheet. The page
then resolves `imageUrl` itself from `imageName`, duplicating the Catalog
derivation, because the response bypasses `preprocessData`.

**Metadata card renders:**

| Element | Field |
|---|---|
| Cover | `imageUrl` (resolved in the page), `title` and `artist` fallback seed |
| Category, title, artist | `category`, `title`, `artist` |
| Difficulty banner | `difficulty` resolved to `name` through the Catalog reference table; accent colour from a map hard-coded in the page, not from `difficulties[].color` |
| Level | `level`, `internalLevelValue` to one decimal |
| Type | `type` resolved to `name` through the Catalog reference table |
| BPM | `bpm`, when present |
| Total notes | `noteCounts.total` |
| Note breakdown | Every non-null `noteCounts` entry except `total` |
| Meta rows | `noteDesigner`, `version`, `releaseDate`, each only when present |
| Video search | Same construction as the dialog |

**The visualizer depends on the Catalog without fetching it.** Type and
difficulty names come from the reference tables the Catalog store builds.
When the Catalog has not loaded, both fall back to the upper-cased code,
silently.

The playback surface (canvas, transport, speed sliders, frame-pacing overlay)
reads engine state only. It is out of scope for the redesign and for this
document.

**In v2** ([ADR-0026](../adr/0026-v2-api-carries-codes-and-read-fields-only.md)),
every `songId` above becomes the song's minted `id`, and the visualizer deep
link is `?song=<id>&type=<type>&difficulty=<code>`. The sheet dialog's
`searchUrl` belongs to the client's placeholder sheets (`utils/sheet.ts`);
the server never sends it.

## Derived fields: where they stand

For each field the client computes today, where it is read and what the
next client does with it.

| Field | Read by | Outcome | Reason |
|---|---|---|---|
| `songNo` | Nothing | **Gone** | Never rendered. It encodes Catalog position, which the server already holds as `source_index`; a list that wants newest-first asks for that order |
| `imageUrl` | Browse, song detail, dialog, visualizer | **Client, one helper** | Needs the asset host, which is client configuration (`GAME.dataSourceUrl`) and would otherwise be a server deploy to change. Today the derivation exists twice (Catalog and visualizer); it moves into one function every entry path calls |
| `imageUrlM` | Nothing | **Gone** | The medium cover is never requested |
| `sheetExpr` | Browse list keys; bookmark re-resolution; the dialog's visualizer link; the per-sheet fetch | **Gone as identity** | Title-derived, and the composite is never displayed. Every use is "a stable key for this sheet", which the minted song id plus type and difficulty code provides ([ADR-0026](../adr/0026-v2-api-carries-codes-and-read-fields-only.md)); no sheet id is minted |
| `notePercents` | Nothing | **Gone** | Never rendered. Recomputable from `noteCounts` in one line if a screen ever wants it |
| `$canonicalSheet` and the prototype chain | Browse (filter, map back to canonical), bookmarks (identity check), dialog (bookmark guard) | **Gone; explicit `song` reference** | Exists so a sheet can show its song's title without a join, and so a region-override object can be swapped in and swapped back. The Catalog keeps nesting `songs[].sheets[]` on the wire (compact: 1.8k songs, 7.3k sheets); the client builds its flat sheet list with `sheet.song` pointing at the Song object. Region overrides stay a map on the sheet; the filter reads the override's values instead of swapping objects. Shipping the Catalog flat, song fields repeated per sheet, was rejected as a roughly fourfold growth of song fields on the wire for no screen that needs it |

**Name resolution stays client-side.** Type and difficulty names, type
abbreviations and version abbreviations are resolved from the five reference
tables on every screen that shows them. Payloads carry codes, not names.
Carrying names on the wire was rejected: two sources of truth for every
label.

**Difficulty colour has one source**, `difficulties[].color`. The
visualizer's hard-coded map goes.

## What each screen needs delivered

The v1 answer was the whole snapshot for everything, for the reason in
[ADR-0007](../adr/0007-client-side-filtering.md): the Catalog is small enough
to hold, and the offline first paint and the bookmark re-resolution both ride
on having all of it. The v2 answer keeps the snapshot where aggregates need
it and fetches one thing everywhere else
([ADR-0021](../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md)).

| Screen | Needs | Why |
|---|---|---|
| Any screen, at boot | **Reference tables** | Five small ordered lists, fetched on their own before any screen renders, so names resolve without the snapshot |
| Browse | **Snapshot** | The filter panel is built from aggregates over every sheet (designer counts, distinct levels), and filtering runs in memory. No server search exists |
| Song modal, opened by URL | **One song** with its sheets | A shared link to a song must not cost the snapshot. The [design direction](design-direction.md) removes the song page; the modal is the shareable surface and this fetch serves it |
| Song modal, opened from the grid | The song in hand | Siblings come from the snapshot already loaded |
| Visualizer, Catalog chart | **One sheet** with its song fields, plus the chart text | Entered by the song's id, the sheet's type and its difficulty code ([ADR-0026](../adr/0026-v2-api-carries-codes-and-read-fields-only.md)) |
| Visualizer, community chart | **One community chart** with its song fields, plus the chart text | Entered by `?communityChart=<id>`; the card variant follows the parameter. Two explicit parameters were chosen over one server-disambiguated id because nothing enforces that ids never collide across the two libraries |
| Community browse | **Page** | Paginated from the first ticket; never part of the Catalog |
| Community detail | **One community song** with its charts | No chart text |
| Upload chooser | **Snapshot** | Searches official titles in memory. An uploader is logged in and few; the snapshot is cached after first use. A title-only search endpoint for this one picker was rejected as a second filter path |
| Reports queue | **Page** | Moderator list, paginated, filtered by resolved |

**Bookmarks persist per browser**, keyed by song id, type and difficulty code, so they
survive a reload and a Catalog refresh. Per-account bookmarks on the server
are a later addition that keeps the same client key.

## Community screens

No code exists. Fields are taken from the `community-charts` tickets
(tables, upload, browse and detail, takedown, reports) and the
`frontend-redesign` community-screens ticket, plus the decisions above where
the tickets were silent.

**Community browse** (a page):

| Element | Field |
|---|---|
| Cover | The linked official song's cover when linked; the generated jacket from `title` and `artist` otherwise. The library has no cover art of its own |
| Title, artist | `title`, `artist` |
| BPM, genre, version | `bpm`, `genre`, `version`, as parsed from the maidata header |
| Linked official song | The official song's id, `title` and `imageName`, carried in the payload so the row renders "fan chart of X" without the snapshot |
| Charts summary | Per chart: `difficulty` (the bare code, resolved to a name through the difficulty reference table), `level` |
| Uploader | GitHub login. A tombstoned account renders as a deleted user |
| Uploaded | `createdAt` |
| Filters | `title`, `artist`, official song |
| Pager | Total from the `X-Total-Count` header |

**Community detail** (one song):

| Element | Field |
|---|---|
| Song header | As browse, uploader included |
| Chart rows | `difficulty`, `level`, `designer`, `noteCounts` (same keys as official: total, tap, hold, slide, touch, break) |
| Play | Link to the visualizer by chart id; chart text comes with the visualizer's own fetch, not with the page |
| Takedown | Shown when the viewer is the uploader or a moderator; needs the viewer's id and role from the session, and the song's `uploaderUserId` |
| Report | Target song or one chart by id, with a reason |

**Upload:**

| Element | Field |
|---|---|
| File picker | One `maidata.txt`; the server parses the header |
| Title, artist | Parsed from the file, shown editable; sent as overrides when changed |
| Official-song chooser | Searches the snapshot by title in memory; links by the song's minted id |
| Success | The new song's id, to navigate to its detail |
| Rejection | Which difficulty (`inote_N`), the token index, the token text, and the cause. Four fields; the ticket forbids flattening them to one message |
| Size limit | 512 KB, enforced by the server; the client can pre-check the file size against the same number |

**Reports queue** (moderator, a page):

| Element | Field |
|---|---|
| Row | Target song and, when set, target chart; `reason`; reporter; `createdAt`; `resolvedAt`, `resolvedBy`, `resolution` when resolved |
| Filter | Resolved or open |
| Resolve action | Records who and what; does not take anything down |
| Pager | Total from the `X-Total-Count` header |

**Visualizer for a community chart:** the metadata card shows `title`,
`artist`, `bpm`, `difficulty`, `level`, `designer`, `noteCounts`, the
uploader, and the linked official song when there is one; no version,
release date or category. The card today keys every optional row on
presence, so the community variant is the same card with two rows added and
three absent.

## Catalog fields not required by any screen

Shipped in `GET /catalog` today, read by no screen and no filter, and not
required by the redesign:

| Field | Note |
|---|---|
| `songs[].isNew` | Nothing reads it |
| `sheets[].isSpecial` | Nothing reads it |
| `versions[].releaseDate` | Only `abbr` and `version` are used |
| `types[].iconUrl`, `types[].iconHeight` | The helper exists; no component calls it |
| `difficulties[].iconUrl`, `difficulties[].iconHeight` | Same |
| `updateTime` | Read only as the "Catalog not loaded yet" sentinel for the filter panel; v2 drops it, and the panel reads the store's loading status instead (`frontend-redesign` 04) |

The design direction may add any of these back, which reopens the payload
for that field only. Whether the server keeps sending them is a question for
the API contract; this document records that no screen needs them.
