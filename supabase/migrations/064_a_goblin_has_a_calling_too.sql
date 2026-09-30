-- =====================================================================
-- 064_a_goblin_has_a_calling_too.sql
-- odyssey1e — NPCs get a class, and it is not much to designate
-- =====================================================================
--
-- 055 gave a CHARACTER a class and stopped at the player side. Every
-- monster in the game has `class_key` NULL, which means
-- `Effective::attacks` hands them one swing and `class::attacks_at` is
-- never asked. Dave's rule is simple: most combat types are fighters or
-- rogues, the shop keeper is a rogue, the bar keeper is a bard.
--
-- ---------------------------------------------------------------------
-- THERE WAS ALREADY A `class`, AND IT IS PROSE
-- ---------------------------------------------------------------------
--
-- `npcs.class` has existed since 022 and holds "Fighter" on the Goblin
-- Fighter and NULL on the Goblin. It is a DESCRIPTOR - what a DM typed
-- on the statblock form - and nothing has ever read it.
--
-- `class_key` is the reference: a value in `classes.key`, by value and
-- with no foreign key, for 004's reason. The text stays because a
-- statblock may want to say "Chieftain" or "Hedge-witch" where no class
-- in the catalogue fits, and losing that would be losing something a DM
-- wrote. The key is the rule; the text is the label. Backfilled from
-- the text where the two agree, so the one statblock that said Fighter
-- now IS one.
--
-- ---------------------------------------------------------------------
-- A CLASS DOES NOT CHANGE A MONSTER'S HIT POINTS
-- ---------------------------------------------------------------------
--
-- This is the line that matters and it is easy to cross by accident.
--
-- 029 settled that a monster's hit points come from LEVEL AND SIZE - a
-- goblin is 2d6 because the Monster Manual says 7 (2d6), and that is
-- what makes "set level" a button rather than a rewrite. 061's
-- `pc_hp` is the other rule, for characters, and it is not this one.
--
-- Giving a goblin a class puts a `class_key` on their row, and
-- `rederive_hp_max` recomputes from class for anything that has one.
-- Left alone, the Goblin Scout would go from 28 hit points to 43 the
-- first time anybody touched their level - silently, and with nothing
-- on screen to explain it. The Rust guard goes in with this migration:
-- an NPC keeps 029's rule.
--
-- WHAT A CLASS GIVES A MONSTER, then, is what it is for here: how many
-- attacks their Attack action buys, and a name for what they are. Both
-- goblin statblocks come out at one swing today, because a Rogue gains
-- none and a level 1 Fighter has not reached five - so nothing about
-- the current fights changes, which is the right way for a sweep to
-- land.
--
-- ---------------------------------------------------------------------
-- WHY THE PLAIN GOBLIN IS A ROGUE
-- ---------------------------------------------------------------------
--
-- Dave's rule allows either. A 5e goblin's signature is Nimble Escape -
-- Disengage or Hide as a bonus action - which is Cunning Action wearing
-- a different name, and Cunning Action is the Rogue's. A goblin is a
-- skirmisher rather than a line fighter, so Rogue is the closer read.
-- The Goblin Fighter statblock says Fighter on its own row and stays
-- one.
--
-- NEITHER GRANTS A CLASS FEATURE, because 055 deliberately has none. A
-- Rogue goblin gets no Sneak Attack and no Expertise from this; what
-- they get is an identity and an attack count.
--
-- ---------------------------------------------------------------------
-- PROFICIENCIES ARE FILLED, NEVER OVERWRITTEN
-- ---------------------------------------------------------------------
--
-- 028 gave NPCs weapon and armour proficiencies and a DM has been
-- authoring them since. A sweep that replaced them with the class's
-- would throw that away - Goblin Fighter 0004 carries `{shortsword}`
-- because somebody put it there.
--
-- So an EMPTY list is filled from the class and a non-empty one is
-- left alone. Empty is the case worth fixing: 028's whole point was
-- that an empty list reads as proficient with nothing, and three
-- Goblin Fighters have been carrying one.
-- =====================================================================

alter table public.npcs
  add column if not exists class_key text;

comment on column public.npcs.class_key is
  'Which class, by value into classes.key - no FK, for 004''s reason. THE RULE; `class` beside it is the DESCRIPTOR a DM typed and may say something no catalogue class covers. A class gives a monster their attack count and a name, NOT their hit points - those stay 029''s level-times-size.';

create index if not exists npcs_class_idx
  on public.npcs (class_key) where class_key is not null;

-- ---------------------------------------------------------------------
-- THE STATBLOCKS
-- ---------------------------------------------------------------------

-- From the descriptor, where it names a class that exists.
update public.npcs n
   set class_key = c.key
  from public.classes c
 where n.class_key is null
   and c.game_id is null
   and lower(btrim(n.class)) = c.key;

-- And the one the descriptor never covered. See the header for why a
-- goblin reads as a skirmisher rather than a line fighter.
update public.npcs
   set class_key = 'rogue'
 where class_key is null and lower(name) like '%goblin%' and lower(coalesce(class,'')) not like '%fighter%';

-- ---------------------------------------------------------------------
-- THE INSTANCES
-- ---------------------------------------------------------------------
--
-- 022 made an instance a full character, so the class goes on the
-- character row where `Effective::attacks` will find it. Matched on the
-- statblock through `encounter_actors.npc_key` where there is one, and
-- on the name where there is not - four Goblin 0003 rows were made
-- outside an encounter and have no actor to ask.

update public.characters c
   set class_key = n.class_key
  from public.encounter_actors a
  join public.npcs n on n.key = a.npc_key
 where a.character_id = c.id
   and c.is_npc = true
   and c.class_key is null
   and n.class_key is not null;

update public.characters
   set class_key = case
     when lower(name) like 'goblin fighter%' then 'fighter'
     else 'rogue'
   end
 where is_npc = true and class_key is null;

-- ---------------------------------------------------------------------
-- THE EMPTY PROFICIENCY LISTS
-- ---------------------------------------------------------------------
--
-- Filled only where there is nothing there. 028's fault, not a
-- preference: an empty list reads as proficient with nothing, and
-- three Goblin Fighters have been swinging at a penalty for it.

update public.characters c
   set weapon_profs = cl.weapon_profs
  from public.classes cl
 where c.is_npc = true
   and cl.key = c.class_key and cl.game_id is null
   and coalesce(cardinality(c.weapon_profs), 0) = 0
   and cardinality(cl.weapon_profs) > 0;

update public.characters c
   set armor_profs = cl.armor_profs
  from public.classes cl
 where c.is_npc = true
   and cl.key = c.class_key and cl.game_id is null
   and coalesce(cardinality(c.armor_profs), 0) = 0
   and cardinality(cl.armor_profs) > 0;

update public.npcs n
   set weapon_profs = cl.weapon_profs
  from public.classes cl
 where cl.key = n.class_key and cl.game_id is null
   and coalesce(cardinality(n.weapon_profs), 0) = 0
   and cardinality(cl.weapon_profs) > 0;

update public.npcs n
   set armor_profs = cl.armor_profs
  from public.classes cl
 where cl.key = n.class_key and cl.game_id is null
   and coalesce(cardinality(n.armor_profs), 0) = 0
   and cardinality(cl.armor_profs) > 0;
