-- =====================================================================
-- 029_level_is_hit_dice.sql
-- odyssey1e — a goblin is 2d6, so a goblin is level 2
-- =====================================================================
--
-- THE DECISION. A monster's level IS its hit dice count. The Monster
-- Manual has always written it that way - `Hit Points 7 (2d6)` - and
-- this stops treating the bracket as trivia.
--
-- WHAT IT REPLACES. `npcs.hp_max` was a magic number: 7, because the
-- book says 7. Nothing could check it, nothing could move it, and a DM
-- who wanted a tougher goblin had to write a second statblock. Level
-- times a die is a number that can be RECOMPUTED, which is the whole
-- difference between "set level" being a button and being a rewrite.
--
-- NOTHING STRUCTURAL CHANGES HERE, and that is worth saying plainly.
-- The columns already existed - `level` since 019, `size` since 011 -
-- and the die comes from size, which the schema has carried since 010.
-- What was missing was the RULE connecting them, and a rule lives in
-- Rust. See `vitality.rs`, whose tests check five printed Monster
-- Manual lines spanning d6 to d20 - the one kind of test here that can
-- be falsified by anyone holding the book.
--
-- THE GOBLIN'S HIT POINTS DO NOT MOVE. It was level 1 with a stated 7;
-- it becomes level 2, and 2d6 averages to exactly 7. The number is the
-- same and it has stopped being arbitrary, which is the only thing this
-- migration is for.
--
-- PROFICIENCY BONUS STAYS STATED, and the divergence is deliberate. The
-- book rates a monster by CHALLENGE, not by hit dice, and the two come
-- apart fast: an ogre is 7 hit dice and CR 2, so the book gives it +2
-- where level would derive +3. 022 made `prof_bonus` nullable for
-- exactly this, and a statblock copied out of the book keeps the book's
-- answer. What `set_actor_level` does is CLEAR that override on the
-- individual it levels - at which point the creature is no longer the
-- monster the book rated, and deriving is the honest reading.
--
-- WHAT DOES NOT RIPPLE, said out loud so nobody assumes it does:
--
--   Skills. A level change moves the proficiency bonus and every skill
--   built on it follows for free - but `npcs` carries no skill
--   proficiencies at all, so today there is nothing riding on it.
--   Giving statblocks skills is its own piece of work.
--
--   Special abilities. There is no table. A goblin's Nimble Escape does
--   not exist anywhere in this schema, so levelling one cannot scale
--   something the database has never heard of. That is a subsystem, not
--   a ripple, and pretending otherwise would be the worse answer.
-- =====================================================================

comment on column public.npcs.level is
  'HIT DICE. A goblin is 2d6 and therefore level 2 - see 029. The die itself comes from `size`, which is why the Monster Manual never states it separately, and hit points are level x the die average plus Constitution PER DIE. The rule is vitality.rs; this column is the input it reads.';

comment on column public.npcs.size is
  'tiny sm med lg huge grg, and it decides the HIT DIE: d4 d6 d8 d10 d12 d20. NULL means the die is unknown and hit points cannot be derived, which is a real state - statblocks written through the DM panel before 029 have no size - and it is reported rather than guessed. Guessing d8 would quietly give every sizeless statblock a medium creature''s hit points.';

comment on column public.characters.level is
  'For a player character, character level. For a monster, HIT DICE - 029 made those the same fact seen from two sides, which is what lets one hit point rule serve both. set_actor_level moves it on an individual and recomputes hp_max from it.';

-- ---------------------------------------------------------------------
-- The goblin, restated. 2d6 averages to 7, so hp_max does not move.
-- ---------------------------------------------------------------------

update public.npcs
   set level = 2
 where key = 'goblin' and game_id is null;

-- And the goblins already on the table. They were instantiated at the
-- statblock's level 1 and their maximum is already 7, so this is the
-- level catching up with hit points that were always 2d6.
--
-- Matched through the actor, because `characters` carries no npc_key -
-- the same route 028's backfill had to take, and for the same reason.
update public.characters c
   set level = 2
  from public.encounter_actors a
 where a.character_id = c.id
   and c.is_npc
   and a.npc_key = 'goblin'
   and c.level = 1;
