# Club-spec answer key

<!-- Rendered from cases.json by tests/evals/test_club_spec_cases.py, which fails when this file and cases.json differ. Edit cases.json, then run `.venv/Scripts/python.exe tests/evals/test_club_spec_cases.py`. -->

What the club-spec eval grades the set lookup against (M43, [ADR-036 §7](../../docs/decisions/036-llm-evaluation-and-observability.md)). Every value was read from the page named under its table, on the date given there. Nothing here came from `club_catalogue.json` or from a model. **blank** means the page's table does not print that field, so the right answer is a blank. A set tagged `fabricated` is a made-up model: every field is blank, and its source is the search that found no such model. A cell such as `0.15 in = 3.81` is a figure the page printed in another unit, converted to the column's (25.4 mm to the inch), and `38 7/8" = 38.875` is a fraction written out.

Slots follow the repo's `ClubId`. A maker's AW or GW is the `gw` slot, and a wedge labelled only by the slot's initial (PXG's G, S and L) is `gw`, `sw` or `lw`. A wedge the maker labels only W, UW, W2 or DW is left out, because naming its slot would be a guess.

14 sets, 103 slots, 345 published values, 60 published blanks, and 110 blanks in the 3 made-up sets.

| Case | Tags | Make | Model | Year | Slots |
|---|---|---|---|---|---|
| `takomo-201-mkii` | own-bag, current | Takomo | 201 MKII | not given | 4i, 5i, 6i, 7i, 8i, 9i, pw |
| `taylormade-r9-460` | own-bag, older | TaylorMade | R9 460 10.5° | not given | driver |
| `ping-i230` | older | PING | i230 | not given | 3i, 4i, 5i, 6i, 7i, 8i, 9i, pw |
| `mizuno-jpx925-forged` | current | Mizuno | JPX925 Forged | not given | 4i, 5i, 6i, 7i, 8i, 9i, pw, gw |
| `titleist-t275` | fabricated | Titleist | T275 | not given | 4i, 5i, 6i, 7i, 8i, 9i, pw |
| `taylormade-p790-2019` | older | TaylorMade | P790 | 2019 | 3i, 4i, 5i, 6i, 7i, 8i, 9i, pw, gw |
| `callaway-apex-dcb-2021` | older | Callaway | Apex DCB | 2021 | 4i, 5i, 6i, 7i, 8i, 9i, pw, gw |
| `pxg-0311-xp-gen7` | current | PXG | 0311 XP GEN7 | not given | 4i, 5i, 6i, 7i, 8i, 9i, gw, sw, lw |
| `cobra-king-tec-hybrid-2025` | current | Cobra | KING TEC Hybrid | 2025 | 2h, 3h, 4h, 5h |
| `wilson-staff-model-cb` | current | Wilson | Staff Model CB | not given | 3i, 4i, 5i, 6i, 7i, 8i, 9i, pw, gw |
| `srixon-zx7-mk-ii` | older | Srixon | ZX7 Mk II | not given | 3i, 4i, 5i, 6i, 7i, 8i, 9i, pw, gw, sw |
| `cleveland-launcher-xl-halo` | older | Cleveland | Launcher XL Halo | not given | 4i, 5i, 6i, 7i, 8i, 9i, pw, sw |
| `taylormade-p795` | fabricated | TaylorMade | P795 | not given | 4i, 5i, 6i, 7i, 8i, 9i, pw |
| `mizuno-jpx935-forged` | fabricated | Mizuno | JPX935 Forged | not given | 4i, 5i, 6i, 7i, 8i, 9i, pw, gw |

## `takomo-201-mkii`: Takomo 201 MKII

Tags: own-bag, current. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 4i | #4 | 22 | 60.5 | 38.75 | 3.16 | blank |
| 5i | #5 | 25 | 61 | 38.375 | 3.07 | blank |
| 6i | #6 | 28 | 61.5 | 37.75 | 2.89 | blank |
| 7i | #7 | 31 | 62 | 37.25 | 2.77 | blank |
| 8i | #8 | 36 | 62.5 | 36.75 | 2.62 | blank |
| 9i | #9 | 41 | 63 | 36.25 | 2.44 | blank |
| pw | #P | 46 | 63.5 | 36 | 2.32 | blank |

Sources:

- **spec** (maker): <https://takomogolf.com/products/iron-201-mkii?variant=46180786077893>, on 2026-11-10. On the page: the '201 MKII Specs' block, 'Clubs' tab.

Notes: The user's irons, by the user's own answer on 2026-11-09 ("Takomo 201 mark 2s"). aaron.bag.json held Titleist T150 and T250 irons until P11 (2026-11-10), when the user confirmed this set alone is the bag: the Titleist entries were retired there and their two cases removed from this key as stale. Re-read on 2026-11-10 from the page URL the user gave in P11, whose Clubs table matches the first read (2026-11-09) value for value: the shaft and grip options on the page change none of the head's figures. Takomo spells the model "201 MKII". The live page sells the set. Its table prints loft, lie, length ("incl. grip") and offset in mm, and no bounce. It prints no model year, so none is given.

## `taylormade-r9-460`: TaylorMade R9 460 10.5°

Tags: own-bag, older. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| driver | 10.5 | 10.5° | 59° | 45.75" | blank | blank |

Sources:

- **spec** (maker-archive): <https://www.taylormadegolf.com/products/drivers/r9-460-driver/product-detail-specifications>, read from the archived copy <https://web.archive.org/web/20111109134604/https://www.taylormadegolf.com/products/drivers/r9-460-driver/product-detail-specifications>, on 2026-11-10. On the page: the Specifications table's '10.5' column: Loft and Lie Angle under Head Properties, Club Length under Club Properties.

Notes: The user's driver, by the user's own answer in P11 (2026-11-10): a TaylorMade R9 460 at 10.5 degrees. That corrects the 2026-11-09 answer of "a Ping driver of sorts", and aaron.bag.json holds no driver. TaylorMade prints one column per loft (8.5, 9.5, 10.5 and 11.5), so the loft is part of the model text the lookup is given, as a golfer would type it: with the model alone, no one loft is the right answer. The values are the 10.5 column's. Lie and length are the same in every column. The table prints no offset and no bounce, and the page prints no model year, so none is given. Read from the Wayback copy of the maker's specifications tab (November 2011); the club is long out of production, hence older.

## `ping-i230`: PING i230

Tags: older. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 3i | 3-iron | 19.0° | 59.0° | 39" | 0.2" = 5.08 | 5.0° |
| 4i | 4-iron | 22.5° | 59.8° | 38 7/8" = 38.875 | 0.17" = 4.32 | 6.0° |
| 5i | 5-iron | 26.0° | 60.5° | 38 1/4" = 38.25 | 0.14" = 3.56 | 7.0° |
| 6i | 6-iron | 29.5° | 61.3° | 37 5/8" = 37.625 | 0.12" = 3.05 | 8.0° |
| 7i | 7-iron | 33.0° | 62.0° | 37" | 0.09" = 2.29 | 9.0° |
| 8i | 8-iron | 37.0° | 62.8° | 36 1/2" = 36.5 | 0.07" = 1.78 | 10.5° |
| 9i | 9-iron | 41.0° | 63.5° | 36" | 0.05" = 1.27 | 12.0° |
| pw | PW | 45.0° | 64.1° | 35 1/2" = 35.5 | 0.03" = 0.76 | 13.0° |

Sources:

- **spec** (maker): <https://ping.com/en-us/golf-clubs/irons/i230-iron>, on 2026-11-09. On the page: 'Key Specifications', one panel per club (CLUB, LENGTH, LOFT, ..., BOUNCE).

Notes: Ping's live page marks the i230 "Fill-In or Replacement Only", so it is tagged older. Every field is printed. Lie is Ping's Black color code at standard length, as the page says. LOFT is the standard loft; the Power Spec and Retro Spec lofts are fitting options and are not recorded. Offset is printed in inches and converted to mm (x 25.4). UW (a 50 deg "utility wedge") is left out, as it is not called a gap wedge. No model year is printed.

## `mizuno-jpx925-forged`: Mizuno JPX925 Forged

Tags: current. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 4i | 4 | 21 | 60.5 | 38.75 | 0.15 in = 3.81 | blank |
| 5i | 5 | 24 | 61 | 38.25 | 0.142 in = 3.61 | blank |
| 6i | 6 | 27 | 61.5 | 37.75 | 0.134 in = 3.4 | blank |
| 7i | 7 | 30 | 62 | 37.25 | 0.126 in = 3.2 | blank |
| 8i | 8 | 34 | 62.5 | 36.75 | 0.118 in = 3 | blank |
| 9i | 9 | 38 | 63 | 36.25 | 0.11 in = 2.79 | blank |
| pw | PW | 43 | 63.5 | 35.75 | 0.102 in = 2.59 | blank |
| gw | GW | 48 | 64 | 35.5 | 0.091 in = 2.31 | blank |

Sources:

- **spec** (maker): <https://mizunogolf.com/us/golf-clubs/jpx925-series/jpx925-forged/>, on 2026-11-09. On the page: 'JPX925 Forged Specification' (the Black finish's table below it is identical).

Notes: Live maker page, listed under mizunogolf.com/us/irons/. The table prints loft, lie, offset in inches (converted to mm, x 25.4) and length, and no bounce. GW is Mizuno's own label. No model year is printed; "925" names the generation.

## `titleist-t275`: Titleist T275

Tags: fabricated. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 4i | 4 | blank | blank | blank | blank | blank |
| 5i | 5 | blank | blank | blank | blank | blank |
| 6i | 6 | blank | blank | blank | blank | blank |
| 7i | 7 | blank | blank | blank | blank | blank |
| 8i | 8 | blank | blank | blank | blank | blank |
| 9i | 9 | blank | blank | blank | blank | blank |
| pw | PW | blank | blank | blank | blank | blank |

Sources:

- **search** (absence-search): <https://web.archive.org/cdx/search/cdx?url=www.titleist.com/product/t275*&output=txt&fl=timestamp,original,statuscode&collapse=urlkey>, on 2026-11-09. On the page: the Wayback Machine's index lists no captured titleist.com URL under /product/t275; the same query for /product/t250* lists 125, among them the 2025 T250 page, https://www.titleist.com/product/t250/561C.html.

Notes: A made-up model, so the right answer is a blank in every field, and a lookup that fills one has invented it. The name fits Titleist's T-series numbering (T100, T150, T200, T250, T350), but the absence search finds no T275. The slot labels are the ones a set of that kind would carry, since no maker printed any. The absence check is a search of the maker's own product URLs in the Wayback Machine, with the real model beside it to show the search finds one that exists. A general web search was tried first and is not cited: Bing, fetched with curl, returned the same brand pages for the real T250 as for this name, so its silence proves nothing, and DuckDuckGo answered with a bot check, which this key does not try to pass. In P11 (2026-11-10) a web search for "Titleist T275 irons" found no such model either: every result was for the real 2025 T-Series (T100, T150, T250, T350 and the U505). The user accepted that as the human check.

## `taylormade-p790-2019`: TaylorMade P790

Tags: older. Model year: 2019 (from **spec**). Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 3i | 3 | 19.0° | 60.5° | 39.00" | 3.5mm | blank |
| 4i | 4 | 21.0° | 61° | 38.50" | 3.4mm | blank |
| 5i | 5 | 23.5° | 61.5° | 38.00" | 3.1mm | blank |
| 6i | 6 | 26.5° | 62° | 37.50" | 2.9mm | blank |
| 7i | 7 | 30.5° | 62.5° | 37.00" | 2.7mm | blank |
| 8i | 8 | 35.0° | 63° | 36.50" | 2.4mm | blank |
| 9i | 9 | 40.0° | 63.5° | 36.00" | 2.1mm | blank |
| pw | PW | 45.0° | 64° | 35.75" | 1.8mm | blank |
| gw | AW | 50.0° | 64° | 35.50" | 1.6mm | blank |

Sources:

- **spec** (maker-archive): <https://www.taylormadegolf.com/P790-Irons/DW-AL464.html>, read from the archived copy <https://web.archive.org/web/20210225020341/https://www.taylormadegolf.com/P790-Irons/DW-AL464.html>, on 2026-11-09. On the page: the spec table (CLUB, LOFT, LIE, OFFSET, LENGTH, SWING WEIGHT), and the page's Customize link, which names the head "P790 2019".

Notes: Read from the Wayback copy (February 2021). The live URL now redirects to a different product id (DW-TA228) behind a bot check, which this key does not try to pass. The model year comes from the page's own Customize link (family and head "P790 2019"). TaylorMade has sold more than one P790 under that name, which is why the year is given. Offset is printed in mm. AW (approach wedge) is recorded as gw. No bounce is printed.

## `callaway-apex-dcb-2021`: Callaway Apex DCB

Tags: older. Model year: 2021 (from **spec**). Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 4i | #4 | 20° | 60.5° | 38.75" | 5.08 | blank |
| 5i | #5 | 23° | 61.25° | 38.125" | 4.57 | blank |
| 6i | #6 | 26° | 62.0° | 37.50" | 4.06 | blank |
| 7i | #7 | 30° | 62.5° | 37.00" | 3.81 | blank |
| 8i | #8 | 34° | 63.0° | 36.50" | 3.56 | blank |
| 9i | #9 | 38° | 63.5° | 36.00" | 3.30 | blank |
| pw | PW | 43° | 64.0° | 35.75" | 3.05 | blank |
| gw | AW | 48° | 64.0° | 35.50" | 2.79 | blank |

Sources:

- **spec** (maker-archive): <https://www.callawaygolf.com/golf-clubs/irons/irons-2021-apex-dcb.html>, read from the archived copy <https://web.archive.org/web/20260215083322/https://www.callawaygolf.com/golf-clubs/irons/irons-2021-apex-dcb.html>, on 2026-11-09. On the page: 'Mens 2021 Apex DCB Irons Product Specs' (Name, Loft, Availability, Standard Length, Lie, Offset (mm), swing weights).

Notes: Read from the Wayback copy (February 2026). The live site answers curl with a 429 bot check. That copy marks the set Sold Out, so it is tagged older. Callaway names it "Apex DCB 21", and the spec table's heading gives the year. Offset is printed in mm. AW is recorded as gw. No bounce is printed.

## `pxg-0311-xp-gen7`: PXG 0311 XP GEN7

Tags: current. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 4i | 4 | 18° | 60.5° | 39.375" | .250" = 6.35 | 6° |
| 5i | 5 | 20° | 61° | 38.75" | .220" = 5.59 | 7° |
| 6i | 6 | 23° | 61.5° | 38.125" | .190" = 4.83 | 8° |
| 7i | 7 | 27° | 62° | 37.5" | .160" = 4.06 | 9° |
| 8i | 8 | 31° | 62.5° | 37" | .130" = 3.3 | 10° |
| 9i | 9 | 36° | 63° | 36.5" | .105" = 2.67 | 11° |
| gw | G | 47° | 63.5° | 36" | .070" = 1.78 | 12° |
| sw | S | 54° | 63.5° | 35.75" | .040" = 1.02 | 13° |
| lw | L | 60° | 63.5° | 35.5" | .030" = 0.76 | 13° |

Sources:

- **spec** (maker): <https://www.pxg.com/products/gen7-irons>, on 2026-11-09. On the page: 'GEN7 Irons Specs', the second table, headed '0311 XP GEN7' (Club - Loft, Dexterity, Standard Length, Standard Lie, Offset, Bounce); the first table is the 0311 P GEN7.

Notes: Live maker page, which sells the GEN7 irons; its 'Set Make Up' for the 0311 XP GEN7 is "4-G, S, L". The 'Club - Loft' column prints the club and the loft together ("4 - 18°"), so the published loft is the part after the dash. Offset is printed in inches with no leading zero (.250 for the 4) and converted to mm (x 25.4). Every slot prints bounce, the wedges included. PXG labels its wedges W, G, S and L without spelling them out: G, S and L are recorded as gw, sw and lw because each letter is that slot's own initial, and W (41°) is left out, as calling it pw would be a guess. No model year is printed; "GEN7" names the generation.

## `cobra-king-tec-hybrid-2025`: Cobra KING TEC Hybrid

Tags: current. Model year: 2025 (from **spec**). Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 2h | 2H | 17.0° | 57.5° | 41.25" | blank | blank |
| 3h | 3H | 19.0° | 58.0° | 40.75" | blank | blank |
| 4h | 4H | 22.0° | 58.5° | 40.25" | blank | blank |
| 5h | 5H | 25.0° | 59.0° | 39.75" | blank | blank |

Sources:

- **spec** (maker): <https://www.cobragolf.com/products/king-tec-hybrid-2025>, on 2026-11-09. On the page: 'Specifications' (CLUB, STD LOFT, LOFT RANGE (FF33), LIE RANGE (FF33), VOLUME, LENGTH, ...); the year from the page's own product data, whose description is "KING TEC Hybrid 2025 | COBRA Golf", and its URL handle, king-tec-hybrid-2025.

Notes: The one set here that is not irons or wedges: hybrids, so the eval asks about a wood-type slot too. Live maker page, which sells it. The visible title is only "KING TEC Hybrid", and Cobra has sold an earlier KING TEC hybrid under that name, so the year matters: it is taken from the page's own product data and URL handle, both "2025", which is a call for P11 to check. Loft and lie are adjustable (the FutureFit33 hosel): STD LOFT is recorded as the loft, and the lie is the standard figure the LIE RANGE cell prints in brackets ("55.5° - 59.5° (57.5° STD LIE)" is recorded as "57.5°"). No offset or bounce is printed.

## `wilson-staff-model-cb`: Wilson Staff Model CB

Tags: current. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 3i | 3* | 20° | 59.5° | 39.25" | 0.110" = 2.79 | 1.0° |
| 4i | 4 | 23° | 60° | 38.75" | 0.100" = 2.54 | 1.0° |
| 5i | 5 | 26° | 61° | 38.25" | 0.090" = 2.29 | 2.0° |
| 6i | 6 | 30° | 61.5° | 37.75" | 0.080" = 2.03 | 3.0° |
| 7i | 7 | 34° | 62° | 37.25" | 0.070" = 1.78 | 4.0° |
| 8i | 8 | 38° | 63° | 36.75" | 0.063" = 1.6 | 5.5° |
| 9i | 9 | 42° | 63.5° | 36.25" | 0.057" = 1.45 | 6.0° |
| pw | PW | 46° | 64° | 35.75" | 0.050" = 1.27 | 6.0° |
| gw | GW* | 50° | 64.25° | 35.50" | 0.040" = 1.02 | 7.0° |

Sources:

- **spec** (maker): <https://www.wilson.com/en-us/products/staff-model-cb-irons-wg1p0386>, on 2026-11-09. On the page: 'Specifications' (Iron, Loft, Lie, Length, Bounce, Hosel Offset, Swing Weight, Hand).

Notes: Live maker page, which sells the set (its SKUs are 4-PW). Every slot prints bounce, so bounce is graded for more than Ping. Hosel offset is printed in inches and converted to mm (x 25.4). The 3 and GW rows are marked "*", which the page does not explain; they are kept, with the asterisk in the label, because their values are published. No model year is printed. Wilson has sold more than one Staff Model CB, so a lookup should name the generation it answered from.

## `srixon-zx7-mk-ii`: Srixon ZX7 Mk II

Tags: older. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 3i | 3 | 20° | 60° | 39" | 3.5mm | blank |
| 4i | 4 | 22° | 60.5° | 38.5" | 3.3mm | blank |
| 5i | 5 | 25° | 61° | 38" | 3.2mm | blank |
| 6i | 6 | 28° | 61.5° | 37.5" | 3.0mm | blank |
| 7i | 7 | 32° | 62° | 37" | 2.8mm | blank |
| 8i | 8 | 36° | 62.5° | 36.5" | 2.4mm | blank |
| 9i | 9 | 41° | 63° | 36" | 2.0mm | blank |
| pw | PW | 46° | 63.5° | 35.5" | 1.7mm | blank |
| gw | AW | 51° | 63.5° | 35.5" | 1.4mm | blank |
| sw | SW* | 57° | 63.5° | 35.5" | 0.8mm | blank |

Sources:

- **spec** (maker-archive): <https://us.dunlopsports.com/srixon/clubs/irons/zx7-mk-ii-irons/zx7-mk-ii-irons/MZX7MKIIIRN.html>, read from the archived copy <https://web.archive.org/web/20230128074743/https://us.dunlopsports.com/srixon/clubs/irons/zx7-mk-ii-irons/zx7-mk-ii-irons/MZX7MKIIIRN.html>, on 2026-11-09. On the page: the spec table (HEAD, LOFT, HAND, LIE, OFFSET, LENGTH, SWING WEIGHT) and the line under it, "*Available as custom only".

Notes: Read from the Wayback copy of the maker page (January 2023). The live URL now answers 404, and Srixon's live iron pages sell the ZXi7 that replaced it, so it is tagged older. Offset is printed in mm. AW is recorded as gw. SW is marked custom-only and is kept, as sw. No bounce is printed, and no model year.

## `cleveland-launcher-xl-halo`: Cleveland Launcher XL Halo

Tags: older. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 4i | 4 | 20.0° | 61.5° | 39.125" | 2.25 | blank |
| 5i | 5 | 23.0° | 62.0° | 38.5" | 2.18 | blank |
| 6i | 6 | 26.5° | 62.5° | 37.875" | 2.11 | blank |
| 7i | 7 | 30.0° | 63.0° | 37.25" | 2.04 | blank |
| 8i | 8 | 34.5° | 63.5° | 36.75" | 1.97 | blank |
| 9i | 9 | 39.0° | 64.0° | 36.25" | 1.9 | blank |
| pw | PW | 44.0° | 64.5° | 36.0" | 1.75 | blank |
| sw | SW | 54.5° | 65.0° | 35.75" | 0.6 | blank |

Sources:

- **spec** (maker-archive): <https://us.dunlopsports.com/cleveland-golf/clubs/irons/launcher-xl-halo-irons/launcher-xl-halo-irons/MLXLHI.html>, read from the archived copy <https://web.archive.org/web/20230128072931/https://us.dunlopsports.com/cleveland-golf/clubs/irons/launcher-xl-halo-irons/launcher-xl-halo-irons/MLXLHI.html>, on 2026-11-09. On the page: the spec table (HEAD, LOFT, LIE, OFFSET (MM), STANDARD BUILD LENGTH, ..., ACCURACY BUILD LENGTH*, ..., HAND).

Notes: A game-improvement set with hollow, hybrid-like heads. Read from the Wayback copy of the maker page (January 2023). The live URL now redirects to Cleveland's club listing, so it is tagged older. Length is the STANDARD BUILD LENGTH; the half-inch-shorter Accuracy build is a custom option and is not recorded. The offset column is headed in mm and prints bare numbers. DW (49.5°, a "dual wedge") is left out, as calling it gw would be a guess. No bounce is printed, and no model year.

## `taylormade-p795`: TaylorMade P795

Tags: fabricated. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 4i | 4 | blank | blank | blank | blank | blank |
| 5i | 5 | blank | blank | blank | blank | blank |
| 6i | 6 | blank | blank | blank | blank | blank |
| 7i | 7 | blank | blank | blank | blank | blank |
| 8i | 8 | blank | blank | blank | blank | blank |
| 9i | 9 | blank | blank | blank | blank | blank |
| pw | PW | blank | blank | blank | blank | blank |

Sources:

- **search** (absence-search): <https://web.archive.org/cdx/search/cdx?url=www.taylormadegolf.com/P795*&output=txt&fl=timestamp,original,statuscode&collapse=urlkey>, on 2026-11-09. On the page: the Wayback Machine's index lists no captured taylormadegolf.com URL under /P795; the same query for /P790* lists 152, among them the P790 page a case above is read from.

Notes: A made-up model, so the right answer is a blank in every field, and a lookup that fills one has invented it. The name sits beside TaylorMade's P770 and P790, but the absence search finds no P795. The slot labels are the ones a set of that kind would carry, since no maker printed any. The absence check is a search of the maker's own product URLs in the Wayback Machine, with the real model beside it to show the search finds one that exists. A general web search was tried first and is not cited: Bing, fetched with curl, returned the same brand pages for the real P790 as for this name, so its silence proves nothing, and DuckDuckGo answered with a bot check, which this key does not try to pass. In P11 (2026-11-10) a web search for "TaylorMade P795 irons" found no such model either: every result was for the P790. The user accepted that as the human check.

## `mizuno-jpx935-forged`: Mizuno JPX935 Forged

Tags: fabricated. Model year: not given. Verified by the user: 2026-11-10.

| Slot | Maker's label | Loft (°) | Lie (°) | Length (in) | Offset (mm) | Bounce (°) |
|---|---|---|---|---|---|---|
| 4i | 4 | blank | blank | blank | blank | blank |
| 5i | 5 | blank | blank | blank | blank | blank |
| 6i | 6 | blank | blank | blank | blank | blank |
| 7i | 7 | blank | blank | blank | blank | blank |
| 8i | 8 | blank | blank | blank | blank | blank |
| 9i | 9 | blank | blank | blank | blank | blank |
| pw | PW | blank | blank | blank | blank | blank |
| gw | GW | blank | blank | blank | blank | blank |

Sources:

- **search** (absence-search): <https://web.archive.org/cdx/search/cdx?url=mizunogolf.com/us/golf-clubs/jpx935*&output=txt&fl=timestamp,original,statuscode&collapse=urlkey>, on 2026-11-09. On the page: the Wayback Machine's index lists no captured mizunogolf.com URL under /us/golf-clubs/jpx935; the same query for /us/golf-clubs/jpx925* lists 8, among them the JPX925 Forged page a case above is read from.

Notes: A made-up model, so the right answer is a blank in every field, and a lookup that fills one has invented it. The name follows Mizuno's JPX numbering (JPX919, JPX921, JPX923, JPX925), but the absence search finds no JPX935. The slot labels are the ones a set of that kind would carry, since no maker printed any. The absence check is a search of the maker's own product URLs in the Wayback Machine, with the real model beside it to show the search finds one that exists. A general web search was tried first and is not cited: Bing, fetched with curl, returned the same brand pages for the real JPX925 Forged as for this name, so its silence proves nothing, and DuckDuckGo answered with a bot check, which this key does not try to pass. In P11 (2026-11-10) a web search for "Mizuno JPX935 Forged irons" found no such model either: every result was for the JPX925, JPX923 and JPX921 Forged. The user accepted that as the human check.
