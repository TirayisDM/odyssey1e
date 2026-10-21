-- 228. A LINK FROM ANY SQUARE, AND SOMEWHERE TO ARRIVE.
--
-- 226 ended with:
--
--     constraint a_link_needs_a_doorway
--       check (leads_to is null or surface = 'doorway')
--
-- which was wrong three ways, and the third is the one that corrects a
-- claim made in 226's own entry.
--
-- ---------------------------------------------------------------------
-- ONE: STAIRS
-- ---------------------------------------------------------------------
--
-- **`stairs` IS THE COMMONEST INTER-LOCATION LINK THERE IS** and the
-- constraint forbade it outright. A flight down to a cellar modelled as
-- a child location could not carry a link, which is the first thing
-- anybody would try to build.
--
-- ---------------------------------------------------------------------
-- TWO: A BREACHED WALL
-- ---------------------------------------------------------------------
--
-- When two areas are separate locations and the wall between them is
-- knocked through, the link has to appear AT THE MOMENT THE WALL COMES
-- DOWN - written onto a square whose cover is then `rubble`. The
-- constraint refused exactly that write. A passage that exists because
-- somebody made it is the most interesting kind, and the schema would
-- only have allowed the ones an architect drew.
--
-- ---------------------------------------------------------------------
-- THREE: UP AND DOWN IS NOT MERELY z, AND 226 SAID IT WAS
-- ---------------------------------------------------------------------
--
-- 226 argued that a ledge, a cliff or a hole in a floor needs no link
-- because the place below is `z - 1` in the same coordinate space.
-- **THAT IS WRONG AND THIS ENTRY CORRECTS IT.**
--
-- A location is one coordinate space with ONE ORIGIN AND ONE SCALE. Two
-- adjacent places are two maps, and the Valley's (0,0) has nothing
-- whatever to do with the Highlands'. Nobody draws one grid spanning a
-- mountain; they draw a map per area and join them. So falling off a
-- cliff lands you in a DIFFERENT LOCATION at coordinates that location
-- chose, and no amount of decrementing z will find them.
--
-- z remains what MAP.md made it - height WITHIN one coordinate space,
-- for a bridge deck over its own floor or a pit under it. The moment
-- the place below is its own map, it needs a link like anywhere else.
--
-- ---------------------------------------------------------------------
-- AND A LINK NEEDS SOMEWHERE TO ARRIVE
-- ---------------------------------------------------------------------
--
-- `leads_to` named a location and dropped you nowhere in it. A door
-- "to the undercroft" is not an answer to where you are standing when
-- you get there - and the destination has its own origin, so there is
-- nothing to infer it from.
--
-- **EXPLICIT COORDINATES RATHER THAN PAIRING.** Two linked squares
-- could find each other by convention, which is tidier while every
-- passage is symmetric and breaks the first time one is not. A chute
-- you slide down and cannot climb, a trapdoor that shuts above you, a
-- one-way portal: dungeons do this constantly, and a convention that
-- cannot express it is a convention somebody works around.
--
-- **ALL OR NOTHING.** A link is four facts and a partial one is a
-- passage to a location at no particular place, which nothing could act
-- on. The constraint below makes the incoherent state unstorable rather
-- than leaving it to whoever writes the first mover.
--
-- NOTHING WALKS THROUGH ONE YET. These columns are storable and
-- unread - the rule that moves a creature between locations does not
-- exist, and neither does the one for falling. They are here because
-- the alternative is a migration against live rows later, and because
-- `leads_to` already existed in a shape that could not work.

alter table public.tiles
  drop constraint a_link_needs_a_doorway;

alter table public.tiles
  add column if not exists to_x int      null,
  add column if not exists to_y int      null,
  add column if not exists to_z smallint null;

alter table public.tiles
  add constraint a_link_is_all_or_nothing check (
    (leads_to is null     and to_x is null     and to_y is null     and to_z is null)
    or
    (leads_to is not null and to_x is not null and to_y is not null and to_z is not null)
  );

comment on column public.tiles.leads_to is
  'The location this square opens onto, or null. ANY SQUARE MAY CARRY '
  'ONE - a doorway, a flight of stairs, a ledge you fall from, a wall '
  'somebody knocked through - because two adjacent places are two maps '
  'with their own origins, so leaving one is never just a change of z. '
  'See 228, which corrects 226 on exactly that.';

comment on column public.tiles.to_x is
  'Where in `leads_to` you arrive. Explicit rather than paired, so a '
  'one-way passage - a chute, a trapdoor that shuts above you - is '
  'expressible. Null only when `leads_to` is null.';

comment on column public.tiles.to_y is
  'Where in `leads_to` you arrive. See to_x.';

comment on column public.tiles.to_z is
  'Which height in `leads_to` you arrive at. See to_x.';
