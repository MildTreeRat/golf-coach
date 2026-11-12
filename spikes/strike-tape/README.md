# The strike-tape truth sheet

**Tier: AS-BUILT for the sheet's geometry, which is [truth-sheet.svg](truth-sheet.svg)'s own coordinates.**
Written 2026-11-12 by M46 P2 ([the plan](../../docs/plans/m46-impact-drill.md), D7), before any label had
been stuck on it. The session that uses it is [the run sheet](../../docs/plans/m46-tape-run-sheet.md) (P3),
and the reader of its photos is P13's `impact::tape::truth`.

**What it is for.** The drill reads where each strike landed from a photo of a STRIKEPRO label on the club.
To measure that reader's error, each finished label is peeled, stuck flat on this sheet, and photographed
straight on. A flat label on a printed millimetre grid has no curvature, no hosel and no perspective worth
the name, so its marks are the truth the on-club readings are compared to. This file says where everything
on the sheet is, because P13 rectifies each photo to these coordinates.

**What the truth cannot see.** P13 finds a mark on the flat label with the same segmentation the reader
uses on the club. So the truth measures the geometry chain (finding the label, squaring it, orienting it,
the face's curvature) and not the segmentation. A mark both of them draw in the wrong place is invisible
here. P13 states this in its own doc as well.

## Printing it

Open the SVG in a browser and print it at **100% ("Actual size")**, never "Fit to page", with the
default margins and in colour if the printer has it. The sheet is 180 mm by 250 mm, which fits A4 and
US Letter inside half-inch margins, so neither paper clips it.

The print is checked in both directions, because a printer can scale across the feed and along it
differently, and P13 trusts the drawn coordinates:

| Check | Drawn | Accepted |
|---|---|---|
| The scale bar, end tick to end tick (across) | 100 mm | 99.5 to 100.5 mm |
| The two left corner squares, outer edge to outer edge (down) | 250 mm | 249 to 251 mm |

Both are a 0.5% tolerance. A print outside either is reprinted, and the measured numbers go into
`labels.txt` (the run sheet's step 1) so P13 can see what the print really was.

## The sheet's frame

One SVG unit is one millimetre. The origin is the sheet's top-left corner, which is the top-left corner
of the top-left fiducial; **x runs right and y runs down**, as SVG's do. Every number below is in this
frame and is read straight from the SVG's `id`s.

### The fiducials

Four solid black squares, 10 mm on a side, which P13 finds and rectifies by. Their centres form a
rectangle 170 mm across and 240 mm down. White surrounds each by at least 3 mm before the grid starts.

| `id` | Box (x0, y0, x1, y1) | Centre |
|---|---|---|
| `fiducial-tl` | 0, 0, 10, 10 | 5, 5 |
| `fiducial-tr` | 170, 0, 180, 10 | 175, 5 |
| `fiducial-bl` | 0, 240, 10, 250 | 5, 245 |
| `fiducial-br` | 170, 240, 180, 250 | 175, 245 |

**The orientation key**, `orientation-key`, is a fifth black square, 5 mm on a side, at
(15, 2.5, 20, 7.5), centre (17.5, 5): 5 mm to the right of the top-left fiducial. Four identical squares
on a rectangle cannot tell a sheet photographed upside down from one the right way up, and a sheet read
upside down would put slot 1's label in slot 6 and turn every label through 180°. The key is a quarter of
a fiducial's area, so nothing that looks for 10 mm squares mistakes it for one. A photo with the key
nearest the bottom-right fiducial is upside down.

### The grid

A 1 mm grid over x 0 to 180 and y 13 to 237, each line centred on its whole millimetre:

| Lines every | Width | Colour |
|---|---|---|
| 1 mm | 0.1 mm | `#a8cbff` |
| 5 mm | 0.2 mm | `#74a9ff` |
| 10 mm | 0.3 mm | `#3f88ff` |

A 0.3 mm border runs round the grid's edge in the 10 mm colour.

**Why blue.** Every grid colour, and the slots' brackets (`#2a6cff`), has a full blue channel, so a
printer lays them with no yellow ink, and in a photo's blue channel the grid should come out close to
white, while the black fiducials and text stay black. That is a design intent and has not been measured
on a print. It is what lets P13 drop the grid by channel if a label turns out to be see-through, so that
the grid under it does not look like marks. On a mono printer the grid comes out light grey and nothing
else changes.

### The slots

Six slots, 70 mm across and 60 mm down, numbered across and then down. Each slot's edges and centre lie
on the grid's 5 mm lines, so a label can be centred by eye.

| Slot | Box (x0, y0, x1, y1) | Centre | Write-in band (y) |
|---|---|---|---|
| `slot-1` | 5, 25, 75, 85 | 40, 55 | 15 to 24 |
| `slot-2` | 105, 25, 175, 85 | 140, 55 | 15 to 24 |
| `slot-3` | 5, 100, 75, 160 | 40, 130 | 90 to 99 |
| `slot-4` | 105, 100, 175, 160 | 140, 130 | 90 to 99 |
| `slot-5` | 5, 175, 75, 235 | 40, 205 | 165 to 174 |
| `slot-6` | 105, 175, 175, 235 | 140, 205 | 165 to 174 |

A slot is marked by **corner brackets** (6 mm legs) and a 3 mm **tick** inward from the middle of each
side, all in blue, rather than by an outline, so that nothing dark runs under a label that overhangs its
slot. Above each slot, a white **write-in band** across the slot's width holds the slot's number and the
fields `Club`, `Hand R / L`, `Label L__` and `Strikes`. The band's text is black and sits at least 1 mm
outside the slot.

**A label wider than its slot.** Nobody had measured a STRIKEPRO label when this was drawn (P0's
finding 2), and a driver's label may run past 70 mm. The columns are 30 mm apart and each sits 5 mm
inside the grid's edge, so a label up to **90 mm** wide still goes in its slot, centred, with at least
10 mm between it and its neighbour; it may then reach past the grid's outer edge onto bare paper, which
is still inside the page. A label wider than 90 mm goes in the left column only (slots 1, 3 and 5), with
the slot to its right left empty. A label taller than 60 mm would cover the next band, and the run sheet
says to stop and note it.

### The text and the scale bar

The top band (y 0 to 12, between the top fiducials) holds the title and the `Sheet` and `Date` fields.
The bottom band (y 237 to 250, between the bottom fiducials) holds the scale bar: a 0.3 mm line from
x 40 to x 140 at y 241.5, end ticks from y 239.5 to 243.5, and ticks every 10 mm between them.

## How a peeled label sits in its slot

The run sheet's step 7 asks for exactly this, and P13 reads by it. A label is peeled from the face, the
side the camera saw kept facing up, and laid in its slot **without turning it**:

- the edge that ran along the face's **top line** goes toward the slot's number, which is **−y** on the
  sheet;
- the **toe** stays on the side it was in the photos: **−x** (left) for a right-handed club and **+x**
  (right) for a left-handed one, since a right-handed club seen face-on with the shaft up has its toe on
  the image's left (the plan's C4).

So a point (x, y) on the sheet, with the label's own centre at (cx, cy), is at this position in the label
frame of C4 (+ toward the toe, + toward the top line), in millimetres:

| Club | Toe (+) | High (+) |
|---|---|---|
| Right-handed | −(x − cx) | −(y − cy) |
| Left-handed | +(x − cx) | −(y − cy) |

The label's centre (D2) is the label's own, found from its edges in the photo, not the slot's centre: a
label stuck 2 mm off its slot's centre is still read from its own middle.

## Status

**Open.** The sheet is drawn and its size checked from the file. Nothing has been printed, and the session
(P3) has not run.
