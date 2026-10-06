-- 144. THE ONES THAT NEVER HAD IT RECORDED AT ALL.
--
-- 143 added the `.nonmagical` qualifier and fixed the 21 creatures whose
-- physical resistance was written down without its condition. It left a
-- second, quieter fault alone and said so: several creatures that have
-- this resistance in the book do not have it in the database at any
-- strength. Dave asked for the Wraith, the Specter and the Shadow.
--
-- THEY ARE NOT THE ONLY THREE, and four of the others are mine. 142
-- gave the Vrock, Hezrou, Glabrezu and Nalfeshnee their elemental
-- resistances and stopped - every demon in the SRD also resists
-- bludgeoning, piercing and slashing from nonmagical attacks, and I
-- wrote the first half of each line yesterday. Fixing the three asked
-- for and leaving four I broke the day before would be a strange place
-- to stop.
--
-- ---------------------------------------------------------------------
-- SEVENTEEN, AND WHY EACH ONE
-- ---------------------------------------------------------------------
--
--   Wraith, Specter, Shadow      the three asked for - incorporeal
--                                undead, and a sword passes through
--   Vrock, Hezrou, Glabrezu,     142's demons, written with their
--     Nalfeshnee, Dretch         elements and not their hides
--   Mummy, Wight                 131's undead; the Wight's is "that
--                                aren't silvered" in the book
--   Gargoyle, Helmed Horror      131's constructs; the Gargoyle's is
--                                "that aren't adamantine"
--   Imp, Quasit, Succubus        131's lesser fiends
--   Deva                         a celestial, and the same line
--   Grick                        the one monstrosity with it
--
-- NOT ADDED, and each for a reason rather than for want of checking:
--
--   Ghast, Ghoul                 no physical resistance in the book -
--                                they are undead that can simply be hit
--   Animated Armor, Flying       constructs with poison and psychic
--     Sword, Scarecrow           immunity and nothing else
--   Lemure                       the weakest devil has fire and poison
--                                immunity and cold resistance, and is
--                                otherwise exactly as stabbable as it
--                                looks
--
-- ---------------------------------------------------------------------
-- ONE FACT PER MIGRATION
-- ---------------------------------------------------------------------
--
-- This adds the physical resistance and nothing else, even where
-- something else is also missing - the Specter should be IMMUNE to
-- necrotic and currently is not. That is a different fact with a
-- different way of being wrong, and bundling it here would mean a
-- migration whose name stops describing what it did. STATUS carries it.
--
-- SRD-DERIVED FROM MEMORY, like everything in this bestiary, and these
-- are lines I am confident about: "bludgeoning, piercing, and slashing
-- from nonmagical attacks" is the single most repeated sentence in the
-- book. The qualifications some of them carry - silvered, adamantine -
-- remain inexpressible and make the resistance apply MORE often than
-- recorded here, so this stays on the honest side of the printed rule.
--
-- THE GUARD MAKES IT IDEMPOTENT and also makes it safe: a creature that
-- somehow already has a slashing grant is skipped rather than given a
-- second one, because two sources of one resistance is still half and a
-- duplicate would just make the sheet say it twice.

update public.npcs
   set grants = grants || jsonb_build_array(
     jsonb_build_object('source', name, 'target', 'resist.bludgeoning.nonmagical'),
     jsonb_build_object('source', name, 'target', 'resist.piercing.nonmagical'),
     jsonb_build_object('source', name, 'target', 'resist.slashing.nonmagical'))
 where game_id is null
   and key in (
     'wraith','specter','shadow',
     'vrock','hezrou','glabrezu','nalfeshnee','dretch',
     'mummy','wight',
     'gargoyle','helmed_horror',
     'imp','quasit','succubus',
     'deva',
     'grick'
   )
   and not exists (
     select 1 from jsonb_array_elements(public.npcs.grants) g
      where g->>'target' like 'resist.slashing%'
   );
