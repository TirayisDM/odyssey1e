# The map and the play surface

**Written 2026-10-18.** A standing plan, not a log — `STATUS.md` is the
log. This file says what we are building, what already exists to build
it from, and in what order. It should be edited as decisions change
rather than appended to.

**Nothing in here is built yet.** Every claim about the old code was
read out of the old code on the day this was written; every claim about
odyssey1e was checked against the live database.

---

## 1. What already exists, across three generations

There are three attempts at this. **They are complementary rather than
redundant** — no generation has what the other two have, which is the
whole reason a fourth is worth starting.

| | OSDUNGEON (Apps Script) | odyssey-engine (Rust) | odyssey1e (today) |
|---|---|---|---|
| where | `G:\My Drive\osdungeon` | desktop `C:\Users\tiray\Dev\odyssey-engine`, laptop `C:\Users\tiray\odyssey-engine` | here |
| **has** | the lighting and fog UX, an authoring flow, prose→recipe AI | **3,761 lines of pure rules, 262 tests** | contents, identity, RLS, the clock, initiative |
| **lacks** | any rules engine | any UI worth keeping | any geometry at all |

### What is genuinely good in the Apps Script prototype

- **The lighting model, which is the single best asset in the archive.**
  Distance-profile arrays per burn phase rather than a falloff formula
  (`[100x8, 50x4, 25x4]`), three burn phases, a directional bull's-eye
  selected by dot-product against facing, and a **max-combine** of four
  contributors: room ambient, carried torch, static placed lights, and
  lights sitting in inventory. All gated through Bresenham. Data-driven
  and tuneable without touching code.
- **Three-state fog** - unseen / explored-dim / visible - as a per-cell
  opacity rather than a separate mask. Cheap and legible.
- **The sparse `{x, y, type}` wire format** with an `"x,y"` string-keyed
  map. Small, handles irregular rooms for free.
- **Cost-aware path preview**: in combat, tiles past your movement
  budget draw red-dashed instead of blue. Copy this UX directly.
- **`generateRoomTileHints`** - prose to a validated JSON recipe drawn
  from a closed tile vocabulary, never coordinates. The one part of the
  prototype that is entirely geometry-free, and it would port today.
- **The seed-and-overlay storage model.** A room stored a recipe plus
  seven sparse arrays of authored edits; tiles were regenerated from
  `scene.id` on every load and never persisted at all.

### What is good in the Rust port

- **It is already pure.** All six modules import zero persistence, zero
  SQLite, zero async, zero `engine_ops`. The old app kept storage behind
  a trait and converted at the boundary - the same line this codebase
  draws between `src/*.rs` and `src/commands/*.rs`.
- `TileGrid` is **already `Serialize`**: flat row-major `Vec<Tile>`,
  `u16` coords, perimeter auto-walled. A JSON blob with no joins.
- Bresenham LOS, symmetric, endpoints excluded.
- `LockProfile` and `TrapProfile` are real rules - DCs, pick attempts,
  permanent jamming, material integrity across three damage channels,
  and a disarm that **fires the trap** if you fail the last allowed
  attempt. Both have `*_with_roll(..., roll)` variants, so their entire
  randomness surface is one call site each and they drop into our roll
  path rather than fighting it.
- **262 tests**, all in-file, no fixtures, no database, no tokio.
- **Verified on the laptop, 2026-10-20**, against the clone at
  `2551f1e` rather than taken on trust: the six are `tile`, `scene`,
  `movement`, `lock`, `trap` and `light`, and their `#[test]` count is
  **262 exactly**. 3,767 lines sit above `#[cfg(test)]` in them, which
  is the 3,761 above give or take where the line is drawn.
- **AND THEY ARE 8% OF THE REPO**, which is the point. The whole of
  odyssey-engine is 47,009 lines and 1,329 tests; the six are a
  deliberate extract. `G:\My Drive\osdungeon\rust` holds 23 loose
  `.rs` files with the same NAMES - no `Cargo.toml`, no git, 38,745
  lines, 1,150 tests, and a `persistence.rs`. **It is the ancestor, not
  this.** Porting from it would discard the curation that makes "262
  tests must pass here" mean anything.

### The thing nobody mentions, so it is said here

**The 8-bit look was never achieved in any generation.** RoomViewer has
a complete texture path - a `tile_textures` table, a fetch, a
blend-mode render - that never fires, because no tile type was ever
given a `texture` key. The Rust renderer hand-rolls 6x6 PNGs with a
zlib encoder inside the rules crate instead. The SVG tiles in
`G:\My Drive\osdungeon\art` have never been wired to anything.

**AND THERE ARE TWO OF THEM.** Corrected 2026-10-22 after actually
looking: `dirt.svg` and `stonewall.svg`, against thirty-three terrain
words - the rest of that folder is character sprites, a logo and the
Illustrator sources. `dirt.svg` is also **26x26**, not 16x16.

This file said "the 16x16 SVG art" in three places and implied a set
existed. **PLANNING A CYCLE AROUND "WIRE UP THE TILE ART" WOULD HAVE
RUN INTO THAT ON THE FIRST AFTERNOON.** `ART.md` is the brief for the
thirty-one that do not exist, and it records why they should be PNG
rather than SVG: a vector format draws shapes smoothly at any size,
which is the opposite of pixel art.

So the art exists and the look does not. That is a thing to build once,
not to port.

---

## 2. Why this generation is the one that can finish it

**In every previous version the doors, locks, traps and secret doors
axis was authored and never consumed.**

- Apps Script generated lock and key pairs with shared UUIDs and a DC
  table, persisted them, and then **nothing ever checked a lock**.
  `container_state` is set and never read. `Lockpick Set`, `Gold Key`
  and `Brass Key` are inert loot. Traps are returned by `loadRoom` and
  never read by the viewer. Secret doors render as plain wall with no
  search action.
- The Rust port's `DoorState` and `ContainerState` are identical
  two-bool structs carrying no lock id, so they **cannot reach**
  `LockProfile`. Those 36 lock tests have never had a caller either.

**odyssey1e is the first version with the consumer.** 218 built scenery
that stands in a place, holds things, refuses to be carried, and is
reachable through RLS. A door is that plus geometry. 052 already lets a
challenge point at an object, so a door with a DC is expressible today.

That is the argument for building this now rather than treating the map
as decoration.

---

## 3. Decisions settled

**A player cannot read a tile they have not seen.** Not hidden by the
client - unreadable. This is 205/206's principle applied to space
instead of to hit points: the engine decides what you may see and hands
back a filtered answer.

**One map per location.** Not a separate "scene" concept. A second
hierarchy beside the location tree is the thing we would regret.

**A diagonal costs 1, and one function says so.** 5e PHB default;
the DMG's alternating 1-2-1 variant is a later arm of the same
function, not a second system.

**A tile is a row.** Not a jsonb grid on `locations`. This reverses an
earlier draft of this file; the reasoning is in section 4, and the
short version is that the fog decision above argues against a blob
because RLS cannot filter inside one.

**z is a height, in the same unit as x and y.** One z is one square is
five feet. Not a layer index, not a bounded range. Negative is legal.

The third needs its reasoning recorded, because three answers currently
coexist in the old code and they contradict each other:

```
distance_to / tiles_in_radius   Chebyshev - a diagonal is 1
TileGrid::adjacent              all 8 neighbours, undifferentiated
the Apps Script A*              4-directional - no diagonals at all
```

The pathfinder would route around a step the distance function calls
adjacent. `movement.rs` does not resolve this: `spend(squares, cost)`
takes a count and has no opinion, so the rule lives in whoever calls
it. **That is not agnosticism worth keeping, it is an unmade decision**
- and distance is asked by four subsystems that must agree or a
fireball covers different squares than it reaches:

    movement cost      melee reach      area of effect      light falloff

So: one `step_cost` in `movement.rs`, with tests, and everything calls
it. Chebyshev means three of the four callers need no change and the
A* becomes 8-directional.

---

## 4. The data model

### Storage: a tile is a row

**This reverses an earlier draft of this file, which put the grid in a
`jsonb` column on `locations`.** That was carried over from OSDUNGEON,
where it made sense for a reason that no longer applies: the prototype
never STORED tiles at all - it regenerated them from `scene.id` on
every load, so there was nothing to contend over. The moment we decided
to store the grid so that a hand-edit survives, the argument for the
blob evaporated. It survived into the first draft only because
`TileGrid` already had serde derives, which is optimising for the port
rather than for the game.

**The fog decision is what settles it.** RLS is row-level and cannot
filter inside a jsonb value, so a blob would need a `SECURITY DEFINER`
function to redact it per character. With rows, the policy IS the
redaction:

| | a jsonb blob | tiles as rows |
|---|---|---|
| fog | a definer function filters and rebuilds | **RLS does it; the function disappears** |
| what a player downloads | the whole map, always | only what they have seen - fifty rows, not sixteen hundred |
| two people writing | read-modify-write, last writer wins | each tile its own row, no collision |
| realtime | the whole grid per door opening | one row |
| merge step on read | yes | none |

Note which way the payload goes: **RLS makes a player's read smaller,
not larger.** A party entering a dark room pulls a handful of rows.

It is also what this codebase already is. 001 made a roll a record; 013
made hit points a log of deltas; 218 made a crate a row. A tile is a
row is the same instinct.

```sql
tiles
  location_id  uuid     not null
  x, y         int      not null
  z            smallint not null default 0
  terrain      text     not null
  is_wall      boolean
  integrity    int
  is_revealed  boolean
  is_trapped   boolean
  ...
  unique (location_id, z, y, x)
```

Read it back with `order by z, y, x` - layer, then row-major within it,
because `TileGrid` wants one contiguous row-major `Vec` per layer.

### What stays jsonb, and why that is not a special case

```sql
locations
  + recipe       jsonb   -- floorType, wallType, lighting, hazardZones[], seed
  + px_per_grid  int
```

The rule that decides it:

> **jsonb for what the DM authors and play does not change.
> Rows for anything two people can change at once.**

`recipe` is authored in the editor and untouched during play, so it has
no contention and never needs filtering. It is worth keeping at all for
the OSDUNGEON reason: **the recipe is the provenance.** The rows are
the record of what the map IS; the recipe is the account of what
produced it, so a room can be regenerated or re-rolled without those
being three separate features. This is 001 again.

`px_per_grid` is UVTT's one genuinely good idea: every position and
size is in grid squares, and exactly one number knows about the image.
Re-render the map at a different resolution and not one stored
coordinate changes.

### z is a height, not a layer

One z = one square = five feet, exactly like x and y.

```
floor       z=0      bridge deck 10 ft up    z=2
pit bottom  z=-1     flying creature 30 ft   z=6
```

Three different things could have been meant by "z", and **two were
already solved**:

| | answered by |
|---|---|
| dungeon LEVELS (floor 1, 2, 3) | the location tree - each level is its own location |
| ELEVATION within a room (a ledge, water depth) | `Tile.elevation: i8`, already in the ported struct |
| two things at one (x,y) at different heights - **a bridge over a chasm, a balcony, a flying creature** | nothing. this is the gap z fills |

**What makes it cheap: tiles are sparse, things are free.** A tile row
exists only where there is a surface, so a bridge over a chasm is
twenty rows above sixteen hundred, and the air between them is nothing
at all. A flying creature needs no tile to be at `z=6`; it simply has a
z.

**Negative is legal** - a pit bottom, a lower vault, depth below a
water surface. A 0-to-99 range would have forbidden all three.

**Movement mapping comes free later** because z is in the same unit as
x and y: a climb of two is two squares, the same arithmetic as moving
two squares sideways, and `MovementMode::Fly` in `movement.rs` already
contemplates it. A layer index would have needed a conversion, and the
conversion is where the bug would have lived.

**The schema takes z now; the Rust stays 2D for now.** That asymmetry
is deliberate. Retrofitting a dimension into the schema is expensive -
every index, the unique constraint, every stored row, a migration over
live data. Retrofitting it into the Rust is cheap, precisely because
the Rust is pure and tested: adding z to `line_of_sight(x1,y1,x2,y2)`
today means rewriting ported code and invalidating the 262 tests that
were the entire reason to port instead of rebuild. So phases 1-5 load
`where z = 0`, and when bridges are wanted the data has not moved and
the ground-layer tests still pass.

### Position goes on rows that are already in a place

No token table. The things that can stand somewhere already have a row
saying which place they are in.

```sql
objects           + x, y, z   -- holder_id already points at the location's entity
characters        + x, y, z   -- location_id already says which room
encounter_actors  + x, y, z   -- live combat position, already checkpoints by round
```

Note the existing asymmetry this rides on, recorded so nobody trips on
it: **a character references a location by `id`; an object is held by
its `entity_id`.** `drop_object` already walks that translation, and a
take runs it backwards.

### Fog

```sql
tile_memory  (character_id, location_id, seen)   -- what each character has seen
```

With tiles as rows this needs no definer function. The `tiles` SELECT
policy is `is_game_dm(...) or seen_by(current_profile(), location_id,
x, y, z)`, and a player simply cannot read a row they have not
uncovered.

**`seen` should be a bitmap**, not a row per tile: a 40x40 layer is
1,600 bits - 200 bytes, one row per character per location. A row per
discovered tile is 1,600 rows doing the work of 200 bytes.

**The one number to measure before Phase 4 commits to this** is that
policy at scale: a bit-test function called once per row, for 1,600
rows. It should be microseconds, and it is cheap to test - but it is a
measurement rather than an assumption, and the fallback (`tile_memory`
as rows, so the policy is an EXISTS join Postgres optimises well) is a
different trade rather than a worse one.

This also fixes something the prototype got wrong: Apps Script reset
fog to `{}` on every room load, so re-entering a room re-fogged it.
Persisting per character fixes that for free, and makes "what has the
party actually mapped" a question the DM can answer.

### Doors are fixtures with geometry

Walls live in the grid. **Doors are objects**, because a door is a thing
with a lock, a DC, contents and a description, and a wall is a shape.
All three previous generations got this wrong in different ways - a
hole in a perimeter, or a two-bool struct that could not reach the lock
system.

UVTT imports along that same split: `portals` become fixture objects,
`line_of_sight` becomes grid geometry.

### Units already agree

1 tile = 5 ft, 6 tiles = 30 ft per round, 1 round = 6 seconds = 1 tick.
Stated three times in the old code and already true of our clock since
215. Nothing to reconcile - and z is the same unit, so a climb of two
is two squares by the same arithmetic as walking two.

---

## 5. The order of work

Each phase is independently shippable and verified the way everything
else here is: a migration with a self-check that refuses to commit if
the property does not hold, Rust tests, a rig run for anything on
screen, and a STATUS entry.

| # | Phase | What lands | Risk |
|---|---|---|---|
| **0** | ~~Port the rules cold~~ **SUPERSEDED - see STATUS 225** | The baseline ran: 262 pass in the engine at `2551f1e`, 0 ignored. It also showed the engine resolves on PERCENTILE and this project on d20 vs DC, so the engine is now a REFERENCE. `grid.rs` and `terrain.rs` are written here instead. | — |
| **1** | **A location can have a map** — **LANDED**, 226-246 | The `tiles` table, `recipe`, `px_per_grid`, bounds, facing, saved versions. **NOT read-only**: 241 made the canvas the authoring surface, which retired the dev hook. UVTT import never built and nobody has wanted it. | low |
| **2** | **Things have positions** — **HALF LANDED**, 230 | `x, y, z` on CHARACTERS, with a trigger; `stride::step` moves them and names a wall. **Objects have none**, so no crate is drawn and clicking one does not open 218's panel. **The map stopped being a picture** for creatures and has not for things. | low |
| **3** | **Doors, locks, traps** | Doors as fixtures. Wire `lock.rs` and `trap.rs` - the first consumer they have ever had. | medium |
| **4** | **Sight and light** | Bresenham LOS, the Apps Script lighting model, `tile_memory` and the fog policy on `tiles`. | medium |
| **5** | **Movement** | `step_cost`, budget, 8-directional A*, into initiative and 051's action economy. | medium |
| **6** | **The editor** — **MOSTLY LANDED EARLY**, 241/246 | It arrived out of order, because a read-only map with a dev hook was worse than an editable one. Click to paint, a brush with cover and facing, the room's recipe, save-as and load. Tile art IS wired - 18 of the 33 words have a tile; see `ART.md`. Still missing: undo, pan and zoom, and a rectangle brush. | medium |

**EVERYTHING FROM HERE TO THE END OF SECTION 5 IS SUPERSEDED.** It is
the case for porting `odyssey-engine` cold, and 225 ran the baseline
and decided against it: the engine resolves on PERCENTILE and this
project rolls d20 against a DC, so it became a reference and
`grid.rs`, `terrain.rs`, `stride.rs` and `ground.rs` were written
here. The four edits below were never made and the defect list was
never worked through. **Kept because the reasoning is still the right
reasoning about that engine**, and because anybody tempted to port it
later should read what the attempt would involve.

**Phase 0 is the one to insist on.** It carries no risk, touches no
schema, and converts "262 tests exist somewhere" into "262 tests pass
here" - which is the fact every later phase rests on. Its real content
is four edits, none of them about persistence:

1. **Decouple `scene.rs` from `item`/`container`** - about 110 lines
   across four functions, which read exactly three item flags and three
   placement fields. A small local value type kills a ~110 KB
   dependency on inventory code we already have our own version of.
2. **Lift the CSS renderer out of `tile.rs`** - 403 lines, including a
   hand-rolled PNG encoder, that belong in `commands/`. Moving them also
   breaks a `tile` <-> `scene` cycle.
3. **Add serde derives to `Scene`**, which has none at all today
   because the old app only ever persisted `TileGrid`.
4. Carry `challenge.rs`, `ObjectIntegrity`, and three small enums.

### Defects to fix on the way in rather than port faithfully

- `LOCK_ID_COUNTER` is a process-global starting at 1, so lock ids are
  not stable across a restart. Make it a caller-supplied uuid.
- `TileGrid::next_uid` is `#[serde(skip)]`, so it resets to 0 on reload
  and collides with existing tiles.
- `SceneState::light_at` is a `// TODO Sprint 4` stub that ignores x and
  y entirely. Per-tile lighting was never implemented in Rust.
- `TerrainType` and `TerrainMovement` are parallel vocabularies with no
  mapping between them, and the terrain-aware half of `movement.rs` is
  tested but never called.
- `Scene::can_see` linear-scans every placement per ray step.
- `bresenham_line` allocates a `Vec` per call; make it an iterator.
- The Apps Script creature spawner's wall check never matches, so
  creatures spawn inside walls.

---

## 6. What not to port

**The Apps Script combat layer is a demo, not a system.** There is no
player attack function at all - `npcAttack_` exists with no counterpart,
so combat can only end in the player's death. The Action/Bonus/Reaction
dots are decorative; `actionUsed` is only ever assigned `false`. The AI
is ten lines. Tokens do not block each other. Attack resolution is a
hardcoded client-side `d20 >= 10` for `1d6`, ignoring AC. Debug text
renders into the player-visible combat log.

**The client-authoritative split.** In the prototype the server owned
inventory, initiative and movement budget while the client owned HP,
torch burn, breath and every damage roll. Do not carry that.

**The Rust command that feeds the renderer** marks every tile
discovered and visible (`// Sprint 4 adds real LOS`) and hardcodes four
actor positions. The CSS contract is reusable; that command is a
placeholder to rewrite.

Treat the DungeonLoader schema - `LOCK_QUALITIES`, chest templates, the
39-item loot table, the creature templates - as **a content spec rather
than as working features**.

---

## 7. Still open

- **Procedural generation.** The Apps Script dungeon generator is a
  random-walk on a coarse grid with no corridors - rooms abut and share
  a doorway - plus a 30% chance of extra loop connections. Whether we
  want generation at all, or only hand-authored and imported maps, is
  not decided. `generateRoomTileHints` is worth porting regardless,
  because it is geometry-free.
- ~~**Whether `encounter_actors.x, y` or `characters.x, y` is the truth
  during a fight.**~~ **SETTLED BY 230: `characters` only.** Two
  columns both answering "where is Bryn" would disagree during a fight
  and need copying back after - the two-places fault with a round
  counter attached. The want behind the second one is PER-ROUND
  HISTORY, not a second present, and it can be built when something
  needs to rewind a round.
- **Scale.** A 45x45 layer is 2,025 tile rows. Nothing here has been
  measured against a REST round trip, and the fog policy runs a
  per-row test for every one of them. See the note in section 4 - this
  is the one number to get before Phase 4.
- **Whether a fixture should be destroyable** - backlog item 1's
  "attacking a thing", unchanged by any of this.

## 8. The look, and a three-dimensional future

Added 2026-10-22, because `surface`/`cover`/`sub` should be read as a
choice made with this known rather than in ignorance of it.

### The near goal is 8 or 16-bit

Flat pixel art on a grid. **Canvas 2D is the right tool and it is not
close**: one line, `imageSmoothingEnabled = false`, and a 16x16 sprite
scales to exactly 32 with hard edges. **The doubling has to be a
whole number** - at the 28px square 235 shipped with, a 16px tile
scales by 1.75 and some source pixels become two screen pixels while
others become one, which destroys pixel art even with smoothing off.

> **236 SETTLED THIS DIFFERENTLY AND BETTER.** Dave chose 32x32 art
> drawn at 32 pixels, so there is no scaling at all - the sharpest
> result available - and the extra resolution leaves room for
> fixtures and characters to match the terrain later. The requirement
> was never the NUMBER 16; it was that the scale be a whole number.
> `map.js` refuses a tile that is not 32x32 and names it in the pane,
> because a wrong-sized tile is a silent blur.

CSS can be made to do it and fights you on sub-pixel positioning;
WebGL is a GPU pipeline for a problem that is not GPU-bound.

**AND SOME OF THE ART EXISTS.** Two terrain tiles, which is less than
this file used to imply - see section 1, which also records the sad
part: the Apps Script prototype has a complete texture path - a
`tile_textures` table, a fetch, a blend-mode render - that never
fires, because no tile type was ever given a `texture` key. The SVGs
are in `G:\My Drive\osdungeon\art`, no generation has ever drawn one, and
**there are two terrain tiles in there, not a set** - see `ART.md`.

The 8-bit look is not a stretch goal. It is the thing three attempts
have had the pieces for and never assembled.

### Why not WebGL now

Not performance. **A tactical map is not a game loop.** It redraws when
something changes - a creature moves, a door opens, a torch is lit -
which is a few times a second at most, against a worst case of perhaps
ten thousand tiles. Canvas is comfortable at fifty thousand draws per
frame at sixty frames per second, so we would be using about two per
cent of it.

WebGL earns its complexity on real-time per-pixel lighting with moving
sources. The model section 1 praises is not that: distance-profile
arrays per burn phase - `[100x8, 50x4, 25x4]` - are discrete rings, and
a max-combine of four contributors is per-TILE compositing. The Apps
Script version achieved it in the DOM, which is a lower bar than
canvas.

In practice WebGL also means PixiJS, and **this project has no
bundler.** That is not fatal but it would be the first real dependency
in a codebase where "no build step" has been load-bearing.

### A Minecraft-like expansion: a different renderer, and that is fine

Voxel 3D is Three.js, and it would **reuse none of the canvas
renderer**. Different geometry, different culling, different
everything. Building canvas now costs nothing against that future,
because a new renderer would be written either way.

What matters is whether the DATA survives it, and two decisions already
do:

**`z` IS A REAL HEIGHT IN FIVE-FOOT UNITS, NOT A LAYER INDEX.** Settled
in section 3 for 5e reasons - falling, bridges, flying creatures - and
it is exactly what a voxel world needs: consistent three-dimensional
addressing. A layer index would not have converted.

**TILES ARE SPARSE WITH A DEFAULT.** "Stone floor unless stated" (234)
is structurally the same idea as "air unless stated", which is how
voxel worlds actually store themselves.

### What would need rethinking, said plainly

**`surface`/`cover`/`sub` IS 2.5D THINKING.** In a voxel world a square
does not have layers: a CUBE has one material, and "a wall standing on
a floor" is a stone block at z = 0 above a stone block at z = -1.

It converts roughly - a cover becomes the block, a surface with no
cover becomes air over a block - but it is a translation rather than a
straight read.

**IT SHOULD NOT BE CHANGED NOW.** Layers are right for a 5e tactical
map; they solved "it snowed" and "a wall fell down" in one move (226).
Voxelising early would make the actual game worse to serve a maybe, and
that is the wrong trade every time.

### The insurance, which costs nothing because it is already done

`grid`, `terrain`, `stride` and `ground` are coordinate maths and
material properties. **They survive both futures untouched.** The
renderer is the part that would be replaced, and section 6 already
insists it stay separable - a renderer tangled into the rules is one
you cannot replace.

Keep the rules in Rust and the drawing in one JS module, and swapping
canvas for Three.js is a new module rather than a rewrite.
