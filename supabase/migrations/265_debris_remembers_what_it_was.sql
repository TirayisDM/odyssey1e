-- 265. DEBRIS REMEMBERS WHAT IT WAS.
--
-- 262 ruled that rubble forks per material, because
-- `Destruction::spoil` returns a WORD and `Terrain` is a closed
-- vocabulary - there is nowhere for "rubble that remembers it used to
-- be stone" to live. The alternative was writing the amount into
-- `cover_hp` at the moment of collapse, which would have made it the
-- first stored number whose MAXIMUM nobody could re-derive on read.
--
-- `rubble` KEEPS ITS NAME AND MEANS STONE. The tidier spelling would
-- have been `rubble_stone`, and it would have renamed every row that
-- already says `rubble`, thrown away art that is already drawn, and
-- bought a prettier word with a migration over live data - for a word
-- whose picture is grey chunks of masonry and which only walls and
-- stone floors have ever produced.
--
-- Three new ones, one per material that leaves anything behind:
--
--   rubble_timber   splintered beams, from timber walls and felled trees
--   rubble_wood     broken boards
--   rubble_earth    dug spoil - loose earth, sand, a drained marsh bed
--
-- Ice and greenery leave nothing: a frozen wall shatters and brambles
-- are cut away, so neither gives you anything to build with.
--
-- 115. THIS DOES NOT EDIT 226 OR 263. The cover constraint is dropped
-- by name and replaced; the two ground constraints are restated
-- unchanged, because `the_migration_and_the_enum_cannot_drift` reads
-- ONE file and diffs all three lists against `terrain::ALL` - a partial
-- file would be a partial guard.

alter table public.tiles
  drop constraint if exists a_surface_is_ground,
  drop constraint if exists a_sub_is_ground_too,
  drop constraint if exists a_cover_stands_on_something;

alter table public.tiles

  -- Unchanged since 263: the eighteen ground words, and no walls.
  add constraint a_surface_is_ground check (surface in (
    'rock','dirt','mud','sand','scree','road','stone_floor','wood_floor',
    'marsh','ice','shallow_water','deep_water','lava','pit','ledge',
    'doorway','ramp','stairs')),

  add constraint a_sub_is_ground_too check (sub is null or sub in (
    'rock','dirt','mud','sand','scree','road','stone_floor','wood_floor',
    'marsh','ice','shallow_water','deep_water','lava','pit','ledge',
    'doorway','ramp','stairs')),

  -- THE THREE NEW WORDS GO HERE and nowhere else. Debris lands on top
  -- of a square whichever layer was broken, so it is cover and only
  -- cover - you do not stand on a heap of spoil as your floor, you
  -- stand on the floor with spoil heaped on it.
  add constraint a_cover_stands_on_something check (cover is null or cover in (
    'grass','snow','brush','trees','vines',
    'rubble','rubble_timber','rubble_wood','rubble_earth',
    'door_open','door_closed','door_broken','iron_grate',
    'stone_wall','timber_wall','rock_wall','wooden_wall','ice_wall'));
