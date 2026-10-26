-- 253. HOW FAR A MOVE WENT.
--
-- Dave: *"do the budget cycle"*.
--
-- `spent.rs` has counted what a creature did in a round since 054, and
-- everything it counts is an OCCURRENCE: three actions is three rows,
-- two swings is two.
--
-- **MOVEMENT IS A QUANTITY AND THAT IS WHY THIS COLUMN EXISTS.** Three
-- moves might be one square, then two across mud, then one - four
-- squares in three rows. Counting the rows would say three and be
-- wrong about the only number anybody cares about, and no column on
-- `actions` carried an amount of anything.
--
-- ---------------------------------------------------------------------
-- WHY NOT A RUNNING TOTAL ON THE CREATURE
-- ---------------------------------------------------------------------
--
-- Because somebody has to reset it, and the somebody is what drifts. A
-- `movement_spent` column on `characters` would need zeroing when the
-- turn passes, when the round advances, when initiative is rebuilt,
-- when a creature is added mid-fight, and when a DM rewinds - and the
-- day one of those paths forgets, a creature is permanently out of
-- movement for a reason nobody can see.
--
-- `this_round` already derives every other tally by summing the rows of
-- the round it was asked about. This is the same derivation with `+=
-- squares` instead of `+= 1`, and it inherits the property that makes
-- the others trustworthy: **the log and the tally cannot disagree,
-- because the tally IS the log.**
--
-- ---------------------------------------------------------------------
-- NULLABLE, AND NULL IS NOT ZERO
-- ---------------------------------------------------------------------
--
-- Null means "this row is not a move" and that is every row written
-- before today. Zero would mean "a move that covered no ground", which
-- is a different and real thing - a creature that tried to walk into a
-- wall and was refused does not get a row at all, but one that walks
-- onto the square it occupies would.
--
-- The same distinction 248 drew for `npcs.speed` and 226 drew for a
-- missing tile row: absent and empty are different answers.
--
-- ---------------------------------------------------------------------
-- SMALLINT, AND A CHECK
-- ---------------------------------------------------------------------
--
-- A move is bounded by the map, and the map is bounded by what a
-- person can draw. 32,767 squares is 150 miles at five feet a square.
--
-- Negative is refused because there is no such move. Zero is allowed
-- for the reason above.

alter table public.actions
  add column if not exists squares smallint null;

alter table public.actions
  add constraint a_move_covers_no_negative_ground
    check (squares is null or squares >= 0);

comment on column public.actions.squares is
  'How far a move went, in five-foot squares, or null for every row '
  'that is not a move - which is every row written before 253. NOT a '
  'running total: `spent::this_round` sums these for the round it is '
  'asked about, so the log and the tally cannot disagree. Written by '
  '`walk_to`, read by `commands/log.rs`. See 253.';
