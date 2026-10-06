-- 122. WHAT A THING IS.
--
-- Fifteen spells already in the catalogue name a creature type in their
-- own text and not one of them can be checked, because nothing in this
-- schema records what anything IS:
--
--   Hold Person        works only on a humanoid
--   Cure Wounds        nothing for a construct or the undead
--   Spare the Dying    the same
--   Protection from    names six types outright, and Detect Evil and
--     Evil and Good    Good and Dispel Evil and Good name the same six
--   Gentle Repose      stops a corpse becoming undead
--   Animate Dead       makes one
--
-- All of it has been a DM call for want of one column.
--
-- ---------------------------------------------------------------------
-- A TYPE IS NOT A SPECIES AND NOT A ROLE
-- ---------------------------------------------------------------------
--
-- `species_key` is what people you are - Unt'garoth, Ny'ook - and is
-- this world's own invention. `is_npc` is whether somebody is playing
-- you. TYPE is 5e's fourteen-way classification and it is the one of
-- the three that spells are written against.
--
-- ---------------------------------------------------------------------
-- THE SPECIES NORMALLY ANSWERS
-- ---------------------------------------------------------------------
--
-- Every Ny'ook is the same thing, so the type belongs to the PEOPLE and
-- is set once rather than on every character made from them. The column
-- on `characters` is the exception rather than the rule - one cursed
-- Unt'garoth who is now undead - and is NULL for everybody else.
--
-- THE SAME ARRANGEMENT `carry_size` HAS: the species' answer unless the
-- character states its own. creature.rs::of is that rule, written once
-- and tested.
--
-- A STATBLOCK STATES ITS OWN, because a monster has no species row to
-- ask. 121 gave `characters.npc_key`, so a creature on the board can
-- reach its statblock the same way it reaches its people.
--
-- ---------------------------------------------------------------------
-- NOTHING IS BACKFILLED TO humanoid, AND THAT IS DELIBERATE
-- ---------------------------------------------------------------------
--
-- Every creature in this game had no type at all until now. Defaulting
-- them to humanoid would assert something about this world that its
-- designer has not said - an Unt'garoth is seven to ten feet of dense
-- bone and muscle and whether that is `humanoid` or `giant` is a design
-- decision, not a migration's to make. NULL reads as "nobody has said",
-- which is true, and a rule that needs the type says so rather than
-- guessing.
--
-- The goblins ARE backfilled, because a goblin is a humanoid in 5e and
-- that is the book's answer rather than anyone's opinion.

-- ---------------------------------------------------------------------
-- TEXT AND A CHECK, which is this schema's house style for a closed
-- vocabulary - see characters_size_check, characters_disposition_check,
-- class_features_recharge_check. The three enums that exist are the
-- oldest tables and nothing has used one since.
--
-- THE LIST IS IN TWO PLACES ON PURPOSE. creature.rs owns it for the
-- code and this constraint owns it for everything that never went
-- through the code - the same arrangement character_slots has had since
-- 107, and the reason is the same: a REST write with the publishable
-- key does not pass through Rust.
-- ---------------------------------------------------------------------

alter table public.species
  add column if not exists creature_type text;

alter table public.species
  drop constraint if exists species_creature_type_check;
alter table public.species
  add constraint species_creature_type_check check (
    creature_type is null or creature_type = any (array[
      'aberration','beast','celestial','construct','dragon','elemental',
      'fey','fiend','giant','humanoid','monstrosity','ooze','plant','undead'])
  );

comment on column public.species.creature_type is
  '5e''s fourteen-way classification for this people - normally what answers for every character of it, since every Ny''ook is the same thing. NULL means nobody has said yet, which is not the same as humanoid. See creature.rs for the list and the groupings the spell catalogue names.';

alter table public.characters
  add column if not exists creature_type text;

alter table public.characters
  drop constraint if exists characters_creature_type_check;
alter table public.characters
  add constraint characters_creature_type_check check (
    creature_type is null or creature_type = any (array[
      'aberration','beast','celestial','construct','dragon','elemental',
      'fey','fiend','giant','humanoid','monstrosity','ooze','plant','undead'])
  );

comment on column public.characters.creature_type is
  'This creature''s own type, overriding what its species or statblock would say. NULL for almost everybody - the species answers. The exception this exists for is the individual who stopped being what their people are: one cursed Unt''garoth who is now undead.';

alter table public.npcs
  add column if not exists creature_type text;

alter table public.npcs
  drop constraint if exists npcs_creature_type_check;
alter table public.npcs
  add constraint npcs_creature_type_check check (
    creature_type is null or creature_type = any (array[
      'aberration','beast','celestial','construct','dragon','elemental',
      'fey','fiend','giant','humanoid','monstrosity','ooze','plant','undead'])
  );

comment on column public.npcs.creature_type is
  'What this statblock is, in 5e''s fourteen. A monster has no species row to ask, so it states its own - reached from a creature on the board through characters.npc_key, which 121 added.';

-- THE BOOK'S ANSWER, not an opinion: a goblin is a humanoid.
update public.npcs set creature_type = 'humanoid'
 where creature_type is null
   and lower(coalesce(species, name)) like '%goblin%';
