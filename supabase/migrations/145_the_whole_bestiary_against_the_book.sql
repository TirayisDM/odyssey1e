-- 145. ALL 203, AGAINST THE BOOK.
--
-- 143 qualified the resistances that were written down without their
-- condition. 144 added the ones that were not written down at all, and
-- found seventeen where Dave had asked about three. Twice in two days a
-- hole turned up by looking rather than by playing, which is the signal
-- to stop patching and read the whole thing.
--
-- So: every creature in the shared bestiary, its recorded resistances
-- against what the SRD prints. 203 checked, ELEVEN wrong, and the
-- eleven are listed below with what each one was.
--
-- ---------------------------------------------------------------------
-- WHAT A SWEEP IS GOOD FOR, AND WHAT IT IS NOT
-- ---------------------------------------------------------------------
--
-- This compares the database against MY RECOLLECTION of the SRD, which
-- is the same source that wrote the rows in the first place. It catches
-- what 131 and 142 FORGOT - and the evidence says that is the common
-- failure, since every one of the eleven below is an omission or a
-- degree, not an invention. It cannot catch what I have remembered
-- wrongly in the same way twice.
--
-- A SECOND READER WOULD BE WORTH MORE THAN A THIRD PASS BY ME. The
-- honest version of this check is an SRD file through `import_creature`,
-- which 142's header already argued for.
--
-- ---------------------------------------------------------------------
-- THE ELEVEN
-- ---------------------------------------------------------------------
--
-- MISSING ENTIRELY - the elementals, which is a family-sized hole:
--
--   Air Elemental     b/p/s from nonmagical
--   Fire Elemental    b/p/s from nonmagical
--   Water Elemental   b/p/s from nonmagical
--   Earth Elemental   b/p/s from nonmagical, AND vulnerable to thunder
--
-- Four of the five elementals resist ordinary weapons and not one of
-- them said so. The Gargoyle - also an elemental - was given it in 144,
-- which is how the family came to be looked at.
--
-- MISSING, SINGLY:
--
--   Swarm of Rats     resists b/p/s, FLATLY and not "from nonmagical":
--                     a swarm is not hard to hit, there is simply too
--                     much of it. Swarm of Insects has had it since 131
--                     and the rats were written beside them without it
--   Shield Guardian   immune to poison
--   Dust Mephit       vulnerable to fire - the other three mephits all
--                     carry their vulnerability and this one did not
--   Ghast             resistant to necrotic
--   Specter           IMMUNE to necrotic - the gap 144 found and
--                     deliberately did not fix, because 144 was about
--                     one fact
--
-- RECORDED AT THE WRONG DEGREE, which is the subtler kind:
--
--   Wraith            necrotic is IMMUNITY, not resistance
--   Shadow            the same
--
-- Half of a large necrotic hit still kills somebody and none of it
-- never does, so this is not a rounding difference - it is the
-- difference between a Wraith that can be worn down by the party's
-- necromancer and one that cannot be touched by them at all.
--
-- ---------------------------------------------------------------------
-- WHAT WAS CHECKED AND IS RIGHT
-- ---------------------------------------------------------------------
--
-- Worth recording, because "I looked and it was fine" is a result:
-- every dragon's element, all four golems, the three remaining mephits,
-- both other oozes, the plants (the Treant and Awakened Tree resist
-- bludgeoning and piercing FLATLY - a magic axe is no better against
-- wood), the Banshee and the Ghost (whose long lines are exactly
-- right), the Lich, both vampires, the Wight, the Mummy, every
-- lycanthrope, all eleven fiends, and the 40-odd beasts and humanoids
-- that correctly have nothing at all.
--
-- ONE LEFT ALONE FOR WANT OF CONFIDENCE: the Gas Spore. I do not trust
-- my memory of its line well enough to write it, and a guess here would
-- be indistinguishable from the eleven above.

-- --- the missing ones, added ------------------------------------------
--
-- IDEMPOTENT BY FILTERING THE TARGETS, not by guarding the row: a
-- creature that already has one of these keeps the one it has and gains
-- only what it lacks. Two sources of one resistance is still half, and a
-- duplicate would only make the sheet say it twice.

update public.npcs n
   set grants = n.grants || coalesce((
         select jsonb_agg(jsonb_build_object('source', n.name, 'target', t))
           from unnest(v.targets) as t
          where not exists (
            select 1 from jsonb_array_elements(n.grants) g where g->>'target' = t)
       ), '[]'::jsonb)
  from (values
    ('air_elemental',   array['resist.bludgeoning.nonmagical','resist.piercing.nonmagical','resist.slashing.nonmagical']),
    ('fire_elemental',  array['resist.bludgeoning.nonmagical','resist.piercing.nonmagical','resist.slashing.nonmagical']),
    ('water_elemental', array['resist.bludgeoning.nonmagical','resist.piercing.nonmagical','resist.slashing.nonmagical']),
    ('earth_elemental', array['resist.bludgeoning.nonmagical','resist.piercing.nonmagical','resist.slashing.nonmagical','vulnerable.thunder']),
    ('swarm_of_rats',   array['resist.bludgeoning','resist.piercing','resist.slashing']),
    ('shield_guardian', array['immune.poison']),
    ('dust_mephit',     array['vulnerable.fire']),
    ('ghast',           array['resist.necrotic']),
    ('specter',         array['immune.necrotic'])
  ) as v(key, targets)
 where n.game_id is null
   and n.key = v.key;

-- --- and the two that were the wrong degree ---------------------------
--
-- Rewritten in place rather than removed and re-added, so the grant
-- keeps its position in the list and the sheet does not reorder itself
-- for a reason nobody can see.

update public.npcs
   set grants = (
     select jsonb_agg(
       case when g->>'target' = 'resist.necrotic'
            then jsonb_set(g, '{target}', to_jsonb('immune.necrotic'::text))
            else g
       end
       order by ordinality)
     from jsonb_array_elements(public.npcs.grants) with ordinality as t(g, ordinality)
   )
 where game_id is null
   and key in ('wraith', 'shadow');
