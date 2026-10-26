-- 254. WHICH PIECE OF A WALL A SQUARE IS.
--
-- Dave, having drawn a corner: *"it needs to be its own cover type as
-- it is wall section but a corner that also needs to be faceable"*,
-- and then *"do the shape column, Also add one for stone_wall_T"*.
--
-- ---------------------------------------------------------------------
-- WHY THIS IS A COLUMN AND NOT FIVE MORE TERRAIN WORDS
-- ---------------------------------------------------------------------
--
-- A corner and a straight run of the same wall are **mechanically
-- identical**: the same `Blocked` footing, the same AC 17 and 60 hit
-- points, the same `Leaves(Rubble)`, the same blocked sight.
-- `terrain.rs` is a vocabulary where every word earns its place by
-- meaning something different TO A RULE, and a corner means something
-- different only to the eye.
--
-- The cost, counted rather than guessed: one new wall word touches
-- SEVENTEEN places in `terrain.rs` and THREE CHECK constraints in 226
-- - and 226 is applied, so rewriting them needs its own migration.
-- `stone_wall_corner` implies the same for timber, rock, wooden and
-- ice: five words, eighty-five edits. Ends and T-junctions would be
-- fifteen more words.
--
-- **This column says it once, for every material and every shape
-- there will ever be.**
--
-- ---------------------------------------------------------------------
-- THE FIRST COLUMN ON `tiles` THAT NO RULE READS
-- ---------------------------------------------------------------------
--
-- Worth saying plainly, because it is a departure. The three layers,
-- the two hit-point columns, `leads_to` and `facing` all mean
-- something to `stride` or `ground`. **This one means something only
-- to `map.js`.**
--
-- It is here rather than in the renderer because it is a fact about a
-- SQUARE that a person chose, and the renderer has nowhere to keep
-- one. The alternative - deriving the shape from which neighbours are
-- also walls, the way a tile engine does - was considered and refused:
-- 252 had just made walls manually faceable at Dave's request, and an
-- auto-derived rotation would fight a chosen one.
--
-- **DO NOT LATER PROMOTE IT TO A RULE.** A corner that blocked
-- differently from a straight wall would be a rule nobody asked for,
-- arrived at because the column was sitting next to `cover`.
--
-- 226 set the precedent in the other direction: `px_per_grid` is a
-- purely-rendering number kept on `locations` on purpose.
--
-- ---------------------------------------------------------------------
-- NO CHECK ON THE VOCABULARY
-- ---------------------------------------------------------------------
--
-- 238's split, applied again: *"the database constrains the RANGE
-- because that is a fact about the type. It leaves the vocabulary
-- alone."*
--
-- A shape has no range. `shape.rs` holds the two words there are -
-- `corner` and `t` - and `paint_tiles` refuses anything else with a
-- sentence, so a typo is caught where it can be explained rather than
-- arriving as a constraint name. A CHECK here would be the vocabulary
-- in a second place and a migration every time somebody draws a new
-- piece.
--
-- ---------------------------------------------------------------------
-- ONE PER SQUARE, LIKE `facing`
-- ---------------------------------------------------------------------
--
-- Not one per layer. 238 made the turn a fact about the square and
-- this follows it, for the same reason: a square is drawn as one
-- picture, and the only shaped thing on it is the wall.
--
-- A square with no shape is a plain tile, which is every square
-- painted before today.

alter table public.tiles
  add column if not exists shape text null;

comment on column public.tiles.shape is
  'Which PIECE of a wall this square is - `corner`, `t`, or null for a '
  'plain straight run, which is every square painted before 254. THE '
  'ONLY COLUMN ON THIS TABLE THAT NO RULE READS: a corner blocks, '
  'breaks and costs exactly what a straight wall does, and this '
  'chooses the PNG - `stone_wall` plus `corner` is '
  '`stone_wall_corner.png`. Deliberately not a CHECK: the vocabulary '
  'lives in `shape.rs` and `paint_tiles` refuses an unknown word with '
  'a sentence. Do not promote it to a rule. See 254.';
