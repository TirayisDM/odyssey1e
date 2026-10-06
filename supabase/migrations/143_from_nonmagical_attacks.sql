-- 143. "FROM NONMAGICAL ATTACKS", WHICH IS THE REST OF THE RULE.
--
-- The commonest resistance in 5e is "bludgeoning, piercing and slashing
-- FROM NONMAGICAL ATTACKS" - every lycanthrope, both vampires, all four
-- golems, most fiends, the lich, the ghost. 116 gave this schema
-- `resist.bludgeoning` and no way to say the second half, so 131 and 142
-- recorded the resistance and dropped the qualifier.
--
-- THAT IS BACKWARDS, AND NOT BY A LITTLE. The qualifier IS the rule: it
-- is the entire reason a magic sword is worth carrying, and without it a
-- party that finally found one watched it get halved by a werewolf
-- exactly like the stick they started with. A quiet, plausible, wrong
-- number - which is the defect this codebase is named after, arriving in
-- the one place a player was supposed to feel rewarded.
--
-- ---------------------------------------------------------------------
-- A SUFFIX, NOT A FOURTH DEGREE
-- ---------------------------------------------------------------------
--
-- `resist.slashing.nonmagical`, and the three-type form that half the
-- Monster Manual wants in one target:
--
--   resist.bludgeoning|piercing|slashing.nonmagical
--
-- "Resistant" and "resistant to nonmagical" are the same degree under a
-- condition rather than two degrees, so `resist::Degree` stays the three
-- 5e has and everything that reasons about halving and doubling is
-- untouched. `resist.rs` reads the suffix, `attack::Attack::magical`
-- says whether the weapon that swung was enchanted, and `resist::applies`
-- is the one place the two meet.
--
-- WHAT COUNTS AS MAGICAL is the weapon's own grants - 100 made an
-- enchantment a list of grants, so a weapon carrying one is enchanted.
-- A ring that grants +1 to attack does NOT make the sword magical; it
-- makes the swing better. The gap left is a weapon that is magical and
-- grants nothing, which has nowhere to say so today and wants a flag of
-- its own when something in the catalogue needs one.
--
-- ---------------------------------------------------------------------
-- TWENTY-ONE CREATURES, AND NOT THE OTHERS
-- ---------------------------------------------------------------------
--
-- Only the ones whose SRD text carries the qualifier. The ones left
-- alone are left alone on purpose, and the distinction is the point of
-- doing this at all:
--
--   Treant, Awakened Tree    resist bludgeoning and piercing, flatly -
--                            a magic axe is no better against wood
--   Swarm of Insects         resist all three, flatly - there is
--                            nothing to hit, enchanted or not
--   Black Pudding, Ochre     immune to slashing, flatly - cutting it
--     Jelly                  in half makes two of it
--   Skeletons, Ice Mephit    VULNERABLE to bludgeoning, which the
--                            qualifier has nothing to do with
--
-- The golems and devils carry further qualifications in the book -
-- "that aren't adamantine", "that aren't silvered" - and those are
-- still not expressible. They are narrower than this one and they make
-- the resistance apply MORE often, so recording the nonmagical half is
-- a move toward the printed rule rather than away from it. Noted rather
-- than pretended about.

update public.npcs
   set grants = (
     select jsonb_agg(
       case
         when g->>'target' in ('resist.bludgeoning','resist.piercing','resist.slashing')
         then jsonb_set(g, '{target}', to_jsonb((g->>'target') || '.nonmagical'))
         else g
       end
       order by ordinality)
     from jsonb_array_elements(public.npcs.grants) with ordinality as t(g, ordinality)
   )
 where game_id is null
   and key in (
     -- undead
     'lich','vampire','vampire_spawn','ghost','banshee',
     -- fiends
     'balor','marilith','bone_devil','erinyes','horned_devil','pit_fiend',
     -- constructs
     'flesh_golem','clay_golem','stone_golem','iron_golem',
     -- lycanthropes
     'wererat','werewolf','wereboar','weretiger','werebear',
     -- and 131's, which has carried the unqualified form the longest
     'intellect_devourer'
   );

-- ---------------------------------------------------------------------
-- WHAT THIS MIGRATION KNOWS IT IS NOT FIXING
-- ---------------------------------------------------------------------
--
-- Several creatures that SHOULD have this resistance do not have it
-- recorded at all - the Wraith, the Specter and the Shadow among them,
-- which 131 wrote with their elemental resistances and no physical ones.
-- Adding a resistance is a different act from qualifying one that is
-- already there, with a different way of being wrong, so it is not
-- bundled in here. STATUS carries it.
