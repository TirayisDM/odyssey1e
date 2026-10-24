# Tile art brief

Written 2026-10-22, after 235 drew the first map in flat colours.

The renderer already works. What it draws is placeholder, and this is
what would replace it. **The filenames are not decorative** — the
renderer looks a tile up by the terrain's own key, so `dirt.png` is
found and `Dirt.png` or `dirt_tile.png` is not.

---

## 1. What already exists, honestly

MAP.md calls this "the 16x16 SVG art". Having looked in
`G:\My Drive\osdungeon\art`:

| file | what it is |
|---|---|
| `dirt.svg` | a terrain tile, 452 bytes, `viewBox="0 0 26 26"` |
| `stonewall.svg` | a terrain tile |
| `actor.svg`, `actor-with-pack.svg` | character sprites |
| `Common.svg`, `CommonF.svg` | character sprites |
| `warrior.png`, `logo.png` | a sprite and a logo |
| `*.ai` | Illustrator sources for the above |

**TWO TERRAIN TILES, AGAINST THIRTY-THREE WORDS.** That is a start and
not a library, and it is worth saying plainly because MAP.md's phrasing
implies a set exists. Planning a cycle around "wire up the tile art"
would have run into it on the first afternoon.

Note also that `dirt.svg` is **26×26**, not 16×16.

---

## 2. Format

### PNG, not SVG — and this contradicts MAP.md

**SVG CANNOT GIVE YOU AN 8-BIT LOOK.** It is a vector format: it
describes shapes, and the renderer draws them smoothly at whatever size
is asked for. That is the opposite of pixel art, where the whole point
is that a pixel is a deliberate, visible unit.

You can draw pixel art *in* SVG — one `<rect>` per pixel — but it is a
perverse way to store a bitmap, and the browser will still antialias
the edges unless every rect lands on an exact device pixel.

So: **PNG-32 (RGBA), 32×32, no antialiasing.** Keep SVG for anything
that must scale arbitrarily — UI icons, the logo — and for the
Illustrator sources.

### 32 × 32, drawn at 32 px

| | |
|---|---|
| source | **32 × 32** pixels |
| displayed at | **32 px** — one for one, no scaling at all |
| scaling | nearest-neighbour (`imageSmoothingEnabled = false`, already set) |

**DECIDED 2026-10-23, AFTER THE FIRST TILE ARRIVED AT 32.** This brief
originally said 16×16 doubled to 32. Dave chose 32 for clarity while
drawing and so that fixtures and characters have the resolution to
match terrain later.

**IT WAS NEVER THE NUMBER 16 THAT MATTERED.** The requirement is that
the scale be a WHOLE NUMBER: at the 28 px square 235 shipped with, a 16
px tile scales by 1.75, some source pixels become two screen pixels
and others become one, and pixel art falls apart even with smoothing
off. One-to-one is the sharpest answer there is, and 32 gives four
times the room to draw in.

The trade, stated: 32×32 reads as modern pixel art rather than
NES-era. Five kinds of wall and three states of door need telling
apart at a glance, and that is worth more than era-accuracy.

`SQUARE` in `src/map.js` is 32.

### Transparency is not optional for covers

A **cover** sits on the ground and the ground must remain visible
underneath — snow on a road still shows the road at the edges, vines on
stone still show stone. Those need real alpha, not a background colour
matching whatever you assumed was beneath.

**CONFIRMED 2026-10-24.** The first cover drawn — `grass` — came back
as an indexed PNG with no alpha channel, and Dave's call is that covers
carry transparency. So an RGBA save (`PNG-32`) or an indexed PNG with a
`tRNS` chunk; what matters is that the renderer can see through it.

**THE RENDERER ALREADY DEPENDS ON THIS.** A flat-colour cover is drawn
at 60% alpha, because a flat colour is an opaque rectangle and would
hide the ground. A cover with ART is drawn at **100%**, on the
assumption that its own transparency does the work. An opaque cover
tile therefore blots out everything beneath it at full strength.

**Ground tiles** may be fully opaque. Indexed-colour PNG (type 3) is
ideal for them — it bakes a fixed palette into the file, which is what
section 3 asks for anyway.

**Walls, trees and shut doors** should also be fully opaque: they
replace the square, because you cannot walk on the floor they stand
on.

---

## 3. Colour

### A fixed palette is what makes it read as an era

Resolution alone does not. The look comes from **a small, shared set of
colours used everywhere** — that is why an NES screen looks like an NES
screen.

**Recommended: DawnBringer 32 (DB32).** Thirty-two colours, designed
for exactly this, freely usable, and widely enough known that any
pixel artist will recognise it. Sixteen (DB16, PICO-8) is tighter and
more authentically 8-bit, but thirty-three terrains need to be told
apart at a glance and sixteen colours makes that genuinely hard.

Whatever is chosen: **one palette, every tile, no exceptions.** A tile
drawn outside it will look wrong in a way nobody can name.

### Starting hues

`src/map.js` already holds a flat colour per terrain, chosen to be
distinguishable rather than handsome. They are a reasonable starting
point for hue and relative value — snap them to the nearest palette
entry rather than inventing from scratch.

### Keep families legible

A player should be able to tell at a glance:

- **stone** (rock, scree, stone floor, stone wall) — cool greys
- **earth** (dirt, mud, sand) — warm browns
- **growth** (grass, brush, trees, vines, marsh) — greens
- **water** (shallow, deep) — blues, with depth readable as value
- **cold** (snow, ice, ice wall) — pale blue-whites
- **worked wood** (wood floor, timber wall) — mid browns, warmer than earth
- **hazard** (lava) — the one saturated warning colour in the set

---

## 4. The thirty-three tiles

Generated from `terrain::ALL`, which is the list the database CHECK is
built from. `blocks` and `sight` are what the rules already say, and
they should be legible in the art without reading a legend.

### Ground — 18 tiles, opaque

| file | notes |
|---|---|
| `rock.png` | bare stone, walkable |
| | **all 18 DONE, 2026-10-24** — 32×32, indexed colour, in `src/assets/tiles/` |
| `dirt.png` | **DONE** — 32×32 RGBA, in `src/assets/tiles/`, drawn by the renderer |
| `mud.png` | difficult; should look slow |
| `sand.png` | difficult |
| `scree.png` | difficult; loose stone |
| `road.png` | the one surface that is reliably easy — should read as made |
| `stone_floor.png` | worked, flat, interior |
| `wood_floor.png` | worked, planks |
| `marsh.png` | difficult; wet ground, not open water |
| `ice.png` | difficult; breakable — it has hit points |
| `shallow_water.png` | difficult; wading, bottom visible |
| `deep_water.png` | needs a swim speed; bottom NOT visible |
| `lava.png` | 10d10 fire — the most dangerous square in the game |
| `pit.png` | an opening; nothing to stand on |
| `ledge.png` | a square you can fall FROM — the edge should be the subject |
| `doorway.png` | a threshold, and a link to another map |
| `ramp.png` | a change of height that costs nothing extra |
| `stairs.png` | the same, built |

### Covers — 10 tiles, **transparent**

| file | blocks | blocks sight | notes |
|---|---|---|---|
| `grass.png` | no | no | the only cover that costs nothing |
| `snow.png` | no | no | difficult; must not hide the road beneath |
| `brush.png` | no | no | difficult; undergrowth |
| `trees.png` | **yes** | **yes** | fills the square — opaque is fine |
| `vines.png` | no | no | difficult |
| `rubble.png` | no | no | difficult; what a wall leaves |
| `door_open.png` | no | no | a door, standing open |
| `door_closed.png` | **yes** | **yes** | opaque |
| `door_broken.png` | **yes** | **yes** | opaque — **jammed, not open.** Should read as ruined and still in the way |
| `iron_grate.png` | **yes** | no | **the hard one.** Stops a body and not an eye, so it must look impassable AND see-through |

### Walls — 5 tiles, opaque

Legal as both a surface and a cover: as a **cover** it is the wall in
your way, as a **surface** it is the top of one you are standing on. A
single tile serves both — z tells them apart — but a variant for the
top would read better later.

| file | notes |
|---|---|
| `stone_wall.png` | worked stone, AC 17 |
| `timber_wall.png` | worked wood, AC 15 |
| `rock_wall.png` | natural stone — should differ from `stone_wall` |
| `wooden_wall.png` | natural deadfall — should differ from `timber_wall` |
| `ice_wall.png` | AC 13, breaks into water |

**`timber_wall` AND `wooden_wall` MUST NOT LOOK ALIKE.** They were
renamed apart in 226 for exactly this reason — one is carpentry, one is
fallen timber — and art that blurs them undoes the rename.

---

## 5. Optional, and worth knowing about

**Variants.** The reference engine had `terrain_pattern_uri(terrain,
variant_id)` — two or three versions of a tile, chosen by position, so
a large floor does not visibly repeat. Nice, not needed, and the
renderer can add it without the art changing: `dirt.png`, `dirt_2.png`,
`dirt_3.png`.

**Creature tokens.** `actor.svg`, `warrior.png` and the two `Common`
files exist. The renderer currently draws a disc with an initial, which
is honest placeholder. A token set is its own brief.

**Lighting.** MAP.md section 8 expects per-tile compositing rather than
per-pixel shaders, so tiles do **not** need to be drawn lit or unlit —
the renderer darkens them. Draw everything at full light.

---

## 6. Where files go

`src/assets/tiles/<key>.png`, named exactly as the terrain key.

**WIRED 2026-10-23 AND WORKING.** The renderer falls back to the flat
colour for any tile that is missing, so **art arrives one at a time and
nothing breaks.** Each file is asked for once: a miss is remembered as
a miss and never re-requested, a hit triggers one repaint because the
image lands after the frame that wanted it.

That is deliberate: MAP.md records that the Apps Script prototype had a
complete texture path which never fired, because no tile type was ever
given a `texture` key. A pipeline that needs thirty-three drawings
before it does anything is a pipeline that never runs.

### How it reaches anyone else

**Committed to git**, like any other source file - `src/assets/` is not
ignored, so a clone gets the art with the code and the two stay
versioned together. Thirty-three tiles at a few kilobytes each is
nothing, even with revisions.

**And bundled into the app.** `tauri.conf.json` sets `frontendDist` to
`../src`, so Tauri packs the whole tree into the executable: a player
who installs the app has the art inside the binary and needs no network
to draw a map.

Art in Supabase Storage, fetched at runtime, becomes worth it the day a
DM wants to upload THEIR OWN tiles. That is a real feature eventually
and it is not this one.
