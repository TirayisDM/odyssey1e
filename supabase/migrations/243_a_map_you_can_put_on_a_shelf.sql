-- 243. A MAP YOU CAN PUT ON A SHELF.
--
-- Dave: *"Need a save feature so once a map is drawn it can be saved
-- to be reloaded, and a save as so it can be saved as a new or
-- different version"*.
--
-- The first half already works and has since 229: every click is an
-- INSERT, so a drawn map survives closing the app whether anybody
-- pressed anything or not. A Save button would write nothing, and a
-- button that reassures you while doing nothing is this project's
-- named defect in its most irritating form.
--
-- **THE SECOND HALF IS THIS TABLE.** A named copy of a map, taken on
-- purpose, that can be put back later.
--
-- ---------------------------------------------------------------------
-- A SEPARATE TABLE, NOT A VERSION COLUMN ON `tiles`
-- ---------------------------------------------------------------------
--
-- The tempting design is `tiles.version_id`, with one version marked
-- current. It is wrong here. `ground::at`, `map_at`, `stride::step`,
-- the four RLS policies and the `locations` row would all have to
-- agree about which version is live, and "which version am I looking
-- at" would become a fact in five places that can disagree.
--
-- With a separate table, **nothing that reads a map changes at all.**
-- The live map is still the `tiles` rows, unqualified. Saving copies
-- out of them; loading replaces them. A reader cannot tell this
-- migration happened, which is the whole point.
--
-- ---------------------------------------------------------------------
-- jsonb FOR THE SQUARES, AND IT IS THE RULE RATHER THAN AN EXCEPTION
-- ---------------------------------------------------------------------
--
-- 226 chose rows over a jsonb blob for `tiles` and recorded why, from
-- MAP.md:
--
--   jsonb for what the DM authors and play does not change.
--   Rows for anything two people can change at once.
--
-- A saved version is authored once and never touched again - play
-- cannot reach it, two people cannot edit it at once, and no player
-- ever reads it. It is the jsonb side of that rule, not a departure
-- from it.
--
-- The other half of 226's reasoning was fog: **RLS is row-level and
-- cannot filter inside a blob**, so a map a player must see only parts
-- of has to be rows. A version is a DM's shelf and is never shown to a
-- player at all, so there is nothing to filter and nothing lost.
--
-- SIZE. A fully painted forty-by-forty level is 1,600 squares, call it
-- 160 kB of jsonb; a hundred-by-hundred is about a megabyte. Postgres
-- puts that out of line and does not care. If this ever becomes a
-- problem it will be because somebody is storing hundreds of versions
-- of enormous maps, and the fix then is deleting some.
--
-- ---------------------------------------------------------------------
-- A VERSION IS NOT JUST THE TILES
-- ---------------------------------------------------------------------
--
-- Test Inn 1 is **four painted rows and "20x20, default mud"**. Put
-- the rows back without the recipe and you get four squares floating
-- in nothing, because 233 resolves an absent row against the room's
-- default and a room with no default has no squares at all.
--
-- So `width`, `height` and `recipe` are here beside the squares. They
-- duplicate three columns of `locations`, and that is not the
-- two-places problem: **a snapshot is a copy by definition.** The
-- failure mode of the two-places problem is two live facts drifting
-- apart; a version is not live, and the day it equals the location
-- again is the day somebody loaded it.
--
-- They are columns rather than jsonb keys so that a list of versions
-- can say "20x20" without parsing a blob.
--
-- ALL LEVELS, NOT ONE. There is no `z` on this table. A dungeon is its
-- floors, and versions taken a level at a time could be restored out
-- of step with each other - a staircase arriving where 233 says there
-- is no floor. The whole map or nothing.
--
-- ---------------------------------------------------------------------
-- WHAT GOES IN A SQUARE, AND THE ONE DANGEROUS FIELD
-- ---------------------------------------------------------------------
--
-- Every column of `tiles` that describes the map: x, y, z, the three
-- layers, both hit-point columns, `facing`, and the link.
--
-- **DAMAGE IS IN, by Dave's decision.** `surface_hp` and `cover_hp`
-- are hit points REMAINING, so a version can carry a half-broken door.
-- Saving mid-fight and putting it back is a legitimate DM move, and a
-- copy that quietly healed everything would be lying about what it
-- copied.
--
-- **CREATURES ARE OUT, by Dave's decision.** A creature is a
-- `characters` row with x/y/z (230) that belongs to a GAME, not to a
-- map. A terrain restore that teleported the party would be a nasty
-- surprise, and 033 already refuses to put a character in another
-- game's location - a version loaded into a different game would walk
-- straight into that refusal.
--
-- **`leads_to` IS THE DANGEROUS ONE.** On `tiles` it is a real foreign
-- key with `on delete set null`, so deleting a place quietly empties
-- the doors that pointed at it. **Inside jsonb there is no foreign key
-- and no cascade**, so a version taken last week can name a location
-- that no longer exists, and putting it back would either violate the
-- FK on insert or - worse, if somebody reached for `on conflict do
-- nothing` - silently drop the square.
--
-- That is a rule, so it lives in Rust: the load command drops a
-- `leads_to` whose location is gone and SAYS SO - "2 links dropped,
-- those places no longer exist" - per 217. It is named here because
-- the absence of the foreign key is deliberate and a later reader
-- would otherwise read it as an oversight.
--
-- ---------------------------------------------------------------------
-- THE NAME IS THE IDENTITY
-- ---------------------------------------------------------------------
--
-- Unique per location, so "Save as" with a name already on the shelf
-- is REFUSED rather than silently overwriting the thing you meant to
-- keep. Overwriting on purpose is deleting and saving again, which is
-- two deliberate acts instead of one ambiguous one.

create table public.map_versions (
  id           uuid        primary key default gen_random_uuid(),
  location_id  uuid        not null references public.locations(id) on delete cascade,

  name         text        not null,
  saved_at     timestamptz not null default now(),
  -- Who shelved it. `on delete set null` because a version outliving
  -- the person who took it is still a usable map.
  saved_by     uuid        null references public.profiles(id) on delete set null,

  -- The room, as it was. See "A VERSION IS NOT JUST THE TILES".
  width        int         null,
  height       int         null,
  recipe       jsonb       null,

  -- The squares, as they were. An array of objects; `[]` is legal and
  -- means a room with a recipe and nothing drawn in it, which is a
  -- real thing to want back.
  squares      jsonb       not null,

  constraint one_name_one_version unique (location_id, name),

  -- A version called "   " is a version nobody can ask for by name.
  constraint a_version_is_named check (length(btrim(name)) > 0),

  -- The ONE thing worth constraining about the blob. A caller that
  -- sent an object or a bare string has a bug, and finding it at the
  -- INSERT is cheaper than finding it when the load produces nothing.
  -- Nothing further is checked here: the vocabulary of terrain words
  -- lives in `terrain.rs` and 226 and 238 both refused to put a second
  -- copy of it in the database.
  constraint squares_is_an_array check (jsonb_typeof(squares) = 'array')
);

comment on table public.map_versions is
  'A named copy of a location''s whole map - the room''s size and '
  'recipe, plus every tile on every level - taken on purpose so it can '
  'be put back. NOT how a map is saved: drawing saves continuously '
  'because every click is an INSERT on `tiles`. This is "save as". '
  'Terrain only: creatures are characters and belong to a game, not a '
  'map. See 243.';

comment on column public.map_versions.squares is
  'Every tile of every level as an array of objects - x, y, z, the '
  'three layers, both hp columns, facing, leads_to. jsonb because a '
  'version is authored once and play never touches it, which is the '
  'jsonb side of MAP.md''s rule rather than an exception to it. '
  'WARNING: `leads_to` here is NOT a foreign key and has no cascade, '
  'so a version can name a deleted location - the load command drops '
  'those and says how many. See 243.';

comment on column public.map_versions.recipe is
  'The room''s defaults as they were. Without this a restored map is '
  'a handful of painted squares floating in nothing, because 233 '
  'resolves an absent row against the default.';

comment on column public.map_versions.width is
  'The declared bounds as they were, alongside `height`. Columns '
  'rather than jsonb keys so a list of versions can say 20x20 without '
  'parsing the blob.';

comment on column public.map_versions.saved_at is
  'When it was shelved. The list reads newest first; `name` is the '
  'identity.';

-- ---------------------------------------------------------------------
-- WHO MAY SEE A SHELF
-- ---------------------------------------------------------------------
--
-- The DM of the game the location belongs to, and nobody else - the
-- same four policies and the same join as `tiles` in 226, for the same
-- reason: `map_versions` does NOT carry `game_id`, because
-- `location_id` already determines it and a second copy could
-- disagree. `is_game_dm` is STABLE, so the subquery is hoisted.
--
-- THESE DO NOT GET A FOG VARIANT IN PHASE 4. A version is an
-- authoring tool; players never read it at all.

alter table public.map_versions enable row level security;

create policy "map_versions: dm reads"
  on public.map_versions for select
  using (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)));

create policy "map_versions: dm writes"
  on public.map_versions for insert
  with check (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)));

create policy "map_versions: dm updates"
  on public.map_versions for update
  using (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)))
  with check (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)));

create policy "map_versions: dm deletes"
  on public.map_versions for delete
  using (public.is_game_dm(
    (select l.game_id from public.locations l where l.id = location_id)));

-- 226's departure, kept consistent: Supabase grants `anon` full table
-- privileges by default, the policies above need `current_profile()`
-- so `anon` fails them anyway, and this is belt and braces.
revoke select, insert, update, delete on public.map_versions from anon;

-- The unique constraint indexes (location_id, name), which answers
-- "does this name exist here". Listing a shelf is `where location_id =
-- $1 order by saved_at desc` - the same index finds the rows and the
-- sort is over a handful of them.
