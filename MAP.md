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
| where | `G:\My Drive\osdungeon` | `C:\Users\tiray\Dev\odyssey-engine` | here |
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

### The thing nobody mentions, so it is said here

**The 8-bit look was never achieved in any generation.** RoomViewer has
a complete texture path - a `tile_textures` table, a fetch, a
blend-mode render - that never fires, because no tile type was ever
given a `texture` key. The Rust renderer hand-rolls 6x6 PNGs with a
zlib encoder inside the rules crate instead. The 16x16 SVG tiles in
`G:\My Drive\osdungeon\art` have never been wired to anything.

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
| **0** | **Port the rules cold** | Six modules plus `challenge.rs` into `src/`, as unused code. **262 tests must pass here.** No UI, no migration. | very low |
| **1** | **A location can have a map** | The `tiles` table, `recipe`, `px_per_grid`. UVTT import. Read-only render, DM only. | low |
| **2** | **Things have positions** | `x, y, z` on objects and characters. Fixtures and creatures drawn on the map; clicking a crate opens 218's panel. **The map stops being a picture.** | low |
| **3** | **Doors, locks, traps** | Doors as fixtures. Wire `lock.rs` and `trap.rs` - the first consumer they have ever had. | medium |
| **4** | **Sight and light** | Bresenham LOS, the Apps Script lighting model, `tile_memory` and the fog policy on `tiles`. | medium |
| **5** | **Movement** | `step_cost`, budget, 8-directional A*, into initiative and 051's action economy. | medium |
| **6** | **The editor** | RoomDesigner's UX on the new model, and the 16x16 SVG tiles wired for the first time. | medium |

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
- **Whether `encounter_actors.x, y` or `characters.x, y` is the truth
  during a fight.** Both, probably, with the encounter row winning while
  one is running - but the checkpoint rule needs stating.
- **Scale.** A 45x45 layer is 2,025 tile rows. Nothing here has been
  measured against a REST round trip, and the fog policy runs a
  per-row test for every one of them. See the note in section 4 - this
  is the one number to get before Phase 4.
- **Whether a fixture should be destroyable** - backlog item 1's
  "attacking a thing", unchanged by any of this.
