-- 248. A MONSTER HAS A SPEED TOO.
--
-- Dave: *"don't characters have a movement - this is what we should be
-- reading"*.
--
-- A character's speed is derivable and always has been:
-- `characters.species_key` -> `species.speed`, which has said 25 / 30 /
-- 40 since 057. **A monster's is not derivable from anything.** `npcs`
-- carries ac, hp_max, size, the six abilities, prof_bonus, level, cr
-- and creature_type, and all 134 of them were imported without a
-- speed, because at the time nothing could have read one.
--
-- `npcs.species` is free text - "wolf", "owlbear" - and does not join
-- `species`, which holds the playable peoples. There is no row anywhere
-- that knows a dire wolf moves 50 feet.
--
-- ---------------------------------------------------------------------
-- NULLABLE, AND NULL MEANS NOBODY HAS SAID
-- ---------------------------------------------------------------------
--
-- **NOT DEFAULT 30.** Every existing row would take it, and 134
-- creatures would silently claim a speed nobody checked - a zombie
-- (20), a giant eagle (10 walking, 80 flying) and a wolf (40) all
-- reading as an average that is wrong for each of them.
--
-- That is this project's named defect exactly: an ordinary-looking
-- answer with nothing behind it. `speed.rs` takes an `Option` and
-- hands back `None` for null, so the screen can say "no speed
-- recorded" - which is true, and which a 30 would have hidden.
--
-- It is the same rule 226 set for a missing tile row: **absent and
-- empty are different answers.**
--
-- ---------------------------------------------------------------------
-- FEET, AND ONE NUMBER
-- ---------------------------------------------------------------------
--
-- Feet because that is what the books and `species.speed` say, and a
-- second unit here would be the two-places fault with a conversion
-- attached. `grid::squares` turns it into squares at the one place
-- 225 put that arithmetic.
--
-- ONE NUMBER, NOT FOUR. An SRD statblock may carry walk, swim, climb,
-- fly and burrow. This column is the WALKING speed, because that is
-- what `stride` spends today: `Mode::Swim` and `Mode::Fly` already
-- exist in `terrain.rs` as things a creature CAN DO, and nothing yet
-- records that a creature can do them.
--
-- The other speeds arrive the day a mode does, and they arrive as
-- their own columns or a jsonb - that is a decision for then, with the
-- modes in front of it. Writing `speeds jsonb` now would be guessing
-- at a shape nothing reads, which is what 226 refused for `recipe`.
--
-- ---------------------------------------------------------------------
-- A ZERO IS A REAL SPEED
-- ---------------------------------------------------------------------
--
-- A shrieker does not move. `check (speed >= 0)` admits 0 and refuses
-- a negative, and 0 is meaningfully different from null: one says "it
-- stays where it is", the other says "nobody has filled this in".

alter table public.npcs
  add column if not exists speed int null;

alter table public.npcs
  add constraint a_speed_is_not_negative check (speed is null or speed >= 0);

comment on column public.npcs.speed is
  'Walking speed in FEET, or null for "nobody has said" - deliberately '
  'not defaulted to 30, because 134 creatures would have claimed a '
  'number nobody checked. 0 is a real answer and means it does not '
  'move. The walking speed only: swim, climb, fly and burrow arrive '
  'with the modes that spend them. Read by `speed.rs`, which is the '
  'one place a creature''s speed is resolved. See 248.';
