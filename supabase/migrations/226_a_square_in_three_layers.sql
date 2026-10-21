-- 226. A SQUARE IN THREE LAYERS.
--
-- The first table the map has. `grid.rs` holds the arithmetic and
-- `terrain.rs` holds the physics; this is where a square's materials
-- live.
--
-- ---------------------------------------------------------------------
-- WHY THREE COLUMNS AND NOT ONE
-- ---------------------------------------------------------------------
--
-- 225 gave a square ONE terrain and it could not say two things:
--
--   IT SNOWED. `Snow` was a terrain, so a winter map was a DIFFERENT
--   MAP - you could not say "it snowed" without rewriting every square
--   and losing the road beneath it. The calendar has named four seasons
--   since 214.
--
--   A WALL FELL DOWN. Destroying it left `Rubble`, which erased the
--   floor it had stood on. 225 wrote that down as a limitation, because
--   with one layer both cannot be true at once.
--
-- A WALL SITS ON A FLOOR. With the wall as `cover` over a `surface`,
-- the square is blocked while it stands and the floor is still there
-- when it comes down. The same move puts grass on dirt and trees on
-- ground - which is why there is no `forest` in the vocabulary. A wood
-- is ground with trees on it.
--
-- `sub` IS THE THIRD AND THE ODD ONE: it takes no part in crossing the
-- square at all. Water under ice changes nothing about walking on the
-- ice; it matters the moment the ice breaks and not before.
--
-- ---------------------------------------------------------------------
-- sub IS AT THE SAME HEIGHT. z IS NOT.
-- ---------------------------------------------------------------------
--
-- These answer different questions and neither replaces the other.
-- `sub` is the substrate of THIS square - the water the ice is a lid
-- over, at the same height as the ice. `z - 1` is a genuinely lower
-- place: a cellar, a pit floor.
--
-- SO THERE IS NO `height` AND NO `depth` COLUMN. A ten-foot wall is
-- cover at z = 0 and again at z = 1; a foundation two squares down is
-- z = -1 and z = -2. MAP.md fixed z as a height in five-foot units, and
-- a second number saying how tall a thing is would be two ways to state
-- one vertical fact.
--
-- ---------------------------------------------------------------------
-- THE VOCABULARY IS A CHECK, NOT A TABLE
-- ---------------------------------------------------------------------
--
-- A `terrain_words` lookup table would be the tidier-looking answer and
-- it would be wrong: the vocabulary is a RULE, and rules live in Rust.
-- Putting it in Postgres as well is the two-places fault this schema
-- keeps finding.
--
-- THESE THREE LISTS WERE GENERATED FROM `terrain::ALL`, not retyped,
-- and a Rust test reads THIS FILE and compares them - so the day
-- somebody adds a word to the enum and not to the constraint, the test
-- says so rather than a player's map refusing to save.
--
-- 23 surface words, 15 cover words, 33 distinct. The five walls are in
-- both lists and are the only overlap: a wall has a top you stand on
-- (surface), a middle that stops you (cover) and a footing below the
-- ground around it (sub). z tells the three apart.
--
-- `sub` SHARES THE SURFACE LIST because a substrate is just the ground
-- that would be revealed.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY ABSENT
-- ---------------------------------------------------------------------
--
-- `is_wall`. MAP.md proposed it; it is now derived from `cover`, and
-- storing it would be a second place for one fact.
--
-- `is_revealed`. MAP.md proposed it too, and a BOOLEAN CANNOT EXPRESS
-- FOG: what one player has seen is not what another has. Fog is Phase 4
-- and already has `tile_memory` planned for it there.
--
-- `is_trapped`. Traps are Phase 3 and belong on a fixture.
--
-- All three would be columns with no reader, which is the thing this
-- project keeps deciding not to carry.
--
-- ---------------------------------------------------------------------
-- A MISSING ROW MEANS UNMAPPED, NOT EMPTY FLOOR
-- ---------------------------------------------------------------------
--
-- Tiles are sparse on purpose - a party entering a dark room pulls a
-- handful of rows, not sixteen hundred. The consequence worth writing
-- down now: NOTHING BELOW YOU IS NOT A FLOOR. When falling is built, an
-- absent tile is empty air rather than ground.
--
-- ---------------------------------------------------------------------
-- RLS: THE DM, AND ONLY THE DM, FOR NOW
-- ---------------------------------------------------------------------
--
-- Phase 1 is a read-only render on the DM's screen. So the policy is
-- the promise that is actually TRUE today - you reach tiles for a
-- location in a game you run - and the fog policy REPLACES it in Phase
-- 4 rather than extending it.
--
-- Writing the player half now would mean writing a policy against a
-- fog model that does not exist, and policies that describe intentions
-- rather than behaviour are how 204 came to hide a character sheet.
--
-- THE JOIN IS THROUGH `locations`, which carries `game_id`. `tiles`
-- does NOT denormalise it: a game id on this row would be a second
-- place for a fact `location_id` already determines, and the two could
-- disagree. `is_game_dm` is STABLE, so the subquery is hoisted.
--
-- AND `anon` IS REVOKED EXPLICITLY, WHICH IS A DEPARTURE. Supabase
-- grants `anon` full table privileges by default and every other table
-- here relies on RLS alone to stop it. 193 revoked `anon`'s execute on
-- functions for exactly this reason and did not sweep the tables. The
-- policies below need `current_profile()` so `anon` fails them anyway -
-- this is belt and braces, and if the inconsistency is worse than the
-- defence, delete the four revokes and nothing else changes.

create table public.tiles (
  id           uuid primary key default gen_random_uuid(),
  location_id  uuid        not null references public.locations(id) on delete cascade,

  x            int         not null,
  y            int         not null,
  z            smallint    not null default 0,

  surface      text        not null,
  cover        text        null,
  sub          text        null,

  surface_hp   int         null,
  cover_hp     int         null,

  leads_to     uuid        null references public.locations(id) on delete set null,

  created_at   timestamptz not null default now(),

  constraint one_square_one_row unique (location_id, z, y, x),

  constraint a_surface_is_ground check (surface in (
    'rock','dirt','mud','sand','scree','road','stone_floor','wood_floor',
    'marsh','ice','shallow_water','deep_water','lava','pit','ledge',
    'doorway','ramp','stairs',
    'stone_wall','timber_wall','rock_wall','wooden_wall','ice_wall')),

  constraint a_sub_is_ground_too check (sub is null or sub in (
    'rock','dirt','mud','sand','scree','road','stone_floor','wood_floor',
    'marsh','ice','shallow_water','deep_water','lava','pit','ledge',
    'doorway','ramp','stairs',
    'stone_wall','timber_wall','rock_wall','wooden_wall','ice_wall')),

  constraint a_cover_stands_on_something check (cover is null or cover in (
    'grass','snow','brush','trees','vines','rubble',
    'door_open','door_closed','door_broken','iron_grate',
    'stone_wall','timber_wall','rock_wall','wooden_wall','ice_wall')),

  -- A link is what a doorway IS for. On any other square it would be a
  -- promise nothing keeps.
  constraint a_link_needs_a_doorway
    check (leads_to is null or surface = 'doorway'),

  -- Hit points are a REMAINDER, not a total: null is "intact at
  -- whatever `terrain.rs` says this is worth". Negative is nonsense and
  -- zero is destroyed, which is a state the row should not linger in.
  constraint damage_is_not_negative
    check ((surface_hp is null or surface_hp >= 0)
       and (cover_hp   is null or cover_hp   >= 0)),

  -- Undamaged cover cannot have hit points, because there is no cover
  -- to have them.
  constraint only_a_cover_can_be_damaged
    check (cover_hp is null or cover is not null)
);

comment on table public.tiles is
  'One square of a map, in three layers - see 226. `surface` is the '
  'ground and is required; `cover` is what stands on it; `sub` is the '
  'substrate AT THE SAME HEIGHT, revealed when the surface is broken '
  'and taking no part in crossing. A missing row is unmapped, not an '
  'empty floor.';

comment on column public.tiles.z is
  'Height in five-foot squares, the same unit as x and y. Not a layer '
  'index: negative is legal and a pit bottom is -1. A ten-foot wall is '
  'two rows, which is why there is no height column.';

comment on column public.tiles.sub is
  'The ground beneath this square''s surface, at the same height. Water '
  'under ice. Read only when the surface is destroyed - see '
  '`Destruction::Reveals` in terrain.rs, which names no terrain for '
  'exactly this reason.';

comment on column public.tiles.surface_hp is
  'Hit points REMAINING, or null for intact. The total lives in '
  'terrain.rs; storing it here would be a second place for one fact.';

comment on column public.tiles.cover_hp is
  'Hit points REMAINING on the cover, or null for intact.';

comment on column public.tiles.leads_to is
  'Where a doorway goes. The rule "this square is a threshold" is the '
  'terrain; the fact "it opens on the undercroft" is this column.';

-- The unique constraint indexes (location_id, z, y, x), which is also
-- the read order - `order by z, y, x` walks it without a sort.

alter table public.tiles enable row level security;

-- PHASE 1 IS THE DM'S SCREEN. Four policies, one audience. The fog
-- policy in Phase 4 replaces these rather than extending them.
create policy "tiles: dm reads"
  on public.tiles for select
  using (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)));

create policy "tiles: dm writes"
  on public.tiles for insert
  with check (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)));

create policy "tiles: dm updates"
  on public.tiles for update
  using (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)))
  with check (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)));

create policy "tiles: dm deletes"
  on public.tiles for delete
  using (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)));

revoke select, insert, update, delete on public.tiles from anon;

-- ---------------------------------------------------------------------
-- AND THE TWO COLUMNS A MAP NEEDS ON ITS LOCATION
-- ---------------------------------------------------------------------
--
-- MAP.md settled both with their reasoning; they arrive here rather
-- than in a migration of their own because a tile table without them
-- is a map that cannot say where it came from or how big it is drawn.
--
-- THE RULE THAT DECIDES jsonb VERSUS ROWS, from MAP.md:
--
--   jsonb for what the DM authors and play does not change.
--   Rows for anything two people can change at once.
--
-- `recipe` is authored in the editor and untouched during play, so it
-- has no contention and never needs filtering per player. It is worth
-- keeping at all for the OSDUNGEON reason: THE RECIPE IS THE
-- PROVENANCE. The rows are the record of what the map IS; the recipe is
-- the account of what PRODUCED it, so a room can be regenerated or
-- re-rolled without those being three separate features. This is 001
-- again - a roll is a record.
--
-- IT IS DELIBERATELY UNCONSTRAINED. MAP.md sketched it as floorType,
-- wallType, lighting, hazardZones[] and a seed, which was written
-- before a square had three layers; the shape it actually wants is
-- defaults for `surface` and `cover` per region. Pinning that down now
-- would be guessing at a generator nobody has written, and a CHECK on
-- a shape that has never been produced is a constraint that only ever
-- refuses honest data.
--
-- `px_per_grid` IS UVTT'S ONE GENUINELY GOOD IDEA. Every position and
-- size in this schema is in GRID SQUARES, and exactly one number knows
-- about the image. Re-render the map at a different resolution and not
-- one stored coordinate changes.
--
-- BOTH ARE NULLABLE because every location that exists today has
-- neither, and a map is something a location MAY have rather than must.

alter table public.locations
  add column if not exists recipe      jsonb null,
  add column if not exists px_per_grid int   null;

alter table public.locations
  add constraint a_grid_square_has_a_size
    check (px_per_grid is null or px_per_grid > 0);

comment on column public.locations.recipe is
  'How this map was produced, as authored - not what it IS, which is '
  'the tiles. Deliberately unconstrained: see 226. jsonb because the DM '
  'authors it and play never changes it, so it has no contention and '
  'needs no per-player filtering.';

comment on column public.locations.px_per_grid is
  'Pixels per five-foot square in the rendered image. The ONLY number '
  'in this schema that knows about pixels - every coordinate is in '
  'squares, so re-rendering at another resolution changes nothing '
  'stored.';
