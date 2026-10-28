-- 263. A WALL IS NOT A FLOOR.
--
-- 226 wrote the three layer constraints from `Terrain::may`, which read
-- `ground || self.is_wall()` for BOTH ground layers - one arm covering
-- two cases - so the five walls were legal as a surface and as a sub.
-- Sixty hit points of floor, by accident.
--
-- 262 agreed to narrow it. 263 pulled it forward, because applying a
-- `Destruction` for the first time showed what it costs: a wall
-- destroyed AS A SURFACE becomes rubble, rubble is cover-only, and the
-- row could not be read back afterwards.
--
-- A wall stands ON a floor. That is why `Leaves(Rubble)` works at all -
-- the debris lands on the surface layer, which is still there precisely
-- because the wall was never in it.
--
-- NOTHING TO MIGRATE. Checked before writing this: no row in `tiles`
-- has ever had a wall in `surface` or in `sub`. The constraint is being
-- narrowed to what the data already is.
--
-- 115. THIS DOES NOT EDIT 226. An applied migration is a record of what
-- happened, not a draft - so the old constraint is dropped by name and
-- a new one with the same name takes its place.

alter table public.tiles
  drop constraint if exists a_surface_is_ground,
  drop constraint if exists a_sub_is_ground_too,
  drop constraint if exists a_cover_stands_on_something;

alter table public.tiles

  -- THE EIGHTEEN GROUND WORDS, and no walls.
  add constraint a_surface_is_ground check (surface in (
    'rock','dirt','mud','sand','scree','road','stone_floor','wood_floor',
    'marsh','ice','shallow_water','deep_water','lava','pit','ledge',
    'doorway','ramp','stairs')),

  -- The same list: `may(Sub)` and `may(Surface)` are one arm, and 261
  -- added a `sub` flag to the picker so that staying the same list is a
  -- fact the tests check rather than one the UI assumes.
  add constraint a_sub_is_ground_too check (sub is null or sub in (
    'rock','dirt','mud','sand','scree','road','stone_floor','wood_floor',
    'marsh','ice','shallow_water','deep_water','lava','pit','ledge',
    'doorway','ramp','stairs')),

  -- UNCHANGED, and restated anyway. `the_migration_and_the_enum_cannot_drift`
  -- reads ONE file and checks all three lists against `terrain::ALL`, so
  -- the newest migration has to be the whole picture or the guard only
  -- covers part of it.
  add constraint a_cover_stands_on_something check (cover is null or cover in (
    'grass','snow','brush','trees','vines','rubble',
    'door_open','door_closed','door_broken','iron_grate',
    'stone_wall','timber_wall','rock_wall','wooden_wall','ice_wall'));
