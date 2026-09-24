-- =====================================================================
-- 044_two_misfiled_sizes.sql
-- odyssey1e — a trident is a spear, and a morningstar is a mace
-- =====================================================================
--
-- 036 said this would happen, in its own header: "5e HAS NO ITEM SIZE
-- TABLE. These sizes are a judgement, made from what a thing IS rather
-- than typed row by row, so the reasoning stays visible and a DM who
-- disagrees edits one row instead of guessing at a pattern."
--
-- Two of them were wrong, and 042 is what made it obvious. Twenty new
-- weapons arrived with sizes chosen one at a time, and standing beside
-- them the bucketed ones showed their seams.
--
-- THE TRIDENT WAS FILED BY ITS NAME, NOT BY WHAT IT IS. It went into
-- 036's "one-handed gear and most of the kit" list, which was a sweep
-- over sixty-eight keys, and came out Small. A spear - the same hafted
-- thrusting weapon, the same `thr` and `ver` properties, and a POUND
-- LIGHTER at 3 to the trident's 4 - is Large. Two weapons of one family
-- at opposite ends of the ladder is not a judgement, it is a miss.
--
-- The spear is the one that is right, so the trident joins it.
--
-- THE MORNINGSTAR HAD NOTHING TO COMPARE TO UNTIL 042. It is Small at
-- 4 lb; the flanged mace 042 added is Medium at 5. Those are the same
-- weapon with a different head, and the newer row is the considered
-- one - it was sized on its own rather than as part of a sweep.
--
-- THE PLAIN MACE STAYS SMALL, deliberately, and this is a decision
-- rather than an omission. A mace is the compact one: a short haft and
-- a solid head. A morningstar and a flanged mace are longer in the
-- handle and heavier in the business end, which is what the extra step
-- on the ladder is for. Three rows, two sizes, and the split is between
-- the compact club and the two that need room to swing.
--
-- NOTHING IS IN PLAY WITH EITHER KEY, checked before this was written.
-- No object rows, so no instance size_override is being masked and
-- nothing a player is carrying changes size underneath them. Had there
-- been, the override on the instance would have won and this migration
-- would have quietly missed them - worth remembering the next time a
-- catalogue size is corrected.
--
-- WHY A MIGRATION AND NOT AN EDIT IN 036. 036 is applied. Migrations
-- are append-only here and have been since 003 and 004 were written as
-- fixes for earlier mistakes rather than as edits to them: the history
-- is meant to show what was believed and when it stopped being true.
-- =====================================================================

update public.items set size = 'lg'
 where key = 'trident' and game_id is null;

update public.items set size = 'med'
 where key = 'morningstar' and game_id is null;
