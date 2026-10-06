-- 132. THE GATE RETUNE WAS NOT SCOPED.
--
-- 129 lowered the technique gates from 1/3/5 to 1/2/4 so that a level-2
-- creature would reach more than one move. The reasoning was sound and
-- the statement was not:
--
--   update techniques set min_level = 2 where game_id is null and min_level = 3;
--   update techniques set min_level = 4 where game_id is null and min_level = 5;
--
-- `where game_id is null` IS NOT "where this is a natural weapon". It is
-- "where this belongs to the global catalogue", and 043's 159 PLAYER
-- weapon techniques are global too. So 129 did not retune creatures - it
-- retuned the whole game, and every character in it got Split the Collar
-- at level 4 instead of 5 and Beard the Shield at 2 instead of 3.
--
-- NOBODY ASKED FOR THAT. It is a change to player progression that
-- arrived as a side effect of a bestiary migration, which is the worst
-- way for a balance change to happen: invisible, undiscussed, and
-- attributed to the wrong author.
--
-- ---------------------------------------------------------------------
-- WHY THE REVERSAL IS EXACT RATHER THAN A GUESS
-- ---------------------------------------------------------------------
--
-- Because nothing was ever authored at gate 2 or 4. Every technique row
-- in `supabase/migrations/` - all 183 of them, across 043, 126 and 129 -
-- was written at 1, 3 or 5. So a weapon technique sitting at 2 today can
-- only have been a 3, and one at 4 can only have been a 5. There is no
-- ambiguity to resolve and no row that could be moved by mistake.
--
-- THE TAG IS THE SCOPE THIS TIME. 126 marked every natural weapon
-- `content_tags = array['natural']` and said in its own comment that
-- nothing read the tag yet and it was there as a hook. This is the hook
-- being used: the join below asks what KIND of weapon the technique
-- belongs to rather than who owns the row, which is the question 129
-- should have asked.
--
-- NATURAL WEAPONS KEEP 1/2/4, which is what 129 was actually for: a
-- creature does not level up, and gating a wolf's repertoire on a
-- progression it will never walk left it with one move.

update public.techniques t
   set min_level = case t.min_level when 2 then 3 when 4 then 5 end
  from public.items i
 where i.key = t.item_key
   and i.game_id is null
   and t.game_id is null
   and t.min_level in (2, 4)
   and not ('natural' = any(i.content_tags));
