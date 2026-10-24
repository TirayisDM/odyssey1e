-- 238. WHICH WAY A SQUARE POINTS.
--
-- Found by drawing. The first eighteen ground tiles arrived on
-- 2026-10-24 and four of them are not symmetrical: a doorway sits in
-- one wall rather than all four, a flight of stairs climbs in a
-- direction, a ramp inclines one way, a ledge has an edge you fall
-- off.
--
-- Dave drew them with **the top of the image as the direction of
-- travel** and asked how they would be turned. The answer is a column,
-- because a turn is a fact about ONE SQUARE and not about the kind of
-- ground.
--
-- ---------------------------------------------------------------------
-- A QUARTER TURN, NOT AN ANGLE
-- ---------------------------------------------------------------------
--
-- `0..3`, clockwise from north, where north is up on the screen.
--
-- Degrees would let somebody store 37, and a square grid has four
-- sides. A smallint with a CHECK says what the thing is; a free number
-- says only that somebody has not thought about it yet.
--
-- ---------------------------------------------------------------------
-- NOT NULL DEFAULT 0, AND NO CHECK ABOUT WHICH WORDS MAY TURN
-- ---------------------------------------------------------------------
--
-- Every square gets a facing whether it means anything or not, because
-- a nullable one would make "this does not face" and "nobody has said"
-- look identical - and rotating a symmetrical tile changes nothing, so
-- 0 is a harmless answer for rock.
--
-- **WHICH WORDS MAY ACTUALLY TURN IS A RULE AND LIVES IN RUST.**
-- `Terrain::faces()` names the eight that do, and `paint_tiles`
-- refuses a turn on anything else with a sentence. A CHECK here could
-- express it - the terrain and the facing are on the same row - and it
-- would be the terrain vocabulary in a second place, which 226 already
-- decided against for exactly this column's neighbours.
--
-- The database constrains the RANGE because that is a fact about the
-- type. It leaves the vocabulary alone.
--
-- ---------------------------------------------------------------------
-- IT IS NOT ONLY A PICTURE
-- ---------------------------------------------------------------------
--
-- `stride.rs` has said since 230 that a ramp makes a change of height
-- an ordinary step rather than a climb. **That was true in every
-- direction, which is wrong.**
--
-- A ramp built north-south is climbed by walking north or south.
-- Walking EAST across it is walking along a slope, not up one, and
-- should cost the climb it always did. So the facing is read by
-- `Facing::along`, and a ramp only eases a step that runs down its
-- axis.
--
-- Both ways along it count - you walk down a ramp as well as up it -
-- and a diagonal with a component along the axis counts too, because
-- making a creature square up first is a rule nobody could see the
-- reason for.

alter table public.tiles
  add column if not exists facing smallint not null default 0;

alter table public.tiles
  add constraint a_facing_is_a_quarter_turn check (facing between 0 and 3);

comment on column public.tiles.facing is
  'Quarter turns clockwise from north, 0..3, where north is up on the '
  'screen and the art is drawn with the top of the image as the '
  'direction of travel. Meaningless for a symmetrical tile and 0 '
  'there; which words may turn is `Terrain::faces()` in Rust, not a '
  'constraint here. NOT ONLY A PICTURE: a ramp only makes a climb an '
  'ordinary step along its own axis - see 238.';
