-- 101. A SPELL CATALOGUE RATHER THAN ONE CHARACTER'S LIST.
--
-- 006 seeded 24 spells out of the AppSheet export and its own header
-- said what they were: ported AS-IS with one character's numbers baked
-- in - spell_atk +7, DC 15, Cure Wounds at 2d8+4. Flagged BAKED in the
-- column comments and never read by anything since.
--
-- That is a sheet, not a catalogue. Three things make it one:
--
--   THE FIELDS A SPELL ACTUALLY HAS. Casting time and components were
--   missing outright, which is half of what a player needs at the
--   table and all of what a ritual or a material cost depends on.
--
--   WHO CAN CAST IT. `classes` is what makes "the cleric list" a query
--   rather than a table of its own - and means the wizard list, when
--   it is asked for, is a seed and not a schema.
--
--   THE BAKED NUMBERS GO. `spell_atk` and `dc` are a property of the
--   CASTER, not the spell: 8 + proficiency + Wisdom, worked out per
--   character. Leaving a 15 in the catalogue would have every cleric
--   in every game rolling against one long-gone character's Wisdom.
--
-- THE UPCAST DUPLICATES GO TOO. "Cure Wounds II" and "III" are the
-- same spell cast with a bigger slot, which is 5e's own rule and not
-- three spells. The export had no way to say that; this does, so they
-- are one row and the slot level decides the dice.
--
-- NOTHING READS THIS TABLE YET, which is what makes the cleanup safe -
-- verified across the Rust and the JavaScript before touching it.

alter table spells
  add column if not exists casting_time text,
  add column if not exists components text[] not null default '{}',
  add column if not exists material text,
  add column if not exists classes text[] not null default '{}';

comment on column spells.casting_time is
  'How long it takes to cast: "1 action", "1 bonus action", "1 reaction", "10 minutes", "1 hour". Text because 5e''s own are text - a reaction carries its trigger and an hour-long ritual is not comparable to an action.';
comment on column spells.components is
  'v, s, m - verbal, somatic, material. An array because a spell has any combination, and because "has a material component" is the question a silenced or bound caster asks.';
comment on column spells.material is
  'What the material component IS, when there is one. The cost matters: a component the spell consumes, or one worth gold pieces, is a thing a character has to actually own.';
comment on column spells.classes is
  'Which classes have this on their list, by classes.key. THIS IS WHAT MAKES A CLASS LIST A QUERY rather than a table of its own - the cleric list is `classes @> {cleric}`, and the wizard list is a seed rather than a schema when it is asked for. A spell can be on several.';

create index if not exists spells_by_class on spells using gin (classes);

-- THE BAKED NUMBERS, CLEARED. A save DC and a spell attack belong to
-- the caster - 8 + proficiency + Wisdom - and 006 said so when it
-- stored them anyway. A catalogue carrying one is a catalogue that
-- makes every cleric roll against a character who left the campaign.
update spells set spell_atk = null, dc = null where game_id is null;

-- THE UPCASTS, FOLDED BACK. One spell, cast with a bigger slot.
delete from spells
 where game_id is null
   and key in ('sp_curewounds2', 'sp_curewounds3', 'sp_prayerofhealing2');