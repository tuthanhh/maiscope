# Notations of simai

> **Source:** [w.atwiki.jp/simai/pages/1003.html](https://w.atwiki.jp/simai/pages/1003.html)
> **Author:** simai (Celeca) · **Last updated:** July 25, 2023

> *Editor's note: this is a cleaned-up pass over a plain-text export of the wiki page above. The duplicate table of contents has been merged into one linked outline, notation examples now use `code formatting` instead of 【full-width brackets】, and a few example lists have been turned into tables for scanability. Two things referenced by the original didn't survive the export — the start/end combination table for SLIDE shapes, and the touch-sensor diagram — see the source link for those.*

## Table of Contents

- [What is simai's Notation?](#what-is-simais-notation)
- [Location of Note Numbers](#location-of-note-numbers)
- [Chart Definition and Ending](#chart-definition-and-ending)
- [Definition of BPM & Length of Notes](#definition-of-bpm--length-of-notes)
  - [How to Define the BPM](#how-to-define-the-bpm)
  - [How to Define the Length Divider](#how-to-define-the-length-divider)
- [TAP](#tap)
- [HOLD](#hold)
- [SLIDE (Basics & Shapes)](#slide-basics--shapes)
- [SLIDE (Multiple & Chaining)](#slide-multiple--chaining)
  - [Multiple SLIDE](#multiple-slide)
  - [Chaining SLIDE](#chaining-slide)
- [TOUCH, TOUCH HOLD, & Firework Effect](#touch-touch-hold--firework-effect)
  - [TOUCH](#touch)
  - [TOUCH HOLD](#touch-hold)
  - [Firework Effect](#firework-effect)
- [EACH](#each)
- [EX Notes](#ex-notes)
- [Other Notations](#other-notations)
  - [Change Normal TAP to Star-shaped TAP](#change-normal-tap-to-star-shaped-tap)
  - [Change Star-shaped TAP to Normal TAP](#change-star-shaped-tap-to-normal-tap)
  - [Notations of Pseudo-TAP HOLD and Pseudo-TOUCH TOUCH HOLD](#notations-of-pseudo-tap-hold-and-pseudo-touch-touch-hold)
  - [Pseudo EACH](#pseudo-each)
  - [SLIDE without Star-shaped TAP](#slide-without-star-shaped-tap)
- [Notes](#notes)

> BREAK's notation is summarized within the TAP, HOLD, and SLIDE sections rather than given its own section.

---

## What is simai's Notation?

simai's notation was created by Celeca in 2013 to denote the various types of notes that appear in maimai, using plain text. By combining half-width alphanumeric characters according to the rules below, any maimai chart can in theory be denoted accurately using plain text alone.

The notation was first used in *simai*, a chart simulator for maimai first released on February 5, 2013, by Celeca. Since then, various maimai-inspired simulators have been created by volunteers, and simai's notation has often been reused as their chart format.

Celeca closed simai to the public on February 5, 2023. A certain number of users still use simai's notation, which is why Celeca created this reference page. Some simulators have extended simai's notation further in their own implementations, but only the rules officially defined as "simai's notation" are listed here.

If you're building a simulator that supports simai's notation, it's recommended that it handle the rules on this page. Celeca would appreciate being informed of such simulators (not required, but welcomed) — Celeca holds the rights to simai's notation.

## Location of Note Numbers

maimai's buttons and sensor areas are numbered 1–8 clockwise, as shown below:

```
    ⑧①
  ⑦    ②
  ⑥    ③
    ⑤④
```

For sensor areas, each numbered location is further subdivided into 4 alphabet-numbered sensor areas (explained in [TOUCH](#touch)). These numbers are used throughout the notation to represent a note's position — worth memorizing.

## Chart Definition and Ending

A chart in simai's notation is a series of commas and note notations. Every comma occupies a fixed length of time — e.g., if a comma is 1 second long and there are 10 commas in the chart, the chart's total length is 10 × 1s = 10 seconds.

`1,1,1,1,1,1,1,1,1,1,1,1,1,` represents 10 TAP notes at BUTTON-1, one per second.

Once a chart's definition begins, it's built from commas and note notations. However, songs with a fade-in section would otherwise need a large number of leading commas just to reach the first note. To avoid this, simai defines a `first` parameter: how many seconds should pass after the music starts playing before the chart begins at its first comma.

Because the chart's start timing needs to be precise, `first` is usually given with decimal places (e.g., `1.234` seconds). With `first = 1.234`, the same `1,1,1,1,1,1,1,1,1,1,1,` chart places its 10 TAP notes at 1.234, 2.234, 3.234, … 10.234 seconds after the music begins.

Just like sheet music needs an ending notation, a simai chart needs one too — otherwise the music could play indefinitely.

> In simai, if the chart definition runs longer than the MP3, the chart still ends when the MP3 finishes.

The end of a chart is denoted by `E`. With `first = 1.234` and the chart `1,1,1,1,1,1,1,1,1,1,1,1,1,E`, the chart progresses to 10.234 seconds as above, and since the final comma is also 1 second long, the chart ends at 11.234 seconds.

A full chart definition is one long string of commas, note notations, and a trailing `E`. Line breaks, spaces, and tabs can be inserted anywhere in the string for readability — they're ignored during parsing.

## Definition of BPM & Length of Notes

In the examples above, each comma is 1 second long. But real charts need to sync to a song's tempo — e.g., a 16th note at 174 BPM is 0.08620689655… seconds long, and calculating that by hand for every comma would be impractical.

The per-comma length can instead be set by specifying a BPM and/or a length divider.

### How to Define the BPM

```
(120)
```

Enclose a BPM value in round brackets. Since BPM needs to be accurate, decimal places are allowed.

### How to Define the Length Divider

```
{2}
```

Enclose the dividing value in curly brackets — e.g., `{4}` sets the per-comma length to a quarter note, `{8}` to an 8th note, `{1}` to a whole note. Decimal values are allowed but best avoided, since they can be confusing.

BPM must be defined *before* the length divider — the length of a note can't be calculated without a BPM. When both are defined together, the divider comes after the BPM: `(120){2}` sets the per-comma length to a half note at 120 BPM (exactly 1 second).

Given BPM `B` and length divider `T`, the per-comma length is:

```
per-comma length (s) = 240 / B / T
```

For cases where BPM is unknown or the goal is to sync precisely to vocals or a sound effect, the per-comma length can be set directly in seconds: `{#0.35}` sets it to 0.35 seconds. Since this determines the length directly, no separate length divider is needed.

Both the BPM and length divider parameters can be redefined anywhere in the chart as needed.

## TAP

From here on, this covers the note notations placed before a comma.

TAP is the most basic note type — a button number followed by a comma.

| Notation | Meaning |
|---|---|
| `1,` | TAP at BUTTON-1 |
| `5,` | TAP at BUTTON-5 |

To make a TAP a BREAK TAP, add `b` before the comma: `1b,`, `5b,`.

## HOLD

A HOLD needs a button number and a held-down length. Held-down length uses the same `[divider:multiplier]` format used by SLIDE: the number before the colon is the length divider, and the number after is the multiplier. So `[2:1]` = 1 × half note = one half note held.

A HOLD is written as a button number, then `h` (marking it as a HOLD), then the held-down length.

| Notation | Meaning |
|---|---|
| `5h[2:1],` | HOLD at BUTTON-5, held for one half note |
| `4h[#5.678],` | HOLD at BUTTON-4, held for exactly 5.678 seconds |
| `4h[150#2:1],` | HOLD at BUTTON-4, held for one half note at 150 BPM |

To make a BREAK HOLD, add `b` after the `h`: `5hb[2:1],` (or `5bh[2:1],` — order doesn't matter).

## SLIDE (Basics & Shapes)

A SLIDE combines: **starting point, track shape, ending point, and tracing length**.

`1-4[8:3],` = BUTTON-1 to BUTTON-4, straight shape, tracing length of three 8th notes.

The `[8:3]` part uses the same length notation as HOLD. For SLIDE specifically, there's also a *waiting time* — the pause after the approaching star-shaped TAP reaches the judgment line and before the tracing star starts moving. By default this is one beat at the current BPM, but it can be overridden:

| Notation | Waiting time | Tracing length |
|---|---|---|
| `1-4[160#8:3],` | 1 beat at 160 BPM | three 8th notes at 160 BPM |
| `1-4[160#2],` | 1 beat at 160 BPM | 2 seconds |
| `1-4[3##1.5],` | 3 seconds | 1.5 seconds |
| `1-4[3##8:3],` | 3 seconds | three 8th notes at the current BPM |
| `1-4[3##160#8:3],` | 3 seconds | three 8th notes at 160 BPM |

*(Status for the above: BUTTON-1 to BUTTON-4, straight shape, 120 BPM.)*

### Track shapes

| Symbol | Shape | Description |
|---|---|---|
| `-` | Straight | Connects start to end in a straight line. |
| `>` `<` `^` | Arc | Follows the circular judgment line. Use `>` if travel is rightward, `<` if leftward. If the distance is shorter than half the circle, `^` works for either direction. |
| `v` | V-shape | Two straight lines, meeting at the screen center as the turning point. |
| `p` `q` | P-shape / Q-shape | Curves around the screen center. |
| `s` `z` | Thunderbolt | Three short lines shaped like a thunderbolt (⚡). |
| `pp` `qq` | Grand P-shape / Grand Q-shape | Curves along an imaginary circle tangent to both the screen center and the circular judgment line. |
| `V` | Grand V-shape | Two straight lines through a middle turning point; the segment from start to the turning point is always short. |
| `w` | Fan | One start expanding into three separate ends, like a folding hand fan — spawns three tracing stars. |

For Grand V-shape (`V`), the turning point needs its own button number, given between the start and end: `1V35` = start at BUTTON-1, turn at BUTTON-3, end at BUTTON-5.

Every SLIDE moves at a constant tracing speed from start to end. To make it a BREAK SLIDE, add `b` after the closing bracket: `1-4[8:3]b,`.

> **Note:** the original page includes a table here of which start/end combinations are valid for each shape — it didn't survive the plain-text export. See the [source page](https://w.atwiki.jp/simai/pages/1003.html) for the diagram.

## SLIDE (Multiple & Chaining)

### Multiple SLIDE

A **Multiple SLIDE** is a single star-shaped TAP with two or more independent arrow tracks. Each track has its own parameters (only the starting point is shared), and tracing speed can differ between them — but since all tracks start moving at the same time, they're treated as an EACH.

`1-4[4:3]*-6[8:5],` = BUTTON-1→4 straight (3 quarter notes) *and* BUTTON-1→6 arc (5 eighth notes).

The starting button number isn't repeated for the second track onward. Add another `*` to chain a third track, and so on.

### Chaining SLIDE

A **Chaining SLIDE** joins multiple arrow tracks end-to-start into a single continuous track. For example, `1V75` is internally two tracks (`1-7` and `7-5`) joined together — and this joining works for any shape, not just Grand V. No matter how long the chain, tracing speed is constant from start to end, calculated from the total tracing length.

`1-4q7-2[1:2],` = BUTTON-1→4 straight, 4→7 Q-shape, 7→2 straight, tracing length of two whole notes overall — the tracing star moves at one constant speed from BUTTON-1 to BUTTON-2.

If different sub-tracks need different tracing lengths, specify each individually: `1-4[2:1]q7[2:1]-2[1:1],`. When doing this, *every* sub-track needs its own length — omitting one causes an error.

To make it a BREAK SLIDE, add `b` after the *last* closing bracket, same as a regular SLIDE. A chaining SLIDE can only be entirely normal or entirely BREAK — there's no partial-BREAK chaining SLIDE, even if different sub-tracks have different speeds.

## TOUCH, TOUCH HOLD, & Firework Effect

### TOUCH

maimai DX has 34 touchable sensor areas on screen. All handle contact events, and SLIDEs require contacts to register in a specific order. Since maimai DX, two note types require directly touching a sensor: TOUCH and TOUCH HOLD.

Sensor areas are grouped into five:

- **Group A** — adjacent to the buttons
- **Group B** — between Group A and the screen center
- **Group C** — the center
- **Group D** — fills the gaps between Group A areas
- **Group E** — slightly inward from Group D, adjacent to Group B

Group C has 2 sensor areas; every other group has 8, numbered clockwise.

To place a TOUCH, give the sensor location the same way as a TAP:

| Notation | Meaning |
|---|---|
| `B1,` | TOUCH at SENSOR-B1 |
| `D4,` | TOUCH at SENSOR-D4 |

Group C is split into C1 and C2, but no TOUCH appears at either individually — it always appears in the middle of the two. So a center TOUCH is just `C,` (no number needed); writing `C1,` or `C2,` works the same and causes no error.

### TOUCH HOLD

TOUCH HOLD requires holding a sensor down. It's counted (and scored) as a HOLD in play results.

The notation directly combines TOUCH and HOLD — just replace a HOLD's button number with a sensor number.

`Ch[4:3],` = TOUCH HOLD at SENSOR-C, held for 3 quarter notes.

TOUCH has appeared across all 34 sensor areas since DX FESTiVAL. TOUCH HOLD, as of DX UNiVERSE PLUS, has officially only appeared on the C sensor — but in simai's notation, a TOUCH HOLD can be placed at any sensor by specifying a non-C sensor number.

### Firework Effect

The rainbow radial effect that spreads from a touched sensor, as if fireworks. There was no official name for this effect when it launched in DX classic, so *3simai* dubbed it "firework effect" for convenience.

Add `f` after the sensor number to enable it:

| Notation | Meaning |
|---|---|
| `B7f,` | TOUCH at SENSOR-B7, with firework effect |
| `Chf[1:2],` | TOUCH HOLD at SENSOR-C, held for two whole notes, with firework effect |

For TOUCH HOLD, `h` and `f` can appear in either order: `hf` or `fh`.

## EACH

Two or more notes occurring at exactly the same time form an **EACH** (also called BOTH). For SLIDEs specifically, tracks are considered part of an EACH if their tracing stars *start* moving simultaneously, even if their tracing lengths differ. EACH notes turn yellow, except BREAK notes.

An EACH is written by separating each component note with `/`.

`1/8h[2:1],` = TAP at BUTTON-1, plus HOLD at BUTTON-8 held for one half note.

Order doesn't matter for most notes — `8h[2:1]/1,` works the same. It *does* matter for SLIDEs: in `1-4[8:1]/2-6[8:1],`, the `1-4[8:1]` SLIDE is displayed as if it started before `2-6[8:1]` — whichever SLIDE is written first is shown as occurring first.

Three or more notes can form an EACH the same way: `note A / note B / note C …`.

Only an EACH made entirely of non-BREAK TAPs can be written without `/`, as in `12`. Otherwise, the `/` is mandatory — even for an EACH with only one non-TAP or BREAK note in it.

## EX Notes

TAP, HOLD, and BREAK notes can all be marked as EX notes. An EX note is judged CRITICAL PERFECT if hit within the GOOD timing window or better. (Simulators running only in autoplay mode always hit CRITICAL PERFECT regardless of a note's EX setting — this notation exists for simulators that respond to player input, and for reproducing official charts precisely.)

Add `x` the same way BREAK's `b` is added:

| Notation | Meaning |
|---|---|
| `1x,` | EX-TAP at BUTTON-1 |
| `3hx[α:β],` | EX-HOLD at BUTTON-3 |
| `5bx,` | EX-BREAK at BUTTON-5 |
| `7bxh[α:β],` | EX-BREAK HOLD at BUTTON-7 |

When `x`, `h`, and `b` are combined, they can appear in any order.

## Other Notations

These notations exist to make special charts (like the UTAGE charts introduced in the MURASAKi version) easier to build.

### Change Normal TAP to Star-shaped TAP

`1,` is a normal TAP at BUTTON-1. Appending `$` — `1$,` — turns it into the star-shaped TAP that normally only appears when a SLIDE is placed.

This can be combined with BREAK or EX-TAP; when `$`, `b`, and `x` all appear together, order doesn't matter.

Stacking two — `$$` — makes the star-shaped TAP rotate, at a fixed, pre-defined rotation speed.

### Change Star-shaped TAP to Normal TAP

A SLIDE like `1-5[8:1],` automatically turns its BUTTON-1 TAP into a star shape. Appending `@` to that TAP — `1@-5[8:1],` — reverts it to a normal TAP. The SLIDE itself still plays out normally; its arrow track just visually starts from a normal-looking TAP.

This also combines with BREAK or EX-TAP, in any order with `@`, `b`, and `x`.

### Notations of Pseudo-TAP HOLD and Pseudo-TOUCH TOUCH HOLD

Take a HOLD like `3h[1:1],`. Shorten its held-down length enough and the required hold time becomes instant — visually it shrinks into a hexagonal TAP shape. This could technically be written as `3h[1000:1],`, but it's simpler to just drop the length entirely: `3h,`.

The same applies to TOUCH HOLD — `Ch,` judges instantly on hit.

This is the standard way to denote a pseudo TAP or pseudo TOUCH. Internally, it's treated as if `[1280:1]` were specified — SEGA's official fan book states the implied held-down length for these notes as a 1280th note.

### Pseudo EACH

Denoting two TAPs that are almost — but not quite — simultaneous normally requires painfully fine-grained subdivision, e.g.:

```
{96}
1,2,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,
{8}
```

Here, a 96th-note divider is used just for that one timing gap, and anything closer than that becomes impractical — a real problem when reproducing an official chart exactly, or fine-tuning a fan chart.

Instead, `` 1`2, `` places the BUTTON-2 TAP just slightly after BUTTON-1 — almost, but not quite, simultaneous. Internally, the backtick delays the following note by exactly 1 millisecond. Because they're not *exactly* simultaneous, the two TAPs don't turn yellow and don't count as an EACH in the play result.

Example: `` 1`2`3/4, `` places BUTTON-2 one millisecond after BUTTON-1, and an EACH of BUTTON-3 + BUTTON-4 one millisecond after that.

> The `` ` `` character is entered by holding Shift and pressing the key to the right of "P" on a JIS keyboard layout, or by pressing the `` ` `` key to the left of "1" on a US/UK layout.

### SLIDE without Star-shaped TAP

Placing a SLIDE normally spawns an approaching star-shaped TAP that travels to the SLIDE's start point — but this can be suppressed.

Given a SLIDE `1-5[2:1],`:

| Notation | Behavior |
|---|---|
| `1?-5[2:1],` | Tracing star fades in before it starts moving |
| `1!-5[2:1],` | Tracing star appears suddenly (no fade-in) when it starts moving |

In both cases the SLIDE's arrow track fades in and the approaching star-shaped TAP doesn't appear. Use `?` when tracing letters, symbols, or other precise shapes with SLIDEs, so it's clear exactly where tracing begins. Use `!` for single-stroke SLIDEs, to avoid extra stars cluttering the screen.

## Notes

- On the backtick used in [Pseudo EACH](#pseudo-each): it sits to the left of "1" on a US/UK keyboard layout (Alt code 96). *— cubruce1103, 2023-02-13*

---

*Last updated: July 25, 2023, 18:34 (JST)*
