-- 230. A CREATURE STANDS ON A SQUARE.
--
-- 226 gave a location a map and 229 let somebody draw one, and nothing
-- could be ON it. `characters` has had a `location_id` since 035 - it
-- knows which ROOM you are in - and no idea where in the room.
--
-- That is why `stride.rs` has no caller. It can say what moving from
-- one square to the next costs, and nothing knows which square anybody
-- is standing on.
--
-- ---------------------------------------------------------------------
-- `location_id` IS ALREADY THE MAP
-- ---------------------------------------------------------------------
--
-- No new identifier, because everything already points at the same
-- row:
--
--     characters.location_id  ─┐
--     encounters.location_id  ─┼─→  locations.id
--     tiles.location_id       ─┘
--
-- A character in Test Inn 1 is on Test Inn 1's map. The map's own
-- properties - `recipe`, `px_per_grid` from 226 - are columns on that
-- same row. These three say WHERE ON IT, and nothing more.
--
-- A LOCATION MAY HAVE NO MAP, which is the ordinary case: a tavern
-- nobody has drawn. The character is still somewhere; x, y and z are
-- simply null.
--
-- ---------------------------------------------------------------------
-- ONE TABLE, AND NOT THE THREE MAP.md NAMES
-- ---------------------------------------------------------------------
--
-- MAP.md puts x, y, z on `objects`, `characters` AND `encounter_actors`.
-- Only the middle one is here.
--
-- **`characters.x` AND `encounter_actors.x` WOULD BOTH ANSWER "WHERE IS
-- BRYN".** During a fight they would disagree, and afterwards somebody
-- has to copy one into the other. That is the two-places fault with a
-- round counter attached.
--
-- The want behind it is real - MAP.md calls it "live combat position,
-- already checkpoints by round" - but that is PER-ROUND HISTORY, not a
-- second present. History arrives when something needs to rewind a
-- round, and it will know then what shape it wants.
--
-- OBJECTS ARE DEFERRED rather than rejected. A chest drawn on the map
-- is worth having; it is not what is blocking movement, and 031's
-- holder chain means only an object held BY A LOCATION has a position
-- at all - a dagger in a backpack does not. That deserves its own
-- thinking rather than riding along here.
--
-- ---------------------------------------------------------------------
-- NO UNIQUE CONSTRAINT ON (location, x, y, z), DELIBERATELY
-- ---------------------------------------------------------------------
--
-- It is the obvious index to add and it would be wrong. **5e puts two
-- creatures in one square routinely**: squeezing through a gap, a
-- swarm, a mount and its rider, somebody prone under a brawl.
--
-- Occupancy is a RULE WITH EXCEPTIONS. A unique index is a schema fact
-- with none, and it would forbid, forever and silently, things the
-- rulebook allows. Whatever decides who may share a square belongs
-- where the exceptions can be written down.
--
-- ---------------------------------------------------------------------
-- AND A TRIGGER, WHICH IS THE ONE PIECE OF REAL GROUNDWORK
-- ---------------------------------------------------------------------
--
-- Walk from the inn to the outhouse and your coordinates are suddenly
-- about a map you are not on. A creature standing at (3, 4) in the
-- wrong room is not an error anything would raise - it is a quietly
-- wrong answer to "who is here", which is this project's named defect.
--
-- A CHECK CANNOT CATCH IT because it is about a TRANSITION rather than
-- a row. So the position is cleared when the map changes, in the
-- database, where nothing can forget to do it. Same shape as 218's
-- `a_fixture_stays_put`.
--
-- IT CLEARS RATHER THAN REFUSES. Moving between rooms is ordinary and
-- must not need the caller to null three columns first; arriving
-- somewhere without a position is exactly right, because nobody has
-- said where on the new map you are.

alter table public.characters
  add column if not exists x int      null,
  add column if not exists y int      null,
  add column if not exists z smallint null;

alter table public.characters
  add constraint a_position_is_three_facts check (
    (x is null     and y is null     and z is null)
    or
    (x is not null and y is not null and z is not null)
  );

alter table public.characters
  add constraint a_position_needs_a_map check (
    x is null or location_id is not null
  );

comment on column public.characters.x is
  'Which square, on the map of `location_id`. Null when nobody has '
  'placed them - which is most of the time, because most places have '
  'no map. Same type as tiles.x so a creature and a square compare '
  'without a cast.';

comment on column public.characters.y is
  'Which square, on the map of `location_id`. See x.';

comment on column public.characters.z is
  'Which height, in the same five-foot squares as tiles.z. A creature '
  'on a bridge deck is z = 2 over a floor at 0.';

-- ---------------------------------------------------------------------

create or replace function public.a_position_belongs_to_one_map()
returns trigger
language plpgsql
as $$
begin
  -- Only when the map itself changed. An ordinary step writes x, y, z
  -- and must not have them wiped by its own update.
  if new.location_id is distinct from old.location_id then
    new.x := null;
    new.y := null;
    new.z := null;
  end if;
  return new;
end;
$$;

comment on function public.a_position_belongs_to_one_map() is
  'Clears x, y, z when a character changes location. Coordinates are '
  'about one map; carrying them to another is a creature standing at '
  '(3,4) in the wrong room, which nothing would raise. See 230.';

drop trigger if exists characters_a_position_belongs_to_one_map on public.characters;
create trigger characters_a_position_belongs_to_one_map
  before update on public.characters
  for each row
  execute function public.a_position_belongs_to_one_map();
