/* ===================================================================
   274. ICE FALLS INTO ICE
   ===================================================================

   Dave: *"Ice can be built with labor, again its up to if the material
   is present and if you are in a frozen landscape and the temp is
   right and you have tools you can build an ice wall or ice
   structures. Ice walls fall and make Ice rubble. We can expand this
   with temperature later but for now it should follow existing
   methodology."*

   THE FINDING THIS CLOSES was carried from 268 and restated every
   cycle since: `ice_wall` was one of the five walls, so a DM could
   paint one and a party could shatter one, but nothing in the world
   left `ICE` debris - so no heap could raise it and 700 lb of wall
   vanished when it came down. The asymmetry was never ruled; it fell
   out of a `None` in the spoil table.

   AND THE FIX NEEDS NO NEW MACHINERY, which is what "follow existing
   methodology" means here. `rubble_ice` is `ICE` at `Bulk::Quarter`,
   exactly as `rubble` is `STONE` at `Bulk::Quarter`, so:

     - 265's fork gives the wall its heap,
     - 268's `mine == theirs` raises the wall back out of it,
     - 270 asks for a shovel, because every quarter-bulk heap scoops,
     - 271 pours and spills it over the ring,
     - 273 weighs and hauls it,

   none of which is touched by this file. One variant, one key, one
   item row and one restated CHECK.

   WHAT IS DEFERRED, AND NAMED RATHER THAN FAKED. Dave's own
   conditions - a frozen landscape, the right temperature - have
   nowhere to live: there is no climate in the model and no clock on a
   terrain word. So an ice wall built in a desert stands there
   indefinitely, and that is a known shortfall rather than a claim.
   Temperature is a later cycle; this one makes ice behave like every
   other material in the meantime.

   =================================================================== */

/* ---------- THE COVER LIST, RESTATED WHOLE ----------

   115. AN APPLIED MIGRATION IS A RECORD, NOT A DRAFT. 265 is the file
   the database currently reads its vocabulary out of, and it cannot be
   edited - so all three constraints are restated here and this file
   becomes that copy. Only the cover list actually changes; the two
   ground lists are reproduced unchanged, because a file that restated
   one of three would leave the guard in terrain.rs reading two files
   to find one vocabulary.

   AND THE GROUND LISTS DO NOT GAIN `rubble_ice`. Debris lands ON a
   square whichever layer was broken - you stand on the floor with
   shards heaped on it, not on the shards as your floor. The same
   ruling 265 made for the other four.                               */

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

  -- THE FIFTH HEAP GOES HERE and nowhere else.
  add constraint a_cover_stands_on_something check (cover is null or cover in (
    'grass','snow','brush','trees','vines',
    'rubble','rubble_timber','rubble_wood','rubble_earth','rubble_ice',
    'door_open','door_closed','door_broken','iron_grate',
    'stone_wall','timber_wall','rock_wall','wooden_wall','ice_wall'));

/* ---------- THE FIVE HEAPS IN THE CATALOGUE ----------

   RESTATED WHOLE FOR THE SAME REASON as the constraints: 273's file
   was the catalogue's copy of the debris vocabulary and this one takes
   that job, so `weighed_words` in terrain.rs has one file to read. The
   four older rows already exist and `on conflict do nothing` leaves
   them alone; the assertion below is what makes that safe to say,
   because 273 learned the hard way that a swallowed row looks exactly
   like a written one.

   POUNDS PER HIT POINT, from terrain::Material::weight - a five-foot
   span's pounds over a five-foot span's hit points:

     stone     2000 / 60 = 33.3333    a heap of 15 is 500 lb
     ice        700 / 30 = 23.3333    a heap of  7 is 163 lb
     timber     800 / 40 = 20         a heap of 10 is 200 lb
     wood       400 / 30 = 13.3333    a heap of  7 is  93 lb
     loose     1200 / 20 = 60         a heap of  5 is 300 lb

   ICE ROUNDS LIKE WOOD, and for its reason: 30 hit points a span is 7
   at a quarter rather than 7.5, so four heaps of ice weigh 653 and not
   the 700 the wall did. That is 265's integer arithmetic showing
   through, losing in the safe direction - the same sentence the boards
   already carry.

   THE COLUMN ORDER IS PART OF THE CONTRACT. `weighed_words` reads the
   fourth value of each row, so this insert names its columns in that
   order and each row's first four fields stay on one line.          */

insert into public.items (key, name, kind, weight, description) values
  ('rubble', 'Broken Stone', 'loot', 33.3333,
   'Masonry and bedrock, broken up. Quantity is HIT POINTS of debris, not a count of lumps - fifteen is one square''s worth and weighs 500 lb. Four of them raise a wall.'),
  ('rubble_timber', 'Splintered Timber', 'loot', 20,
   'Beams and standing timber, broken up. Ten is one square''s worth, 200 lb.'),
  ('rubble_wood', 'Broken Boards', 'loot', 13.3333,
   'Planks and boards, broken up. Seven is one square''s worth - a quarter of thirty rounded down, which is why a heap of boards is 93 lb rather than 100.'),
  ('rubble_earth', 'Spoil', 'loot', 60,
   'Dug earth, sand, scree. Five is one square''s worth and weighs 300 lb: the lightest heap to break and the heaviest to carry. Raises nothing - no wall in the vocabulary is made of loose earth - so backfill is the only thing to do with it.'),
  ('rubble_ice', 'Shattered Ice', 'loot', 23.3333,
   'Blocks and shards off an ice wall or a broken lid. Seven is one square''s worth, 163 lb. Raises an ice wall again, given a shovel and the hours - 274 defers whether the cold will keep it standing.')
on conflict do nothing;

/* ---------- AND PROVE IT, BEFORE THIS COMMITS ----------

   273'S LESSON, KEPT. `on conflict do nothing` is kept so the
   migration can be replayed, which means it can still swallow a row -
   so the swallowing is what gets checked. Five debris words, one
   global row each, right kind, carrying a weight. The NUMBERS are
   checked against the Rust table by terrain.rs's
   the_weights_and_the_migration_cannot_drift; what cannot be checked
   from there is whether the row arrived at all.                     */

do $check$
declare
  n int;
  bad text;
begin
  select count(*) into n
    from public.items
   where game_id is null
     and key in ('rubble', 'rubble_timber', 'rubble_wood', 'rubble_earth',
                 'rubble_ice');
  if n <> 5 then
    raise exception 'expected five debris rows, found %', n;
  end if;

  select string_agg(key || ' (' || kind || ', weight ' ||
                    coalesce(weight::text, 'NULL') || ')', ', ')
    into bad
    from public.items
   where game_id is null
     and key in ('rubble', 'rubble_timber', 'rubble_wood', 'rubble_earth',
                 'rubble_ice')
     and (kind <> 'loot' or weight is null or weight <= 0);
  if bad is not null then
    raise exception 'a heap that cannot be carried or weighed: %', bad;
  end if;
end
$check$;

/* ---------- WHAT IS DELIBERATELY NOT HERE ----------

   NO TEMPERATURE, NO CLIMATE, NO MELTING. Dave's conditions for
   building in ice are real and have nowhere to be stored. Inventing a
   `temperature` column to satisfy a sentence in a commit message would
   be worse than the gap, because the column would then be read by
   nothing and trusted by somebody.

   NO ICE STRUCTURES BEYOND THE WALL. Dave said "ice wall or ice
   structures". The wall is a word the vocabulary already has; a
   structure is a shape over many squares, which nothing in this
   project can express for stone either. Not an ice problem.

   NO CHANGE TO ICE'S NUMBERS. AC 13, 30 hit points a span, threshold
   5, quoted from the DMG substance table since 264 and left exactly as
   they were. This file gives ice a heap, not a buff.                */
