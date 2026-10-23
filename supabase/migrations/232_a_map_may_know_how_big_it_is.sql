-- 232. A MAP MAY KNOW HOW BIG IT IS.
--
-- 226 gave a location tiles and no extent. `tiles.x` and `tiles.y` are
-- plain `int` - give or take two billion either way - so a map was
-- "wherever squares happen to be", and its size had to be worked out
-- from the data every time anybody drew it.
--
-- ---------------------------------------------------------------------
-- THE SHARP EDGE: A TYPO IS A HANG, NOT AN ERROR
-- ---------------------------------------------------------------------
--
-- Paint a square at x = 999999 in a forty-wide room and nothing refuses
-- it. The row is valid, every constraint passes, and the next thing to
-- render that location computes an extent of 0..999999 and tries to
-- draw a million columns.
--
-- **THAT IS THE PROJECT'S NAMED DEFECT WEARING A COORDINATE**: a
-- perfectly ordinary-looking answer that is wrong, with nothing between
-- it and the screen.
--
-- ---------------------------------------------------------------------
-- WIDTH AND HEIGHT, AND NOT A z RANGE
-- ---------------------------------------------------------------------
--
-- `z` STAYS UNBOUNDED on purpose. Height is genuinely open-ended - a
-- cellar at -2, a bridge deck at +2, a flying creature at +6 - and it
-- is far sparser than the floor. Bounding it would buy nothing and cost
-- a decision every time somebody digs.
--
-- The floor is different: a room is a rectangle somebody can describe,
-- and the number of squares in it is a fact about the place.
--
-- ---------------------------------------------------------------------
-- NULL MEANS UNBOUNDED, WHICH IS WHAT EVERY MAP IS TODAY
-- ---------------------------------------------------------------------
--
-- Both columns are nullable and nothing is backfilled. A sketch map
-- somebody is feeling out should not have to declare its size first,
-- and an unbounded map is still perfectly drawable from its own extent.
--
-- Declaring them is also the CURE for the typo above, which is why this
-- is worth having rather than a sanity cap invented out of nothing.
--
-- ---------------------------------------------------------------------
-- (0,0) IS THE CORNER
-- ---------------------------------------------------------------------
--
-- Where bounds exist the map is `0 .. width-1` by `0 .. height-1`, so a
-- negative coordinate is out of bounds rather than merely unusual. That
-- is the convention every map tool uses, and 228's arrival coordinates
-- are chosen by the destination anyway, so nothing needed negatives.
--
-- ---------------------------------------------------------------------
-- A TRIGGER, BECAUSE A CHECK CANNOT REACH ANOTHER TABLE
-- ---------------------------------------------------------------------
--
-- The bounds live on `locations` and the coordinates on `tiles`, and a
-- CHECK constraint cannot run a subquery. So it is a trigger, the same
-- shape as 218's `a_fixture_stays_put` and 230's position-clearer.
--
-- `paint_tiles` CHECKS FIRST so the refusal is a sentence naming the
-- square; this is the backstop for anything arriving by another route,
-- which is exactly the division 226 drew for the terrain vocabulary.
--
-- ---------------------------------------------------------------------
-- WHAT THIS DELIBERATELY DOES NOT DO
-- ---------------------------------------------------------------------
--
-- **SHRINKING A MAP DOES NOT DELETE WHAT FALLS OUTSIDE IT.** The
-- trigger is on `tiles`, so setting `width` smaller than existing
-- squares leaves them in place, out of bounds and still real.
--
-- That is deliberate rather than overlooked: a DM narrowing a room to
-- see how it feels should not silently destroy the east wing, and a
-- cascade here would be a delete nobody asked for. The renderer's job
-- is to span the bounds AND whatever exists, so nothing ever becomes
-- invisible - a tile outside the declared room should look wrong, not
-- vanish.

alter table public.locations
  add column if not exists width  int null,
  add column if not exists height int null;

alter table public.locations
  add constraint a_map_has_a_positive_size check (
    (width  is null or width  > 0)
    and
    (height is null or height > 0)
  );

comment on column public.locations.width is
  'How many squares across, or null for a map with no declared extent. '
  'Where it is set the map is 0..width-1 and a negative x is out of '
  'bounds. Declaring it is what stops a mistyped coordinate becoming a '
  'million-column render - see 232.';

comment on column public.locations.height is
  'How many squares down, or null. See width.';

-- ---------------------------------------------------------------------

create or replace function public.a_square_is_inside_its_map()
returns trigger
language plpgsql
as $$
declare
  w int;
  h int;
begin
  select l.width, l.height into w, h
    from public.locations l
   where l.id = new.location_id;

  -- No declared extent is not a licence to be anywhere; it is a map
  -- nobody has measured. Both halves are checked separately because a
  -- width without a height is a legal, if odd, thing to declare.
  if w is not null and (new.x < 0 or new.x >= w) then
    raise exception
      'x % is outside this map, which is % squares across', new.x, w;
  end if;

  if h is not null and (new.y < 0 or new.y >= h) then
    raise exception
      'y % is outside this map, which is % squares down', new.y, h;
  end if;

  return new;
end;
$$;

comment on function public.a_square_is_inside_its_map() is
  'Refuses a tile outside its location''s declared width or height. A '
  'CHECK cannot reach across tables, so this is the backstop; '
  'paint_tiles refuses first and names the square. See 232.';

drop trigger if exists tiles_a_square_is_inside_its_map on public.tiles;
create trigger tiles_a_square_is_inside_its_map
  before insert or update on public.tiles
  for each row
  execute function public.a_square_is_inside_its_map();
