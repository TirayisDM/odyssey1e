-- 146. CHECKED AGAINST THE SRD ITSELF, AT LAST.
--
-- 145 sweeping the bestiary against my own recollection said in its own
-- header: "it compares the database against MY RECOLLECTION of the SRD,
-- which is the same source that wrote the rows... it cannot catch what I
-- have remembered wrongly in the same way twice. A second reader would
-- be worth more than a third pass by me."
--
-- Dave found the second reader: the SRD is published online. 184 of our
-- creatures matched a page and were compared field by field - armour
-- class, hit points, all six abilities, and every resistance parsed back
-- into 116's vocabulary. FIFTEEN were wrong, and not in the ways the
-- last three migrations were looking.
--
-- ---------------------------------------------------------------------
-- IMMUNE, NOT RESISTANT - TEN OF THEM
-- ---------------------------------------------------------------------
--
-- The single biggest finding, and invisible to every pass so far because
-- every pass was checking WHETHER the physical line was there and not
-- what DEGREE it was:
--
--   all five lycanthropes   immune to b/p/s from nonmagical attacks
--   all four golems         immune
--   the Lich                immune
--
-- 143 built the qualifier and 144 spread it, both reading "resistance"
-- off a memory that had the condition right and the degree wrong. A
-- werewolf was taking half from an ordinary sword and should have been
-- taking NONE - which is a bigger error than the one 143 existed to fix,
-- sitting underneath it the whole time.
--
-- ---------------------------------------------------------------------
-- AND ONE THAT 144 GOT BACKWARDS
-- ---------------------------------------------------------------------
--
-- THE DRETCH HAS NO PHYSICAL RESISTANCE AT ALL. 144 gave it one on the
-- reasoning that "every demon in the SRD also resists bludgeoning,
-- piercing and slashing from nonmagical attacks" - which is true of the
-- vrock, the hezrou, the glabrezu and the nalfeshnee, and not true of
-- the dretch, which is the weakest of them and resists only the three
-- elements. A rule applied to a family, and the family had an exception.
--
-- Removed. This is exactly what a second reader is for, and worth
-- recording as the shape of the mistake rather than just the fix: the
-- error was not a bad memory of the dretch, it was confidence in a
-- pattern.
--
-- ---------------------------------------------------------------------
-- THE REST
-- ---------------------------------------------------------------------
--
--   Horned Devil      148 hit points -> 178
--   Werewolf          AC 12 -> 11
--   Wereboar          AC 12 -> 10
--   Giant Scorpion    DEX 13 -> 11
--   Wight             immune to poison, which was missing
--   Assassin          resistant to poison, which was missing
--
-- 169 of the 184 matched EXACTLY on every field. The numbers in this
-- bestiary were written from memory and were, on the whole, right - the
-- failures cluster in degrees and in families, not in digits.
--
-- STILL UNVERIFIED: our own creations - the Goblin Boss, Guard Captain,
-- Orc War Chief, Harpy Matriarch, Dire Boar, Kobold Dragonshield - have
-- no SRD page by design and were skipped, as Dave asked. The Archer,
-- Banshee, Pixie, Acolyte, Druid, Carrion Crawler, Hook Horror, Helmed
-- Horror, Intellect Devourer, Scarecrow, Twig Blight, Yuan-ti Pureblood
-- and Gas Spore have no page on that wiki either; several of those are
-- genuinely outside the SRD and are now effectively ours too.

-- --- the ten that are immune rather than resistant ---------------------

update public.npcs
   set grants = (
     select jsonb_agg(
       case when g->>'target' like 'resist.%.nonmagical'
            then jsonb_set(g, '{target}',
                   to_jsonb(replace(g->>'target', 'resist.', 'immune.')))
            else g
       end
       order by ordinality)
     from jsonb_array_elements(public.npcs.grants) with ordinality as t(g, ordinality)
   )
 where game_id is null
   and key in ('werewolf','wereboar','wererat','weretiger','werebear',
               'clay_golem','flesh_golem','iron_golem','stone_golem',
               'lich');

-- --- the dretch, which never had one ----------------------------------

update public.npcs
   set grants = coalesce((
     select jsonb_agg(g order by ordinality)
       from jsonb_array_elements(public.npcs.grants) with ordinality as t(g, ordinality)
      where g->>'target' not like '%.nonmagical'
   ), '[]'::jsonb)
 where game_id is null
   and key = 'dretch';

-- --- two that were missing a line -------------------------------------

update public.npcs n
   set grants = n.grants || jsonb_build_object('source', n.name, 'target', v.target)
  from (values ('wight','immune.poison'), ('assassin','resist.poison')) as v(key, target)
 where n.game_id is null
   and n.key = v.key
   and not exists (
     select 1 from jsonb_array_elements(n.grants) g where g->>'target' = v.target);

-- --- and four plain numbers -------------------------------------------

update public.npcs set hp_max = 178 where game_id is null and key = 'horned_devil';
update public.npcs set ac     = 11  where game_id is null and key = 'werewolf';
update public.npcs set ac     = 10  where game_id is null and key = 'wereboar';
update public.npcs set dex    = 11  where game_id is null and key = 'giant_scorpion';
