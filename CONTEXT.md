# Domain glossary

Vocabulary for maiscope. Terms are defined here once; everything else uses them.

## Terms

### Catalog

The official maimai song library as mirrored from upstream: songs, sheets,
and the five reference tables. Delivered to the client as one snapshot.
Never contains community content.

- **Avoid:** "official library" (use Catalog), "data.json" (the upstream
  file, not the concept).

### Community library

Charts uploaded by users, standalone or linked to a Catalog song. Browsed
page by page; never part of the Catalog snapshot.

### Song

One piece of music in either library. In the Catalog it owns display fields
(title, artist, category, BPM, version, release date, cover) and a set of
Sheets.

### Sheet

One playable slot of a Catalog Song: a type (standard or DX) and a
difficulty. Carries level, note designer, note counts, region availability.

- **Avoid:** "chart" when the metadata is meant; a Chart is the note text.

### Chart

The simai note text that the visualizer plays. A Catalog Sheet has at most
one; a Community Song has one per difficulty.

### Reference table

One of categories, versions, types, difficulties, regions: an ordered list
the client resolves codes against for display.

- **Avoid:** "lookup" (the act), "enum".

### Song modal

The information view for a Catalog Song: opened over the grid, addressable
by URL, one difficulty shown at a time. There is no song page.

### Chart page

The page that plays one Chart: the visualizer's playback surface plus the
chart's title block, actions, description and chart text panel.

- **Avoid:** "visualizer page" in design documents; "visualizer" names the
  engine surface inside it.

### Designer line

The third line of a Community card: the chart's note designer from the file,
falling back to the uploader. The one visual tell between the libraries.
